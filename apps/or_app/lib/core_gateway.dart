import 'package:or_app_bridge/or_app_bridge.dart'
    show
        AppInfo,
        Capability,
        EncodingPlanView,
        HealthStatus,
        PerformanceReportView;

export 'package:or_app_bridge/or_app_bridge.dart'
    show
        AppInfo,
        Capability,
        EncodingPlanView,
        HealthStatus,
        PerformanceReportView;

abstract interface class CoreGateway {
  Future<AppInfo> appInfo();
  Future<HealthStatus> health();
  Future<List<Capability>> capabilities();

  /// This machine's execution-relevant facts: CPU, GPU adapter, the encoders
  /// and decoders this build actually opened, and the plan each profile implies.
  Future<PerformanceReportView> performanceReport();

  /// The plan for one profile, honouring an encoder the person chose.
  Future<EncodingPlanView> encodingPlan({
    required String profile,
    String? preferredH264Encoder,
  });

  /// Records the graphics backend for the renderer's next start.
  ///
  /// The device is opened once per process, so this is a stored preference,
  /// not a live switch; Settings says so next to the control.
  Future<void> setGraphicsPreference(String backend);
}
