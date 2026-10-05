"""Operational admission: external release adoption AND a separately pinned product task.

Control and product history share the operator anchor, not each other's HEAD.
Product snapshots stay immutable while the distinct integration clone advances.
"""
from pathlib import Path
import copy
import hashlib

from . import contracts as c


def release_active(authority):
    facts = c._release_authority(authority)
    cert, adoption = facts['certification'], facts['adoption']
    if cert is None or adoption is None:
        raise c.ContractError('product execution requires externally certified adoption')
    record = c.proposal_record(
        build_authorization_digest=c.canonical_digest(facts['build']),
        implementation_phase='M5', completed_phases=list(c.IMPLEMENTATION_PHASES),
        acceptance_cases_passed=list(c.ACCEPTANCE_CASE_IDS), certification='CERTIFIED_ACTIVE',
        certified_release_sha=cert['release_sha'], operational_adoption='ADOPTED',
        authority_kind='V2_FROZEN_CONTROL_RELEASE', adopted_release_sha=adoption['certified_release_sha'],
        blocking_limitations=cert['blocking_limitations'], lifecycle_state='CERTIFIED_ACTIVE',
        full_auto_eligible=True, **{k: adoption[k] for k in ('parent_sha', 'plan_digest', 'state_digest')})
    record.update({k + '_digest': cert[k]['digest'] for k in
                   ('live_certification', 'independent_review', 'qualified_models')})
    if not c.full_auto_eligible(record, schemas=facts['schemas'], external=authority):
        raise c.ContractError('incomplete or stale adopted release')
    c.validate_authority_source('V2_FROZEN_CONTROL_RELEASE', cert['release_sha'], external=authority)
    return facts


def operational_manifest(authority, product_root, base_sha):
    """Facts only; this does not mint product authority or approve a task."""
    facts = release_active(authority)
    root = c._authority_repo(Path(product_root), facts['git']['anchor_oid'])
    for other in c.authority_roots(authority):
        if root == other or root in other.parents or other in root.parents:
            raise c.ContractError('product snapshot must be independent of release/controller roots')
    c._commit(root, base_sha)
    c._ancestor(root, facts['git']['anchor_oid'], base_sha)
    if c._git(root, 'rev-parse', 'HEAD').decode().strip() != base_sha or c._git(root, 'status', '--porcelain=v1', '--untracked-files=all').strip():
        raise c.ContractError('dirty or stale product base materialization')
    base = c.build_manifest(root, base_sha, _base_input=True)
    c._verify_materialization(root, base)
    result = copy.deepcopy(facts['git'])
    result.update(base_oid=base_sha, base_manifest=base, purpose='OPERATIONAL',
                  adoption_digest=c.canonical_digest(facts['adoption']))
    result.pop('authority_digest')
    result['authority_digest'] = c.canonical_digest(result)
    return result


def _blob(authority, path):
    c._validate_relative_path(path, 'product controller record path')
    mode = c._git(Path(authority.controller_root), 'ls-tree', authority.source_sha, '--', path)
    if not mode.startswith((b'100644 blob ', b'100755 blob ')):
        raise c.ContractError('product authority requires an immutable regular Git blob')
    return c._git(Path(authority.controller_root), 'show', authority.source_sha + ':' + path)


