#!/usr/bin/env python3
"""M3 model lifecycle: real pinned transports; fixtures only for fault injection.

No inference is spent here: live-binary probes use unavailable-model argv
that fails before dispatch, and container wiring uses a deterministic fake
Docker transport that can never certify isolation. Live OpenCode/Codex and
container certification is M5.
"""
from __future__ import annotations
import copy
import hashlib
import json
import os
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'scripts'))
from model_orchestrator import contracts as c, adapters as a, orchestrator as o
from model_orchestrator import store as s, sandbox as b, workspace as w, guards as g
from model_orchestrator.tests.test_contracts import shared_provenance, valid_task, SCHEMAS

OPENCODE_BIN = Path(os.environ.get('OR_V2_OPENCODE_BIN', '/opt/homebrew/bin/opencode'))
_BINARY = None


def opencode_binary():
    global _BINARY
    if _BINARY is None:
        _BINARY = a.OpenCodeBinary(OPENCODE_BIN,
                                   os.environ.get('OR_V2_OPENCODE_SHA', a.OPENCODE_SHA256),
                                   os.environ.get('OR_V2_OPENCODE_VERSION', a.OPENCODE_VERSION))
    return _BINARY


def seal_enrollment(record):
    """Fault-injection enrollment under real independent Git authority; no production certification."""
    _, authority, task = m3_task()
    record = dict(record)
    record.pop('reasoning_requested', None)
    record['operator_adoption_identity'] = c._release_authority(authority)['build']['authorization_id']
    digest = c.canonical_digest(record)
    task['role_enrollment_ids'] = [digest]
    return a.load_enrollment(record, authority=authority, task=task, expected_digest=digest)


def make_enrollment(model_id, family, roles, reasoning=('LOW', 'HIGH'), qualify=None, variants=None, price=0):
    qualified_roles = qualify if qualify is not None else roles
    return seal_enrollment({
        'provider_id': model_id.split('/')[0], 'model_id': model_id, 'family': family,
        'allowed_roles': list(roles), 'reasoning_capabilities': list(reasoning),
        'task_class_qualification': {r: True for r in qualified_roles},
        'adapter_certification_digest': opencode_binary().certification_digest(),
        'budget': {}, 'variants': dict(variants or {}),
        'qualification_evidence': [dict(role=r, task_class=r, reasoning_efforts=list(reasoning),
              quality_passed=True, scope_compliance=True, tool_use_correct=True,
              evidence_digest=c.canonical_digest(['fault-injection-only', model_id, r])) for r in qualified_roles],
        'availability_observation': dict(state='available', quota_remaining=100, quota_scarce=False),
        'pricing_observation': dict(kind='free' if price == 0 else 'metered',
              effort_cost_microusd={r: price for r in reasoning}, retry_cost_microusd=0,
              budget_pressure_microusd=0)})


def worker_enrollment(**kw):
    args = dict(model_id='fixture-provider/fixture-worker', family='fixture-a',
                roles=['IMPLEMENTATION'], reasoning=['HIGH'])
    args.update(kw)
    return make_enrollment(**args)


def reviewer_enrollment(**kw):
    args = dict(model_id='fixture-provider/fixture-reviewer', family='fixture-b',
                roles=['INVESTIGATION_REVIEW'], reasoning=['HIGH'])
    args.update(kw)
    return make_enrollment(**args)


def m3_task():
    fixture = shared_provenance()
    authority = fixture.authorities['M3']
    payload = c._release_authority(authority)
    build = payload['build']
    task = valid_task()
    task.update(task_id=build['task_id'], checkpoint_id='M3', base_sha=build['base_sha'],
                candidate_branch=build['candidate_branch'],
                authority_digest=payload['git']['authority_digest'])
    return fixture, authority, task


class SelectionTests(unittest.TestCase):
    """CP19: role-qualified selection reaches argv; locks never bypass enrollment."""
    @classmethod
    def setUpClass(cls):
        cls.binary = opencode_binary()
        cls.worker = worker_enrollment()
        cls.reviewer = reviewer_enrollment()

    def test_cheapest_qualified_worker_selected(self):
        expensive = worker_enrollment(model_id='fixture-provider/fixture-pro', price=500)
        availability = {e['model_id']: 'available' for e in (expensive, self.worker)}
        chosen = o.select_worker(valid_task(), [expensive, self.worker], availability=availability)
        self.assertEqual(chosen['model_id'], self.worker['model_id'])
        self.assertEqual(chosen['reasoning_requested'], 'HIGH')

    def test_locked_model_honored_and_unavailable_lock_is_not_failure(self):
        availability = {self.worker['model_id']: 'available'}
        chosen = o.select_worker(valid_task(), [self.worker], availability=availability,
                                 locked_model=self.worker['model_id'])
        self.assertEqual(chosen['model_id'], self.worker['model_id'])
        with self.assertRaises(o.OrchestratorError) as ctx:
            o.select_worker(valid_task(), [self.worker], availability={self.worker['model_id']: 'unavailable'}, locked_model=self.worker['model_id'])
        self.assertEqual(ctx.exception.code, 'UNAVAILABLE')
        with self.assertRaises(a.AdapterError):
            a.select_model('IMPLEMENTATION', [self.worker], task_budget={}, required_reasoning='HIGH',
                           locked_model='other/model', availability=availability)

    def test_packet_preference_cannot_bypass_enrollment(self):
        partial = worker_enrollment(roles=['IMPLEMENTATION', 'MECHANICAL'], qualify=['MECHANICAL'])
        with self.assertRaises(a.AdapterError):
            a.select_model('IMPLEMENTATION', [partial], task_budget={}, required_reasoning='HIGH',
                           locked_model=partial['model_id'], availability={partial['model_id']: 'available'})
        with self.assertRaises(a.AdapterError):
            a.select_model('IMPLEMENTATION', ['opencode/foo-free'], task_budget={}, availability={})
        with self.assertRaises(a.AdapterError):
            a.validate_enrollment(dict(self.worker, family='unknown'))
        with self.assertRaises(a.AdapterError):
            a.validate_enrollment(dict(self.worker, task_class_qualification={}))

    def test_binary_pin_mismatch_refuses(self):
        with self.assertRaises(a.AdapterError):
            a.OpenCodeBinary(OPENCODE_BIN, '0' * 64, a.OPENCODE_VERSION)
        with self.assertRaises(a.AdapterError):
            a.OpenCodeBinary(OPENCODE_BIN, a.OPENCODE_SHA256, '0.0.0')
        with self.assertRaises(a.AdapterError):
            a.OpenCodeBinary(Path('/nonexistent/opencode'), '0' * 64, '0.0.0')

    def test_selected_model_reaches_actual_argv(self):
        # Real pinned process with unavailable-model argv: fails before any
        # dispatch, so no inference is spent and the argv reach is observed.
        enrollment = worker_enrollment(model_id='no-such-provider/no-such-model', family='probe-unavailable')
        with tempfile.TemporaryDirectory() as home:
            adapter = a.OpenCodeAdapter(self.binary, role='IMPLEMENTATION',
                                        enrollment=enrollment.with_reasoning('HIGH'),
                                        workdir=Path(home), limits=a.StreamLimits(1 << 20, 64, 120),
                                        env={'PATH': os.environ.get('PATH', '/usr/bin:/bin'), 'LANG': 'C', 'HOME': home})
            self.assertNotIn('--variant', adapter.build_argv(['hi'], agent='orch-worker'))
            observation = adapter.run(['say ok'], agent='orch-worker', timeout_seconds=60)
        self.assertIn('--model', observation.argv)
        self.assertEqual(observation.argv[observation.argv.index('--model') + 1], enrollment['model_id'])
        self.assertTrue(observation.argv_digest)
        self.assertIsNotNone(observation.exit_code)
        self.assertNotEqual(observation.exit_code, 0)
        self.assertFalse(observation.timed_out)
        self.assertIsNotNone(observation.session_id)
        self.assertIsNotNone(observation.error)
        self.assertEqual(observation.error['type'], 'error')
        self.assertIsNone(observation.final_payload)
        self.assertIsNone(observation.reasoning_sent)
        self.assertEqual(observation.reasoning_confirmed, 'DEFAULT_PROVIDER')
        self.assertEqual((observation.model_id, observation.provider_id, observation.family),
                         (enrollment['model_id'], 'no-such-provider', 'probe-unavailable'))

    def test_pinned_variant_reaches_argv(self):
        enrollment = worker_enrollment(variants={'HIGH': 'max'})
        adapter = a.OpenCodeAdapter(self.binary, role='IMPLEMENTATION',
                                    enrollment=enrollment.with_reasoning('HIGH'),
                                    workdir=Path('/candidate'), limits=a.StreamLimits(1 << 20, 64, 120))
        argv = adapter.build_argv(['hi'])
        self.assertEqual(argv[argv.index('--variant') + 1], 'max')
        self.assertEqual((adapter.requested, adapter.sent, adapter.confirmed), ('HIGH', 'max', 'UNCONFIRMED'))


