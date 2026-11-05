import 'package:flutter/material.dart';

import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../design/or_typography.dart';
import '../widgets/or_widgets.dart';

/// Where one piece of bundled content came from and what its licence allows.
///
/// Every library entry in this app is either generated on the device or is
/// covered by a licence that permits redistribution in an MIT product. The
/// credits below are the source of that claim: they are shown in the app, not
/// only in a repository file, because a person choosing a font or a sound is
/// entitled to know what they may do with it.
class ContentCredit {
  const ContentCredit({
    required this.id,
    required this.name,
    required this.kind,
    required this.licence,
    required this.source,
    required this.allows,
  });

  final String id;
  final String name;
  final String kind;

  /// The licence as a person reads it, not as a SPDX token.
  final String licence;

  /// Where the file came from, or how it is produced.
  final String source;

  /// What the licence permits, in one sentence, without hedging.
  final String allows;
}

/// Fonts shipped inside the app bundle.
const List<ContentCredit> fontCredits = [
  ContentCredit(
    id: 'inter',
    name: 'Inter',
    kind: 'Font · 4 weights',
    licence: 'SIL Open Font License 1.1',
    source: 'bundled at assets/fonts/inter, from rsms.me/inter',
    allows:
        'Use, modify and redistribute inside this app, including commercial '
        'video. The font files themselves may not be sold on their own.',
  ),
  ContentCredit(
    id: 'be-vietnam-pro',
    name: 'Be Vietnam Pro',
    kind: 'Font · 4 weights',
    licence: 'SIL Open Font License 1.1',
    source: 'bundled at assets/fonts/bevietnampro, from bevietnampro.be',
    allows:
        'Use, modify and redistribute inside this app, including commercial '
        'video. The font files themselves may not be sold on their own.',
  ),
];

/// Content the app produces itself, so there is no third-party licence.
const List<ContentCredit> generatedCredits = [
  ContentCredit(
    id: 'sound-effects',
    name: 'Sound effects',
    kind: 'Audio · 7 effects',
    licence: 'Generated on this device',
    source: 'synthesised by the app from its own shapes, no recordings',
    allows:
        'Anything, including commercial video. No attribution is required '
        'because no third party owns the samples.',
  ),
  ContentCredit(
    id: 'music',
    name: 'Music beds',
    kind: 'Audio · generated per project',
    licence: 'Generated on this device',
    source: 'the offline music generator in the editor',
    allows:
        'Anything, including commercial video. The bed is written into the '
        'project as an ordinary audio file you own.',
  ),
  ContentCredit(
    id: 'templates',
    name: 'Project templates and text looks',
    kind: 'Project content',
    licence: 'MIT, same as the app',
    source: 'authored for Opencut Reinforced',
    allows:
        'Anything. A template is a starting project; the video you make from '
        'it is yours.',
  ),
  ContentCredit(
    id: 'motion',
    name: 'Motion scenes and assets',
    kind: 'Project content',
    licence: 'MIT, same as the app',
    source: 'authored for Opencut Reinforced',
    allows: 'Anything, including reuse in another project.',
  ),
];

/// Every bundled credit, in the order the library shows the content.
List<ContentCredit> get contentCredits => [...fontCredits, ...generatedCredits];

/// The licence panel shown at the end of the library guide.
///
/// It is deliberately a plain list: a licence is a fact, not a decoration, and
/// the reader should be able to copy a sentence out of it.
class ContentCreditsPanel extends StatelessWidget {
  const ContentCreditsPanel({super.key});

  @override
  Widget build(BuildContext context) {
    return OrPanel(
      key: const ValueKey('library-credits'),
      title: 'Bundled content and licences',
      trailing: const OrBadge('Nothing downloaded'),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            'Everything in this library ships with the app or is generated on '
            'this machine. Nothing here was taken from an unclear source, and '
            'none of it needs a network call to use.',
            style: OrType.body.copyWith(color: OrColors.textSecondary),
          ),
          const SizedBox(height: OrSpacing.x4),
          for (final credit in contentCredits)
            Padding(
              padding: const EdgeInsets.only(bottom: OrSpacing.x3),
              child: _CreditRow(credit: credit),
            ),
        ],
      ),
    );
  }
}

class _CreditRow extends StatelessWidget {
  const _CreditRow({required this.credit});

  final ContentCredit credit;

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(
          width: 168,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                credit.name,
                style: OrType.metadata.copyWith(
                  color: OrColors.text,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(height: OrSpacing.x1),
              Text(
                credit.kind,
                style: OrType.metadataCompact.copyWith(
                  color: OrColors.textMuted,
                ),
              ),
            ],
          ),
        ),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                credit.licence,
                key: ValueKey('library-credit-licence-${credit.id}'),
                style: OrType.metadata.copyWith(
                  color: OrColors.success,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(height: OrSpacing.x1),
              Text(
                credit.allows,
                style: OrType.metadata.copyWith(color: OrColors.textSecondary),
              ),
              const SizedBox(height: OrSpacing.x1),
              Text(
                credit.source,
                style: OrType.metadataCompact.copyWith(
                  color: OrColors.textMuted,
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}
