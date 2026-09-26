import 'dart:async';
import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../core_gateway.dart';
import '../design/or_colors.dart';
import '../design/or_spacing.dart';
import '../project/project_file_picker.dart';
import '../project/project_gateway.dart';
import '../screens/asset_library_screen.dart';
import '../screens/editor_shell_preview_screen.dart';
import '../screens/home_screen.dart';
import '../screens/projects_screen.dart';
import '../screens/settings_screen.dart';
import '../screens/templates_screen.dart';
import 'app_navigation.dart';
import 'app_top_bar.dart';
import 'command_palette.dart';

const _androidProjectAccessMessage =
    'Project file access on Android requires Storage Access Framework integration and is not available in this Developer Preview.';
const _mediaPageSize = 50;

class AppShell extends StatefulWidget {
  const AppShell({
    super.key,
    required this.gateway,
    required this.projectGateway,
    required this.projectFilePicker,
  });

  final CoreGateway gateway;
  final ProjectGateway projectGateway;
  final ProjectFilePicker projectFilePicker;

  @override
  State<AppShell> createState() => _AppShellState();
}

class _AppShellState extends State<AppShell> {
  AppDestination _destination = AppDestination.home;
  ProjectSessionHandle? _activeSession;
  ProjectReadModel? _activeProject;
  String? _activeProjectPath;
  String? _projectNotice;
  StreamSubscription<ProjectHostEvent>? _eventSubscription;
  Future<void> _eventQueue = Future<void>.value();
  BigInt _lastEventSequence = BigInt.zero;
  bool _busy = false;
  ProjectMediaPage? _mediaPage;
  bool _mediaLoading = false;
  bool _mediaLoadingMore = false;
  String? _mediaLoadError;
  int _mediaRefreshGeneration = 0;
  late final AppLifecycleListener _lifecycleListener;

  bool get _hasProjectWorkspace =>
      _destination == AppDestination.editorPreview && _activeSession != null;

  @override
  void initState() {
    super.initState();
    _lifecycleListener = AppLifecycleListener(
      onExitRequested: _onExitRequested,
    );
  }

