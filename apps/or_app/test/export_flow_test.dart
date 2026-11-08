import 'package:flutter_test/flutter_test.dart';

import 'package:or_app/project/export_profile.dart';
import 'package:or_app/project/project_file_picker.dart';
import 'package:or_app/project/project_gateway.dart';
import 'package:or_app/shell/export_flow.dart';

class _Session implements ProjectSessionHandle {}

final _project = ProjectReadModel(
  projectId: 'project-a',
  projectInstanceId: 'instance-a',
  revision: BigInt.zero,
  name: 'Local voice journey',
  dirty: false,
  descriptorPath: '/tmp/project-a.orproj',
);

ProjectExportJob _job(String state, {String? detail}) => ProjectExportJob(
  succeeded: state != 'failed',
  errorCode: '',
  message: '',
  jobId: '7',
  state: state,
  progressCompleted: state == 'running' ? BigInt.from(3) : null,
  progressTotal: state == 'running' ? BigInt.from(9) : null,
  detail: detail,
);

class _FakeGateway implements ProjectGateway {
  int summaryCalls = 0;
  int startCalls = 0;
  int statusCalls = 0;
  String? destination;
  String? quality;
  int? lastShortSide;
  String? lastPerformanceProfile;
  String? lastPreferredH264Encoder;
  ProjectExportJob status = _job('succeeded');
  int statusFailuresRemaining = 0;
  ProjectRationalRate? sequenceFrameRate = const ProjectRationalRate(30, 1);
  int frameRateSetCalls = 0;
  ProjectRationalRate? lastFrameRateSet;

  @override
  Future<ProjectReadModel> summary(ProjectSessionHandle session) async {
    summaryCalls += 1;
    return _project;
  }

  @override
  Future<ProjectExportJob> startExport(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String destination, {
    String? quality,
    int? shortSide,
    String? performanceProfile,
    String? preferredH264Encoder,
  }) async {
    startCalls += 1;
    this.destination = destination;
    this.quality = quality;
    lastShortSide = shortSide;
    lastPerformanceProfile = performanceProfile;
    lastPreferredH264Encoder = preferredH264Encoder;
    return _job('running');
  }

  @override
  Future<ProjectExportJob> exportStatus(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String jobId,
  ) async {
    statusCalls += 1;
    if (statusFailuresRemaining > 0) {
      statusFailuresRemaining -= 1;
      throw const ProjectGatewayException('EXPORT_STATUS', 'status lost');
    }
    return status;
  }

  @override
  Future<ProjectExportJob> cancelExport(
    ProjectSessionHandle session,
    ProjectReadModel current,
    String jobId,
  ) async => _job('cancelled');

  @override
  Future<ProjectTimelineSequenceSettings> timelineSequenceSettings(
    ProjectSessionHandle session,
  ) async =>
      ProjectTimelineSequenceSettings(sequenceFrameRate: sequenceFrameRate);

  @override
  Future<ProjectActionResult> setTimelineSequenceFrameRate(
    ProjectSessionHandle session,
    ProjectReadModel current,
    ProjectRationalRate? rate,
  ) async {
    frameRateSetCalls += 1;
    lastFrameRateSet = rate;
    sequenceFrameRate = rate;
    return ProjectActionResult(
      succeeded: true,
      view: ProjectReadModel(
        projectId: current.projectId,
        projectInstanceId: current.projectInstanceId,
        revision: current.revision + BigInt.one,
        name: current.name,
        dirty: true,
        descriptorPath: current.descriptorPath,
      ),
    );
  }

  @override
  Future<ProjectActionResult> setTimelineSequenceCanvas(
    ProjectSessionHandle session,
    ProjectReadModel current,
    ProjectSequenceCanvas? canvas,
  ) async => ProjectActionResult(succeeded: true, view: current);

  @override
  Future<ProjectPreviewState> previewSetSpeed(
    ProjectSessionHandle session,
    double speed,
  ) async => ProjectPreviewState(
    position: ProjectRationalTime(BigInt.zero, 1),
    playing: true,
    generation: BigInt.one,
    frameSequence: BigInt.one,
    width: 1920,
    height: 1080,
  );

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

class _FakePicker implements ProjectFilePicker {
  _FakePicker({this.publishFailure});

