import '../project/project_gateway.dart';

/// What the owner chose on the "Recovery checkpoint found" prompt.
enum RecoveryChoice { recover, discard, cancel }

/// The checkpoint action an accepted prompt commits to.
enum RecoveryMutation { apply, discard }

/// What the open/create flow must do about one recovery inspection.
///
/// The inspection itself is decided by Rust; this only says which question, if
/// any, the owner has to answer before the project can open.
enum RecoveryStep {
  /// Create only: a project or checkpoint already occupies the location.
  refuseExisting,
  /// Nothing to recover.
  openDirectly,
  /// Open the saved project and say the stale checkpoint was ignored.
  openWithStaleNotice,
  /// Ask Recover / Discard / Cancel.
  askCandidate,
  /// The checkpoint does not match the lineage, so it can only be discarded.
  askConflictDiscard,
  /// The checkpoint is malformed, so it can only be discarded.
  askInvalidDiscard,
}

/// The shell's decision before any checkpoint mutation has run.
class RecoveryPreparation {
  const RecoveryPreparation({required this.proceed, this.notice, this.mutation});

  final bool proceed;
  final String? notice;
  final RecoveryMutation? mutation;
}

const String recoveryExistingMessage =
    'A project or recovery checkpoint already exists at the selected location.';

const String staleRecoveryNotice =
    'A stale recovery checkpoint is present. The saved project was opened '
    'without applying it.';

const String recoveryFailedMessage = 'Recovery could not be completed.';

/// The marker a cleanup-pending result carries; that message is shown as-is
/// because it names work the owner can still see happening.
const String cleanupPendingMarker = 'cleanup is pending';

/// Reads an inspection into the step the flow must take.
RecoveryStep planRecovery(
  ProjectRecoveryInspection inspection, {
  required bool forCreate,
}) {
  if (forCreate && inspection.kind != ProjectRecoveryKind.none) {
    return RecoveryStep.refuseExisting;
  }
  switch (inspection.kind) {
    case ProjectRecoveryKind.none:
      return RecoveryStep.openDirectly;
    case ProjectRecoveryKind.stale:
      return RecoveryStep.openWithStaleNotice;
    case ProjectRecoveryKind.candidate:
      return RecoveryStep.askCandidate;
    case ProjectRecoveryKind.conflict:
      return RecoveryStep.askConflictDiscard;
    case ProjectRecoveryKind.invalid:
      return RecoveryStep.askInvalidDiscard;
  }
}

/// The preparation an answered (or dismissed) prompt stands for.
///
/// A dismissed prompt is a cancel: the flow stops, and nothing on disk changes.
RecoveryPreparation preparationForChoice(RecoveryChoice? choice) {
  if (choice == null || choice == RecoveryChoice.cancel) {
    return const RecoveryPreparation(proceed: false);
  }
  return RecoveryPreparation(
    proceed: true,
    mutation: choice == RecoveryChoice.recover
        ? RecoveryMutation.apply
        : RecoveryMutation.discard,
  );
}

/// The preparation after an accepted discard-only prompt.
RecoveryPreparation discardPreparation() => const RecoveryPreparation(
  proceed: true,
  mutation: RecoveryMutation.discard,
);

/// Joins two optional notices, keeping the order they happened in.
String? combineNotices(String? first, String? second) {
  if (first == null || first.isEmpty) return second;
  if (second == null || second.isEmpty) return first;
  return '$first\n$second';
}

/// The notice to show after a checkpoint mutation succeeded.
///
/// A cleanup-pending result is the interesting news and replaces the earlier
/// stale-checkpoint note; otherwise the earlier note survives and the sync
/// notice is appended.
String? recoveryNoticeAfter({
  required RecoveryMutation mutation,
  required String resultMessage,
  required String? priorNotice,
  required String? syncNotice,
}) {
  final chosen =
      mutation == RecoveryMutation.apply &&
          resultMessage.contains(cleanupPendingMarker)
      ? resultMessage
      : priorNotice;
  return combineNotices(chosen, syncNotice);
}
