import 'core_gateway.dart';
import 'src/rust/api.dart' as rust;

class RustCoreGateway implements CoreGateway {
  const RustCoreGateway();

  @override
  Future<rust.AppInfo> appInfo() => rust.appInfo();

  @override
  Future<rust.HealthStatus> health() => rust.health();

  @override
  Future<List<rust.Capability>> capabilities() => rust.capabilities();
}
