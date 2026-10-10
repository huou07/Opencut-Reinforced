#!/usr/bin/env python3
"""Negative checks of report validation; these synthetic reports are not acceptance."""
import copy
import re
import unittest
from pathlib import Path
from verify_android_saf_preview import CHECKS, verify


def report():
    return {'androidSafAcceptance': {
        'checks': dict.fromkeys(CHECKS, True), 'providerUid': 10001, 'appUid': 10002,
        'sourceUri': 'content://dev.opencut.saffixture.documents/document/late65',
        'mediaImportSourceUris': [
            'content://dev.opencut.saffixture.documents/document/media',
            'content://dev.opencut.saffixture.documents/document/media-second',
            'content://dev.opencut.saffixture.documents/document/media-audio',
        ],
        'mediaImportAudioOnlyPcmWav': True,
        'mediaImportH264AacMp4': True,
        'audioPlaybackErrorCode': '', 'audioPlaybackClockNumerator': '1',
        'audioPausedPositionNumerator': '1',
        'mediaImportMicros': 1000, 'mediaImportRevision': '2', 'projectRevision': '1',
        'providerProjectSha256AtSeed': 'a' * 64,
        'providerProjectSha256BeforeRestart': 'a' * 64,
        'visiblePixelRgba': [254, 0, 0, 255], 'backgroundResumePixelRgba': [254, 0, 0, 255],
        'recoveredPixelRgba': [254, 0, 0, 255],
        'surfaceLifecycleResources': {'cleanupDelta': 1, 'restorationDelta': 1, 'cleanups': 1, 'restorations': 1},
        'providerOpens': 4, 'uiPlayMicros': 100,
        'sameSourceRegistrations': [1, 1], 'sameSourceProviderOpens': [2, 2],
        'journeyResources': {'maxFrameAcquireMicros': 1, 'totalFrameAcquireMicros': 1,
                             'maxBitmapCopyMicros': 1, 'totalBitmapCopyMicros': 1,
                             'bitmapAllocations': 1, 'maxBitmapAllocationMicros': 1,
                             'maxSurfaceResizeMicros': 1, 'maxCanvasLockMicros': 1,
                             'maxCanvasDrawMicros': 1, 'maxCanvasPostMicros': 1},
        'stressResources': {'presentedFrames': 1, 'bitmapBytes': 1024, 'peakPendingFrameResults': 8,
                            'peakQueuedOperations': 8, 'pendingPresentations': 0, 'inFlightLeases': 0,
                            'duplicatedMediaFds': 1, 'maxFrameAcquireMicros': 1,
                            'totalFrameAcquireMicros': 1, 'maxBitmapCopyMicros': 1,
                            'totalBitmapCopyMicros': 1, 'bitmapAllocations': 1,
                            'maxBitmapAllocationMicros': 1, 'maxSurfaceResizeMicros': 1,
                            'maxCanvasLockMicros': 1, 'maxCanvasDrawMicros': 1,
                            'maxCanvasPostMicros': 1},
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
    def test_workflow_requires_screenshots_emitted_by_the_real_saf_journey(self):
        root = Path(__file__).resolve().parents[1]
        workflow = (root / '.github/workflows/platform-verification.yml').read_text()
        journey = (root / 'apps/or_app/integration_test/android_saf_preview_test.dart').read_text()
        match = re.search(r'for image in ([a-z0-9 -]+); do', workflow)
        self.assertIsNotNone(match)
        required = set(match.group(1).split())
        emitted = set(re.findall(r"['\"](saf-[a-z0-9-]+)['\"]", journey))
        self.assertTrue(required <= emitted)
        self.assertIn('saf-background-resumed-texture', required)

    def test_complete_report_shape(self):
        self.assertIs(verify(report())['checks']['visibleTexturePixels'], True)

    def test_missing_assertion_and_wrong_uid_are_rejected(self):
        for mutate in (lambda data: data['checks'].pop('nativeDocumentsUiAndEditorControls'),
                       lambda data: data['checks'].pop('mobileSelectedClipInspectorSheet'),
                       lambda data: data['checks'].pop('androidSafExportToDocumentsUi'),
                       lambda data: data['checks'].pop('androidAudioTrackPlaybackClock'),
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

    def test_viewer_stage_measurements_are_required_and_nonnegative(self):
        for field, invalid in (
            ('maxFrameAcquireMicros', None),
            ('totalFrameAcquireMicros', -1),
            ('maxBitmapCopyMicros', 'unknown'),
            ('totalBitmapCopyMicros', -1),
            ('bitmapAllocations', None),
            ('maxBitmapAllocationMicros', -1),
            ('maxSurfaceResizeMicros', -1),
            ('maxCanvasLockMicros', None),
            ('maxCanvasDrawMicros', 'unknown'),
            ('maxCanvasPostMicros', -1),
        ):
            value = report()
            value['androidSafAcceptance']['journeyResources'][field] = invalid
            with self.assertRaises(ValueError): verify(value)

    def test_background_resume_without_surface_destruction_is_valid(self):
        value = report()
        value['androidSafAcceptance']['surfaceLifecycleResources'].update(
            cleanupDelta=0, restorationDelta=0, cleanups=0, restorations=0,
        )
        self.assertIs(verify(value)['checks']['visibleTexturePixels'], True)

    def test_surface_cleanup_requires_restoration_before_reuse(self):
        value = report()
        value['androidSafAcceptance']['surfaceLifecycleResources']['restorationDelta'] = 0
        with self.assertRaises(ValueError): verify(value)

    def test_android_export_requires_valid_provider_matroska(self):
        value = report()
        value['androidSafAcceptance']['exportValidMatroska'] = False
        with self.assertRaises(ValueError): verify(value)

    def test_android_media_import_requires_mp4_and_audio_only_wav(self):
        for invalid in (False, None):
            value = report()
            value['androidSafAcceptance']['mediaImportAudioOnlyPcmWav'] = invalid
            with self.assertRaises(ValueError): verify(value)
        value = report()
        value['androidSafAcceptance']['mediaImportH264AacMp4'] = False
        with self.assertRaises(ValueError): verify(value)
        value = report()
        value['androidSafAcceptance']['mediaImportSourceUris'].pop()
        with self.assertRaises(ValueError): verify(value)

    def test_android_audio_output_requires_advancing_device_master_clock(self):
        for mutation in (
            {'audioPlaybackErrorCode': 'AUDIO_OUTPUT_UNAVAILABLE'},
            {'audioPlaybackClockNumerator': '0'},
            {'audioPausedPositionNumerator': '0'},
        ):
            value = report()
            value['androidSafAcceptance'].update(mutation)
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
