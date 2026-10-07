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
                    "noFlutterException": True,
                },
                "projectPath": "/data/user/0/io.github.huou07.or_app/files/or-projects/project.orproj",
                "projectName": "Process recovery acceptance",
                "projectRevision": "3",
                "recoveryKindAfterApply": "none",
                "frameSequence": "1",
            }
        }

    def test_complete_process_recovery_journey_is_accepted(self):
        verify(self.process, self.prepared, self.recovered)

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


if __name__ == "__main__":
    unittest.main()
