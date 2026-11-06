import 'package:or_app_bridge/or_app_bridge.dart'
    show
        AppInfo,
        Capability,
        DecoderProbeView,
        EncoderProbeView,
        EncodingPlanView,
        HealthStatus,
        PerformanceReportView;

export 'package:or_app_bridge/or_app_bridge.dart'
    show
        AppInfo,
        Capability,
        DecoderProbeView,
        EncoderProbeView,
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
}
