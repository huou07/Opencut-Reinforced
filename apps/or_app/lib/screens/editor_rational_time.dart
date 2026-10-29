import '../project/project_gateway.dart';

/// A canonical microsecond time from a wall-clock second count.
ProjectRationalTime rationalTimeFromSeconds(double seconds) =>
    ProjectRationalTime(
      BigInt.from((seconds.clamp(0, double.maxFinite) * 1000000).round()),
      1000000,
    );
