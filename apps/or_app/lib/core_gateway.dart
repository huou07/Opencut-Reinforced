import 'package:or_app_bridge/or_app_bridge.dart'
    show AppInfo, Capability, HealthStatus;

export 'package:or_app_bridge/or_app_bridge.dart'
    show AppInfo, Capability, HealthStatus;

abstract interface class CoreGateway {
  Future<AppInfo> appInfo();
  Future<HealthStatus> health();
  Future<List<Capability>> capabilities();
}
