import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';

/// The one transient message surface.
///
/// Previously the shell raised a styled floating snackbar while the editor
/// raised a bare default one, so the same confirmation looked different
/// depending on which screen you were on.
class OrSnackBar {
  const OrSnackBar._();

  /// How long a message stays up without interaction.
  static const displayDuration = Duration(seconds: 4);

  /// Shows one message above whatever occupies the bottom of the screen.
  ///
  /// [bottomInset] is the height of a bottom-anchored control the message must
  /// not cover. The compact editor's mobile tool dock spans the bottom edge, and
  /// a floating SnackBar without this inset sits on top of it, so a tap meant for
  /// the dock lands on the message instead. Reproduced 2026-10-18 with
  /// `scripts/or_ui.py`: the compact journey's Media-library tap never reached the
  /// dock until the message had expired, and the same frame showed the message's
  /// semantics covering the dock's.
  static void show(
    BuildContext context,
    String message, {
    double bottomInset = 0,
  }) {
    final margin = bottomInset <= 0
        ? null
        : EdgeInsets.fromLTRB(
            OrSpacing.x2,
            0,
            OrSpacing.x2,
            bottomInset + OrSpacing.x2,
          );
    ScaffoldMessenger.of(context)
      ..hideCurrentSnackBar()
      ..showSnackBar(
        SnackBar(
          // At most two lines: a long message on a 320x440 compact viewport with
          // the dock clearance applied overflowed the window by 17 px
          // (measured 2026-10-18 through scripts/or_ui.py's compact journey), and
          // a message that cannot be read because it is clipped is worse than
          // one that ellipsizes.
          content: Text(message, maxLines: 2, overflow: TextOverflow.ellipsis),
          behavior: SnackBarBehavior.floating,
          margin: margin,
          backgroundColor: OrColors.surface,
          showCloseIcon: true,
          duration: displayDuration,
        ),
      );
  }
}

class OrPageLayout extends StatelessWidget {
  const OrPageLayout({
    super.key,
    required this.title,
    required this.subtitle,
    required this.child,
    this.action,
  });

  final String title;
  final String subtitle;
  final Widget child;
  final Widget? action;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final compact = OrBreakpoints.isCompact(constraints.maxWidth);
        return SingleChildScrollView(
          padding: EdgeInsets.fromLTRB(
            compact ? OrSpacing.x4 : OrSpacing.x8,
            OrSpacing.x6,
            compact ? OrSpacing.x4 : OrSpacing.x8,
            OrSpacing.x8,
          ),
          child: Center(
            child: ConstrainedBox(
              constraints: const BoxConstraints(
                maxWidth: OrSpacing.contentMaxWidth,
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  OrPageHeader(
                    title: title,
                    subtitle: subtitle,
                    action: action,
                    compact: compact,
                  ),
                  const SizedBox(height: OrSpacing.x6),
                  child,
                ],
              ),
            ),
          ),
        );
      },
    );
  }
}

class OrPageHeader extends StatelessWidget {
  const OrPageHeader({
    super.key,
    required this.title,
    required this.subtitle,
    this.action,
    this.compact = false,
  });

  final String title;
  final String subtitle;
  final Widget? action;
  final bool compact;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final titleWidth = compact || constraints.maxWidth < 560
            ? constraints.maxWidth
            : 560.0;
        return Wrap(
          alignment: WrapAlignment.spaceBetween,
          crossAxisAlignment: WrapCrossAlignment.end,
          runSpacing: OrSpacing.x3,
          children: [
            SizedBox(
              width: titleWidth,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    title,
                    style: const TextStyle(
                      color: OrColors.text,
                      fontSize: OrType.pageTitleSize,
                      fontWeight: FontWeight.w600,
                      height: 1.15,
                      letterSpacing: -0.6,
                    ),
                  ),
                  const SizedBox(height: OrSpacing.x2),
                  Text(
                    subtitle,
                    style: const TextStyle(
                      color: OrColors.textSecondary,
                      fontSize: OrType.metadataSize,
                    ),
                  ),
                ],
              ),
            ),
            ?action,
          ],
        );
      },
    );
  }
}

class OrPanel extends StatelessWidget {
  const OrPanel({
    super.key,
    this.title,
    this.trailing,
    required this.child,
    this.padding = const EdgeInsets.all(OrSpacing.x4),
  });

  final String? title;
  final Widget? trailing;
  final Widget child;
  final EdgeInsetsGeometry padding;

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: BoxDecoration(
        color: OrColors.backgroundRaised,
        border: Border.all(color: OrColors.border),
        borderRadius: BorderRadius.circular(OrRadii.panel),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (title != null)
            Padding(
              padding: const EdgeInsets.fromLTRB(
                OrSpacing.x4,
                OrSpacing.x3,
                OrSpacing.x3,
                OrSpacing.x3,
              ),
              child: Row(
                children: [
                  Expanded(
                    child: Text(
                      title!,
                      style: const TextStyle(
                        fontSize: OrType.bodySize,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  ?trailing,
                ],
              ),
            ),
          if (title != null) const Divider(height: 1),
          Padding(padding: padding, child: child),
        ],
      ),
    );
  }
}

class OrSectionHeader extends StatelessWidget {
  const OrSectionHeader(this.title, {super.key, this.trailing});