class RoutingObservationTests(unittest.TestCase):
    """Real pinned authority plus deterministic provider observations, no inference."""
    def enrollment(self, *, model='fixture/model', price=5, efforts=('LOW', 'MEDIUM', 'HIGH', 'XHIGH', 'MAX'), **updates):
        record = dict(make_enrollment(model, 'family-' + model.split('/')[-1], ['IMPLEMENTATION'],
                                      reasoning=efforts, price=price))
        record.update(updates)
        return seal_enrollment(record)

    def choose(self, enrollments, *, floor='LOW', budget=100, **kw):
        return a.select_model('IMPLEMENTATION', enrollments, task_budget={'cost_microusd': budget},
                              required_reasoning=floor, **kw)

    def test_unknown_price_and_missing_price_never_mean_free(self):
        unknown = self.enrollment(pricing_observation=dict(kind='unknown', effort_cost_microusd={},
                         retry_cost_microusd=0, budget_pressure_microusd=0))
        paid = self.enrollment(model='fixture/paid')
        self.assertEqual(self.choose([unknown, paid])['model_id'], paid['model_id'])
        with self.assertRaises(a.AdapterError):
            self.choose([unknown])
        raw = dict(paid)
        del raw['pricing_observation']
        with self.assertRaises(a.AdapterError):
            seal_enrollment(raw)

    def test_over_budget_and_locked_over_budget_refuse(self):
        paid = self.enrollment(price=101)
        for lock in (None, paid['model_id']):
            with self.subTest(lock=lock), self.assertRaises(a.AdapterError) as ctx:
                self.choose([paid], locked_model=lock)
            self.assertEqual(ctx.exception.code, a.MODEL_UNAVAILABLE)

    def test_free_qualified_prefers_highest_effort_and_unqualified_cannot_win(self):
        free = self.enrollment(model='fixture/free', price=0)
        paid = self.enrollment(model='fixture/paid')
        selected = self.choose([paid, free])
        self.assertEqual((selected['model_id'], selected['reasoning_requested']), ('fixture/free', 'MAX'))
        raw = dict(free)
        raw['qualification_evidence'][0]['quality_passed'] = False
        unqualified = seal_enrollment(raw)
        self.assertEqual(self.choose([unqualified, paid])['model_id'], paid['model_id'])

    def test_unavailable_and_exhausted_quota_are_not_selectable(self):
        other = self.enrollment(model='fixture/other', price=10)
        for state, quota in (('unavailable', 100), ('quota_exhausted', 0), ('rate_limited', 100), ('available', 0)):
            cheap = self.enrollment(price=0, availability_observation=dict(state=state, quota_remaining=quota, quota_scarce=False))
            with self.subTest(state=state, quota=quota):
                self.assertEqual(self.choose([cheap, other])['model_id'], other['model_id'])
        cheap = self.enrollment(price=0, availability_observation=dict(state='unavailable', quota_remaining=100, quota_scarce=False))
        with self.assertRaises(a.AdapterError):
            self.choose([cheap], availability={cheap['model_id']: 'available'})

    def test_prepaid_capacity_and_changed_pressure_change_ranking(self):
        metered = self.enrollment(model='fixture/metered', price=3)
        prepaid = self.enrollment(model='fixture/prepaid', pricing_observation=dict(kind='prepaid',
                     effort_cost_microusd={e: 0 for e in ('LOW', 'MEDIUM', 'HIGH', 'XHIGH', 'MAX')},
                     retry_cost_microusd=0, budget_pressure_microusd=0))
        self.assertEqual(self.choose([metered, prepaid])['model_id'], prepaid['model_id'])
        raw = dict(prepaid)
        raw['pricing_observation']['budget_pressure_microusd'] = 20
        pressured = seal_enrollment(raw)
        self.assertEqual(self.choose([metered, pressured])['model_id'], metered['model_id'])
        self.assertNotEqual(pressured.enrollment_digest, prepaid.enrollment_digest)

    def test_paid_uses_lowest_sufficient_effort_and_free_scarce_quota_does_too(self):
        paid = self.enrollment()
        for floor in ('LOW', 'MEDIUM', 'HIGH', 'XHIGH'):
            with self.subTest(floor=floor):
                self.assertEqual(self.choose([paid], floor=floor)['reasoning_requested'], floor)
        free = self.enrollment(price=0, availability_observation=dict(state='available', quota_remaining=5, quota_scarce=True))
        self.assertEqual(self.choose([free], floor='MEDIUM')['reasoning_requested'], 'MEDIUM')

    def test_unsupported_effort_and_wrong_task_class_are_unavailable(self):
        low = self.enrollment(efforts=('LOW', 'MEDIUM'))
        with self.assertRaises(a.AdapterError):
            self.choose([low], floor='HIGH')
        with self.assertRaises(a.AdapterError):
            self.choose([low], task_class='security')
        with self.assertRaises(a.AdapterError):
            low.with_reasoning('MAX')
        wrong = dict(low)
        wrong['variants'] = {'MAX': 'max'}
        with self.assertRaises(a.AdapterError):
            seal_enrollment(wrong)

    def test_retry_and_pressure_cost_count_toward_budget(self):
        raw = dict(self.enrollment(price=5))
        raw['pricing_observation'].update(retry_cost_microusd=8, budget_pressure_microusd=2)
        enrolled = seal_enrollment(raw)
        with self.assertRaises(a.AdapterError):
            self.choose([enrolled], budget=14)
        self.assertEqual(self.choose([enrolled], budget=15)['model_id'], enrolled['model_id'])

    def test_task_class_quality_evidence_not_marketing_or_other_class(self):
        raw = dict(self.enrollment(price=0))
        raw['qualification_evidence'][0].update(task_class='mechanical', reasoning_efforts=['LOW', 'MEDIUM'])
        mechanical = seal_enrollment(raw)
        self.assertEqual(self.choose([mechanical], task_class='mechanical')['reasoning_requested'], 'MEDIUM')
        with self.assertRaises(a.AdapterError):
            self.choose([mechanical])

    def test_raw_dict_modified_digest_or_operator_identity_refuse(self):
        enrolled = self.enrollment()
        with self.assertRaises(a.AdapterError):
            self.choose([dict(enrolled)])
        fixture, authority, task = m3_task()
        digest = enrolled.enrollment_digest
        task['role_enrollment_ids'] = [digest]
        changed = dict(enrolled)
        changed['pricing_observation']['effort_cost_microusd']['LOW'] = 0
        with self.assertRaises(a.AdapterError):
            a.load_enrollment(changed, authority=authority, task=task, expected_digest=digest)
        with self.assertRaises(a.AdapterError):
            a.load_enrollment(dict(enrolled), authority=authority, task=task, expected_digest='f' * 64)
        changed = dict(enrolled)
        changed['operator_adoption_identity'] = 'candidate-self-approved'
        task['role_enrollment_ids'] = [c.canonical_digest(changed)]
        with self.assertRaises(a.AdapterError):
            a.load_enrollment(changed, authority=authority, task=task, expected_digest=c.canonical_digest(changed))
        with self.assertRaises(a.AdapterError):
            a.ValidatedEnrollment(dict(enrolled), task_contract_digest='a' * 64, authority_digest='b' * 64)

    def test_runtime_certification_digest_mismatch_refuses(self):
        raw = dict(self.enrollment(efforts=('HIGH',)))
        raw['adapter_certification_digest'] = '0' * 64
        enrolled = seal_enrollment(raw).with_reasoning('HIGH')
        with self.assertRaises(a.AdapterError):
            a.OpenCodeAdapter(opencode_binary(), role='IMPLEMENTATION', enrollment=enrolled,
                              workdir=Path('/candidate'), limits=a.StreamLimits(1 << 20, 64, 120))


class EffortTests(unittest.TestCase):
    """CP20: requested/sent/confirmed recorded honestly; unsupported effort blocks."""
    def test_reviewer_defaults_never_claim_high(self):
        adapter = a.OpenCodeAdapter(opencode_binary(), role='INVESTIGATION_REVIEW',
                                    enrollment=reviewer_enrollment().with_reasoning('HIGH'),
                                    workdir=Path('/candidate'), limits=a.StreamLimits(1 << 20, 64, 120))
        self.assertEqual((adapter.requested, adapter.sent, adapter.confirmed), ('HIGH', None, 'DEFAULT_PROVIDER'))

    def test_required_confirmed_effort_blocks_when_unsupplied(self):
        with self.assertRaises(a.AdapterError) as ctx:
            a.OpenCodeAdapter(opencode_binary(), role='INVESTIGATION_REVIEW',
                              enrollment=reviewer_enrollment().with_reasoning('HIGH'),
                              workdir=Path('/candidate'), limits=a.StreamLimits(1 << 20, 64, 120),
                              require_confirmed_effort=True)
        self.assertEqual(ctx.exception.code, a.EFFORT_PENDING)

    def test_same_or_unknown_family_leaves_review_pending(self):
        task, candidate = valid_task(), 'a' * 40
        report = {'schema_version': 1, 'task_id': task['task_id'], 'task_contract_digest': c.canonical_digest(task),
                  'candidate_sha': candidate, 'verdict': 'PASS', 'coverage': ['unit'],
                  'findings': [], 'quality_flag_dispositions': [], 'unresolved_questions': []}
        for family in ('fixture-a', 'unknown', ''):
            with self.subTest(family=family), self.assertRaises(a.AdapterError) as ctx:
                a.parse_review_report(copy.deepcopy(report), task=task, candidate_sha=candidate, schemas=SCHEMAS,
                                      reviewer_family=family, implementation_family='fixture-a')
            self.assertEqual(ctx.exception.code, a.REVIEW_PENDING)


def passing_report(task, candidate_sha):
    return {'schema_version': 1, 'task_id': task['task_id'], 'task_contract_digest': c.canonical_digest(task),
            'candidate_sha': candidate_sha, 'verdict': 'PASS', 'coverage': ['unit', 'guards'],
            'findings': [], 'quality_flag_dispositions': [], 'unresolved_questions': []}


