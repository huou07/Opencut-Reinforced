import 'package:flutter/material.dart';

import 'or_colors.dart';
import 'or_spacing.dart';

abstract final class OrTheme {
  static ThemeData get dark {
    final border = OutlineInputBorder(
      borderRadius: BorderRadius.circular(OrRadii.control),
      borderSide: const BorderSide(color: OrColors.borderStrong),
    );

    return ThemeData(
      useMaterial3: true,
      brightness: Brightness.dark,
      scaffoldBackgroundColor: OrColors.background,
      colorScheme: const ColorScheme.dark(
        primary: OrColors.primary,
        onPrimary: OrColors.primaryText,
        secondary: OrColors.selection,
        onSecondary: OrColors.background,
        surface: OrColors.surface,
        onSurface: OrColors.text,
        error: OrColors.danger,
        onError: OrColors.primaryText,
        outline: OrColors.border,
      ),
      dividerColor: OrColors.border,
      focusColor: Color(0x443FC7FF),
      hoverColor: OrColors.surfaceHover,
      splashFactory: InkRipple.splashFactory,
      appBarTheme: const AppBarTheme(
        backgroundColor: OrColors.backgroundRaised,
        foregroundColor: OrColors.text,
        elevation: 0,
        scrolledUnderElevation: 0,
      ),
      dividerTheme: const DividerThemeData(
        color: OrColors.border,
        thickness: 1,
        space: 1,
      ),
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: OrColors.background,
        contentPadding: const EdgeInsets.symmetric(
          horizontal: OrSpacing.x3,
          vertical: OrSpacing.x2,
        ),
        hintStyle: const TextStyle(color: OrColors.textMuted, fontSize: 13),
        enabledBorder: border,
        disabledBorder: border,
        focusedBorder: border.copyWith(
          borderSide: const BorderSide(color: OrColors.selection, width: 1.5),
        ),
      ),
      filledButtonTheme: FilledButtonThemeData(
        style: FilledButton.styleFrom(
          backgroundColor: OrColors.primary,
          foregroundColor: OrColors.primaryText,
          minimumSize: const Size(0, 40),
          padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x4),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(OrRadii.control),
          ),
          textStyle: const TextStyle(fontSize: 14, fontWeight: FontWeight.w600),
        ),
      ),
      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          foregroundColor: OrColors.text,
          minimumSize: const Size(0, 40),
          padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x4),
          side: const BorderSide(color: OrColors.borderStrong),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(OrRadii.control),
          ),
          textStyle: const TextStyle(fontSize: 14, fontWeight: FontWeight.w500),
        ),
      ),
    );
  }
}
