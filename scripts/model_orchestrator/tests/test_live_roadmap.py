"""CP44: opt-in real externally adopted multi-checkpoint roadmap acceptance."""
import os
import unittest


@unittest.skipUnless(os.environ.get('OR_V2_ROADMAP_LIVE')=='1','real external adoption and hosted roadmap inputs required')
class RoadmapLiveTests(unittest.TestCase):
    """CP44: one operator delegation, real two-checkpoint worker/reviewer/supervisor journey."""
    def test_two_checkpoints_under_one_operator_delegation(self):
        # The external prepared host supplies immutable runtime/capability roots;
        # the test module owns the actual assertions and invokes the real engine.
        from model_orchestrator.roadmap import run_roadmap
        from model_orchestrator.__main__ import bootstrap_from_dict
        from model_orchestrator import contracts as c,roadmap as r
        from pathlib import Path
        cfg=c.load_json_strict(Path(os.environ['OR_V2_ROADMAP_INPUTS']).read_text())
        pin=r.RoadmapPin(cfg['roadmap_pin']['source_sha'],c.RecordPin(**cfg['roadmap_pin']['delegation']))
        result=run_roadmap(Path(cfg['candidate_root']),Path(cfg['controller_root']),Path(cfg['product_root']),bootstrap_from_dict(cfg['bootstrap']),pin,Path(cfg['runtime_root']))
        self.assertEqual(result['result'],'ROADMAP_COMPLETE')
        self.assertEqual(result['completed'],2)
        state=c.load_json_strict((Path(cfg['product_root'])/'docs/execution/STATE.json').read_text())
        self.assertIsNone(state['current_next'])
        self.assertEqual(state['checkpoints'],{'OR-A':'DONE','OR-B':'DONE'})