class ReviewReportTests(unittest.TestCase):
    """CP21: strict report bound to task/candidate; stale data cannot authorize."""
    def setUp(self):
        self.task, self.candidate = valid_task(), 'a' * 40

    def check(self, payload, **kw):
        args = dict(task=self.task, candidate_sha=self.candidate, schemas=SCHEMAS,
                    reviewer_family='fixture-b', implementation_family='fixture-a', flag_ids=[])
        args.update(kw)
        return a.parse_review_report(payload, **args)

    def test_valid_pass_parses(self):
        self.assertEqual(self.check(passing_report(self.task, self.candidate))['verdict'], 'PASS')

    def test_prose_missing_wrong_and_stale_refuse(self):
        with self.assertRaises(a.AdapterError):
            self.check('looks good, ship it')
        bad = passing_report(self.task, self.candidate)
        del bad['coverage']
        with self.assertRaises(a.AdapterError):
            self.check(bad)
        bad = passing_report(self.task, self.candidate)
        bad['candidate_sha'] = 'b' * 40
        with self.assertRaises(a.AdapterError):
            self.check(bad)
        bad = passing_report(self.task, self.candidate)
        bad['task_contract_digest'] = 'b' * 64
        with self.assertRaises(a.AdapterError):
            self.check(bad)

    def test_pass_with_blocking_unknown_or_stale_flag_refuses(self):
        bad = passing_report(self.task, self.candidate)
        bad['findings'] = [dict(id='f1', severity='BLOCKING', classification='UNKNOWN', citation='src/x.py:1',
                                claim='unknown cause', competing_hypotheses=['a', 'b'], discriminating_check='run x')]
        with self.assertRaises(a.AdapterError):
            self.check(bad)
        bad = passing_report(self.task, self.candidate)
        bad['unresolved_questions'] = ['is the cache bounded?']
        with self.assertRaises(a.AdapterError):
            self.check(bad)
        bad = passing_report(self.task, self.candidate)
        bad['quality_flag_dispositions'] = [dict(id='stale-flag', disposition='NOT_LOWERING', evidence_digest='d' * 64)]
        with self.assertRaises(a.AdapterError):
            self.check(bad)
        bad = passing_report(self.task, self.candidate)
        bad['coverage'] = ['guards']
        with self.assertRaises(a.AdapterError):
            self.check(bad)

    def test_non_pass_with_evidence_parses_but_authorizes_nothing(self):
        bad = passing_report(self.task, self.candidate)
        bad.update(verdict='DEFECT_FOUND', findings=[dict(id='f1', severity='BLOCKING', classification='PROVEN',
                     citation='src/x.py:1', claim='unbounded queue', competing_hypotheses=[], discriminating_check='measure')])
        self.assertEqual(self.check(bad)['verdict'], 'DEFECT_FOUND')


class EventStreamTests(unittest.TestCase):
    """CP22: strict pinned-envelope parsing; exactly one proven final payload."""
    limits = a.StreamLimits(1 << 20, 64, 120)

    def stream(self, *lines):
        return ('\n'.join(lines) + '\n').encode()

    def test_real_error_event_shape_parses(self):
        with tempfile.TemporaryDirectory() as home:
            adapter = a.OpenCodeAdapter(opencode_binary(), role='IMPLEMENTATION',
                                        enrollment=worker_enrollment(model_id='no-such-provider/no-such-model-2', family='probe2').with_reasoning('HIGH'),
                                        workdir=Path(home), limits=self.limits,
                                        env={'PATH': os.environ.get('PATH', '/usr/bin:/bin'), 'LANG': 'C', 'HOME': home})
            observation = adapter.run(['probe'], timeout_seconds=60)
        self.assertEqual(observation.error['type'], 'error')
        self.assertTrue(observation.session_id.startswith('ses_'))
        self.assertIsNone(observation.final_payload)

    def test_final_text_honored_and_earlier_text_ignored(self):
        data = self.stream('{"type":"reasoning","sessionID":"ses_1","timestamp":1,"text":"hmm"}',
                           '{"type":"tool.execute","sessionID":"ses_1","timestamp":2}',
                           '{"type":"text","sessionID":"ses_1","timestamp":3,"part":{"type":"text","text":"{\\"verdict\\":1}"}}')
        parsed = a.parse_event_stream(data, limits=self.limits, event_contract=a.OPENCODE_EVENT_CONTRACT)
        self.assertEqual((parsed.session_id, parsed.final_payload, parsed.error), ('ses_1', '{"verdict":1}', None))

    def test_step_closed_final_ignores_intermediate_text_and_refuses_incomplete_runs(self):
        def event(kind, **part):
            return c.canonical_json(dict(type=kind, sessionID='ses_steps', part=part))
        start = event('step_start', type='step-start')
        intermediate = event('text', type='text', text='\n\n')
        tool_finish = event('step_finish', type='step-finish', reason='tool-calls')
        final = event('text', type='text', text='{"verdict":"PASS"}')
        stop = event('step_finish', type='step-finish', reason='stop')
        good = [start, intermediate, tool_finish, start, final, stop]
        parsed = a.parse_event_stream(self.stream(*good), limits=self.limits, event_contract=a.OPENCODE_EVENT_CONTRACT)
        self.assertEqual(parsed.final_payload, '{"verdict":"PASS"}')
        attacks = [good[:-1], [start, intermediate, tool_finish], [start, final, final, stop],
                   good + [start, final, stop], [start, final, event('step_finish', reason='length')],
                   [final, start, final, stop], [start, start, final, stop],
                   [start, final, event('step_finish', reason='unknown')]]
        for rows in attacks:
            with self.subTest(rows=rows), self.assertRaises(a.AdapterError):
                a.parse_event_stream(self.stream(*rows), limits=self.limits, event_contract=a.OPENCODE_EVENT_CONTRACT)

    def test_negative_matrix(self):
        good = '{"type":"text","sessionID":"ses_1","part":{"type":"text","text":"done"}}'
        cases = [
            self.stream('{"type":"text","sessionID":"ses_1","part":{"type":"text","text":"a"}}', '{"type":"text","sessionID":"ses_1","part":{"type":"text","text":"b"}}'),
            self.stream('{"type":"text","sessionID":"ses_1","part":{"type":"text","text":"a"}}', '{"type":"text","sessionID":"ses_2","part":{"type":"text","text":"b"}}'),
            self.stream('{"type":"text","sessionID":"ses_1","text":"a"}').rstrip(b'\n'),
            self.stream('{"type":"text","text":"a"}'),
            self.stream('{"type":"text","sessionID":"ses_1"}'),
            self.stream('{"type":"text","sessionID":"ses_1","timestamp":"now","text":"a"}'),
            self.stream('{"type":"text","sessionID":"ses_1","text":"a","text":"b"}'),
            self.stream('not json'),
            'bad\xffbytes\n'.encode('latin1'),
            b'',
        ]
        for data in cases:
            with self.subTest(data=data[:60]), self.assertRaises(a.AdapterError):
                a.parse_event_stream(data, limits=self.limits, event_contract=a.OPENCODE_EVENT_CONTRACT)
        with self.assertRaises(a.AdapterError):
            a.parse_event_stream(good.encode() * 100000, limits=a.StreamLimits(100, 64, 120), event_contract=a.OPENCODE_EVENT_CONTRACT)
        with self.assertRaises(a.AdapterError):
            a.parse_event_stream(self.stream(*([good] * 5)), limits=a.StreamLimits(1 << 20, 4, 120), event_contract=a.OPENCODE_EVENT_CONTRACT)


class RouterTests(unittest.TestCase):
    """CP23: fixed advisory label only; deterministic rules decide alone."""
    def test_label_mapping(self):
        self.assertEqual(o.triage_label({'architecture_question': True}), 'ESCALATE_ARCHITECTURE')
        self.assertEqual(o.triage_label({'blocking_unknowns': 2}), 'INVESTIGATE')
        self.assertEqual(o.triage_label({'failed_checks': ['unit'], 'speculative_used': 2, 'speculative_budget': 2}), 'INVESTIGATE')
        self.assertEqual(o.triage_label({'failed_checks': ['unit'], 'speculative_used': 0}), 'MECHANICAL_FIX')
        self.assertEqual(o.triage_label({}), 'NO_MODEL_ACTION')
        with self.assertRaises(o.OrchestratorError):
            o.triage_label('route it')


