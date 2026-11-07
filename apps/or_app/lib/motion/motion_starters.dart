import 'dart:io';

import 'package:flutter/services.dart';

/// One MotionScene that ships inside the app bundle.
///
/// The files are ordinary `.ormotion.json` documents, so a starter goes through
/// the same parse, validate and adopt path as a scene the user authored: the app
/// copies the document into the motion library root and opens it from there.
/// Nothing about a starter is special once it is on disk — it can be edited,
/// re-saved to the library, or rendered again like any composition.
class MotionStarter {
  const MotionStarter({
    required this.id,
    required this.name,
    required this.summary,
    required this.assetPath,
  });

  /// Stable id, also the file stem the document is copied to.
  final String id;

  /// The name the panel lists the starter under.
  final String name;

  /// One line describing what the composition is for.
  final String summary;

  /// The Flutter asset holding the scene document.
  final String assetPath;
}

/// The starters the app bundles: the four compositions a first project most
/// often needs. Order is the order the panel offers them.
const List<MotionStarter> motionStarters = <MotionStarter>[
  MotionStarter(
    id: 'title-card',
    name: 'Title card',
    summary: 'Headline, rule and one supporting line.',
    assetPath: 'assets/motion_starters/title-card.ormotion.json',
  ),
  MotionStarter(
    id: 'name-bar',
    name: 'Name bar',
    summary: 'Lower-third name and role that still frame.',
    assetPath: 'assets/motion_starters/name-bar.ormotion.json',
  ),
  MotionStarter(
    id: 'end-card',
    name: 'End card',
    summary: 'Sign-off, rule and one call to action.',
    assetPath: 'assets/motion_starters/end-card.ormotion.json',
  ),
  MotionStarter(
    id: 'quote-card',
    name: 'Quote card',
    summary: 'Pull quote with an attribution line.',
    assetPath: 'assets/motion_starters/quote-card.ormotion.json',
  ),
];

/// Copies a bundled starter into `root/starters` and returns the file path.
///
/// The copy happens on every call so an app update ships corrected content
/// rather than leaving a stale scene from an older build; the document is about
/// three kilobytes, so the rewrite is cheaper than tracking versions.
Future<String> installMotionStarter(MotionStarter starter, String root) async {
  final bundle = await rootBundle.load(starter.assetPath);
  final bytes = bundle.buffer.asUint8List(bundle.offsetInBytes, bundle.lengthInBytes);
  final directory = Directory(
    '${root.replaceAll(RegExp(r'[\\/]+$'), '')}${Platform.pathSeparator}starters',
  );
  await directory.create(recursive: true);
  final file = File(
    '${directory.path}${Platform.pathSeparator}${starter.id}.ormotion.json',
  );
  await file.writeAsBytes(bytes, flush: true);
  return file.path;
}