  final String? publishFailure;
  int saveCalls = 0;
  int publishCalls = 0;
  int discardCalls = 0;
  int cancelPublishCalls = 0;
  String? publishedPath;

  @override
  bool get supportsExport => true;

  @override
  Future<String?> saveExportPath({
    required String suggestedName,
    required ExportProfile profile,
  }) async {
    saveCalls += 1;
    return '/tmp/export.mp4';
  }

  @override
  Future<void> publishExportPath(String path) async {
    publishCalls += 1;
    final failure = publishFailure;
    if (failure != null) throw ProjectSafStorageException(failure);
    publishedPath = path;
  }

  @override
  Future<void> discardExportPath(String path) async {
    discardCalls += 1;
  }

  @override
  Future<void> cancelExportPublish(String path) async {
    cancelPublishCalls += 1;
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

ExportFlow _flow({
  required _FakeGateway gateway,
  required _FakePicker picker,
  ExportOptions? choice,
  Future<ProjectRationalRate?> Function()? chooseFrameRate,
  void Function(ProjectReadModel)? onProjectSummary,
  void Function(String, String?)? onExported,
}) => ExportFlow(
  gateway: gateway,
  picker: picker,
  chooseOptions: () async =>
      choice ??
      (
        profile: ExportProfile.defaultForPlatform,
        quality: ExportQuality.defaultChoice,
        shortSide: null,
        performanceProfile: null,
        preferredH264Encoder: null,
      ),
  chooseFrameRate:
      chooseFrameRate ?? () async => const ProjectRationalRate(30, 1),
  onProjectSummary: onProjectSummary ?? (_) {},
  onExported: onExported ?? (_, _) {},
);

void main() {
  test(
    'a new export is not poisoned by the previous export failures',
    () async {
      final gateway = _FakeGateway();
      final picker = _FakePicker();
      final exported = <String>[];
      final flow = _flow(
        gateway: gateway,
        picker: picker,
        onExported: (path, _) => exported.add(path),
      );

      final session = _Session();
      gateway.statusFailuresRemaining = 5;
      await flow.start(session);
      await Future<void>.delayed(const Duration(milliseconds: 2600));
      expect(flow.failed, isTrue);
      // A job that gave up pollng must not keep the slot claimed.
      expect(flow.isActive, isFalse);

      gateway.statusFailuresRemaining = 1;
      gateway.status = _job('succeeded');
      await flow.start(session);
      await Future<void>.delayed(const Duration(milliseconds: 1400));
      expect(flow.failed, isFalse);
      expect(exported.single, '/tmp/export.mp4');
      flow.dispose();
    },
  );

  test('starting publishes the finished file and reports it once', () async {
    final gateway = _FakeGateway();
    final picker = _FakePicker();
    final exported = <String>[];
    final summaries = <ProjectReadModel>[];
    final flow = _flow(
      gateway: gateway,
      picker: picker,
      onProjectSummary: summaries.add,
      onExported: (path, _) => exported.add(path),
    );

    await flow.start(_Session());
    expect(gateway.startCalls, 1);
    expect(gateway.destination, '/tmp/export.mp4');
    expect(flow.job?.state, 'running');
    expect(flow.isActive, isTrue);
    expect(flow.status, 'Export 3/9');
    expect(summaries.single.projectId, 'project-a');

    await Future<void>.delayed(const Duration(milliseconds: 500));
    expect(picker.publishCalls, 1);
    expect(picker.publishedPath, '/tmp/export.mp4');
    expect(exported.single, '/tmp/export.mp4');
    expect(flow.status, 'Export complete');
    expect(flow.isActive, isFalse);
    expect(flow.publishing, isFalse);
    flow.dispose();
  });

  test('publishing failure keeps the message and discards the file', () async {
    final gateway = _FakeGateway();
    final picker = _FakePicker(publishFailure: 'The folder is not writable.');
    final flow = _flow(gateway: gateway, picker: picker);

    await flow.start(_Session());
    await Future<void>.delayed(const Duration(milliseconds: 500));
    expect(flow.failed, isTrue);
    expect(flow.status, 'The folder is not writable.');
    expect(picker.discardCalls, 1);
    expect(flow.publishing, isFalse);
    flow.dispose();
  });

  test('a transient status failure keeps polling a running job', () async {
    final gateway = _FakeGateway()
      ..status = _job('running')
      ..statusFailuresRemaining = 2;
    final picker = _FakePicker();
    final exported = <String>[];
    final flow = _flow(
      gateway: gateway,
      picker: picker,
      onExported: (path, _) => exported.add(path),
    );

    await flow.start(_Session());
    await Future<void>.delayed(const Duration(milliseconds: 1400));
    // Two failed reads did not stop the watcher: the export is still active.
    expect(flow.isActive, isTrue);
    expect(flow.failed, isFalse);
    expect(flow.status, 'Export 3/9');
    expect(gateway.statusCalls, greaterThan(2));

    gateway.status = _job('succeeded');
    await Future<void>.delayed(const Duration(milliseconds: 600));
    expect(exported.single, '/tmp/export.mp4');
    expect(flow.status, 'Export complete');
    flow.dispose();
  });

  test('detach stops polling and forgets the job', () async {
    final gateway = _FakeGateway();
    final picker = _FakePicker();
    final flow = _flow(gateway: gateway, picker: picker);

    await flow.start(_Session());
    flow.detach();
    final statusCalls = gateway.statusCalls;
    await Future<void>.delayed(const Duration(milliseconds: 500));
    expect(flow.job, isNull);
    expect(flow.status, isNull);
    expect(flow.isActive, isFalse);
    expect(gateway.statusCalls, statusCalls);
    flow.dispose();
  });

  test('a cancelled destination asks the host to discard it', () async {
    final gateway = _FakeGateway();
    final picker = _FakePicker();
    final flow = _flow(gateway: gateway, picker: picker);

    await flow.start(_Session());
    flow.detach();
    expect(picker.discardCalls, 0);
    flow.dispose();
  });

  test('the receipt carries the encoder the job actually opened', () async {
    final gateway = _FakeGateway()
      ..status = _job(
        'succeeded',
        detail: 'Encoding with h264_videotoolbox (hardware).',
      );
    final picker = _FakePicker();
    final details = <String?>[];
    final flow = _flow(
      gateway: gateway,
      picker: picker,
      onExported: (_, detail) => details.add(detail),
    );

    await flow.start(_Session());
    await Future<void>.delayed(const Duration(milliseconds: 900));

    expect(details, ['Encoding with h264_videotoolbox (hardware).']);
  });

  test(
    'a project without a frame rate gets one before the options dialog',
    () async {
      final gateway = _FakeGateway()..sequenceFrameRate = null;
      final picker = _FakePicker();
      var asked = 0;
      final flow = _flow(
        gateway: gateway,
        picker: picker,
        chooseFrameRate: () async {
          asked += 1;
          return const ProjectRationalRate(24, 1);
        },
      );

      await flow.start(_Session());
      await Future<void>.delayed(const Duration(milliseconds: 900));

      expect(asked, 1);
      expect(gateway.frameRateSetCalls, 1);
      expect(gateway.lastFrameRateSet?.canonical, '24/1');
      // The export still runs after the rate is stored, at the new rate.
      expect(gateway.startCalls, 1);
      expect(flow.failed, isFalse);
      flow.dispose();
    },
  );

  test('declining the frame rate writes nothing', () async {
    final gateway = _FakeGateway()..sequenceFrameRate = null;
    final picker = _FakePicker();
    final flow = _flow(
      gateway: gateway,
      picker: picker,
      chooseFrameRate: () async => null,
    );

    await flow.start(_Session());

    expect(gateway.frameRateSetCalls, 0);
    expect(picker.saveCalls, 0);
    expect(gateway.startCalls, 0);
    expect(flow.failed, isFalse);
    flow.dispose();
  });
}
