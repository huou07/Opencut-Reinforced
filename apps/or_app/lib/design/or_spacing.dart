abstract final class OrSpacing {
  static const x1 = 4.0;
  static const x2 = 8.0;
  static const x3 = 12.0;
  static const x4 = 16.0;
  static const x5 = 20.0;
  static const x6 = 24.0;
  static const x8 = 32.0;
  static const x10 = 40.0;
  static const x12 = 48.0;

  static const topBarHeight = 58.0;
  static const sidebarWidth = 224.0;
  static const contentMaxWidth = 1180.0;
}

abstract final class OrRadii {
  static const small = 6.0;
  static const control = 8.0;
  static const card = 10.0;
  static const panel = 12.0;
  static const sheet = 14.0;
}

abstract final class OrBreakpoints {
  static const compact = 760.0;
  static const wide = 1180.0;

  static bool isCompact(double width) => width < compact;
  static bool isWide(double width) => width >= wide;
}