class AttemptLedgerTests(unittest.TestCase):
    """CP24: two speculative corrections exhaust the episode; causal packet or escalate."""
    def test_budget_and_causal_gate(self):
        ledger = o.AttemptLedger()
        episode = o.AttemptLedger.episode_id('task-M3', 'worker', 'CP19')
        self.assertEqual(ledger.admit(episode), 'ADMIT')
        ledger.record(episode, 'speculative', None, 1)
        self.assertEqual(ledger.admit(episode), 'ADMIT')
        ledger.record(episode, 'speculative', None, 2)
        self.assertEqual(ledger.admit(episode), 'REQUIRE_CAUSAL')
        with self.assertRaises(o.OrchestratorError):
            ledger.admit(episode, causal_packet={'failure_evidence_digest': 'd' * 64})
        causal = {'failure_evidence_digest': 'd' * 64, 'falsifiable_cause': 'dns',
                  'discriminating_result': 'direct dial fails', 'patch_explanation': 'pin nameserver'}
        self.assertEqual(ledger.admit(episode, causal_packet=causal), 'ADMIT')
        ledger.record(episode, 'causal', 'd' * 64, 3)
        self.assertEqual(ledger.admit(episode, causal_packet=causal), 'ESCALATE')
        other = o.AttemptLedger.episode_id('task-M3', 'reviewer', 'CP21')
        self.assertEqual(ledger.admit(other), 'ADMIT')

    def test_fingerprint_change_never_resets_budget(self):
        ledger = o.AttemptLedger()
        episode = o.AttemptLedger.episode_id('task-M3', 'worker', 'CP19')
        ledger.record(episode, 'speculative', None, 1)
        ledger.record(episode, 'speculative', None, 1)
        self.assertEqual(ledger.admit(episode), 'REQUIRE_CAUSAL')

    def test_persist_and_load_roundtrip(self):
        fixture, authority, task = m3_task()
        with tempfile.TemporaryDirectory() as directory:
            runtime = s.RuntimeStore(Path(directory).resolve() / 'runtime', authority)
            runtime.initialize()
            runtime.register_task(task, copy.deepcopy(task))
            ledger = o.AttemptLedger()
            episode = o.AttemptLedger.episode_id(task['task_id'], 'worker', 'CP19')
            digest = ledger.persist(runtime, task['task_id'], ledger.record(episode, 'speculative', None, 0))
            self.assertTrue(digest)
            reloaded = o.AttemptLedger.load(runtime)
            self.assertEqual(reloaded.speculative_used(episode), 1)
            self.assertEqual(reloaded.admit(episode), 'ADMIT')


class DisconnectTests(unittest.TestCase):
    """CP25: a historical disconnect is UNKNOWN, never infrastructure PASS."""
    def test_disconnect_never_passes(self):
        classified = a.classify_disconnect({'kind': 'disconnect', 'note': 'driver dropped'})
        self.assertEqual(classified['classification'], 'UNKNOWN')
        with self.assertRaises(o.OrchestratorError):
            o.diagnose_disconnect({'kind': 'disconnect'}, hypotheses=['app'], discriminating_observation='log')
        with self.assertRaises(o.OrchestratorError):
            o.diagnose_disconnect({'kind': 'disconnect'}, hypotheses=['app', 'driver'])
        diagnosis = o.diagnose_disconnect({'kind': 'disconnect'}, hypotheses=['app main-thread stall', 'driver service death'],
                                          discriminating_observation='capture main-thread stack during stall')
        self.assertEqual(diagnosis['verdict'], 'UNKNOWN')
        self.assertEqual(len(diagnosis['hypotheses']), 2)


class CodexQuotaTests(unittest.TestCase):
    """CP26: quota defers with identical facts; malformed/auth/tool errors never become contracts."""
    FAKE = '#!/usr/bin/env python3\nimport os,sys,json\nif "--version" in sys.argv:\n    print("codex-cli 0.158.0")\n    sys.exit(0)\nscenario=os.environ.get("OR_V2_FAKE_CODEX","decision")\nif scenario=="quota":\n    sys.stderr.write("codex: rate limited: quota exhausted for codex-cli 0.158.0, retry later\\n")\n    sys.exit(1)\nif scenario=="auth":\n    sys.stderr.write("codex: not logged in\\n")\n    sys.exit(1)\nif scenario=="garbage":\n    print("not json at all")\n    sys.exit(0)\ndecision={"schema_version":1,"packet_digest":os.environ.get("OR_V2_FAKE_PACKET_DIGEST","PACKET"),"disposition":"NEEDS_DISCRIMINATING_EVIDENCE","decision":"collect stacks","constraints":["read-only"],"required_verification":["stacks"],"stop_conditions":["no prod writes"],"cited_facts":[{"fact":"exit 23","classification":"PROVEN"}]}\nopen(os.environ["OR_V2_FAKE_LAST_MESSAGE"],"w").write(json.dumps(decision))\nprint(json.dumps({"type":"thread.started","thread_id":"fixture-only"}))\nprint(json.dumps({"type":"turn.started"}))\nprint(json.dumps({"type":"item.completed","item":{"type":"agent_message","text":json.dumps(decision)}}))\nprint(json.dumps({"type":"turn.completed","usage":{}}))\n'

    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        cls.addClassCleanup(cls.temp.cleanup)
        cls.exe = Path(cls.temp.name) / 'codex'
        cls.exe.write_text(cls.FAKE)
        cls.exe.chmod(0o755)
        cls.digest = hashlib.sha256(cls.exe.read_bytes()).hexdigest()
        cls.schema = Path(cls.temp.name) / 'decision.schema.json'
        cls.schema.write_text(c.canonical_json(a.architecture_decision_schema()))
        cls.outdir = Path(cls.temp.name) / 'out'
        cls.outdir.mkdir()

    def adapter(self, signatures=(), scenario='decision', packet_digest='PACKET'):
        for name in ('codex-last-message.json', 'codex-events.jsonl', 'codex-stderr.txt'):
            (self.outdir / name).unlink(missing_ok=True)
        binary = a.CodexBinary(self.exe, self.digest, 'codex-cli 0.158.0')
        return a.CodexAdapter(binary, model_id='fixture/architect', workdir=Path('/candidate'),
                              decision_schema_path=self.schema, quota_signatures=list(signatures),
                              env={'PATH': '/usr/bin:/bin', 'LANG': 'C', 'OR_V2_FAKE_CODEX': scenario,
                                   'OR_V2_FAKE_LAST_MESSAGE': str(self.outdir / 'codex-last-message.json'),
                                   'OR_V2_FAKE_PACKET_DIGEST': packet_digest})

    def packet(self):
        return {'schema_version': 1, 'task_id': 'task-M3', 'checkpoint_id': 'M3', 'authority_digest': 'd' * 64,
                'base_sha': 'a' * 40, 'kind': 'difficult_root_cause', 'failure_facts': {'exit': 23},
                'attempt_digest': 'd' * 64, 'invariant_citations': [], 'question': 'why?', 'scope': 'diagnose',
                'packet_digest': 'PLACEHOLDER'}

    def test_quota_deferral_preserves_packet(self):
        adapter = self.adapter([{'version': 'codex-cli 0.158.0', 'stderr_contains': 'quota exhausted'}], scenario='quota')
        packet = self.packet()
        packet['packet_digest'] = c.canonical_digest({k: v for k, v in packet.items() if k != 'packet_digest'})
        with self.assertRaises(a.AdapterError) as ctx:
            adapter.run_architecture('decide', packet=packet, output_dir=self.outdir)
        self.assertEqual(ctx.exception.code, a.QUOTA_DEFERRED)
        deferred = adapter.defer_quota(packet)
        self.assertEqual((deferred.packet_digest, deferred.availability), (packet['packet_digest'], 'quota'))
        self.assertEqual(deferred.packet, packet)

    def test_auth_tool_and_malformed_are_not_quota_or_decision(self):
        adapter = self.adapter([{'version': 'codex-cli 0.158.0', 'stderr_contains': 'quota exhausted'}], scenario='auth')
        packet = self.packet()
        packet['packet_digest'] = c.canonical_digest({k: v for k, v in packet.items() if k != 'packet_digest'})
        with self.assertRaises(a.AdapterError) as ctx:
            adapter.run_architecture('decide', packet=packet, output_dir=self.outdir)
        self.assertEqual(ctx.exception.code, a.TRANSPORT_ERROR)
        adapter = self.adapter(scenario='garbage')
        with self.assertRaises(a.AdapterError) as ctx:
            adapter.run_architecture('decide', packet=packet, output_dir=self.outdir)
        self.assertEqual(ctx.exception.code, a.PROTOCOL_ERROR)

    def test_strict_decision_binds_packet(self):
        packet = self.packet()
        packet['packet_digest'] = c.canonical_digest({k: v for k, v in packet.items() if k != 'packet_digest'})
        adapter = self.adapter(packet_digest=packet['packet_digest'])
        last = self.outdir / 'codex-last-message.json'
        if last.exists():
            last.unlink()
        decision, deferred = adapter.run_architecture('decide', packet=packet, output_dir=self.outdir)
        self.assertIsNone(deferred)
        decision['packet_digest'] = packet['packet_digest']
        checked = a.validate_architecture_decision(decision, packet)
        self.assertEqual(checked['disposition'], 'NEEDS_DISCRIMINATING_EVIDENCE')
        bad = dict(decision, packet_digest='0' * 64)
        with self.assertRaises(a.AdapterError):
            a.validate_architecture_decision(bad, packet)


class DesktopSnapshotTests(unittest.TestCase):
    """CP31: read-only published facts; missing runtime creates nothing."""
    def test_snapshot_is_pure_and_missing_runtime_creates_nothing(self):
        with tempfile.TemporaryDirectory() as directory:
            before = sorted(p.name for p in Path(directory).iterdir())
            snapshot = o.publish_snapshot(plan_checkpoint='9B', operational_stage='READY', next_legal_action='CLAIM',
                                          model_availability={}, health='ok', blockers=[], pending_escalation=None,
                                          runtime_present=False)
            self.assertEqual(snapshot['runtime'], 'ABSENT')
            self.assertEqual(sorted(p.name for p in Path(directory).iterdir()), before)
            self.assertEqual(set(snapshot), set(o.SNAPSHOT_KEYS))

    def test_snapshot_refuses_bad_facts(self):
        with self.assertRaises(o.OrchestratorError):
            o.publish_snapshot(plan_checkpoint='', operational_stage='READY', next_legal_action='CLAIM',
                               model_availability={}, health='ok', blockers=[], pending_escalation=None,
                               runtime_present=True)


