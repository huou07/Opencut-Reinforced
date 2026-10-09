#!/usr/bin/env python3
"""Negative checks of report validation; these synthetic reports are not acceptance."""
import copy
import unittest
from verify_android_saf_preview import CHECKS, verify


def report():
    return {'androidSafAcceptance': {
        'checks': dict.fromkeys(CHECKS, True), 'providerUid': 10001, 'appUid': 10002,
        'sourceUri': 'content://dev.opencut.saffixture.documents/document/late65',
        'mediaImportSourceUris': [
            'content://dev.opencut.saffixture.documents/document/media',
            'content://dev.opencut.saffixture.documents/document/media-second',
        ],
        'mediaImportMicros': 1000, 'mediaImportRevision': '2', 'projectRevision': '1',
        'providerProjectSha256AtSeed': 'a' * 64,
        'providerProjectSha256BeforeRestart': 'a' * 64,
        'visiblePixelRgba': [254, 0, 0, 255], 'backgroundResumePixelRgba': [254, 0, 0, 255],
        'recoveredPixelRgba': [254, 0, 0, 255],
        'recreatedPixelRgba': [254, 0, 0, 255],
        'providerOpens': 4, 'uiPlayMicros': 100,
        'sameSourceRegistrations': [1, 1], 'sameSourceProviderOpens': [2, 2],
        'stressResources': {'presentedFrames': 1, 'bitmapBytes': 1024, 'peakPendingFrameResults': 8,
                            'peakQueuedOperations': 8, 'pendingPresentations': 0, 'inFlightLeases': 0, 'duplicatedMediaFds': 1},
        'finalResources': {'duplicatedMediaFds': 0, 'inFlightLeases': 0, 'latestFrameBytes': 0},
        'exportBytes': 4096, 'exportValidMatroska': True,
        'captionImportRevision': '3', 'captionImportCalls': 1,
        'captionExportCalls': 1, 'captionExportBytes': 128,
        'captionExportValidSrt': True,
        'mediaRelinkCalls': 1,
        'mediaRelinkSucceeded': True,
        'mediaRelinkErrorCode': '',
        'mediaRelinkResultRevision': '4',
        'mediaRelinkMediaIdBefore': '00000041-2222-4222-8222-222222222222',
        'mediaRelinkMediaIdAfter': '00000041-2222-4222-8222-222222222222',
        'mediaRelinkSourceBefore': 'content://dev.opencut.saffixture.documents/document/late65',
        'mediaRelinkSourceAfter': 'content://dev.opencut.saffixture.documents/document/relink-replacement',
        'mediaRelinkTimelineReferencePreserved': True,
        'mediaRelinkRevision': '4',
        'finalOsMediaFds': 0, 'softwareFallback': 'packaged_ffmpeg_shared_render_bounded_bgra', 'hardware': 'UNVERIFIED',
    }}


class ReportTest(unittest.TestCase):
    def test_complete_report_shape(self):
        self.assertIs(verify(report())['checks']['visibleTexturePixels'], True)

    def test_missing_assertion_and_wrong_uid_are_rejected(self):
        for mutate in (lambda data: data['checks'].pop('nativeDocumentsUiAndEditorControls'),
                       lambda data: data['checks'].pop('mobileSelectedClipInspectorSheet'),
                       lambda data: data['checks'].pop('androidSafExportToDocumentsUi'),
                       lambda data: data['checks'].pop('foregroundBackgroundPlaybackPausesAndSurfaceRecovers'),
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
        for mutation in ({'visiblePixelRgba': [0, 0, 0, 255]}, {'backgroundResumePixelRgba': [0, 0, 0, 255]},
                         {'finalOsMediaFds': 1}):
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

    def test_android_caption_interchange_requires_real_import_and_export(self):
        value = report()
        value['androidSafAcceptance']['captionExportValidSrt'] = False
        with self.assertRaises(ValueError): verify(value)

    def test_android_relink_requires_persisted_source_and_timeline_identity(self):
        for mutation in (
            {'mediaRelinkCalls': 0},
            {'mediaRelinkSucceeded': False},
            {'mediaRelinkErrorCode': 'REVISION_CONFLICT'},
            {'mediaRelinkResultRevision': '3'},
            {'mediaRelinkMediaIdBefore': '00000001-2222-4222-8222-222222222222'},
            {'mediaRelinkMediaIdAfter': '00000002-2222-4222-8222-222222222222'},
            {'mediaRelinkTimelineReferencePreserved': False},
            {'mediaRelinkSourceAfter': 'content://dev.opencut.saffixture.documents/document/media-second'},
            {'mediaRelinkSourceAfter': 'content://dev.opencut.saffixture.documents/document/late65'},
        ):
            value = report()
            value['androidSafAcceptance'].update(mutation)
            with self.assertRaises(ValueError): verify(value)


if __name__ == '__main__': unittest.main()
