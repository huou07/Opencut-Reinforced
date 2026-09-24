import 'src/rust/api.dart' show AppInfo, Capability, HealthStatus;

export 'src/rust/api.dart' show AppInfo, Capability, HealthStatus;

abstract interface class CoreGateway {
  Future<AppInfo> appInfo();
  Future<HealthStatus> health();
  Future<List<Capability>> capabilities();
}
