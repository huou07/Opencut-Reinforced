import 'core_gateway.dart';

import 'package:or_app_bridge/or_app_bridge.dart' as rust;

class RustCoreGateway implements CoreGateway {
  const RustCoreGateway();

  @override
  Future<rust.AppInfo> appInfo() => rust.appInfo();

  @override
  Future<rust.HealthStatus> health() => rust.health();

  @override
  Future<List<rust.Capability>> capabilities() => rust.capabilities();

  @override
  Future<rust.PerformanceReportView> performanceReport() =>
      rust.performanceReport();

  @override
  Future<rust.EncodingPlanView> encodingPlan({
    required String profile,
    String? preferredH264Encoder,
  }) => rust.encodingPlan(
    profile: profile,
    preferredH264Encoder: preferredH264Encoder,
  );

  @override
  Future<void> setGraphicsPreference(String backend) =>
      rust.setGraphicsPreference(backend: backend);
}
