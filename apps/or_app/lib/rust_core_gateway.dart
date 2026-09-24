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
}