def load_operational_authority(candidate_root, controller_root, product_root, *, bootstrap, product_pin):
    """Only the trusted host can select this typed product authorization pin."""
    if type(product_pin) is not c.RecordPin:
        raise c.ContractError('typed externally approved product task pin required')
    release = c.load_release_authority(Path(candidate_root), Path(controller_root), bootstrap=bootstrap)
    facts = release_active(release)
    auth = c.load_json_strict(_blob(release, product_pin.path).decode())
    c._shape(auth, 'product_task_authorization', facts['schemas'])
    if c.canonical_digest(auth) != product_pin.digest:
        raise c.ContractError('product authorization pin differs from Git blob')
    if auth['release_sha'] != bootstrap.release_sha or auth['adoption_digest'] != c.canonical_digest(facts['adoption']):
        raise c.ContractError('product task binds a different or stale adoption')
    root = c.resolve_authority_root(Path(product_root))
    manifest = operational_manifest(release, root, auth['product_base_sha'])
    plan_bytes = c._git(root, 'show', auth['product_base_sha'] + ':docs/execution/PLAN.json')
    state_bytes = c._git(root, 'show', auth['product_base_sha'] + ':docs/execution/STATE.json')
    if auth['plan_digest'] != hashlib.sha256(plan_bytes).hexdigest() or auth['state_digest'] != hashlib.sha256(state_bytes).hexdigest():
        raise c.ContractError('product PLAN/STATE differs from approved exact base')
    plan, state = c.load_json_strict(plan_bytes.decode()), c.load_json_strict(state_bytes.decode())
    import execution_plan
    checkpoint = execution_plan.checkpoint_for_id(plan, auth['checkpoint_id'])
    if state.get('current_next') != auth['checkpoint_id'] or state.get('checkpoints', {}).get(auth['checkpoint_id']) != 'NEXT' or sum(v == 'NEXT' for v in state.get('checkpoints', {}).values()) != 1:
        raise c.ContractError('product authorization is not exact current NEXT')
    if any(state['checkpoints'].get(p) != 'DONE' for p in checkpoint['prerequisite_checkpoint_ids']):
        raise c.ContractError('product checkpoint prerequisites are incomplete')
    execution_plan.validate_contract_versions(state['verified_contract_versions'], 'product STATE versions')
    versions = execution_plan.read_contract_versions(root)
    if versions != manifest['contract_versions']:
        raise c.ContractError('product contracts differ from the adopted supported versions')
    execution_plan.validate_contract_transition(plan, state, execution_plan.load_architecture_policy(root), versions, checkpoint_id=checkpoint['id'])
    task = c.load_json_strict(_blob(release, auth['task_path']).decode())
    catalog = c.load_json_strict(_blob(release, auth['catalog_path']).decode())
    if c.canonical_digest(task) != auth['task_digest'] or c.canonical_digest(catalog) != auth['catalog_digest']:
        raise c.ContractError('product task/catalog differs from operator authorization')
    c.validate_task_contract(task, facts['schemas'], frozen_template=task)
    expected = dict(task_kind='product_checkpoint', repository=c.REPOSITORY_IDENTITY,
                    checkpoint_id=auth['checkpoint_id'], base_sha=auth['product_base_sha'],
                    authority_digest=manifest['authority_digest'])
    c._bind(task, expected, expected)
    if set(task['evidence_classes']) != set(checkpoint['required_evidence_classes']):
        raise c.ContractError('product task omits or substitutes checkpoint evidence classes')
    effects = dict(project_schema_effect='expected_project_schema_effect_category',
                   recovery_schema_effect='expected_project_schema_effect_category', ipc_effect='expected_ipc_effect_category')
    for field, policy in effects.items():
        if task[field] != checkpoint[policy]:
            raise c.ContractError('product task changes the locked checkpoint contract effect')
    if checkpoint['spec_document'] not in task['required_documentation']:
        raise c.ContractError('product task omits its locked phase specification')
    from agent_supervisor import is_protected_execution_path
    exceptions = set(checkpoint.get('runner_allowed_protected_paths', []))
    def protected(path):
        return (is_protected_execution_path(path) or path in c.CONFIG_PATHS
                or path.startswith(('scripts/model_orchestrator/', 'docs/execution/automation/', '.opencode/')))
    if any(protected(p) and p not in exceptions for p in task['allowed_paths']):
        raise c.ContractError('product task scope includes forbidden execution-control paths')
    if auth['execution_profile'] == 'PRODUCT':
        if set(task['evidence_classes']) & {'PERFORMANCE', 'RESOURCE_STRESS'} and task['performance_applicability'] != 'applicable':
            raise c.ContractError('locked performance/resource classes require an actual measurement floor')
        if auth['plan_digest'] != facts['adoption']['plan_digest']:
            raise c.ContractError('product task changed the adopted immutable PLAN')
        accepted = {f'https://github.com/{c.REPOSITORY_IDENTITY}.git', f'git@github.com:{c.REPOSITORY_IDENTITY}.git'}
        if auth['destination_url'] not in accepted:
            raise c.ContractError('product publication must use the approved repository')
    else:
        destination = Path(auth['destination_url'])
        if not destination.is_absolute() or not destination.is_dir() or destination.is_symlink():
            raise c.ContractError('isolated product fixture requires an explicit existing local bare remote')
        if c._git(destination, 'rev-parse', '--is-bare-repository').strip() != b'true':
            raise c.ContractError('fixture publication target is not a bare repository')
    payload = dict(facts, git=manifest, operational=dict(authorization=auth, task=task, checkpoint=checkpoint))
    authority = c.ValidatedReleaseAuthority(payload, Path(release.candidate_root), Path(release.controller_root),
                                      release.source_sha, product_root=root, _seal=c._PROVENANCE_SEAL)
    from .guards import load_floor
    load_floor(authority, auth['task_path'], auth['catalog_path'])
    return authority


def verify_product_base(authority):
    """Fresh admission observes the selected publication main; later recovery uses receipts."""
    facts = c._release_authority(authority)
    if facts['git']['purpose'] != 'OPERATIONAL' or 'operational' not in facts:
        raise c.ContractError('disabled authority cannot admit product work')
    auth = facts['operational']['authorization']
    observed = c._git(c.base_source_root(authority), 'ls-remote', '--exit-code', auth['destination_url'], 'refs/heads/main').decode().split()
    if observed != [auth['product_base_sha'], 'refs/heads/main']:
        raise c.ContractError('publication main is not the exact approved product base')
    return facts


