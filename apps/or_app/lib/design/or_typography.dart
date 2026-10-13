import 'package:flutter/material.dart';

/// The OR type scale.
///
/// Sizes come straight from the DESIGN.md suggested hierarchy (display,
/// page title, section title, control, body, metadata, tiny/status). UI code
/// references these names instead of carrying its own literal font sizes, so
/// the scale is a contract rather than a convention.
abstract final class OrType {
  // -- Sizes ---------------------------------------------------------------
  static const displaySize = 44.0;
  static const pageTitleSize = 30.0;
  static const sectionTitleSize = 18.0;
  static const controlSize = 15.0;
  static const bodySize = 14.0;

  /// Metadata has a 12-13 px range in DESIGN.md. Both ends are in use, so
  /// both are named rather than silently collapsing to one and restyling
  /// every call site.
  static const metadataSize = 13.0;
  static const metadataCompactSize = 12.0;

  static const tinySize = 11.0;

  // -- Styles --------------------------------------------------------------
  static const display = TextStyle(
    fontSize: displaySize,
    fontWeight: FontWeight.w600,
    height: 1.15,
    letterSpacing: -0.8,
  );

  static const pageTitle = TextStyle(
    fontSize: pageTitleSize,
    fontWeight: FontWeight.w600,
    height: 1.15,
    letterSpacing: -0.6,
  );

  static const sectionTitle = TextStyle(
    fontSize: sectionTitleSize,
    fontWeight: FontWeight.w600,
    height: 1.2,
  );

  static const control = TextStyle(
    fontSize: controlSize,
    fontWeight: FontWeight.w500,
    height: 1.2,
  );

  static const body = TextStyle(fontSize: bodySize, height: 1.45);

  static const metadata = TextStyle(fontSize: metadataSize, height: 1.35);

  static const metadataCompact = TextStyle(
    fontSize: metadataCompactSize,
    height: 1.35,
  );

  static const tiny = TextStyle(fontSize: tinySize, height: 1.3);

  /// Builds the Material text theme from the scale above so widgets that
  /// resolve through `Theme.of(context).textTheme` follow the same one.
  static TextTheme textTheme(Color textColor, [Color? mutedColor]) {
    final muted = mutedColor ?? textColor;
    return TextTheme(
      displayLarge: display.copyWith(color: textColor),
      displayMedium: display.copyWith(color: textColor),
      displaySmall: sectionTitle.copyWith(
        fontSize: 22,
        fontWeight: FontWeight.w600,
        color: textColor,
      ),
      headlineLarge: pageTitle.copyWith(color: textColor),
      headlineMedium: pageTitle.copyWith(color: textColor),
      headlineSmall: pageTitle.copyWith(color: textColor),
      titleLarge: sectionTitle.copyWith(color: textColor),
      titleMedium: sectionTitle.copyWith(color: textColor),
      titleSmall: control.copyWith(color: textColor),
      bodyLarge: body.copyWith(
        fontSize: controlSize,
        height: 1.4,
        color: textColor,
      ),
      bodyMedium: body.copyWith(color: textColor),
      bodySmall: metadata.copyWith(color: muted),
      labelLarge: control.copyWith(color: textColor),
      labelMedium: metadataCompact.copyWith(color: muted),
      labelSmall: tiny.copyWith(color: muted),
    );
  }
}

/// The OR motion scale.
///
/// Durations come from the DESIGN.md motion tiers. Motion in OR is
/// functional, not decorative, so these are only used where a real
/// transition exists and every consumer honours the platform's reduced-motion
/// request.
abstract final class OrMotion {
  /// Hover feedback.
  static const hover = Duration(milliseconds: 120);

  /// Panel expansion and collapse.
  static const panel = Duration(milliseconds: 180);

  /// Modal appearance.
  static const modal = Duration(milliseconds: 200);

  /// Bottom sheet and sheet-like surface.
  static const sheet = Duration(milliseconds: 220);

  /// Resolves a motion token against the platform's reduced-motion request,
  /// so a user who asks for less motion gets none.
  static Duration resolve(BuildContext context, Duration token) {
    if (MediaQuery.disableAnimationsOf(context)) return Duration.zero;
    return token;
  }
}
