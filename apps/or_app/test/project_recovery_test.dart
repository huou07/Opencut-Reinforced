import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/project/project_gateway.dart';
import 'package:or_app/shell/project_recovery.dart';

ProjectRecoveryInspection inspection(
  ProjectRecoveryKind kind, {
  String conflictReason = '',
  String recoveryName = 'Rough cut',
}) => ProjectRecoveryInspection(
  kind: kind,
  projectId: 'p1',
  baseRevision: BigInt.from(4),
  recoveryRevision: BigInt.from(5),
  recoveryName: recoveryName,
  conflictReason: conflictReason,
  message: '',
);

void main() {
  test('an inspection reads into the question the owner gets', () {
    expect(
      planRecovery(inspection(ProjectRecoveryKind.none), forCreate: false),
      RecoveryStep.openDirectly,
    );
    expect(
      planRecovery(inspection(ProjectRecoveryKind.stale), forCreate: false),
      RecoveryStep.openWithStaleNotice,
    );
    expect(
      planRecovery(inspection(ProjectRecoveryKind.candidate), forCreate: false),
      RecoveryStep.askCandidate,
    );
    expect(
      planRecovery(inspection(ProjectRecoveryKind.conflict), forCreate: false),
      RecoveryStep.askConflictDiscard,
    );
    expect(
      planRecovery(inspection(ProjectRecoveryKind.invalid), forCreate: false),
      RecoveryStep.askInvalidDiscard,
    );
  });

  test('a new project never adopts an existing location', () {
    // Creating on top of a checkpoint would either overwrite or silently
    // recover someone else's work, so the location is refused before any
    // question is asked.
    expect(
      planRecovery(inspection(ProjectRecoveryKind.candidate), forCreate: true),
      RecoveryStep.refuseExisting,
    );
    expect(
      planRecovery(inspection(ProjectRecoveryKind.stale), forCreate: true),
      RecoveryStep.refuseExisting,
    );
    expect(
      planRecovery(inspection(ProjectRecoveryKind.none), forCreate: true),
      RecoveryStep.openDirectly,
    );
  });

  test('a dismissed question stops the flow and changes nothing', () {
    expect(preparationForChoice(null).proceed, isFalse);
    expect(preparationForChoice(null).mutation, isNull);
    expect(preparationForChoice(RecoveryChoice.cancel).proceed, isFalse);
    expect(preparationForChoice(RecoveryChoice.cancel).mutation, isNull);
  });

  test('an answered question carries the mutation it stands for', () {
    final recover = preparationForChoice(RecoveryChoice.recover);
    expect(recover.proceed, isTrue);
    expect(recover.mutation, RecoveryMutation.apply);

    final discard = preparationForChoice(RecoveryChoice.discard);
    expect(discard.proceed, isTrue);
    expect(discard.mutation, RecoveryMutation.discard);

    expect(discardPreparation().proceed, isTrue);
    expect(discardPreparation().mutation, RecoveryMutation.discard);
  });

  test('notices keep their order and drop empty halves', () {
    expect(combineNotices(null, null), isNull);
    expect(combineNotices('', null), isNull);
    expect(combineNotices(null, 'b'), 'b');
    expect(combineNotices('a', null), 'a');
    expect(combineNotices('', 'b'), 'b');
    expect(combineNotices('a', ''), 'a');
    expect(combineNotices('a', 'b'), 'a\nb');
  });

  test('a cleanup-pending recovery says so instead of the stale note', () {
    expect(
      recoveryNoticeAfter(
        mutation: RecoveryMutation.apply,
        resultMessage: 'Applied the checkpoint; cleanup is pending.',
        priorNotice: staleRecoveryNotice,
        syncNotice: null,
      ),
      'Applied the checkpoint; cleanup is pending.',
    );
  });

  test('an applied recovery keeps the earlier notice and appends the sync', () {
    expect(
      recoveryNoticeAfter(
        mutation: RecoveryMutation.apply,
        resultMessage: 'Recovered revision 5.',
        priorNotice: staleRecoveryNotice,
        syncNotice: 'The document did not support readback verification.',
      ),
      '$staleRecoveryNotice\nThe document did not support readback verification.',
    );
  });

  test('a discard never claims a cleanup that is not its own', () {
    // A discard result can mention cleanup too, but the run did not apply the
    // checkpoint, so the earlier note stays the truth.
    expect(
      recoveryNoticeAfter(
        mutation: RecoveryMutation.discard,
        resultMessage: 'Discarded the checkpoint; cleanup is pending.',
        priorNotice: 'Checkpoint ignored.',
        syncNotice: null,
      ),
      'Checkpoint ignored.',
    );
  });
}