  final String title;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: OrSpacing.x3),
      child: Row(
        children: [
          Expanded(
            child: Text(
              title,
              style: const TextStyle(
                fontSize: OrType.sectionTitleSize,
                fontWeight: FontWeight.w600,
              ),
            ),
          ),
          ?trailing,
        ],
      ),
    );
  }
}

class OrEmptyState extends StatelessWidget {
  const OrEmptyState({
    super.key,
    required this.title,
    required this.message,
    this.icon = Icons.folder_open_outlined,
  });

  final String title;
  final String message;
  final IconData icon;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(
        horizontal: OrSpacing.x4,
        vertical: OrSpacing.x12,
      ),
      child: Column(
        children: [
          Icon(icon, color: OrColors.textMuted, size: 24),
          const SizedBox(height: OrSpacing.x3),
          Text(
            title,
            textAlign: TextAlign.center,
            style: const TextStyle(
              fontSize: OrType.controlSize,
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: OrSpacing.x2),
          ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 380),
            child: Text(
              message,
              textAlign: TextAlign.center,
              style: const TextStyle(
                color: OrColors.textSecondary,
                fontSize: OrType.metadataSize,
                height: 1.45,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class OrBadge extends StatelessWidget {
  const OrBadge(this.label, {super.key, this.color = OrColors.textSecondary});

  final String label;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(
        horizontal: OrSpacing.x2,
        vertical: 5,
      ),
      decoration: BoxDecoration(
        border: Border.all(color: OrColors.border),
        borderRadius: BorderRadius.circular(OrRadii.small),
        color: OrColors.surface,
      ),
      child: Text(
        label,
        style: TextStyle(
          color: color,
          fontSize: OrType.tinySize,
          fontWeight: FontWeight.w500,
        ),
      ),
    );
  }
}

class OrUnavailableButton extends StatelessWidget {
  const OrUnavailableButton({
    super.key,
    required this.label,
    required this.reason,
    required this.onPressed,
    this.icon,
    this.primary = false,
  });

  final String label;
  final String reason;
  final VoidCallback onPressed;
  final IconData? icon;
  final bool primary;

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: reason,
      child: OutlinedButton.icon(
        onPressed: onPressed,
        icon: Icon(icon ?? Icons.info_outline, size: 18),
        label: Text(label),
        style: OutlinedButton.styleFrom(
          foregroundColor: primary ? OrColors.textSecondary : OrColors.text,
          backgroundColor: primary ? OrColors.surface : Colors.transparent,
          side: BorderSide(
            color: primary ? OrColors.border : OrColors.borderStrong,
          ),
          minimumSize: const Size(0, OrControlSizes.buttonTouch),
          textStyle: const TextStyle(fontWeight: FontWeight.w500),
        ),
      ),
    );
  }
}

class OrDisabledChip extends StatelessWidget {
  const OrDisabledChip(this.label, {super.key});

  final String label;

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: 'Unavailable in this Developer Preview',
      child: OutlinedButton(
        onPressed: null,
        style: OutlinedButton.styleFrom(
          foregroundColor: OrColors.textMuted,
          disabledForegroundColor: OrColors.textMuted,
          side: const BorderSide(color: OrColors.border),
          minimumSize: const Size(0, OrControlSizes.button),
          padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x3),
          textStyle: const TextStyle(fontSize: OrType.metadataCompactSize),
        ),
        child: Text(label),
      ),
    );
  }
}

class OrSearchField extends StatelessWidget {
  const OrSearchField({super.key, required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 260,
      child: TextField(
        enabled: false,
        decoration: InputDecoration(
          prefixIcon: const Icon(Icons.search_outlined, size: 18),
          hintText: label,
          suffixIcon: const Tooltip(
            message: 'Catalog search is unavailable in this Developer Preview',
            child: Icon(Icons.info_outline, size: 16),
          ),
        ),
      ),
    );
  }
}

/// A visible keyboard-focus ring for controls that carry their own decoration.
///
/// `ThemeData.focusColor` only tints Material's own overlays, so a control
/// built from plain containers or icon buttons can receive focus with nothing
/// distinct on screen. The border space is reserved whether or not the
/// control is focused, so showing the ring never reflows the layout it
/// annotates.
class OrFocusRing extends StatefulWidget {
  const OrFocusRing({
    super.key,
    required this.child,
    this.radius = OrRadii.small,
  });

  final Widget child;
  final double radius;

  @override
  State<OrFocusRing> createState() => _OrFocusRingState();
}

class _OrFocusRingState extends State<OrFocusRing> {
  bool _focused = false;

  @override
  Widget build(BuildContext context) {
    return Focus(
      // This node only observes: the wrapped control keeps its own focus and
      // traversal behaviour, and the ring simply follows it.
      canRequestFocus: false,
      skipTraversal: true,
      onFocusChange: (focused) {
        if (focused != _focused) setState(() => _focused = focused);
      },
      // Motion here is functional feedback for an accessible indicator, and
      // it collapses to no motion when the platform asks for reduced motion.
      child: AnimatedContainer(
        duration: OrMotion.resolve(context, OrMotion.panel),
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(widget.radius),
          border: Border.all(
            color: _focused ? OrColors.focus : Colors.transparent,
            width: 2,
          ),
        ),
        child: widget.child,
      ),
    );
  }
}
