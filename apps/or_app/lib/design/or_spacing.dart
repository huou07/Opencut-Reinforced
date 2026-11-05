import 'package:flutter/widgets.dart';

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

/// Dialog content width.
///
/// A fixed width overflows on a small window or a 125% Windows display scale,
/// so the preferred width is capped by the host window.
abstract final class OrDialogSizes {
  static double width(BuildContext context, double preferred) {
    final available = MediaQuery.sizeOf(context).width - 64;
    return preferred < available
        ? preferred
        : (available < 280 ? 280 : available);
  }
}

abstract final class OrBreakpoints {
  static const compact = 760.0;
  static const wide = 1180.0;

  /// The editor keeps three columns — tools, viewer, assistant — and each one
  /// needs a usable share of the width. Below this figure the viewer would fall
  /// under about 450 px, so the editor stacks instead of squeezing the frame the
  /// user is judging. Other screens keep the 760 px figure because a list at
  /// 800 px is still comfortable.
  static const editorCompact = 980.0;

  static bool isCompact(double width) => width < compact;
  static bool isWide(double width) => width >= wide;
  static bool isEditorCompact(double width) => width < editorCompact;
}

/// Minimum pointer/contact area for a control.
///
/// docs/design/design-language.md requires usable touch targets; the compact layout gets the
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