def validate_hosted_product_receipts(authority, receipt, hosted_record, *, request=None):
    """Bind completion measurements to exact-SHA GitHub artifacts and named gate steps.

    A supplied PASS JSON is never sufficient. Each required class has a bounded
    artifact `or-v2-product-TASK-CLASS` containing production-receipt.json and
    observation.json. The observation carries cases and measurements from the
    approved harness; archive and observation byte digests are checked separately.
    """
    import io
    import zipfile
    from . import hosted
    facts = c._release_authority(authority)
    if facts['operational']['authorization']['execution_profile'] != 'PRODUCT':
        raise c.ContractError('fixture receipts cannot complete product checkpoints')
    task = facts['operational']['task']
    rows = receipt['production_acceptance_receipts']
    required = {r['class_id']: r for r in task['acceptance_requirements']}
    if len(rows) != len(required) or {r['class_id'] for r in rows} != set(required):
        raise c.ContractError('production receipts do not cover the exact required classes')
    request = request or hosted.github
    prefix = '/repos/' + c.REPOSITORY_IDENTITY
    get = lambda path: c.load_json_strict(request(prefix + path).decode())
    proofs = hosted_record.get('evidence_classes', [])
    for row in rows:
        req = required[row['class_id']]
        if set(row['case_ids']) != set(req['case_ids']) or any(row[k] != req[k] for k in
                ('harness_digest', 'package_digest', 'environment_digest', 'permission_digest')):
            raise c.ContractError('production receipt differs from pinned acceptance floor')
        if row['authority_digest'] != task['authority_digest'] or not any(
                proof['class'] == row['class_id'] and proof['run_id'] == row['run_id']
                and proof['job_id'] == row['job_id'] and proof['step_name'] == row['step'] for proof in proofs):
            raise c.ContractError('production receipt lacks the exact successful hosted class step')
        run = get('/actions/runs/' + str(row['run_id']))
        if (run['head_sha'] != hosted_record['implementation_sha'] or run['run_attempt'] != row['attempt']
                or run['repository']['full_name'] != c.REPOSITORY_IDENTITY or run['conclusion'] != 'success'):
            raise c.ContractError('production artifact run is stale or binds another source')
        name = 'or-v2-product-' + task['task_id'] + '-' + row['class_id']
        artifacts = get('/actions/runs/' + str(row['run_id']) + '/artifacts?per_page=100')['artifacts']
        matches = [r for r in artifacts if r['name'] == name and not r['expired']]
        if len(matches) != 1 or matches[0]['workflow_run']['head_sha'] != hosted_record['implementation_sha']:
            raise c.ContractError('production artifact identity/uniqueness differs')
        artifact = matches[0]
        data = request(prefix + '/actions/artifacts/' + str(artifact['id']) + '/zip')
        if len(data) > hosted.MAX_ARTIFACT or artifact['digest'] != 'sha256:' + hashlib.sha256(data).hexdigest():
            raise c.ContractError('production artifact archive digest differs')
        with zipfile.ZipFile(io.BytesIO(data)) as archive:
            if set(archive.namelist()) != {'production-receipt.json', 'observation.json'} or len(archive.infolist()) != 2:
                raise c.ContractError('unexpected production artifact contents')
            if any(e.file_size > hosted.MAX_ARTIFACT or (e.external_attr >> 16 & 0o170000) == 0o120000 for e in archive.infolist()):
                raise c.ContractError('unsafe production artifact entry')
            actual = c.load_json_strict(archive.read('production-receipt.json').decode())
            observation_bytes = archive.read('observation.json')
        observation = c.load_json_strict(observation_bytes.decode())
        if actual != row or row['artifact_digest'] != hashlib.sha256(observation_bytes).hexdigest():
            raise c.ContractError('production receipt differs from downloaded measured artifact')
        expected = dict(task_id=task['task_id'], task_contract_digest=c.canonical_digest(task),
            candidate_sha=hosted_record['implementation_sha'], class_id=row['class_id'],
            **{key: row[key] for key in ('authority_digest', 'harness_digest', 'package_digest', 'environment_digest',
                                       'permission_digest', 'executed_cases', 'passed_cases', 'failed_cases',
                                       'skipped_cases', 'measurements', 'result')})
        c._bind(observation, expected, expected)
        if row['class_id'] in ('PERFORMANCE', 'RESOURCE_STRESS'):
            measures = {m['metric']: m['value'] for m in row['measurements']}
            bounds = task['performance_budgets'].get('bounds', [])
            if not bounds or any(b['metric'] not in measures or measures[b['metric']] > min(b['maximum'], b['baseline_maximum']) for b in bounds):
                raise c.ContractError('production performance/resource floor was not measured or failed')