class TelemetryTests(unittest.TestCase):
    """CP32: factual telemetry with null unknowns; listings never enroll."""
    def record(self):
        return {'task_class': 'M3', 'model_id': 'fixture-provider/fixture-worker', 'provider_id': 'fixture-provider',
                'family': 'fixture-a', 'adapter_version': a.adapter_version(), 'reasoning_requested': 'HIGH',
                'reasoning_sent': None, 'reasoning_confirmed': 'DEFAULT_PROVIDER', 'candidate_outcome': 'SETTLED',
                'reviewer_defects': 0, 'repair_count': 0, 'wall_time': 1.5, 'tool_loops': 3,
                'acceptance_outcome': 'PENDING', 'tokens_if_available': None, 'cost_if_available': None}

    def test_append_and_null_unknowns(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'telemetry.jsonl'
            a.append_telemetry(path, self.record())
            stored = c.load_json_strict(path.read_text().strip())
            self.assertIsNone(stored['tokens_if_available'])
            self.assertIsNone(stored['cost_if_available'])

    def test_bad_telemetry_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'telemetry.jsonl'
            bad = self.record()
            del bad['wall_time']
            with self.assertRaises(a.AdapterError):
                a.append_telemetry(path, bad)


FAKE_DOCKER = '''#!/usr/bin/env python3
"""Deterministic fake Docker transport: decision-logic wiring only, never isolation proof."""
import json, os, sys
from pathlib import Path
here = Path(sys.argv[0]).resolve().parent
state = Path(os.environ.get("OR_V2_FAKE_DOCKER_STATE", here / "docker-state"))
transcript_path = Path(os.environ.get("OR_V2_FAKE_TRANSCRIPT", here / "transcript.bin"))
exit_code = int(os.environ.get("OR_V2_FAKE_EXIT", "0"))
argv = sys.argv[1:]
if argv and argv[0].startswith("--host="):
    argv = argv[1:]
log = state / "calls.jsonl"
with open(log, "a") as handle:
    handle.write(json.dumps(argv) + "\\n")
cmd = argv[0]
def read(cid):
    return json.loads((state / (cid + ".json")).read_text())
def write(cid, spec):
    (state / (cid + ".json")).write_text(json.dumps(spec))
if cmd == "info":
    print(json.dumps({"OSType": "linux", "ID": "daemon-fake", "SecurityOptions": ["name=rootless"],
                      "CgroupVersion": "2", "MemoryLimit": True, "SwapLimit": True, "PidsLimit": True,
                      "CpuCfsQuota": True, "CpuCfsPeriod": True, "Architecture": "arm64"}))
elif cmd == "image":
    img = argv[2]
    print(json.dumps([{"Id": img, "Os": "linux", "Architecture": "arm64", "Config": {"Volumes": None, "Env": None}}]))
elif cmd == "create":
    spec = {"labels": {}, "mounts": [], "env": [], "ulimits": {}, "name": None, "entrypoint": None,
            "image": None, "cmd": [], "tmpfs": None, "exit": int(os.environ.get("OR_V2_FAKE_EXIT", "0"))}
    index = 1
    while index < len(argv):
        item = argv[index]
        if spec["image"] is not None:
            spec["cmd"].append(item)
            index += 1
        elif item == "--label":
            key, _, value = argv[index + 1].partition("=")
            spec["labels"][key] = value
            index += 2
        elif item.startswith("--name="):
            spec["name"] = item.split("=", 1)[1]
            index += 1
        elif item.startswith("--entrypoint="):
            spec["entrypoint"] = item.split("=", 1)[1]
            index += 1
        elif item.startswith("--mount="):
            fields = item.split("=", 1)[1].split(",")
            options = dict(field.split("=", 1) for field in fields if "=" in field)
            spec["mounts"].append({"src": options["src"], "dst": options["dst"], "ro": "readonly" in fields})
            index += 1
        elif item.startswith("--env="):
            spec["env"].append(item.split("=", 1)[1])
            index += 1
        elif item.startswith("--tmpfs="):
            spec["tmpfs"] = item.split("=", 1)[1].split(":", 1)[1]
            index += 1
        elif item.startswith("--ulimit="):
            name, _, value = item.split("=", 1)[1].partition("=")
            soft, _, hard = value.partition(":")
            spec["ulimits"][name] = (int(soft), int(hard))
            index += 1
        elif item.startswith("--memory="):
            spec["memory"] = int(item.split("=", 1)[1])
            index += 1
        elif item.startswith("--memory-swap="):
            spec["swap"] = int(item.split("=", 1)[1])
            index += 1
        elif item.startswith("--cpus="):
            spec["cpus"] = int(item.split("=", 1)[1])
            index += 1
        elif item.startswith("--pids-limit="):
            spec["pids"] = int(item.split("=", 1)[1])
            index += 1
        elif item.startswith("--user="):
            spec["user"] = item.split("=", 1)[1]
            index += 1
        elif item.startswith("--workdir="):
            spec["workdir"] = item.split("=", 1)[1]
            index += 1
        elif item.startswith("--network="):
            spec["network"] = item.split("=", 1)[1]
            index += 1
        elif item.startswith("--ipc="):
            spec["ipc"] = item.split("=", 1)[1]
            index += 1
        elif item.startswith("--"):
            index += 1
        elif item.startswith("sha256:") and spec["image"] is None:
            spec["image"] = item
            index += 1
        else:
            spec["cmd"].append(item)
            index += 1
    cid = "f" * 64
    spec["status"] = "created"
    write(cid, spec)
    print(cid)
elif cmd == "inspect":
    spec = read(argv[1])
    running = spec["status"] == "running"
    print(json.dumps([{"Id": argv[1], "Image": spec["image"],
        "Config": {"User": spec.get("user", "10001:10001"), "Labels": spec["labels"],
                   "WorkingDir": spec.get("workdir", "/candidate"), "Entrypoint": [spec["entrypoint"]],
                   "Cmd": spec["cmd"], "Env": spec["env"] + ["PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"]},
        "HostConfig": {"ReadonlyRootfs": True, "Privileged": False, "CapAdd": None, "CapDrop": ["ALL"],
            "SecurityOpt": ["no-new-privileges:true"], "NetworkMode": spec.get("network", "none"),
            "PidMode": "", "IpcMode": spec.get("ipc", "none"), "UTSMode": "", "UsernsMode": "",
            "CgroupnsMode": "private", "Devices": [], "DeviceRequests": [], "DeviceCgroupRules": [],
            "VolumesFrom": [], "Links": [], "ExtraHosts": [],
            "Memory": spec["memory"], "MemorySwap": spec["swap"], "NanoCpus": spec["cpus"] * 1000000000,
            "PidsLimit": spec["pids"], "RestartPolicy": {"Name": "no", "MaximumRetryCount": 0},
            "LogConfig": {"Type": "none", "Config": {}}, "Tmpfs": {"/scratch": spec["tmpfs"]},
            "Ulimits": [{"Name": "nofile", "Soft": 256, "Hard": 256},
                        {"Name": "fsize", "Soft": spec["ulimits"]["fsize"][0], "Hard": spec["ulimits"]["fsize"][1]}]},
        "Mounts": [{"Type": "bind", "Source": m["src"], "Destination": m["dst"], "RW": not m["ro"], "Propagation": "rprivate"} for m in spec["mounts"]],
        "State": {"Status": spec["status"], "Running": running, "Paused": False,
                  "Restarting": False, "Pid": 0, "ExitCode": spec["exit"]}}]))
elif cmd == "start":
    cid = argv[-1]
    spec = read(cid)
    if "--attach" in argv:
        sys.stdout.buffer.write(transcript_path.read_bytes())
        spec["exit"] = exit_code
    spec["status"] = "exited"
    write(cid, spec)
elif cmd == "kill":
    spec = read(argv[1])
    spec["status"] = "exited"
    write(argv[1], spec)
elif cmd == "rm":
    (state / (argv[1] + ".json")).unlink(missing_ok=True)
elif cmd == "ps":
    for name in sorted(p.name for p in state.glob("f*.json") if p.name != "calls.jsonl"):
        print(name[:-5])
else:
    sys.exit(99)
'''


class FakeDocker:
    """Deterministic transport double; asserts wiring, proves no isolation."""
    def __init__(self, directory):
        self.state = Path(directory) / 'docker-state'
        self.state.mkdir()
        self.exe = Path(directory) / 'docker'
        self.exe.write_text(FAKE_DOCKER)
        self.exe.chmod(0o755)
        self.digest = hashlib.sha256(self.exe.read_bytes()).hexdigest()
        self.executable = self.exe
        self.endpoint = None
        self.transcript = Path(directory) / 'transcript.bin'
        self.transcript.write_bytes(b'')

    def __call__(self, args):
        env = dict(os.environ, OR_V2_FAKE_DOCKER_STATE=str(self.state),
                   OR_V2_FAKE_TRANSCRIPT=str(self.transcript),
                   OR_V2_FAKE_EXIT=os.environ.get('OR_V2_FAKE_EXIT', '0'))
        return subprocess.check_output([str(self.exe), *args], env=env, stderr=subprocess.STDOUT).decode()

    def logged(self):
        path = self.state / 'calls.jsonl'
        if not path.exists():
            return []
        return [json.loads(line) for line in path.read_text().splitlines()]

    def box(self):
        return b.ContainerSandbox(self, boot_identity='boot', host_platform='linux', fixture_only=True)

    def export_env(self, test):
        # The attach subprocess is spawned directly, so the fake reads its
        # control variables from process environment, not the transport call.
        previous = dict(os.environ)
        os.environ['OR_V2_FAKE_DOCKER_STATE'] = str(self.state)
        os.environ['OR_V2_FAKE_TRANSCRIPT'] = str(self.transcript)
        os.environ['OR_V2_FAKE_EXIT'] = '0'
        test.addCleanup(lambda: (os.environ.clear(), os.environ.update(previous)))


def fake_limits():
    return b.Limits(1, 1 << 30, 64, 1 << 30, 1 << 20, 1 << 20, 60)


class WorkerWiringTests(unittest.TestCase):
    """Launch wiring through the real adapter path; fixture proofs cannot settle."""
    def test_worker_launch_wiring_and_fixture_boundary(self):
        fixture, authority, task = m3_task()
        original = worker_enrollment()
        record = c.load_json_strict(original.payload_json)
        task['role_enrollment_ids'] = [c.canonical_digest(record)]
        enrollment = a.load_enrollment(record, authority=authority, task=task,
                                       expected_digest=c.canonical_digest(record))
        with tempfile.TemporaryDirectory() as directory:
            parent = Path(directory).resolve()
            runtime = s.RuntimeStore(parent / 'runtime', authority)
            runtime.initialize()
            digest = runtime.register_task(task, copy.deepcopy(task))
            candidate = b.create_candidate(fixture.candidate, parent / 'worker', c.TRUSTED_DESIGN_BASE, authority=authority)
            runtime.register_candidate(task['task_id'], candidate)
            fake = FakeDocker(directory)
            fake.export_env(self)
            fake.transcript.write_bytes(b'{"type":"text","sessionID":"ses_w","timestamp":1,"part":{"type":"text","text":"changed files"}}\n')
            box = fake.box()
            with runtime.lock('task', task['task_id']):
                stage = runtime.claim(task['task_id'], digest, owner_nonce='owner', boot_identity='boot',
                                      stage_id='stage', stage_nonce='nonce')
                with self.assertRaises(c.ContractError):
                    o.run_worker(store=runtime, stage=stage, box=box, candidate=candidate, task=task,
                                 enrollment=enrollment.with_reasoning('HIGH'), binary=opencode_binary(),
                                 agent='orch-worker', prompt='implement the task', image='sha256:' + 'b' * 64,
                                 limits=fake_limits(), network='none', container_binary='/usr/local/bin/opencode',
                                 storage_root=parent, timeout_seconds=60)
            calls = fake.logged()
            create = next(call for call in calls if call[0] == 'create')
            self.assertIn('--model', create)
            self.assertEqual(create[create.index('--model') + 1], enrollment['model_id'])
            self.assertIn('/usr/local/bin/opencode', create)
            joined = ' '.join(create)
            self.assertIn('or.v2.task=' + task['task_id'], joined)
            self.assertIn('--network=none', create)
            self.assertIn('--name=or-v2-', joined)
            # Attach runs through a direct pipe, not the logged transport; the
            # fake state file proves the stage started and exited.
            state = json.loads((fake.state / ('f' * 64 + '.json')).read_text())
            self.assertEqual(state['status'], 'exited')
            # The fixture transport observed everything but cannot release a production lease.
            self.assertEqual(runtime.inspect()['tasks'][task['task_id']]['status'], 'CLAIMED')

    def test_transcript_capture_and_bridge_profile(self):
        fixture, authority, task = m3_task()
        original = worker_enrollment()
        record = c.load_json_strict(original.payload_json)
        task['role_enrollment_ids'] = [c.canonical_digest(record)]
        enrollment = a.load_enrollment(record, authority=authority, task=task,
                                       expected_digest=c.canonical_digest(record))
        with tempfile.TemporaryDirectory() as directory:
            parent = Path(directory).resolve()
            candidate = b.create_candidate(fixture.candidate, parent / 'worker', c.TRUSTED_DESIGN_BASE, authority=authority)
            fake = FakeDocker(directory)
            fake.export_env(self)
            fake.transcript.write_bytes(b'line one\nline two\n')
            box = fake.box()
            view = a.build_worker_view(candidate, parent, task['task_id'], 'stage', 'IMPLEMENTATION')
            overlays = b.write_role_overlays(parent / 'overlays', 'IMPLEMENTATION')
            mounts = b._validated_role_overlays(overlays, 'IMPLEMENTATION', candidate.root)
            adapter = a.OpenCodeAdapter(opencode_binary(), role='IMPLEMENTATION',
                                        enrollment=enrollment.with_reasoning('HIGH'),
                                        workdir=Path('/candidate'), limits=a.StreamLimits(1 << 20, 64, 120))
            labels = b.StageIdentity('0' * 64, 'boot', 'owner', 'stage', 1, 'nonce',
                                     task['authority_digest'], 'IMPLEMENTATION', task['task_id']).labels()
            result = a.launch_model_stage(box=box, candidate=candidate, view=view, role='IMPLEMENTATION',
                                          container_binary='/usr/local/bin/opencode', message_parts=['do it'],
                                          agent='orch-worker', adapter=adapter, image='sha256:' + 'b' * 64,
                                          network='none', limits=fake_limits(), overlay_mounts=mounts,
                                          labels=labels, name='or-v2-test-none', timeout_seconds=60)
            self.assertEqual(result.transcript, b'line one\nline two\n')
            self.assertFalse(result.truncated)
            self.assertEqual(result.exit_code, 0)
            creates = [call for call in fake.logged() if call[0] == 'create']
            self.assertIn('--network=none', creates[0])
            profile = {'image': 'sha256:' + 'b' * 64,
                       'mounts': [(str(view), '/candidate', True)] + [(str(s), d, False) for s, d in mounts],
                       'limits': fake_limits(), 'command': logged_command(fake),
                       'environment': ['HOME=/worker-home', 'TMPDIR=/scratch',
                                       'XDG_CONFIG_HOME=/worker-home/.config',
                                       'XDG_DATA_HOME=/worker-home/.local/share',
                                       'XDG_STATE_HOME=/worker-home/.local/state',
                                       'OPENCODE_CONFIG=/worker-home/opencode.json',
                                       'GIT_CONFIG_NOSYSTEM=1', 'GIT_CONFIG_GLOBAL=/dev/null'],
                       'created_only': False}
            # The launcher already verified the M1 least-privilege profile
            # pre-start. The bridge checker must reject this least-privilege
            # instance before the shared fake identity is reused below.
            with self.assertRaises(c.ContractError):
                a._verify_bridge_effective(box, a._stage_identity(result.container_id, labels, 'IMPLEMENTATION'), profile)
            bridged = a.launch_model_stage(box=box, candidate=candidate, view=view, role='IMPLEMENTATION',
                                           container_binary='/usr/local/bin/opencode', message_parts=['do it'],
                                           agent='orch-worker', adapter=adapter, image='sha256:' + 'b' * 64,
                                           network='bridge', limits=fake_limits(), overlay_mounts=mounts,
                                           labels=labels, name='or-v2-test-bridge', timeout_seconds=60)
            creates = [call for call in fake.logged() if call[0] == 'create']
            self.assertIn('--network=bridge', creates[-1])
            # The bridge checker accepts the egress profile; the M1 checker
            # accepts only the least-privilege profile.
            bridged_profile = dict(profile, command=logged_command(fake))
            a._verify_bridge_effective(box, a._stage_identity(bridged.container_id, labels, 'IMPLEMENTATION'),
                                       bridged_profile)
            with self.assertRaises(c.ContractError):
                box.verify_effective(a._stage_identity(bridged.container_id, labels, 'IMPLEMENTATION'), bridged_profile)


def logged_command(fake):
    """Reconstruct the (entrypoint, *cmd) tuple the fake docker observed at create."""
    create = [call for call in fake.logged() if call[0] == 'create'][-1]
    entry = next(item.split('=', 1)[1] for item in create if item.startswith('--entrypoint='))
    index = next(i for i, item in enumerate(create) if item.startswith('sha256:'))
    return tuple([entry] + create[index + 1:])


class ReviewerWiringTests(unittest.TestCase):
    """Reviewer transport and strict parsing; guard vetoes stop review."""
    def report_event(self, task, candidate_sha):
        report = passing_report(task, candidate_sha)
        return ('{"type":"reasoning","sessionID":"ses_r","timestamp":1}\n'
                + '{"type":"text","sessionID":"ses_r","timestamp":2,"part":{"type":"text","text":' + json.dumps(json.dumps(report)) + '}}\n').encode()

    def test_reviewer_launch_parses_strict_report(self):
        fixture, authority, task = m3_task()
        candidate_sha = 'b' * 40
        with tempfile.TemporaryDirectory() as directory:
            parent = Path(directory).resolve()
            candidate = b.create_candidate(fixture.candidate, parent / 'worker', c.TRUSTED_DESIGN_BASE, authority=authority)
            fake = FakeDocker(directory)
            fake.export_env(self)
            fake.transcript.write_bytes(self.report_event(task, candidate_sha))
            box = fake.box()
            enrollment = reviewer_enrollment()
            adapter = a.OpenCodeAdapter(opencode_binary(), role='INVESTIGATION_REVIEW',
                                        enrollment=enrollment.with_reasoning('HIGH'),
                                        workdir=Path('/candidate'), limits=a.StreamLimits(1 << 20, 64, 120))
            view = a.build_worker_view(candidate, parent, task['task_id'], 'review', 'INVESTIGATION_REVIEW')
            overlays = b.write_role_overlays(parent / 'overlays', 'INVESTIGATION_REVIEW')
            mounts = b._validated_role_overlays(overlays, 'INVESTIGATION_REVIEW', candidate.root)
            labels = b.StageIdentity('0' * 64, 'boot', 'review-ab12', 'review', 1, 'review-ab12',
                                     task['authority_digest'], 'INVESTIGATION_REVIEW', task['task_id']).labels()
            result = a.launch_model_stage(box=box, candidate=candidate, view=view, role='INVESTIGATION_REVIEW',
                                          container_binary='/usr/local/bin/opencode', message_parts=['review it'],
                                          agent='orch-reviewer', adapter=adapter, image='sha256:' + 'b' * 64,
                                          network='none', limits=fake_limits(), overlay_mounts=mounts,
                                          labels=labels, name='or-v2-review-test', timeout_seconds=60)
            box.docker(['rm', result.container_id])
            stream = a.parse_event_stream(result.transcript, limits=a.StreamLimits(1 << 20, 64, 120),
                                          event_contract=opencode_binary().event_contract)
            self.assertIsNone(stream.error)
            report = a.parse_review_report(stream.final_payload, task=task, candidate_sha=candidate_sha,
                                           schemas=SCHEMAS, reviewer_family='fixture-b',
                                           implementation_family='fixture-a', flag_ids=[])
            self.assertEqual(report['verdict'], 'PASS')
            calls = fake.logged()
            create = next(call for call in calls if call[0] == 'create')
            self.assertIn('--network=none', create)
            self.assertIn('--agent', create)
            self.assertIn('orch-reviewer', create)
            self.assertTrue(any(call == ['rm', result.container_id] for call in calls))

    def test_guard_veto_stops_review_before_transport(self):
        fixture, authority, task = m3_task()
        with tempfile.TemporaryDirectory() as directory:
            parent = Path(directory).resolve()
            floor = g.Floor(authority, copy.deepcopy(task), {}, seal=g._SEAL)
            dest = parent / 'quarantine'
            dest.mkdir(mode=0o700)
            (dest / 'candidate').mkdir()
            (dest / 'candidate' / 'note.txt').write_text('guarded')
            manifest = w.manifest(dest / 'candidate')
            entry = dest.stat()
            record = dict(schema_version=1, nonce='n' * 32, task_id=task['task_id'], floor_digest=floor.digest,
                          source=str(parent / 'source'), source_device=entry.st_dev, source_inode=entry.st_ino,
                          base=task['base_sha'], head='b' * 40, phase='GUARDED', vetoes=['scope/protected path: x @ deadbeef'],
                          flags=[], commits=[], tree_digest='d' * 64, manifest=manifest,
                          directory_identity=[entry.st_dev, entry.st_ino])
            g.write_json(dest / 'reservation.json', record)
            g.write_json(dest / 'guard.json', record)
            receipt = dict(schema_version=1, task_id=task['task_id'], candidate_sha='b' * 40, base_sha=task['base_sha'],
                           tree_digest='d' * 64, imported=True, clean_product_tree=True,
                           guard_receipt_digest=c.canonical_digest(record),
                           authority_digest=task['authority_digest'], task_contract_digest=c.canonical_digest(task))
            g.write_json(dest / 'candidate-receipt.json', receipt)
            guarded = g.restore_guarded(floor, dest)
            fake = FakeDocker(str(parent))
            box = fake.box()
            with self.assertRaises(o.OrchestratorError):
                o.run_reviewer(authority=authority, floor=floor, guarded=guarded, attempt_dir=parent / 'attempt',
                               enrollment=reviewer_enrollment(), binary=opencode_binary(), agent='orch-reviewer',
                               box=box, image='sha256:' + 'b' * 64, container_binary='/usr/local/bin/opencode',
                               implementation_family='fixture-a', limits=fake_limits())
            self.assertEqual([call[0] for call in fake.logged()], [])


def put_object(runtime, kind, payload):
    current = runtime.inspect()
    record = dict(schema_version=1, kind=kind, payload=dict(schema_version=1, **payload))
    runtime.transaction(current['sequence'], current['epoch'], lambda state: None, objects=[record])
    return c.canonical_digest(record)


class LifecycleTransitionTests(unittest.TestCase):
    """CP35: every state/action pair is a defined transition or a typed refusal."""
    def test_legal_table(self):
        for stage, actions in o.LEGAL_ACTIONS.items():
            for action in actions:
                o.check_transition(stage, action)
        for stage, action in [('READY', 'REVIEW'), ('CLAIMED', 'CLAIM'), ('PLANNED_TASK', 'CLAIM'), ('PROMOTION_READY', 'LAUNCH')]:
            with self.subTest(stage=stage, action=action), self.assertRaises(o.OrchestratorError):
                o.check_transition(stage, action)
        with self.assertRaises(o.OrchestratorError):
            o.legal_actions('UNKNOWN')

    def test_derive_stage_progression_and_refusals(self):
        fixture, authority, task = m3_task()
        with tempfile.TemporaryDirectory() as directory:
            parent = Path(directory).resolve()
            runtime = s.RuntimeStore(parent / 'runtime', authority)
            runtime.initialize()
            digest = runtime.register_task(task, copy.deepcopy(task))
            self.assertEqual(o.derive_stage(runtime, task['task_id']), 'READY')
            candidate = b.create_candidate(fixture.candidate, parent / 'worker', c.TRUSTED_DESIGN_BASE, authority=authority)
            runtime.register_candidate(task['task_id'], candidate)
            with runtime.lock('task', task['task_id']):
                stage = runtime.claim(task['task_id'], digest, owner_nonce='owner', boot_identity='boot',
                                      stage_id='stage', stage_nonce='nonce')
                self.assertEqual(o.derive_stage(runtime, task['task_id']), 'CLAIMED')
                w.reserve_launch(runtime, stage, candidate, parent, 'daemon-fake', 'sha256:' + 'b' * 64, 'IMPLEMENTATION')
                self.assertEqual(o.derive_stage(runtime, task['task_id']), 'IMPLEMENTING')
            with self.assertRaises(o.OrchestratorError):
                o.derive_stage(runtime, 'unknown-task')
            # Receipt-driven derivation uses test-only settled setup; the live
            # settle path owns the proof requirement and is covered live in M5.
            current = runtime.inspect()

            def settle_test_only(state):
                state['tasks'][task['task_id']]['status'] = 'SETTLED'
                state['tasks'][task['task_id']]['stage'] = None
                state['active_task'] = None

            runtime.transaction(current['sequence'], current['epoch'], settle_test_only)
            put_object(runtime, 'receipt', {'kind': 'guard-handoff', 'task_id': task['task_id']})
            self.assertEqual(o.derive_stage(runtime, task['task_id']), 'CANDIDATE')
            put_object(runtime, 'receipt', {'kind': 'verification-readiness', 'task_id': task['task_id'], 'verification_passed': False})
            with self.assertRaises(o.OrchestratorError):
                o.derive_stage(runtime, task['task_id'])
            runtime2 = s.RuntimeStore(parent / 'runtime2', authority)
            runtime2.initialize()
            runtime2.register_task(task, copy.deepcopy(task))
            current2 = runtime2.inspect()

            def settle_test_only2(state):
                state['tasks'][task['task_id']]['status'] = 'SETTLED'
                state['tasks'][task['task_id']]['stage'] = None
                state['active_task'] = None

            runtime2.transaction(current2['sequence'], current2['epoch'], settle_test_only2)
            put_object(runtime2, 'receipt', {'kind': 'guard-handoff', 'task_id': task['task_id']})
            put_object(runtime2, 'receipt', {'kind': 'verification-readiness', 'task_id': task['task_id'], 'verification_passed': True})
            self.assertEqual(o.derive_stage(runtime2, task['task_id']), 'LOCAL_VERIFY')
            runtime2.set_paused(True)
            self.assertEqual(o.derive_stage(runtime2, task['task_id']), 'REVIEW_PENDING')
            runtime2.set_paused(False)
            o.persist_review(runtime2, task['task_id'], passing_report(task, 'a' * 40), {'adapter': 'test'})
            self.assertEqual(o.derive_stage(runtime2, task['task_id']), 'REVIEWING')
            put_object(runtime2, 'authorization', {'task_id': task['task_id']})
            self.assertEqual(o.derive_stage(runtime2, task['task_id']), 'PROMOTION_READY')

    def test_double_admit_and_double_claim_refuse(self):
        fixture, authority, task = m3_task()
        with tempfile.TemporaryDirectory() as directory:
            runtime = s.RuntimeStore(Path(directory).resolve() / 'runtime', authority)
            runtime.initialize()
            digest = runtime.register_task(task, copy.deepcopy(task))
            with self.assertRaises(s.StoreError):
                runtime.register_task(task, copy.deepcopy(task))
            with runtime.lock('task', task['task_id']):
                runtime.claim(task['task_id'], digest, owner_nonce='owner', boot_identity='boot',
                              stage_id='stage', stage_nonce='nonce')
                with self.assertRaises(s.StoreError):
                    runtime.claim(task['task_id'], digest, owner_nonce='other', boot_identity='boot',
                                  stage_id='other', stage_nonce='other')


def _fixture_proof(stage):
    identity = b.StageIdentity('f' * 64, stage['host_boot_identity'], stage['owner_nonce'], stage['stage_id'],
                               stage['lease_epoch'], stage['stage_nonce'], 'd' * 64, 'IMPLEMENTATION', stage['task_id'])
    return b.TerminatedStageIdentity(identity, True, _seal=b._SEAL)


def bootstrap_dict(bootstrap):
    raw = {name: getattr(bootstrap, name) for name in ('source_sha', 'anchor_sha', 'base_sha', 'release_sha',
            'candidate_branch', 'authorization_id', 'task_id', 'sequence', 'nonce', 'sandbox_digest')}
    for name in ('build', 'certification', 'adoption'):
        pin = getattr(bootstrap, name)
        raw[name] = None if pin is None else {'path': pin.path, 'digest': pin.digest}
    return raw


def cli(*argv):
    proc = subprocess.run([sys.executable, str(ROOT / 'scripts/model_orchestrator/__main__.py'), *argv],
                          cwd=ROOT, capture_output=True, text=True, timeout=120)
    return proc.returncode, (c.load_json_strict(proc.stdout.strip()) if proc.stdout.strip() else None)


class CLIExitTests(unittest.TestCase):
    """CP35: the CLI distinguishes success, refusal, unavailability, and conflict."""
    def context(self, directory):
        fixture = shared_provenance()
        authority = fixture.authorities['M3']
        payload = c._release_authority(authority)
        build = payload['build']
        task = valid_task()
        task.update(task_id=build['task_id'], checkpoint_id='M3', base_sha=build['base_sha'],
                    candidate_branch=build['candidate_branch'],
                    authority_digest=payload['git']['authority_digest'])
        parent = Path(directory).resolve()
        bootstrap_path = parent / 'bootstrap.json'
        bootstrap_path.write_text(json.dumps(bootstrap_dict(fixture.bootstrap('M3'))))
        task_path = parent / 'task.json'
        task_path.write_text(json.dumps(task))
        base = ['--bootstrap', str(bootstrap_path), '--candidate-root', str(fixture.candidate),
                '--controller-root', str(fixture.controller), '--runtime', str(parent / 'runtime')]
        return base, task

    def test_admit_claim_snapshot_exit_codes(self):
        with tempfile.TemporaryDirectory() as directory:
            base, task = self.context(directory)
            code, payload = cli(*base, 'admit', '--task', str(Path(directory).resolve() / 'task.json'),
                                '--template', str(Path(directory).resolve() / 'task.json'))
            self.assertEqual((code, payload['status'], payload['stage']), (0, 'OK', 'READY'))
            code, payload = cli(*base, 'admit', '--task', str(Path(directory).resolve() / 'task.json'),
                                '--template', str(Path(directory).resolve() / 'task.json'))
            self.assertEqual(code, 4)
            self.assertEqual(payload['status'], 'CONFLICT')
            code, payload = cli(*base, 'claim', '--task-id', task['task_id'], '--owner', 'owner', '--boot', 'boot',
                                '--stage-id', 'stage', '--stage-nonce', 'nonce')
            self.assertEqual((code, payload['status'], payload['stage']), (0, 'OK', 'CLAIMED'))
            code, payload = cli(*base, 'claim', '--task-id', task['task_id'], '--owner', 'other', '--boot', 'boot',
                                '--stage-id', 'other', '--stage-nonce', 'other')
            self.assertEqual(code, 4)
            code, payload = cli(*base, 'claim', '--task-id', 'unknown-task', '--owner', 'o', '--boot', 'b',
                                '--stage-id', 's', '--stage-nonce', 'n')
            self.assertEqual(code, 2)
            code, payload = cli(*base, 'snapshot', '--plan-checkpoint', 'M3', '--next-action', 'LAUNCH',
                                '--health', 'ok', '--task-id', task['task_id'])
            self.assertEqual((code, payload['status'], payload['snapshot']['operational_stage']), (0, 'OK', 'CLAIMED'))

    def test_launch_enrollment_pin_and_budget_checked_before_claim(self):
        """Actual CLI re-reads the immutable admitted task; file/checksum cannot grant enrollment."""
        for defect in ('not_enrolled', 'mutated_price', 'wrong_operator', 'wrong_pin', 'over_budget'):
            with self.subTest(defect=defect), tempfile.TemporaryDirectory() as directory:
                base, task = self.context(directory)
                parent = Path(directory).resolve()
                record = dict(worker_enrollment(price=5 if defect == 'over_budget' else 0))
                enrolled_digest = c.canonical_digest(record)
                task['role_enrollment_ids'] = [enrolled_digest] if defect != 'not_enrolled' else ['fixture']
                (parent / 'task.json').write_text(json.dumps(task))
                code, payload = cli(*base, 'admit', '--task', str(parent / 'task.json'), '--template', str(parent / 'task.json'))
                self.assertEqual(code, 0, payload)
                if defect == 'mutated_price':
                    record['pricing_observation']['retry_cost_microusd'] = 1
                if defect == 'wrong_operator':
                    record['operator_adoption_identity'] = 'candidate-issued'
                enrollment_path = parent / 'enrollment.json'
                enrollment_path.write_text(json.dumps(record))
                requested_pin = 'f' * 64 if defect == 'wrong_pin' else c.canonical_digest(record)
                code, payload = cli(*base, 'claim', '--task-id', task['task_id'], '--owner', 'owner',
                                    '--boot', 'boot', '--stage-id', 'stage', '--stage-nonce', 'nonce', '--launch',
                                    '--enrollment', str(enrollment_path), '--enrollment-digest', requested_pin,
                                    '--availability', json.dumps({record['model_id']: 'available'}))
                expected = (3, 'UNAVAILABLE') if defect == 'over_budget' else (2, 'REFUSED')
                self.assertEqual((code, payload['status']), expected, payload)
                state = json.loads((parent / 'runtime/state.json').read_text())
                self.assertEqual(state['tasks'][task['task_id']]['status'], 'READY')
                self.assertIsNone(state['tasks'][task['task_id']]['stage'])

    def test_snapshot_missing_runtime_is_absent_and_pure(self):
        with tempfile.TemporaryDirectory() as directory:
            base, task = self.context(directory)
            runtime = Path(directory).resolve() / 'runtime'
            code, payload = cli(*base, 'snapshot', '--plan-checkpoint', 'M3', '--next-action', 'ADMIT', '--health', 'ok')
            self.assertEqual((code, payload['snapshot']['runtime']), (0, 'ABSENT'))
            self.assertFalse(runtime.exists())

    def test_admit_rewritten_checkpoint_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            base, task = self.context(directory)
            bad = copy.deepcopy(task)
            bad['checkpoint_id'] = 'M4'
            bad_path = Path(directory).resolve() / 'bad.json'
            bad_path.write_text(json.dumps(bad))
            code, payload = cli(*base, 'admit', '--task', str(bad_path), '--template', str(bad_path))
            self.assertEqual(code, 2)
            self.assertEqual(payload['status'], 'REFUSED')

    def test_escalate_packet_recorded_and_bad_kind_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            base, task = self.context(directory)
            parent = Path(directory).resolve()
            code, payload = cli(*base, 'admit', '--task', str(parent / 'task.json'), '--template', str(parent / 'task.json'))
            self.assertEqual(code, 0)
            facts = parent / 'facts.json'
            facts.write_text(json.dumps({'exit': 23}))
            code, payload = cli(*base, 'escalate', '--task-id', task['task_id'], '--kind', 'difficult_root_cause',
                                '--failure-facts', str(facts), '--question', 'why?', '--scope', 'diagnose')
            self.assertEqual((code, payload['status']), (0, 'OK'))
            self.assertTrue(payload['packet_digest'])
            code, payload = cli(*base, 'escalate', '--task-id', task['task_id'], '--kind', 'formatting',
                                '--failure-facts', str(facts), '--question', 'why?', '--scope', 'x')
            self.assertEqual(code, 2)


class AgentDocTests(unittest.TestCase):
    """Agent configs deny shell/edit/dispatch where the frozen boundary requires it."""
    def read(self, name):
        text = (ROOT / '.opencode/agents' / name).read_text(encoding='utf-8')
        _, _, body = text.split('---', 2)
        return body

    def frontmatter(self, name):
        text = (ROOT / '.opencode/agents' / name).read_text(encoding='utf-8')
        _, front, _ = text.split('---', 2)
        return front

    def test_dispatcher_has_no_tools(self):
        front = self.frontmatter('model-dispatcher.md')
        for key in ('edit: deny', 'task: deny', 'external_directory: deny', '"*": deny'):
            self.assertIn(key, front)
        self.assertNotIn('allow', front)
        self.assertNotIn('model:', front)

    def test_worker_is_bounded_without_globs_or_dispatch(self):
        front = self.frontmatter('orch-worker.md')
        self.assertIn('task: deny', front)
        self.assertIn('external_directory: deny', front)
        allows = [line for line in front.splitlines() if 'allow' in line]
        self.assertTrue(allows)
        self.assertFalse(any('*' in line for line in allows))
        body = self.read('orch-worker.md')
        self.assertNotIn('opencode models', body)
        for token in ('opencode run', 'codex exec', '`opencode`', 'subagent'):
            self.assertNotIn(token, body)
        self.assertNotIn('model:', front)

    def test_reviewer_is_read_only(self):
        front = self.frontmatter('orch-reviewer.md')
        for key in ('edit: deny', 'task: deny', '"*": deny'):
            self.assertIn(key, front)
        self.assertNotIn('allow', front)
        body = self.read('orch-reviewer.md')
        self.assertIn('no test execution', body)
        self.assertNotIn('model:', front)


if __name__ == '__main__':
    unittest.main(verbosity=2)
