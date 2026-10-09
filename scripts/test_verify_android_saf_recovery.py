from pathlib import Path
import unittest

from scripts.verify_android_saf_recovery import verify


class AndroidSafRecoveryReportTest(unittest.TestCase):
    def setUp(self):
        self.process = {
            "package": "io.github.huou07.or_app",
            "oldPid": 120,
            "emptyAfterForceStop": True,
            "newPid": 128,
            "result": "PASS",
        }
        self.prepared = {
            "androidSafAcceptance": {
                "checks": {"recoveryCheckpointPersistedBeforeProcessStop": True},
                "recoveryName": "Process recovery acceptance",
                "recoveryBaseRevision": "2",
                "recoveryRevision": "3",
                "recoveryKindAfterClose": "candidate",
            }
        }
        self.recovered = {
            "androidSafRecoveryAcceptance": {
                "checks": {
                    "nativeDocumentsUiReopenedSameSafProject": True,
                    "recoveryCandidateShownAfterProcessRestart": True,
                    "explicitRecoveryApplied": True,
                    "recoverySidecarRemovedAfterApply": True,
                    "previewPlaybackResumedFromRecoveredProject": True,
                    "relinkedMediaAndTimelineRecovered": True,
                    "relinkedMediaGrantSurvivedProcessRestart": True,
                    "noFlutterException": True,
                },
                "relinkedMediaId": "00000041-2222-4222-8222-222222222222",
                "relinkedMediaSource": "content://dev.opencut.saffixture.documents/document/relink-replacement",
                "projectPath": "/data/user/0/io.github.huou07.or_app/files/or-projects/project.orproj",
                "providerProjectSha256BeforeOpen": "a" * 64,
                "projectName": "Process recovery acceptance",
                "projectRevision": "3",
                "recoveryKindAfterApply": "none",
                "frameSequence": "1",
            }
        }

    def test_complete_process_recovery_journey_is_accepted(self):
        verify(self.process, self.prepared, self.recovered)

    def test_android_data_alias_is_accepted_for_app_private_working_copy(self):
        self.recovered["androidSafRecoveryAcceptance"]["projectPath"] = (
            "/data/data/io.github.huou07.or_app/files/or-projects/project.orproj"
        )
        verify(self.process, self.prepared, self.recovered)

    def test_working_copy_outside_app_private_project_directory_is_rejected(self):
        self.recovered["androidSafRecoveryAcceptance"]["projectPath"] = (
            "/storage/emulated/0/project.orproj"
        )
        with self.assertRaisesRegex(ValueError, "app-private SAF working copy"):
            verify(self.process, self.prepared, self.recovered)

    def test_workflow_reads_process_report_from_recovery_artifact_directory(self):
        root = Path(__file__).resolve().parents[1]
        runner = (root / "scripts/run-android-preview-check.sh").read_text()
        workflow = (root / ".github/workflows/platform-verification.yml").read_text()

        self.assertIn(
            'process_relaunch_report="$recovery_output/process-relaunch.json"',
            runner,
        )
        self.assertIn('python3 - "$process_relaunch_report"', runner)
        self.assertIn(
            '--process "$OR_ANDROID_RECOVERY_ACCEPTANCE_OUTPUT/process-relaunch.json"',
            workflow,
        )

    def test_missing_force_stop_disappearance_is_rejected(self):
        self.process["emptyAfterForceStop"] = False
        with self.assertRaisesRegex(ValueError, "force-stop"):
            verify(self.process, self.prepared, self.recovered)

    def test_same_process_identity_is_rejected(self):
        self.process["newPid"] = self.process["oldPid"]
        with self.assertRaisesRegex(ValueError, "process identity"):
            verify(self.process, self.prepared, self.recovered)

    def test_missing_recovery_assertion_is_rejected(self):
        self.recovered["androidSafRecoveryAcceptance"]["checks"][
            "explicitRecoveryApplied"
        ] = False
        with self.assertRaisesRegex(ValueError, "product assertion"):
            verify(self.process, self.prepared, self.recovered)

    def test_missing_recovered_preview_frame_is_rejected(self):
        self.recovered["androidSafRecoveryAcceptance"]["frameSequence"] = "0"
        with self.assertRaisesRegex(ValueError, "did not present"):
            verify(self.process, self.prepared, self.recovered)

    def test_relinked_media_and_permission_must_survive_real_process_restart(self):
        checks = self.recovered["androidSafRecoveryAcceptance"]["checks"]
        checks["relinkedMediaGrantSurvivedProcessRestart"] = False
        with self.assertRaisesRegex(ValueError, "product assertion"):
            verify(self.process, self.prepared, self.recovered)

        checks["relinkedMediaGrantSurvivedProcessRestart"] = True
        self.recovered["androidSafRecoveryAcceptance"]["relinkedMediaId"] = (
            "00000001-2222-4222-8222-222222222222"
        )
        with self.assertRaisesRegex(ValueError, "relinked media identity"):
            verify(self.process, self.prepared, self.recovered)


if __name__ == "__main__":
    unittest.main()
