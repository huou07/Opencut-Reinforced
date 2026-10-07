#!/usr/bin/env python3
"""Check the report produced only after the real Android SAF journey assertions."""
import argparse
import json
from pathlib import Path

CHECKS = {
    'nativeDocumentsUiAndEditorControls', 'visibleTexturePixels', 'externalUidPermissionEnforcement',
    'activeLateSourceBeyond64', 'unchangedProjectRevision', 'playWithoutSeek', 'revokedPermissionUiRecovery',
    'missingPartialOpenRollbackAndRecovery', 'nonseekableNativeRegistrationRejected', 'clearDuringOpenDropsStaleBinding',
    'editAndGenerationDropPreparedFrame', 'surfaceRecreationAndRelease', 'osMediaFdsAndNativeLeasesReleased',
    'boundedPresentationStress',
    'sameSourceSeeksReuseProviderCapability',
    'mobileTouchScrubbingAndTransport', 'mobileMediaLibrarySheet',
    'mobileSelectedClipInspectorSheet',
    'androidSafExportToDocumentsUi',
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
    for field in ('visiblePixelRgba', 'recoveredPixelRgba', 'recreatedPixelRgba'):
        r, g, b, a = data[field]
        if not (200 <= r <= 255 and 0 <= g <= 40 and 0 <= b <= 40 and a == 255):
            raise ValueError('Composed Flutter Texture pixels were not the fixture frame')
    if data['providerOpens'] <= 0 or data['uiPlayMicros'] <= 0:
        raise ValueError('Provider and actual Play measurements are required')
    if data['exportBytes'] <= 4 or data['exportValidMatroska'] is not True:
        raise ValueError('Android SAF export must produce a non-empty Matroska file in the selected provider')
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
    print('Android SAF picker/editor/export/pixel/resource assertions verified.')
    print(json.dumps({key: data[key] for key in ('uiPlayMicros', 'exportBytes', 'journeyResources', 'stressResources', 'finalResources')}, indent=2))
