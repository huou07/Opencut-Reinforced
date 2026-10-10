#!/usr/bin/env python3
"""Check the report produced only after the real Android SAF journey assertions."""
import argparse
import json
from pathlib import Path

CHECKS = {
    'nativeDocumentsUiAndEditorControls', 'safMediaImportThroughDocumentsUi', 'androidAudioTrackPlaybackClock', 'visibleTexturePixels', 'externalUidPermissionEnforcement',
    'activeLateSourceBeyond64', 'unchangedProjectRevision', 'playWithoutSeek', 'revokedPermissionUiRecovery',
    'missingPartialOpenRollbackAndRecovery', 'nonseekableNativeRegistrationRejected', 'clearDuringOpenDropsStaleBinding',
    'editAndGenerationDropPreparedFrame', 'surfaceLifecycleCallbacksConsistent', 'osMediaFdsAndNativeLeasesReleased',
    'boundedPresentationStress',
    'sameSourceSeeksReuseProviderCapability',
    'foregroundBackgroundPlaybackPausesAndSurfaceRecovers',
    'mobileTouchScrubbingAndTransport', 'mobileMediaLibrarySheet',
    'mobileSelectedClipInspectorSheet',
    'androidSafExportToDocumentsUi',
    'androidSafCaptionImportAndExportThroughDocumentsUi',
    'androidSafMediaRelinkThroughDocumentsUi',
    'recoveryCheckpointPersistedBeforeProcessStop',
}


def verify(report):
    data = report['androidSafAcceptance']
    checks = data['checks']
    if set(checks) != CHECKS or any(checks[key] is not True for key in CHECKS):
        raise ValueError('Missing or failed real-journey assertion')
    if data['providerUid'] == data['appUid']:
        raise ValueError('The provider must have a separate Android UID')
    if data['sourceUri'] != 'content://dev.opencut.saffixture.documents/document/late65':
        raise ValueError('Expected actual late-library provider URI')
    for field in (
        'visiblePixelRgba',
        'backgroundResumePixelRgba',
        'recoveredPixelRgba',
    ):
        r, g, b, a = data[field]
        if not (200 <= r <= 255 and 0 <= g <= 40 and 0 <= b <= 40 and a == 255):
            raise ValueError('Composed Flutter Texture pixels were not the fixture frame')
    lifecycle = data.get('surfaceLifecycleResources', {})
    cleanup_delta = lifecycle.get('cleanupDelta')
    restoration_delta = lifecycle.get('restorationDelta')
    if (not isinstance(cleanup_delta, int) or cleanup_delta < 0
            or not isinstance(restoration_delta, int) or restoration_delta < 0
            or (cleanup_delta > 0 and restoration_delta == 0)):
        raise ValueError('An Android surface cleanup was not followed by restoration')
    if data['providerOpens'] <= 0 or data['uiPlayMicros'] <= 0:
        raise ValueError('Provider and actual Play measurements are required')
    for field in ('providerProjectSha256AtSeed', 'providerProjectSha256BeforeRestart'):
        digest = data.get(field)
        if not isinstance(digest, str) or len(digest) != 64 or any(c not in '0123456789abcdef' for c in digest):
            raise ValueError('Provider project digest observations are required')
    if data['exportBytes'] <= 4 or data['exportValidMatroska'] is not True:
        raise ValueError('Android SAF export must produce a non-empty Matroska file in the selected provider')
    if (data.get('mediaImportSourceUris') != [
                'content://dev.opencut.saffixture.documents/document/media',
                'content://dev.opencut.saffixture.documents/document/media-second',
                'content://dev.opencut.saffixture.documents/document/media-audio',
            ]
            or data.get('mediaImportAudioOnlyPcmWav') is not True
            or data.get('mediaImportMicros', 0) <= 0
            or int(data.get('mediaImportRevision', 0)) <= int(data['projectRevision'])):
        raise ValueError('Android SAF import must persist the selected source and report its runtime measurement')
    if (data.get('audioPlaybackErrorCode') != ''
            or int(data.get('audioPlaybackClockNumerator', 0)) <= 0
            or int(data.get('audioPausedPositionNumerator', 0)) <= 0):
        raise ValueError('Android audio-track playback must advance the device-master clock without an output error')
    if (data.get('captionImportCalls') != 1 or data.get('captionExportCalls') != 1
            or int(data.get('captionImportRevision', 0)) <= int(data.get('mediaImportRevision', 0))
            or data.get('captionExportBytes', 0) <= 0
            or data.get('captionExportValidSrt') is not True):
        raise ValueError('Android caption SRT import and provider export must complete through the real app')
    relink_id = data.get('mediaRelinkMediaIdBefore')
    if (data.get('mediaRelinkCalls') != 1
            or data.get('mediaRelinkSucceeded') is not True
            or data.get('mediaRelinkErrorCode') != ''
            or not isinstance(relink_id, str)
            or relink_id != '00000041-2222-4222-8222-222222222222'
            or relink_id != data.get('mediaRelinkMediaIdAfter')
            or data.get('mediaRelinkSourceBefore') != 'content://dev.opencut.saffixture.documents/document/late65'
            or data.get('mediaRelinkSourceAfter') != 'content://dev.opencut.saffixture.documents/document/relink-replacement'
            or data.get('mediaRelinkTimelineReferencePreserved') is not True
            or int(data.get('mediaRelinkResultRevision', 0)) <= int(data.get('captionImportRevision', 0))
            or int(data.get('mediaRelinkRevision', 0)) <= int(data.get('captionImportRevision', 0))):
        raise ValueError('Android SAF relink must preserve the media and timeline identity and persist a new source')
    for field in ('sameSourceRegistrations', 'sameSourceProviderOpens'):
        before, after = data[field]
        if before <= 0 or before != after:
            raise ValueError('An unchanged active source was reopened during repeated seeks')
    stress = data['stressResources']
    if not (stress['presentedFrames'] > 0 and 0 < stress['bitmapBytes'] <= 1920 * 1080 * 4
            and stress['peakPendingFrameResults'] <= 8 and stress['peakQueuedOperations'] <= 8
            and stress['pendingPresentations'] == 0 and stress['inFlightLeases'] == 0
            and stress['duplicatedMediaFds'] == 1):
        raise ValueError('Presentation or resource stress bounds failed')
    final = data['finalResources']
    if any(final[key] != 0 for key in ('duplicatedMediaFds', 'inFlightLeases', 'latestFrameBytes')) or data['finalOsMediaFds'] != 0:
        raise ValueError('Real media FDs or viewer leases remained after project clear')
    if data['softwareFallback'] != 'packaged_ffmpeg_shared_render_bounded_bgra' or data['hardware'] != 'UNVERIFIED':
        raise ValueError('The software/hardware evidence class changed')
    return data


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('report', type=Path)
    args = parser.parse_args()
    data = verify(json.loads(args.report.read_text()))
    print('Android SAF picker/import/audio-playback/export/pixel/resource assertions verified.')
    print(json.dumps({key: data[key] for key in (
        'uiPlayMicros', 'mediaImportMicros', 'audioPlaybackClockNumerator',
        'audioPlaybackErrorCode', 'audioPausedPositionNumerator', 'exportBytes',
        'journeyResources', 'stressResources', 'finalResources',
    )}, indent=2))
