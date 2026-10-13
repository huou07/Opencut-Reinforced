import 'package:flutter/material.dart';

abstract final class OrColors {
  static const background = Color(0xFF090909);
  static const backgroundRaised = Color(0xFF0F0F10);
  static const surface = Color(0xFF151516);
  static const surfaceHover = Color(0xFF1C1C1E);
  static const border = Color(0xFF2A2A2D);
  static const borderStrong = Color(0xFF3A3A3E);
  static const text = Color(0xFFF5F5F5);
  static const textSecondary = Color(0xFFA6A6AB);
  // 4.5:1 or better on background, backgroundRaised and surface, so muted
  // metadata stays readable rather than merely decorative.
  static const textMuted = Color(0xFF828288);
  static const primary = Color(0xFFF5F5F5);
  static const primaryText = Color(0xFF090909);
  static const selection = Color(0xFF3FC7FF);
  // Always-visible keyboard focus. Opaque so it never composites down to an
  // unreadable tint over the dark surfaces.
  static const focus = Color(0xFF3FC7FF);
  static const ai = Color(0xFF9B7CFF);
  static const success = Color(0xFF47B881);
  static const warning = Color(0xFFD8A542);
  static const danger = Color(0xFFE05A5A);
}
