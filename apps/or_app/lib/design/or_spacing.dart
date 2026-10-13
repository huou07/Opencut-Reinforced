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

/// Minimum pointer/contact area for a control.
///
/// DESIGN.md requires usable touch targets; the compact layout gets the
/// larger figure because it is the one reached by touch.
abstract final class OrTouchTargets {
  /// Pointer-driven desktop minimum.
  static const minimum = 24.0;

  /// Touch-driven compact minimum.
  static const comfortable = 44.0;
}

/// Control height.
///
/// Controls were previously sized at 28, 36, 40 and 44 px depending on which
/// screen you were on, so the same action changed identity between the shell
/// and the editor.
abstract final class OrControlSizes {
  /// Standard minimum height for a prompted control.
  static const button = 40.0;

  /// Minimum for a control driven by touch rather than a pointer.
  static const buttonTouch = 44.0;
}
