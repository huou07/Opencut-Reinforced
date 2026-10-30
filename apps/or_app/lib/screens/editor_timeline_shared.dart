import 'package:flutter/material.dart';

import '../design/or_spacing.dart';
import '../project/project_gateway.dart';

/// Pure timeline helpers shared by the panel and its dialog surfaces: unit
/// conversion, track labelling and the exact-rational field they all use.
Map<String, String> timelineTrackLabels(List<ProjectTimelineTrack> tracks) {
  var video = 0;
  var audio = 0;
  var text = 0;
  var caption = 0;
  return {
    for (final track in tracks)
      track.trackId: switch (track.kind) {
        ProjectTimelineTrackKind.video => 'V${++video}',
        ProjectTimelineTrackKind.audio => 'A${++audio}',
        ProjectTimelineTrackKind.text => 'T${++text}',
        ProjectTimelineTrackKind.caption => 'C${++caption}',
      },
  };
}

int compareProjectRational(
  ProjectRationalTime left,
  ProjectRationalTime right,
) => (left.numerator * BigInt.from(right.denominator)).compareTo(
  right.numerator * BigInt.from(left.denominator),
);

class ExactRationalField extends StatelessWidget {
  const ExactRationalField({
    super.key,
    required this.fieldKey,
    required this.label,
    required this.controller,
    required this.onChanged,
  });

  final Key fieldKey;
  final String label;
  final TextEditingController controller;
  final VoidCallback onChanged;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(top: OrSpacing.x2),
    child: TextField(
      key: fieldKey,
      controller: controller,
      onChanged: (_) => onChanged(),
      keyboardType: TextInputType.text,
      decoration: InputDecoration(
        labelText: label,
        hintText: 'NUM/DEN',
        isDense: true,
      ),
    ),
  );
}