  @override
  void dispose() {
    _lifecycleListener.dispose();
    unawaited(_eventSubscription?.cancel());
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final compact = OrBreakpoints.isCompact(constraints.maxWidth);
        final editor = _destination == AppDestination.editorPreview;
        final title = _hasProjectWorkspace
            ? (_activeProject?.name ?? 'Opening project…')
            : _destination.label;

        return Shortcuts(
          shortcuts: const {
            SingleActivator(LogicalKeyboardKey.keyK, control: true):
                _OpenCommandPaletteIntent(),
            SingleActivator(LogicalKeyboardKey.keyK, meta: true):
                _OpenCommandPaletteIntent(),
            SingleActivator(LogicalKeyboardKey.keyS, control: true):
                _SaveProjectIntent(),
            SingleActivator(LogicalKeyboardKey.keyS, meta: true):
                _SaveProjectIntent(),
            SingleActivator(LogicalKeyboardKey.keyZ, control: true):
                _UndoProjectIntent(),
            SingleActivator(LogicalKeyboardKey.keyZ, meta: true):
                _UndoProjectIntent(),
            SingleActivator(
              LogicalKeyboardKey.keyZ,
              control: true,
              shift: true,
            ): _RedoProjectIntent(),
            SingleActivator(LogicalKeyboardKey.keyZ, meta: true, shift: true):
                _RedoProjectIntent(),
          },
          child: Actions(
            actions: {
              _OpenCommandPaletteIntent:
                  CallbackAction<_OpenCommandPaletteIntent>(
                    onInvoke: (_) {
                      unawaited(_openCommandPalette());
                      return null;
                    },
                  ),
              _SaveProjectIntent: CallbackAction<_SaveProjectIntent>(
                onInvoke: (_) {
                  if (!_textInputFocused) unawaited(_saveProject());
                  return null;
                },
              ),
              _UndoProjectIntent: CallbackAction<_UndoProjectIntent>(
                onInvoke: (_) {
                  if (!_textInputFocused) unawaited(_undoProject());
                  return null;
                },
              ),
              _RedoProjectIntent: CallbackAction<_RedoProjectIntent>(
                onInvoke: (_) {
                  if (!_textInputFocused) unawaited(_redoProject());
                  return null;
                },
              ),
            },
            child: Focus(
              autofocus: true,
              child: Scaffold(
                body: SafeArea(
                  bottom: false,
                  child: Column(
                    children: [
                      AppTopBar(
                        title: title,
                        compact: compact,
                        onHome: () => _select(AppDestination.home),
                        onOpenCommandPalette: _openCommandPalette,
                        onExitEditorPreview: editor
                            ? (_activeSession == null
                                  ? () => _select(AppDestination.home)
                                  : _closeProject)
                            : null,
                      ),
                      Expanded(
                        child: Row(
                          children: [
                            if (!compact && !editor) ...[
                              DesktopNavigation(
                                selected: _destination,
                                onSelected: _select,
                              ),
                              const VerticalDivider(width: 1),
                            ],
                            Expanded(child: _screen()),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
                bottomNavigationBar: compact
                    ? CompactNavigation(
                        selected: _destination,
                        onSelected: _select,
                      )
                    : null,
              ),
            ),
          ),
        );
      },
    );
  }

  bool get _textInputFocused =>
      FocusManager.instance.primaryFocus?.context?.widget is EditableText;

  Widget _screen() => switch (_destination) {
    AppDestination.home => HomeScreen(
      canPickProjects: widget.projectFilePicker.isSupported,
      onNewProject: _createProject,
      onOpenProject: _openProject,
      onOpenEditorPreview: () => _select(AppDestination.editorPreview),
      onOpenProjects: () => _select(AppDestination.projects),
    ),
    AppDestination.projects => ProjectsScreen(
      project: _activeProject,
      canPickProjects: widget.projectFilePicker.isSupported,
      onNewProject: _createProject,
      onOpenProject: _openProject,
      onOpenWorkspace: _openWorkspace,
      onRenameProject: _renameProject,
      onSaveProject: _saveProject,
      onCloseProject: _closeProject,
    ),
    AppDestination.templates => TemplatesScreen(
      onUnavailable: _showUnavailable,
    ),
    AppDestination.assets => AssetLibraryScreen(
      onUnavailable: _showUnavailable,
    ),
    AppDestination.settings => SettingsScreen(
      gateway: widget.gateway,
      project: _activeProject,
      onCopyDescriptor: _copyDescriptorPath,
      onOpenEditorPreview: () => _select(AppDestination.editorPreview),
    ),
    AppDestination.editorPreview =>
      _activeSession == null
          ? const EditorShellPreviewScreen()
          : EditorShellPreviewScreen(
              isProjectWorkspace: true,
              project: _activeProject,
              notice: _projectNotice,
              busy: _busy,
              mediaPage: _mediaPage,
              mediaLoading: _mediaLoading,
              mediaLoadingMore: _mediaLoadingMore,
              mediaLoadError: _mediaLoadError,
              onImportMedia: _importMedia,
              onLoadMoreMedia: _loadMoreMedia,
              onRefreshMedia: _refreshMediaLibrary,
              onRemoveMedia: _removeMedia,
              onSave: _saveProject,
              onRename: _renameProject,
              onUndo: _undoProject,
              onRedo: _redoProject,
              onClose: _closeProject,
              onDiscardRecovery: _discardStaleRecovery,
            ),
  };

  void _select(AppDestination destination) {
    setState(() => _destination = destination);
  }

  Future<void> _openCommandPalette() => showOrCommandPalette(
    context,
    _select,
    actions: [
      CommandPaletteAction(label: 'New Project', onSelected: _createProject),
      CommandPaletteAction(label: 'Open Project', onSelected: _openProject),
      if (_activeSession != null) ...[
        CommandPaletteAction(label: 'Save Project', onSelected: _saveProject),
        CommandPaletteAction(
          label: 'Rename Project',
          onSelected: _renameProject,
        ),
        CommandPaletteAction(
          label: 'Undo Project Change',
          onSelected: _undoProject,
        ),
        CommandPaletteAction(
          label: 'Redo Project Change',
          onSelected: _redoProject,
        ),
        CommandPaletteAction(label: 'Close Project', onSelected: _closeProject),
      ],
    ],
  );

  void _openWorkspace() {
    if (_activeSession != null) _select(AppDestination.editorPreview);
  }

  Future<void> _createProject() async {
    if (!_checkFileLifecycleAvailable()) return;
    if (_busy) return;
    setState(() => _busy = true);
    try {
      final name = await _askProjectName();
      if (name == null || !mounted) return;
      final selectedPath = await widget.projectFilePicker.saveProjectPath(
        suggestedName: 'project.orproj',
      );
      if (selectedPath == null || !mounted) return;
      final path = _withProjectExtension(selectedPath);
      if (path == _activeProjectPath) {
        _showUnavailable('That project is already open.');
        return;
      }
      final recovery = await _prepareRecovery(path, forCreate: true);
      if (!recovery.proceed || !mounted) return;
      var resolvedRecovery = recovery;
      if (!await _leaveCurrentProject(
            beforeClose: () async {
              resolvedRecovery = await _resolveRecovery(path, recovery);
              return resolvedRecovery.proceed;
            },
          ) ||
          !mounted) {
        return;
      }
      final session = await widget.projectGateway.createProject(path, name);
      await _startProject(session, path, notice: resolvedRecovery.notice);
    } on ProjectGatewayException catch (error) {
      _showProjectError(error);
    } catch (_) {
      _showUnavailable('The new project could not be created.');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _openProject() async {
    if (!_checkFileLifecycleAvailable()) return;
    if (_busy) return;
    setState(() => _busy = true);
    try {
      final path = await widget.projectFilePicker.openProjectPath();
      if (path == null || !mounted) return;
      if (path == _activeProjectPath) {
        _openWorkspace();
        return;
      }
      final recovery = await _prepareRecovery(path);
      if (!recovery.proceed || !mounted) return;
      var resolvedRecovery = recovery;
      if (!await _leaveCurrentProject(
            beforeClose: () async {
              resolvedRecovery = await _resolveRecovery(path, recovery);
              return resolvedRecovery.proceed;
            },
          ) ||
          !mounted) {
        return;
      }
      final session = await widget.projectGateway.openProject(path);
      await _startProject(session, path, notice: resolvedRecovery.notice);
    } on ProjectGatewayException catch (error) {
      _showProjectError(error);
    } catch (_) {
      _showUnavailable('The selected project could not be opened.');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  bool _checkFileLifecycleAvailable() {
    if (widget.projectFilePicker.isSupported) return true;
    _showUnavailable(_androidProjectAccessMessage);
    return false;
  }

  Future<String?> _askProjectName() async {
    return showDialog<String>(
      context: context,
      builder: (_) => const _ProjectNameDialog(
        title: 'New Project',
        initialName: '',
        fieldKey: 'new-project-name',
        confirmKey: 'confirm-new-project',
        confirmLabel: 'Continue',
      ),
    );
  }

  String _withProjectExtension(String path) =>
      path.toLowerCase().endsWith('.orproj') ? path : '$path.orproj';

  Future<_RecoveryPreparation> _prepareRecovery(
    String path, {
    bool forCreate = false,
  }) async {
    final inspection = await widget.projectGateway.inspectRecovery(path);
    if (forCreate && inspection.kind != ProjectRecoveryKind.none) {
      _showUnavailable(
        'A project or recovery checkpoint already exists at the selected location.',
      );
      return const _RecoveryPreparation(proceed: false);
    }
    switch (inspection.kind) {
      case ProjectRecoveryKind.none:
        return const _RecoveryPreparation(proceed: true);
      case ProjectRecoveryKind.stale:
        return const _RecoveryPreparation(
          proceed: true,
          notice: 'A stale recovery checkpoint is present. The saved project was opened without applying it.',
        );
      case ProjectRecoveryKind.candidate:
        final choice = await _askCandidateRecovery(inspection);
        if (choice == null || choice == _RecoveryChoice.cancel) {
          return const _RecoveryPreparation(proceed: false);
        }
        return _RecoveryPreparation(
          proceed: true,
          mutation: choice == _RecoveryChoice.recover
              ? _RecoveryMutation.apply
              : _RecoveryMutation.discard,
        );
      case ProjectRecoveryKind.conflict:
        final discard = await _askConflictingRecoveryDiscard(
          inspection.conflictReason,
        );
        if (!discard) return const _RecoveryPreparation(proceed: false);
        return const _RecoveryPreparation(
          proceed: true,
          mutation: _RecoveryMutation.discard,
        );
      case ProjectRecoveryKind.invalid:
        final discard = await _askInvalidRecoveryDiscard();
        if (!discard) return const _RecoveryPreparation(proceed: false);
        return const _RecoveryPreparation(
          proceed: true,
          mutation: _RecoveryMutation.discard,
        );
    }
  }

  Future<_RecoveryPreparation> _resolveRecovery(
    String path,
    _RecoveryPreparation preparation,
  ) async {
    if (preparation.mutation == null) return preparation;
    final result = preparation.mutation == _RecoveryMutation.apply
        ? await widget.projectGateway.applyRecovery(path)
        : await widget.projectGateway.discardRecovery(path);
    if (!result.succeeded) {
      _showUnavailable(
        result.message.isEmpty
            ? 'Recovery could not be completed.'
            : result.message,
      );
      return const _RecoveryPreparation(proceed: false);
    }
    return _RecoveryPreparation(
      proceed: true,
      notice:
          preparation.mutation == _RecoveryMutation.apply &&
              result.message.contains('cleanup is pending')
          ? result.message
          : preparation.notice,
    );
  }

  Future<_RecoveryChoice?> _askCandidateRecovery(
    ProjectRecoveryInspection inspection,
  ) => showDialog<_RecoveryChoice>(
    context: context,
    builder: (context) => AlertDialog(
      title: const Text('Recovery checkpoint found'),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'A recoverable project checkpoint is available${inspection.recoveryName.isEmpty ? '' : ' for “${inspection.recoveryName}”'}.',
          ),
          const SizedBox(height: OrSpacing.x3),
          Text('Saved revision: ${inspection.baseRevision}'),
          Text('Recovery revision: ${inspection.recoveryRevision}'),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, _RecoveryChoice.cancel),
          child: const Text('Cancel'),
        ),
        TextButton(
          onPressed: () => Navigator.pop(context, _RecoveryChoice.discard),
          child: const Text('Discard'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(context, _RecoveryChoice.recover),
          child: const Text('Recover'),
        ),
      ],
    ),
  );

  Future<bool> _askConflictingRecoveryDiscard(String reason) async {
    final discard = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Recovery checkpoint conflict'),
        content: Text(
          'This checkpoint does not match the project lineage${reason.isEmpty ? '.' : ' ($reason).'} No recovery data will be applied.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('Discard checkpoint'),
          ),
        ],
      ),
    );
    return discard == true && await _confirmRecoveryDiscard();
  }

  Future<bool> _askInvalidRecoveryDiscard() async {
    final discard = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Invalid recovery checkpoint'),
        content: const Text(
          'This recovery checkpoint is malformed or failed validation. Its contents cannot be recovered, and the saved project will remain unchanged.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('Discard checkpoint'),
          ),
        ],
      ),
    );
    return discard == true && await _confirmRecoveryDiscard();
  }

  Future<bool> _confirmRecoveryDiscard() async =>
      await showDialog<bool>(
        context: context,
        builder: (context) => AlertDialog(
          title: const Text('Discard recovery data?'),
          content: const Text(
            'This removes the recovery checkpoint permanently.',
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context, false),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(context, true),
              child: const Text('Discard'),
            ),
          ],
        ),
      ) ??
      false;

  Future<bool> _leaveCurrentProject({
    Future<bool> Function()? beforeClose,
  }) async {
    final session = _activeSession;
    if (session == null) {
      return beforeClose == null ? true : beforeClose();
    }

    final current = await widget.projectGateway.summary(session);
    if (mounted && identical(session, _activeSession)) {
      setState(() => _activeProject = current);
    }
    if (!current.dirty) {
      if (beforeClose != null && !await beforeClose()) return false;
      await widget.projectGateway.close(session, discardUnsaved: false);
      _clearActiveProject(session);
      return true;
    }

    final choice = await _askDirtyProjectChoice();
    if (!mounted || !identical(session, _activeSession)) {
      return _activeSession == null;
    }
    if (choice == _LeaveChoice.cancel || choice == null) return false;

    if (choice == _LeaveChoice.save) {
      final result = await widget.projectGateway.save(session);
      if (!result.succeeded) {
        if (result.errorCode == 'PROJECT_FILE_CHANGED') {
          _showUnavailable(
            'The project file changed outside OR. The save was blocked and the project remains open.',
          );
        } else {
          _showProjectError(
            ProjectGatewayException(result.errorCode, result.message),
          );
        }
        return false;
      }
      final saved = result.view ?? await widget.projectGateway.summary(session);
      if (mounted) setState(() => _activeProject = saved);
    }
    if (beforeClose != null && !await beforeClose()) return false;
    await widget.projectGateway.close(
      session,
      discardUnsaved: choice == _LeaveChoice.discard,
    );
    _clearActiveProject(session);
    return true;
  }

  Future<_LeaveChoice?> _askDirtyProjectChoice() => showDialog<_LeaveChoice>(
    context: context,
    builder: (context) => AlertDialog(
      title: const Text('Unsaved project changes'),
      content: const Text('Save your changes before continuing?'),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, _LeaveChoice.cancel),
          child: const Text('Cancel'),
        ),
        TextButton(
          onPressed: () => Navigator.pop(context, _LeaveChoice.discard),
          child: const Text('Discard'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(context, _LeaveChoice.save),
          child: const Text('Save'),
        ),
      ],
    ),
  );

  Future<void> _startProject(
    ProjectSessionHandle session,
    String path, {
    String? notice,
  }) async {
    final refreshGeneration = ++_mediaRefreshGeneration;
    _eventSubscription = widget.projectGateway
        .watch(session)
        .listen(
          (event) {
            _eventQueue = _eventQueue
                .then((_) => _handleProjectEvent(session, event))
                .catchError((Object _) {});
          },
          onError: (Object _) {
            if (mounted && identical(session, _activeSession)) {
              _showUnavailable(
                'Live project updates are unavailable. Refresh the project to continue.',
              );
            }
          },
        );
    _lastEventSequence = BigInt.zero;
    setState(() {
      _activeSession = session;
      _activeProject = null;
      _mediaPage = null;
      _mediaLoading = true;
      _mediaLoadingMore = false;
      _mediaLoadError = null;
      _activeProjectPath = path;
      _projectNotice = notice;
      _destination = AppDestination.editorPreview;
    });
    try {
      final view = await widget.projectGateway.summary(session);
      if (!mounted ||
          !identical(session, _activeSession) ||
          refreshGeneration != _mediaRefreshGeneration) {
        return;
      }
      setState(() => _activeProject = view);
      await _refreshMediaPage(
        session,
        project: view,
        refreshGeneration: refreshGeneration,
      );
    } catch (_) {
      await widget.projectGateway.close(session, discardUnsaved: true);
      _clearActiveProject(session);
      rethrow;
    }
  }

  Future<void> _handleProjectEvent(
    ProjectSessionHandle session,
    ProjectHostEvent event,
  ) async {
    if (!mounted || !identical(session, _activeSession)) return;
    if (event.sequence <= _lastEventSequence) return;
    _lastEventSequence = event.sequence;

    if (event.kind == 'session_closing') {
      _clearActiveProject(session);
      _showUnavailable('The attached local project session was closed.');
      return;
    }
    if (event.kind == 'project_changed' || event.kind == 'project_saved') {
      await _refreshProjectState(session);
    }
  }

  Future<ProjectReadModel?> _runProjectAction(
    Future<ProjectActionResult> Function(ProjectSessionHandle, ProjectReadModel)
    operation,
  ) async {
    final session = _activeSession;
    final current = _activeProject;
    if (session == null || current == null || _busy) return null;
    setState(() => _busy = true);
    try {
      final result = await operation(session, current);
      if (!mounted || !identical(session, _activeSession)) return null;
      if (!result.succeeded) {
        if (result.errorCode == 'REVISION_CONFLICT') {
          await _refreshProjectState(session);
          _showUnavailable(
            'The project changed in the attached CLI. The summary is refreshed; this action was not retried.',
          );
        } else if (result.errorCode == 'PROBE_BACKEND_UNAVAILABLE') {
          _showUnavailable(
            'Media probe backend is unavailable.\nThis Developer Preview currently requires a system-provided ffprobe.',
          );
        } else if (result.errorCode == 'PROJECT_FILE_CHANGED') {
          _showUnavailable(
            'The project file changed outside OR. The save was blocked to protect those changes.',
          );
        } else {
          _showUnavailable(
            result.message.isEmpty
                ? 'The project action failed.'
                : result.message,
          );
        }
        return null;
      }
      final updated =
          result.view ?? await widget.projectGateway.summary(session);
      final changed = updated.revision != current.revision;
      if (mounted && identical(session, _activeSession)) {
        setState(() {
          _activeProject = updated;
          if (changed) _mediaLoading = true;
        });
        if (changed) await _refreshProjectState(session);
      }
      return updated;
    } on ProjectGatewayException catch (error) {
      _showProjectError(error);
      return null;
    } catch (_) {
      _showUnavailable('The project action failed.');
      return null;
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _saveProject() async {
    final session = _activeSession;
    if (session == null) return;
    final current = _activeProject;
    if (current == null) return;
    await _runProjectAction(
      (session, _) => widget.projectGateway.save(session),
    );
  }

  Future<void> _undoProject() async {
    await _runProjectAction(widget.projectGateway.undo);
  }

  Future<void> _redoProject() async {
    await _runProjectAction(widget.projectGateway.redo);
  }

  Future<void> _renameProject() async {
    final current = _activeProject;
    if (current == null) return;
    final name = await _askProjectRename(current.name);
    if (name == null) return;
    await _runProjectAction(
      (session, view) => widget.projectGateway.rename(session, view, name),
    );
  }

  Future<void> _importMedia() async {
    final session = _activeSession;
    if (session == null || _busy) return;
    String? path;
    try {
      path = await widget.projectFilePicker.openMediaPath();
    } catch (_) {
      _showUnavailable('A media file could not be selected.');
      return;
    }
    if (path == null || !mounted || !identical(session, _activeSession)) {
      return;
    }

    await _runProjectAction((session, _) async {
      // Capture the identity and revision immediately before Rust probes the
      // file. The bridge keeps this pre-probe revision for the add command.
      final current = await widget.projectGateway.summary(session);
      if (!mounted || !identical(session, _activeSession)) {
        return const ProjectActionResult(
          succeeded: false,
          errorCode: 'PROJECT_CLOSING',
          message: 'The project is closing.',
        );
      }
      setState(() => _activeProject = current);
      return widget.projectGateway.importMedia(session, current, path!);
    });
  }

  Future<void> _removeMedia(ProjectMediaItem item) async {
    final session = _activeSession;
    if (session == null || _activeProject == null || _busy) return;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Remove media from project?'),
        content: const Text(
          'This removes the item from the project library. The source file will not be deleted.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            key: const ValueKey('confirm-remove-media'),
            onPressed: () => Navigator.pop(context, true),
            child: const Text('Remove from Project'),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted || !identical(session, _activeSession)) {
      return;
    }
    await _runProjectAction(
      (session, current) =>
          widget.projectGateway.removeMedia(session, current, item.mediaId),
    );
  }

  Future<void> _loadMoreMedia() async {
    final session = _activeSession;
    final project = _activeProject;
    final page = _mediaPage;
    final offset = page?.nextOffset;
    final refreshGeneration = _mediaRefreshGeneration;
    if (session == null || project == null || page == null || offset == null) {
      return;
    }
    if (_mediaLoading || _mediaLoadingMore) return;
    setState(() => _mediaLoadingMore = true);
    try {
      final next = await widget.projectGateway.listMediaPage(
        session,
        offset: offset,
        limit: _mediaPageSize,
      );
      if (!mounted ||
          !identical(session, _activeSession) ||
          refreshGeneration != _mediaRefreshGeneration) {
        return;
      }
      if (!_matchesProject(next, project)) {
        await _refreshProjectState(session);
        return;
      }
      setState(() {
        _mediaPage = ProjectMediaPage(
          projectId: page.projectId,
          projectInstanceId: page.projectInstanceId,
          projectRevision: page.projectRevision,
          items: List.unmodifiable([...page.items, ...next.items]),
          totalCount: next.totalCount,
          offset: 0,
          limit: page.limit,
          nextOffset: next.nextOffset,
        );
        _mediaLoadError = null;
      });
    } on ProjectGatewayException catch (error) {
      if (mounted && identical(session, _activeSession)) {
        setState(() => _mediaLoadError = error.message);
      }
    } catch (_) {
      if (mounted && identical(session, _activeSession)) {
        setState(() => _mediaLoadError = 'The media library could not load.');
      }
    } finally {
      if (mounted && identical(session, _activeSession)) {
        setState(() => _mediaLoadingMore = false);
      }
    }
  }

  Future<void> _refreshMediaLibrary() async {
    final session = _activeSession;
    if (session != null) await _refreshProjectState(session);
  }

  Future<void> _refreshProjectState(ProjectSessionHandle session) async {
    if (!mounted || !identical(session, _activeSession)) return;
    final refreshGeneration = ++_mediaRefreshGeneration;
    try {
      final view = await widget.projectGateway.summary(session);
      if (!mounted ||
          !identical(session, _activeSession) ||
          refreshGeneration != _mediaRefreshGeneration) {
        return;
      }
      setState(() {
        _activeProject = view;
        _mediaLoading = true;
        _mediaLoadError = null;
      });
      await _refreshMediaPage(
        session,
        project: view,
        refreshGeneration: refreshGeneration,
      );
    } catch (_) {
      if (mounted && identical(session, _activeSession)) {
        _showUnavailable('The project summary could not be refreshed.');
      }
    }
  }

  Future<void> _refreshMediaPage(
    ProjectSessionHandle session, {
    required ProjectReadModel project,
    int? refreshGeneration,
  }) async {
    final generation = refreshGeneration ?? ++_mediaRefreshGeneration;
    var current = project;
    try {
      for (var attempt = 0; attempt < 2; attempt++) {
        final page = await widget.projectGateway.listMediaPage(
          session,
          offset: 0,
          limit: _mediaPageSize,
        );
        if (!mounted ||
            !identical(session, _activeSession) ||
            generation != _mediaRefreshGeneration) {
          return;
        }
        if (_matchesProject(page, current)) {
          setState(() {
            _activeProject = current;
            _mediaPage = page;
            _mediaLoading = false;
            _mediaLoadError = null;
          });
          return;
        }
        current = await widget.projectGateway.summary(session);
        if (!mounted ||
            !identical(session, _activeSession) ||
            generation != _mediaRefreshGeneration) {
          return;
        }
        setState(() => _activeProject = current);
      }
      setState(() {
        _mediaPage = null;
        _mediaLoading = false;
        _mediaLoadError = 'The media library changed while it was refreshing.';
      });
    } on ProjectGatewayException catch (error) {
      if (mounted &&
          identical(session, _activeSession) &&
          generation == _mediaRefreshGeneration) {
        setState(() {
          _mediaLoading = false;
          _mediaLoadError = error.message;
        });
      }
    } catch (_) {
      if (mounted &&
          identical(session, _activeSession) &&
          generation == _mediaRefreshGeneration) {
        setState(() {
          _mediaLoading = false;
          _mediaLoadError = 'The media library could not load.';
        });
      }
    }
  }

  bool _matchesProject(ProjectMediaPage page, ProjectReadModel project) =>
      page.projectId == project.projectId &&
      page.projectInstanceId == project.projectInstanceId &&
      page.projectRevision == project.revision;

  Future<String?> _askProjectRename(String currentName) async {
    return showDialog<String>(
      context: context,
      builder: (_) => _ProjectNameDialog(
        title: 'Rename Project',
        initialName: currentName,
        fieldKey: 'rename-project-name',
        confirmKey: 'confirm-rename-project',
        confirmLabel: 'Rename',
      ),
    );
  }

  Future<void> _closeProject() async {
    if (_busy) return;
    try {
      if (await _leaveCurrentProject() && mounted) {
        setState(() => _destination = AppDestination.home);
      }
    } on ProjectGatewayException catch (error) {
      _showProjectError(error);
    } catch (_) {
      _showUnavailable('The project could not be closed. It remains open.');
    }
  }

  Future<AppExitResponse> _onExitRequested() async {
    if (_activeSession == null) return AppExitResponse.exit;
    try {
      final close = await _leaveCurrentProject();
      return close ? AppExitResponse.exit : AppExitResponse.cancel;
    } catch (error) {
      if (mounted) {
        _showUnavailable(
          'The project remains open because it could not be safely closed.',
        );
      }
      return AppExitResponse.cancel;
    }
  }

  void _clearActiveProject(ProjectSessionHandle session) {
    if (!identical(session, _activeSession)) return;
    _mediaRefreshGeneration++;
    unawaited(_eventSubscription?.cancel());
    if (!mounted) return;
    setState(() {
      _eventSubscription = null;
      _activeSession = null;
      _activeProject = null;
      _mediaPage = null;
      _mediaLoading = false;
      _mediaLoadingMore = false;
      _mediaLoadError = null;
      _activeProjectPath = null;
      _projectNotice = null;
    });
  }

  Future<void> _discardStaleRecovery() async {
    final path = _activeProjectPath;
    if (path == null) return;
    final result = await widget.projectGateway.discardRecovery(path);
    if (!mounted) return;
    if (!result.succeeded) {
      _showUnavailable(result.message);
    } else {
      setState(() => _projectNotice = null);
      if (result.changed) {
        _showUnavailable('Stale recovery checkpoint discarded.');
      }
    }
  }

  Future<void> _copyDescriptorPath() async {
    final path = _activeProject?.descriptorPath;
    if (path == null) return;
    await Clipboard.setData(ClipboardData(text: path));
    if (mounted) _showUnavailable('Local CLI descriptor path copied.');
  }

  void _showProjectError(ProjectGatewayException error) {
    final message = switch (error.code) {
      'DESTINATION_EXISTS' => 'A file already exists at that location. Choose another name or use Open Project.',
      'PROJECT_FILE_CHANGED' =>
        'The project file changed outside OR. The save was blocked.',
      'RECOVERY_REQUIRED' => 'This project needs explicit recovery handling before it can be opened.',
      'LOCAL_IPC_ERROR' => 'The local CLI session could not be started. The project remains on disk.',
      _ =>
        error.message.isEmpty ? 'The project operation failed.' : error.message,
    };
    _showUnavailable(message);
  }

  void _showUnavailable(String message) {
    if (!mounted) return;
    ScaffoldMessenger.of(context)
      ..hideCurrentSnackBar()
      ..showSnackBar(
        SnackBar(
          content: Text(message),
          behavior: SnackBarBehavior.floating,
          backgroundColor: OrColors.surface,
          showCloseIcon: true,
          duration: const Duration(seconds: 4),
        ),
      );
  }
}

class _ProjectNameDialog extends StatefulWidget {
  const _ProjectNameDialog({
    required this.title,
    required this.initialName,
    required this.fieldKey,
    required this.confirmKey,
    required this.confirmLabel,
  });

  final String title;
  final String initialName;
  final String fieldKey;
  final String confirmKey;
  final String confirmLabel;

  @override
  State<_ProjectNameDialog> createState() => _ProjectNameDialogState();
}

class _ProjectNameDialogState extends State<_ProjectNameDialog> {
  late final TextEditingController _controller = TextEditingController(
    text: widget.initialName,
  );
  String? _validationMessage;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _submit() {
    final value = _controller.text;
    if (value.isEmpty) {
      setState(() => _validationMessage = 'Enter a project name.');
      return;
    }
    Navigator.of(context).pop(value);
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: Text(widget.title),
    content: TextField(
      key: ValueKey(widget.fieldKey),
      controller: _controller,
      autofocus: true,
      decoration: InputDecoration(
        labelText: 'Project name',
        errorText: _validationMessage,
      ),
      onSubmitted: (_) => _submit(),
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.of(context).pop(),
        child: const Text('Cancel'),
      ),
      FilledButton(
        key: ValueKey(widget.confirmKey),
        onPressed: _submit,
        child: Text(widget.confirmLabel),
      ),
    ],
  );
}

enum _LeaveChoice { save, discard, cancel }

enum _RecoveryChoice { recover, discard, cancel }

enum _RecoveryMutation { apply, discard }

class _RecoveryPreparation {
  const _RecoveryPreparation({
    required this.proceed,
    this.notice,
    this.mutation,
  });

  final bool proceed;
  final String? notice;
  final _RecoveryMutation? mutation;
}

class _OpenCommandPaletteIntent extends Intent {
  const _OpenCommandPaletteIntent();
}

class _SaveProjectIntent extends Intent {
  const _SaveProjectIntent();
}

class _UndoProjectIntent extends Intent {
  const _UndoProjectIntent();
}

class _RedoProjectIntent extends Intent {
  const _RedoProjectIntent();
}
