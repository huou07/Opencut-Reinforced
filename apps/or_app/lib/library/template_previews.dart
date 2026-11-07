import 'dart:io';

import 'package:flutter/services.dart';
import 'package:path_provider/path_provider.dart';

import '../motion/motion_library_controller.dart';
import '../motion/motion_starters.dart';
import 'project_templates.dart';

/// Supplies the still a template card shows before the template is used.
///
/// The picture is the composition the template opens with, rendered by the same
/// bridge call the Motion panel previews with, so the card cannot promise
/// something the editor will not draw.
abstract class TemplatePreviews {
  Future<Uint8List?> still(ProjectTemplate template);
}

/// Renders each template's opening still once and keeps it for the session.
///
/// Renders are memoised per template id, including failures (a null result is
/// cached as null), so scrolling the grid never re-runs the renderer for the
/// same card.
class MotionTemplatePreviews implements TemplatePreviews {
  MotionTemplatePreviews({
    MotionLibraryGateway? gateway,
    Future<Directory> Function()? supportDirectory,
    this.stillAtMilliseconds = 1400,
  }) : _gateway = gateway ?? const RustMotionLibraryGateway(),
       _supportDirectory = supportDirectory ?? getApplicationSupportDirectory;

  final MotionLibraryGateway _gateway;
  final Future<Directory> Function() _supportDirectory;
  final int stillAtMilliseconds;
  final Map<String, Future<Uint8List?>> _stills =
      <String, Future<Uint8List?>>{};

  @override
  Future<Uint8List?> still(ProjectTemplate template) {
    return _stills.putIfAbsent(template.id, () => _render(template));
  }

  Future<Uint8List?> _render(ProjectTemplate template) async {
    final starterId = template.motionStarterId;
    if (starterId == null) return null;
    MotionStarter? starter;
    for (final candidate in motionStarters) {
      if (candidate.id == starterId) {
        starter = candidate;
        break;
      }
    }
    if (starter == null) return null;
    try {
      final support = await _supportDirectory();
      final root = '${support.path}${Platform.pathSeparator}motion-library';
      final scene = await rootBundle.loadString(starter.assetPath);
      final view = await _gateway.preview(
        root: root,
        sceneJson: scene,
        atMilliseconds: stillAtMilliseconds,
      );
      if (!view.ok || view.png.isEmpty) return null;
      return view.png;
    } catch (_) {
      return null;
    }
  }
}
