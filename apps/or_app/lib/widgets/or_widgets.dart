import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';

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
                      fontSize: 30,
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
                      fontSize: 13,
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
                        fontSize: 14,
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
              style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
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
            style: const TextStyle(fontSize: 15, fontWeight: FontWeight.w600),
          ),
          const SizedBox(height: OrSpacing.x2),
          ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 380),
            child: Text(
              message,
              textAlign: TextAlign.center,
              style: const TextStyle(
                color: OrColors.textSecondary,
                fontSize: 13,
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
          fontSize: 11,
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
          minimumSize: const Size(0, 44),
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
          minimumSize: const Size(0, 36),
          padding: const EdgeInsets.symmetric(horizontal: OrSpacing.x3),
          textStyle: const TextStyle(fontSize: 12),
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
