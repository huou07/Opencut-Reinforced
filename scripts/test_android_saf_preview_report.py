#!/usr/bin/env python3
"""Negative checks of report validation; these synthetic reports are not acceptance."""
import copy
import unittest
from verify_android_saf_preview import CHECKS, verify


def report():
    return {'androidSafAcceptance': {
        'checks': dict.fromkeys(CHECKS, True), 'providerUid': 10001, 'appUid': 10002,
        'sourceUri': 'content://dev.opencut.saffixture.documents/document/late65',
        'visiblePixelRgba': [254, 0, 0, 255], 'recoveredPixelRgba': [254, 0, 0, 255],
        'recreatedPixelRgba': [254, 0, 0, 255],
        'providerOpens': 4, 'uiPlayMicros': 100,
        'sameSourceRegistrations': [1, 1], 'sameSourceProviderOpens': [2, 2],
        'stressResources': {'presentedFrames': 1, 'bitmapBytes': 1024, 'peakPendingFrameResults': 8,
                            'peakQueuedOperations': 8, 'pendingPresentations': 0, 'inFlightLeases': 0, 'duplicatedMediaFds': 1},
        'finalResources': {'duplicatedMediaFds': 0, 'inFlightLeases': 0, 'latestFrameBytes': 0},
        'exportBytes': 4096, 'exportValidMatroska': True,
        'finalOsMediaFds': 0, 'softwareFallback': 'packaged_ffmpeg_shared_render_bounded_bgra', 'hardware': 'UNVERIFIED',
    }}


class ReportTest(unittest.TestCase):
    def test_complete_report_shape(self):
        self.assertIs(verify(report())['checks']['visibleTexturePixels'], True)

    def test_missing_assertion_and_wrong_uid_are_rejected(self):
        for mutate in (lambda data: data['checks'].pop('nativeDocumentsUiAndEditorControls'),
                       lambda data: data['checks'].pop('mobileSelectedClipInspectorSheet'),
                       lambda data: data['checks'].pop('androidSafExportToDocumentsUi'),
                       lambda data: data.update(providerUid=data['appUid'])):
            value = report()
            mutate(value['androidSafAcceptance'])
            with self.assertRaises(ValueError): verify(value)

    def test_leak_and_queue_growth_are_rejected(self):
        original = report()
        for field, key, value in [('finalResources', 'inFlightLeases', 1), ('finalResources', 'duplicatedMediaFds', 1),
                                  ('stressResources', 'peakPendingFrameResults', 9), ('stressResources', 'peakQueuedOperations', 9)]:
            changed = copy.deepcopy(original)
            changed['androidSafAcceptance'][field][key] = value
            with self.assertRaises(ValueError): verify(changed)

    def test_black_texture_and_real_os_fd_leak_are_rejected(self):
        for mutation in ({'visiblePixelRgba': [0, 0, 0, 255]}, {'finalOsMediaFds': 1}):
            value = report()
            value['androidSafAcceptance'].update(mutation)
            with self.assertRaises(ValueError): verify(value)

    def test_recreated_surface_must_present_the_frame_not_the_backdrop(self):
        value = report()
        value['androidSafAcceptance']['recreatedPixelRgba'] = [254, 247, 255, 255]
        with self.assertRaises(ValueError): verify(value)

    def test_android_export_requires_valid_provider_matroska(self):
        value = report()
        value['androidSafAcceptance']['exportValidMatroska'] = False
        with self.assertRaises(ValueError): verify(value)


if __name__ == '__main__': unittest.main()
