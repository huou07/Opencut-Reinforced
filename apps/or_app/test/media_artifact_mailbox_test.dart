import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/project/project_gateway.dart';
import 'package:or_app/production/media_artifact_mailbox.dart';
import 'package:or_app/production/media_preview_kinds.dart';

ProjectMediaArtifactEvent _event({
  String mediaId = 'media-1',
  String cacheKey = 'cache-1',
  String jobId = 'job-1',
}) => ProjectMediaArtifactEvent(
  sequence: BigInt.one,
  mediaId: mediaId,
  kind: ProjectMediaArtifactKind.thumbnail,
  cacheKey: cacheKey,
  jobId: jobId,
  state: ProjectMediaArtifactEventState.succeeded,
);

MediaPreviewTicket _ticket({
  String mediaId = 'media-1',
  String cacheKey = 'cache-1',
  String jobId = 'job-1',
}) => MediaPreviewTicket(
  mediaId: mediaId,
  kind: ProjectMediaArtifactKind.thumbnail,
  cacheKey: cacheKey,
  jobId: jobId,
);

void main() {
  test('an event settles only the ticket that names it', () {
    final mailbox = MediaArtifactMailbox()
      ..add(_ticket())
      ..add(_ticket(cacheKey: 'cache-2'));
    final taken = mailbox.takeMatching(_event());
    expect(taken, hasLength(1));
    expect(taken.single.cacheKey, 'cache-1');
    expect(mailbox.tickets, hasLength(1));
    expect(mailbox.matching(_event()), isEmpty);
  });

  test('early events survive only while a submission is in flight', () {
    final mailbox = MediaArtifactMailbox();
    mailbox.beginSubmission();
    mailbox.rememberEarly('k', _event());
    expect(mailbox.earlyFor('k'), isNotNull);
    mailbox.endSubmission();
    expect(mailbox.earlyFor('k'), isNull);
  });

  test('an out-of-flight event is not remembered', () {
    final mailbox = MediaArtifactMailbox()
      ..rememberEarly('k', _event());
    expect(mailbox.earlyFor('k'), isNull);
  });

  test('reset drops tickets, early events and the counter', () {
    final mailbox = MediaArtifactMailbox()
      ..beginSubmission()
      ..add(_ticket())
      ..rememberEarly('k', _event())
      ..reset();
    expect(mailbox.tickets, isEmpty);
    expect(mailbox.earlyFor('k'), isNull);
    expect(mailbox.hasSubmissionsInFlight, isFalse);
    mailbox.endSubmission();
    expect(mailbox.hasSubmissionsInFlight, isFalse);
  });
}
