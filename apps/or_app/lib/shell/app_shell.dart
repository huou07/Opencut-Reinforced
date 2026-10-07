import 'dart:async';
import 'dart:collection';
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

const _mediaPageSize = 50;
const _timelineClipPageSize = 100;
const _timelineMarkerPageSize = 100;
const _maxActiveMediaPreviewRequests = 8;
const _autosaveInterval = Duration(seconds: 30);

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
  StreamSubscription<ProjectMediaArtifactEvent>? _mediaArtifactSubscription;
  Timer? _autosaveTimer;
  Timer? _exportPollTimer;
  Future<void> _eventQueue = Future<void>.value();
  BigInt _lastEventSequence = BigInt.zero;
  bool _busy = false;
  bool _autosaveInFlight = false;
  String? _autosaveStatus;
  bool _autosaveFailed = false;
  bool _exportPollInFlight = false;
  bool _exportPublishing = false;
  bool _exportFailed = false;
  ProjectExportJob? _exportJob;
  String? _exportDestinationPath;
  String? _exportStatus;
  ProjectMediaPage? _mediaPage;
  final Map<String, ProjectMediaPreview> _mediaPreviews = {};
  final Queue<ProjectMediaItem> _mediaPreviewQueue = Queue();
  final Set<String> _queuedMediaPreviewIds = {};
  final List<_MediaPreviewTicket> _pendingMediaPreviewTickets = [];
  final Map<String, ProjectMediaArtifactEvent> _earlyMediaArtifactEvents = {};
  int _activeMediaPreviewRequests = 0;
  int _mediaPreviewSubmissionsInFlight = 0;
  BigInt _lastMediaArtifactSequence = BigInt.zero;
  bool _mediaLoading = false;
  bool _mediaLoadingMore = false;
  String? _mediaLoadError;
  int _mediaRefreshGeneration = 0;
  ProjectTimelineTracks? _timelineTracks;
  final Map<String, ProjectTimelineClipPage> _timelineClipPages = {};
  ProjectTimelineMarkerPage? _timelineMarkerPage;
  final Set<String> _timelineLoadingMoreTracks = {};
  bool _timelineLoadingMoreMarkers = false;
  bool _timelineLoading = false;
  String? _timelineLoadError;
  int _timelineRefreshGeneration = 0;
  late final AppLifecycleListener _lifecycleListener;

  bool get _hasProjectWorkspace =>
      _destination == AppDestination.editorPreview && _activeSession != null;

  @override
  void initState() {
    super.initState();
    _autosaveTimer = Timer.periodic(
      _autosaveInterval,
      (_) => unawaited(_autosaveProject()),
    );
    _lifecycleListener = AppLifecycleListener(
      onExitRequested: _onExitRequested,
    );
  }

  @override
  void dispose() {
    _autosaveTimer?.cancel();
    _exportPollTimer?.cancel();
    _lifecycleListener.dispose();
    unawaited(_eventSubscription?.cancel());
    unawaited(_mediaArtifactSubscription?.cancel());
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
                        statusLabel: editor
                            ? (_exportStatus ?? _autosaveStatus)
                            : null,
                        statusIsError: _exportStatus != null
                            ? _exportFailed
                            : _autosaveFailed,
                        onExport:
                            editor &&
                                _activeSession != null &&
                                widget.projectFilePicker.supportsExport &&
                                !(_exportJob?.isActive ?? false) &&
                                !_exportPublishing
                            ? _startExport
                            : null,
                        onCancelExport:
                            _exportJob?.isActive == true || _exportPublishing
                            ? _cancelExport
                            : null,
                        exportIsActive:
                            (_exportJob?.isActive ?? false) ||
                            _exportPublishing,
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
              projectGateway: widget.projectGateway,
              projectSession: _activeSession,
              notice: _projectNotice,
              busy: _busy,
              mediaPage: _mediaPage,
              mediaPreviews: Map.unmodifiable(_mediaPreviews),
              mediaLoading: _mediaLoading,
              mediaLoadingMore: _mediaLoadingMore,
              mediaLoadError: _mediaLoadError,
              timelineTracks: _timelineTracks,
              timelineClipPages: Map.unmodifiable(_timelineClipPages),
              timelineMarkerPage: _timelineMarkerPage,
              timelineLoadingMoreTracks: Set.unmodifiable(
                _timelineLoadingMoreTracks,
              ),
              timelineLoadingMoreMarkers: _timelineLoadingMoreMarkers,
              timelineLoading: _timelineLoading,
              timelineLoadError: _timelineLoadError,
              onImportMedia: widget.projectFilePicker.supportsMediaImport
                  ? _importMedia
                  : null,
              onLoadMoreMedia: _loadMoreMedia,
              onRefreshMedia: _refreshMediaLibrary,
              onRemoveMedia: _removeMedia,
              onAddVideoTrack: () =>
                  _addTimelineTrack(ProjectTimelineTrackKind.video),
              onAddAudioTrack: () =>
                  _addTimelineTrack(ProjectTimelineTrackKind.audio),
              onAddTextTrack: () =>
                  _addTimelineTrack(ProjectTimelineTrackKind.text),
              onAddCaptionTrack: () =>
                  _addTimelineTrack(ProjectTimelineTrackKind.caption),
              onRemoveTimelineTrack: _removeTimelineTrack,
              onSetTimelineTrackState: _setTimelineTrackState,
              onLoadMoreTimelineClips: _loadMoreTimelineClips,
              onLoadMoreTimelineMarkers: _loadMoreTimelineMarkers,
              onRefreshTimeline: _refreshTimelineFromUi,
              onAddMediaToTimeline: _insertMediaIntoTimeline,
              onInsertTimelineTextClip: _insertTimelineTextClip,
              onUpdateTimelineTextClip: _updateTimelineTextClip,
              onUpdateTimelineClipVisualSettings:
                  _updateTimelineClipVisualSettings,
              onUpdateTimelineClipAudioSettings:
                  _updateTimelineClipAudioSettings,
              onMoveTimelineClip: _moveTimelineClip,
              onDuplicateTimelineClip: _duplicateTimelineClip,
              onResolveTimelineSnap: _resolveTimelineSnap,
              onDeleteTimelineClip: _deleteTimelineClip,
              onTrimTimelineClip: _trimTimelineClip,
              onSplitTimelineClip: _splitTimelineClip,
              onRippleDeleteTimelineClip: _rippleDeleteTimelineClip,
              onAddTimelineMarker: _addTimelineMarker,
              onMoveTimelineMarker: _moveTimelineMarker,
              onRenameTimelineMarker: _renameTimelineMarker,
              onDeleteTimelineMarker: _deleteTimelineMarker,
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
      final syncNotice = await _syncNotice(path);
      await _startProject(
        session,
        path,
        notice: _combineNotices(resolvedRecovery.notice, syncNotice),
      );
    } on ProjectGatewayException catch (error) {
      _showProjectError(error);
    } on ProjectSafStorageException catch (error) {
      _showUnavailable(error.message);
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
    } on ProjectSafStorageException catch (error) {
      _showUnavailable(error.message);
    } catch (_) {
      _showUnavailable('The selected project could not be opened.');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  bool _checkFileLifecycleAvailable() {
    if (widget.projectFilePicker.isSupported) return true;
    _showUnavailable('Project storage is not available on this platform.');
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
    final syncNotice = preparation.mutation == _RecoveryMutation.apply
        ? await _syncNotice(path)
        : null;
    return _RecoveryPreparation(
      proceed: true,
      notice: _combineNotices(
        preparation.mutation == _RecoveryMutation.apply &&
                result.message.contains('cleanup is pending')
            ? result.message
            : preparation.notice,
        syncNotice,
      ),
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
    if (_exportJob?.isActive == true || _exportPublishing) {
      _showUnavailable(
        'Wait for the export to finish or cancel it before closing the project.',
      );
      return false;
    }
    final session = _activeSession;
    if (session == null) {
      return beforeClose == null ? true : beforeClose();
    }

    final current = await widget.projectGateway.summary(session);
    if (mounted && identical(session, _activeSession)) {
      setState(() => _activeProject = current);
    }
    if (!current.dirty) {
      final path = _activeProjectPath;
      if (path != null && !await _syncBeforeClose(path)) return false;
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
      final path = _activeProjectPath;
      if (path != null && !await _syncBeforeClose(path)) return false;
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
    _mediaArtifactSubscription = widget.projectGateway
        .watchMediaArtifacts(session)
        .listen(
          (event) => _handleMediaArtifactEvent(session, event),
          onError: (Object _) {},
        );
    _lastEventSequence = BigInt.zero;
    _lastMediaArtifactSequence = BigInt.zero;
    _timelineRefreshGeneration++;
    _mediaPreviewQueue.clear();
    _queuedMediaPreviewIds.clear();
    _pendingMediaPreviewTickets.clear();
    _earlyMediaArtifactEvents.clear();
    _activeMediaPreviewRequests = 0;
    _mediaPreviewSubmissionsInFlight = 0;
    setState(() {
      _activeSession = session;
      _activeProject = null;
      _mediaPage = null;
      _mediaPreviews.clear();
      _mediaLoading = true;
      _mediaLoadingMore = false;
      _mediaLoadError = null;
      _timelineTracks = null;
      _timelineClipPages.clear();
      _timelineMarkerPage = null;
      _timelineLoadingMoreTracks.clear();
      _timelineLoadingMoreMarkers = false;
      _timelineLoading = true;
      _timelineLoadError = null;
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
      await Future.wait([
        _refreshMediaPage(
          session,
          project: view,
          refreshGeneration: refreshGeneration,
        ),
        _refreshTimeline(session, project: view),
      ]);
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
      if (event.kind == 'project_saved') {
        setState(() {
          _autosaveStatus = null;
          _autosaveFailed = false;
        });
      }
      await _refreshProjectState(session);
    }
  }

  Future<ProjectReadModel?> _runProjectAction(
    Future<ProjectActionResult> Function(ProjectSessionHandle, ProjectReadModel)
    operation,
  ) {
    final current = _activeProject;
    if (current == null) return Future.value(null);
    return _runProjectActionAtSnapshot(current, operation);
  }

  Future<ProjectReadModel?> _runProjectActionAtSnapshot(
    ProjectReadModel expected,
    Future<ProjectActionResult> Function(ProjectSessionHandle, ProjectReadModel)
    operation,
  ) async {
    final session = _activeSession;
    final current = _activeProject;
    if (session == null || current == null || _busy) return null;
    if (!_sameProjectIdentity(expected, current)) {
      _showUnavailable(
        'The project changed while this action was open. Refresh the project before trying again.',
      );
      return null;
    }
    setState(() => _busy = true);
    try {
      final result = await operation(session, expected);
      if (!mounted || !identical(session, _activeSession)) return null;
      if (!result.succeeded) {
        if (result.errorCode == 'REVISION_CONFLICT') {
          await _refreshProjectState(session);
          _showUnavailable(
            'The project changed while this action was open. The current state is refreshed; this action was not retried.',
          );
        } else if (result.errorCode == 'PROBE_BACKEND_UNAVAILABLE') {
          _showUnavailable(
            'The packaged media inspector could not start. Check the app installation and try again.',
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
      final changed = updated.revision != expected.revision;
      if (mounted && identical(session, _activeSession)) {
        setState(() {
          _activeProject = updated;
          if (changed) {
            _mediaLoading = true;
            _timelineLoading = true;
            _timelineLoadingMoreMarkers = false;
          }
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
    final updated = await _runProjectAction(
      (session, _) => widget.projectGateway.save(session),
    );
    if (updated == null || !mounted) return;
    final path = _activeProjectPath;
    if (path == null) return;
    final notice = await _syncNotice(path);
    if (!mounted) return;
    setState(() => _projectNotice = notice);
    if (notice != null) {
      _showUnavailable(notice);
    }
  }

  Future<bool> _syncBeforeClose(String path) async {
    try {
      final result = await widget.projectFilePicker.synchronizeProjectPath(
        path,
      );
      if (result?.verified == false) {
        _showUnavailable(
          'The project was saved, but the external document did not support readback verification.',
        );
      }
      return true;
    } on ProjectSafStorageException catch (error) {
      _showUnavailable(error.message);
      return false;
    } catch (_) {
      _showUnavailable(
        'The project is saved on this device, but could not be synchronized to the selected document.',
      );
      return false;
    }
  }

  Future<String?> _syncNotice(String path) async {
    try {
      final result = await widget.projectFilePicker.synchronizeProjectPath(
        path,
      );
      if (result?.verified == false) {
        return 'The project was saved, but the external document did not support readback verification.';
      }
      return null;
    } on ProjectSafStorageException catch (error) {
      return error.message;
    } catch (_) {
      return 'The project is saved on this device, but could not be synchronized to the selected document.';
    }
  }

  static String? _combineNotices(String? first, String? second) {
    if (first == null || first.isEmpty) return second;
    if (second == null || second.isEmpty) return first;
    return '$first\n$second';
  }

  Future<void> _startExport() async {
    final session = _activeSession;
    if (session == null || !widget.projectFilePicker.supportsExport) return;
    String? destination;
    try {
      final current = await widget.projectGateway.summary(session);
      if (!mounted || !identical(session, _activeSession)) return;
      final suggestedName = _exportSuggestedName(current.name);
      destination = await widget.projectFilePicker.saveExportPath(
        suggestedName: suggestedName,
      );
      if (destination == null) {
        return;
      }
      if (!mounted || !identical(session, _activeSession)) {
        await _discardExportDestination(destination);
        return;
      }
      final result = await widget.projectGateway.startExport(
        session,
        current,
        destination,
      );
      if (!mounted || !identical(session, _activeSession)) {
        await _discardExportDestination(destination);
        return;
      }
      setState(() {
        _activeProject = current;
        _exportJob = result;
        _exportDestinationPath = destination;
        _exportFailed = !result.succeeded;
        _exportStatus = _exportJobLabel(result);
      });
      if (result.succeeded && result.isActive) {
        _exportPollTimer?.cancel();
        _exportPollTimer = Timer.periodic(
          const Duration(milliseconds: 400),
          (_) => unawaited(_pollExport()),
        );
      } else if (!result.succeeded) {
        await _discardExportDestination(destination);
      }
    } on ProjectGatewayException catch (error) {
      if (destination != null) await _discardExportDestination(destination);
      if (mounted && identical(session, _activeSession)) {
        setState(() {
          _exportStatus = error.message;
          _exportFailed = true;
        });
      }
    } catch (_) {
      if (destination != null) await _discardExportDestination(destination);
      if (mounted && identical(session, _activeSession)) {
        setState(() {
          _exportStatus = 'Export could not start';
          _exportFailed = true;
        });
      }
    }
  }

  Future<void> _pollExport() async {
    final session = _activeSession;
    final project = _activeProject;
    final job = _exportJob;
    if (session == null ||
        project == null ||
        job == null ||
        _exportPollInFlight) {
      return;
    }
    _exportPollInFlight = true;
    try {
      final status = await widget.projectGateway.exportStatus(
        session,
        project,
        job.jobId,
      );
      if (!mounted || !identical(session, _activeSession)) return;
      if (status.succeeded && status.state == 'succeeded') {
        final destination = _exportDestinationPath;
        if (destination != null) {
          setState(() {
            _exportJob = status;
            _exportPublishing = true;
            _exportFailed = false;
            _exportStatus = 'Saving export to the selected location…';
          });
          try {
            await widget.projectFilePicker.publishExportPath(destination);
            if (!mounted || !identical(session, _activeSession)) return;
            setState(() => _exportStatus = 'Export complete');
          } on ProjectSafStorageException catch (error) {
            if (!mounted || !identical(session, _activeSession)) return;
            setState(() {
              _exportStatus = error.message;
              _exportFailed = true;
            });
            await _discardExportDestination(destination);
          } catch (_) {
            if (!mounted || !identical(session, _activeSession)) return;
            setState(() {
              _exportStatus =
                  'The export could not be saved to the selected location.';
              _exportFailed = true;
            });
            await _discardExportDestination(destination);
          } finally {
            _exportDestinationPath = null;
            if (mounted) setState(() => _exportPublishing = false);
            _exportPollTimer?.cancel();
          }
          return;
        }
      }
      setState(() {
        _exportJob = status;
        _exportFailed = !status.succeeded || status.state == 'failed';
        _exportStatus = _exportJobLabel(status);
      });
      if (!status.succeeded || !status.isActive) {
        _exportPollTimer?.cancel();
        if (!status.succeeded || status.state == 'cancelled') {
          final destination = _exportDestinationPath;
          _exportDestinationPath = null;
          if (destination != null) await _discardExportDestination(destination);
        }
      }
    } catch (_) {
      if (!mounted || !identical(session, _activeSession)) return;
      setState(() {
        _exportStatus = 'Export status unavailable';
        _exportFailed = true;
      });
      _exportPollTimer?.cancel();
    } finally {
      _exportPollInFlight = false;
    }
  }

  Future<void> _cancelExport() async {
    final session = _activeSession;
    final project = _activeProject;
    final job = _exportJob;
    final destination = _exportDestinationPath;
    if (_exportPublishing && destination != null) {
      try {
        await widget.projectFilePicker.cancelExportPublish(destination);
      } catch (_) {
        if (mounted) {
          setState(() {
            _exportStatus = 'Export cancellation failed';
            _exportFailed = true;
          });
        }
      }
      return;
    }
    if (session == null || project == null || job == null || !job.isActive) {
      return;
    }
    try {
      final result = await widget.projectGateway.cancelExport(
        session,
        project,
        job.jobId,
      );
      if (!mounted || !identical(session, _activeSession)) return;
      setState(() {
        _exportJob = result;
        _exportFailed = !result.succeeded;
        _exportStatus = _exportJobLabel(result);
      });
      if (!result.succeeded || !result.isActive) {
        _exportPollTimer?.cancel();
        if (result.state == 'cancelled') {
          _exportDestinationPath = null;
          if (destination != null) await _discardExportDestination(destination);
        }
      }
    } catch (_) {
      if (!mounted || !identical(session, _activeSession)) return;
      setState(() {
        _exportStatus = 'Export cancellation failed';
        _exportFailed = true;
      });
    }
  }

  Future<void> _discardExportDestination(String path) async {
    try {
      await widget.projectFilePicker.discardExportPath(path);
    } catch (_) {
      // Preserve the primary export result if cleanup fails.
    }
  }

  static String _exportSuggestedName(String projectName) {
    final base = projectName.replaceAll(RegExp(r'[\\/:*?"<>|]'), '_').trim();
    return '${base.isEmpty ? 'Untitled' : base}.mkv';
  }

  static String _exportJobLabel(ProjectExportJob job) {
    if (!job.succeeded) {
      return job.message.isEmpty ? 'Export failed' : job.message;
    }
    return switch (job.state) {
      'queued' => 'Export queued',
      'running' =>
        job.progressCompleted != null && job.progressTotal != null
            ? 'Export ${job.progressCompleted}/${job.progressTotal}'
            : 'Exporting',
      'succeeded' => 'Export complete',
      'failed' => job.failureMessage ?? 'Export failed',
      'cancelled' => 'Export cancelled',
      _ => job.message,
    };
  }

  Future<void> _autosaveProject() async {
    final session = _activeSession;
    final project = _activeProject;
    if (session == null ||
        project == null ||
        !project.dirty ||
        _autosaveInFlight) {
      return;
    }
    _autosaveInFlight = true;
    if (mounted) {
      setState(() {
        _autosaveStatus = 'Autosaving recovery';
        _autosaveFailed = false;
      });
    }
    try {
      final result = await widget.projectGateway.autosaveCheckpoint(session);
      if (!mounted || !identical(session, _activeSession)) return;
      setState(() {
        _autosaveFailed = !result.succeeded;
        _autosaveStatus = result.succeeded
            ? 'Recovery saved'
            : 'Autosave needs attention';
      });
    } catch (_) {
      if (!mounted || !identical(session, _activeSession)) return;
      setState(() {
        _autosaveStatus = 'Autosave needs attention';
        _autosaveFailed = true;
      });
    } finally {
      _autosaveInFlight = false;
    }
  }

  Future<void> _undoProject() async {
    await _runProjectAction(widget.projectGateway.undo);
  }

  Future<void> _redoProject() async {
    await _runProjectAction(widget.projectGateway.redo);
  }

  Future<void> _addTimelineTrack(ProjectTimelineTrackKind kind) async {
    await _runProjectAction(
      (session, current) =>
          widget.projectGateway.addTimelineTrack(session, current, kind),
    );
  }

  Future<void> _removeTimelineTrack(
    ProjectReadModel expected,
    ProjectTimelineTrack track,
  ) async {
    await _runProjectActionAtSnapshot(
      expected,
      (session, current) => widget.projectGateway.removeTimelineTrack(
        session,
        current,
        track.trackId,
      ),
    );
  }

  Future<void> _setTimelineTrackState(
    ProjectReadModel expected,
    ProjectTimelineTrack track,
    ProjectTimelineTrackState state,
  ) async {
    await _runProjectActionAtSnapshot(
      expected,
      (session, current) => widget.projectGateway.setTimelineTrackState(
        session,
        current,
        trackId: track.trackId,
        state: state,
      ),
    );
  }

  Future<void> _insertMediaIntoTimeline(
    ProjectReadModel expected,
    ProjectMediaItem media,
    String trackId,
    ProjectRationalTime timelineStart,
    ProjectRationalTime sourceStart,
    ProjectRationalTime duration,
  ) async {
    await _runProjectActionAtSnapshot(
      expected,
      (session, current) => widget.projectGateway.insertTimelineClip(
        session,
        current,
        trackId: trackId,
        mediaId: media.mediaId,
        timelineStart: timelineStart,
        sourceStart: sourceStart,
        duration: duration,
      ),
    );
  }

  Future<void> _insertTimelineTextClip(
    ProjectReadModel expected,
    ProjectTimelineTrack track,
    ProjectRationalTime timelineStart,
    ProjectRationalTime timelineDuration,
    ProjectTimelineTextContent content,
  ) async {
    await _runProjectActionAtSnapshot(
      expected,
      (session, current) => widget.projectGateway.insertTimelineTextClip(
        session,
        current,
        trackId: track.trackId,
        timelineStart: timelineStart,
        timelineDuration: timelineDuration,
        content: content,
      ),
    );
  }

  Future<void> _updateTimelineTextClip(
    ProjectReadModel expected,
    ProjectTimelineTrack track,
    ProjectTimelineClip clip,
    ProjectRationalTime timelineDuration,
    ProjectTimelineTextContent content,
  ) async {
    await _runProjectActionAtSnapshot(
      expected,
      (session, current) => widget.projectGateway.updateTimelineTextClip(
        session,
        current,
        trackId: track.trackId,
        clipId: clip.clipId,
        timelineDuration: timelineDuration,
        content: content,
      ),
    );
  }

  Future<ProjectReadModel?> _updateTimelineClipVisualSettings(
    ProjectReadModel expected,
    String trackId,
    String clipId,
    ProjectTimelineVisualSettings settings,
  ) => _runProjectActionAtSnapshot(
    expected,
    (session, current) =>
        widget.projectGateway.updateTimelineClipVisualSettings(
          session,
          current,
          trackId: trackId,
          clipId: clipId,
          settings: settings,
        ),
  );

  Future<ProjectReadModel?> _updateTimelineClipAudioSettings(
    ProjectReadModel expected,
    String trackId,
    String clipId,
    ProjectTimelineAudioSettings settings,
  ) => _runProjectActionAtSnapshot(
    expected,
    (session, current) => widget.projectGateway.updateTimelineClipAudioSettings(
      session,
      current,
      trackId: trackId,
      clipId: clipId,
      settings: settings,
    ),
  );

  Future<void> _moveTimelineClip(
    ProjectReadModel expected,
    ProjectTimelineClip clip,
    String trackId,
    ProjectRationalTime timelineStart,
  ) async {
    final session = _activeSession;
    final result = await _runProjectActionAtSnapshot(
      expected,
      (session, current) => widget.projectGateway.moveTimelineClip(
        session,
        current,
        clipId: clip.clipId,
        trackId: trackId,
        timelineStart: timelineStart,
      ),
    );
    if (result == null &&
        mounted &&
        session != null &&
        identical(session, _activeSession)) {
      await _refreshProjectState(session);
    }
  }

  Future<void> _duplicateTimelineClip(
    ProjectReadModel expected,
    ProjectTimelineTrack track,
    ProjectTimelineClip clip,
  ) async {
    final timelineStart = ProjectRationalTime.tryParse(
      clip.timelineStart.add(clip.timelineDuration).canonical,
    );
    if (timelineStart == null) {
      _showUnavailable('The clip cannot be duplicated at that exact time.');
      return;
    }
    if (clip.contentKind != ProjectTimelineClipContentKind.media) {
      await _insertTimelineTextClip(
        expected,
        track,
        timelineStart,
        clip.timelineDuration,
        ProjectTimelineTextContent(
          kind: clip.contentKind,
          text: clip.text ?? '',
          formatting: clip.formatting ?? ProjectTextFormatting.defaults,
        ),
      );
      return;
    }
    final mediaId = clip.mediaId;
    final sourceStart = clip.sourceStart;
    if (mediaId == null || sourceStart == null) {
      _showUnavailable('The media clip is missing its source range.');
      return;
    }
    await _runProjectActionAtSnapshot(
      expected,
      (session, current) => widget.projectGateway.insertTimelineClip(
        session,
        current,
        trackId: track.trackId,
        mediaId: mediaId,
        timelineStart: timelineStart,
        sourceStart: sourceStart,
        duration: clip.timelineDuration,
      ),
    );
  }

  Future<ProjectTimelineSnapResult> _resolveTimelineSnap(
    ProjectReadModel expected,
    ProjectTimelineSnapOperation operation,
    String clipId,
    String? targetTrackId,
    ProjectRationalTime targetTime,
  ) async {
    final session = _activeSession;
    final current = _activeProject;
    if (session == null ||
        current == null ||
        _busy ||
        !_sameProjectIdentity(expected, current) ||
        expected.revision != current.revision) {
      if (session != null && mounted && identical(session, _activeSession)) {
        await _refreshProjectState(session);
      }
      throw ProjectGatewayException(
        'REVISION_CONFLICT',
        'The project changed while the timeline gesture was active.',
      );
    }
    final result = await widget.projectGateway.resolveTimelineSnap(
      session,
      expected,
      operation: operation,
      clipId: clipId,
      targetTrackId: targetTrackId,
      targetTime: targetTime,
    );
    final latest = _activeProject;
    if (!mounted ||
        !identical(session, _activeSession) ||
        latest == null ||
        !_sameProjectIdentity(expected, latest) ||
        expected.revision != latest.revision ||
        result.projectId != expected.projectId ||
        result.projectInstanceId != expected.projectInstanceId ||
        result.projectRevision != expected.revision ||
        result.rawTargetTime.canonical != targetTime.canonical) {
      throw ProjectGatewayException(
        'REVISION_CONFLICT',
        'The timeline snap result is stale.',
      );
    }
    return result;
  }

  Future<void> _deleteTimelineClip(
    ProjectReadModel expected,
    ProjectTimelineClip clip,
  ) async {
    await _runProjectActionAtSnapshot(
      expected,
      (session, current) => widget.projectGateway.deleteTimelineClip(
        session,
        current,
        clip.clipId,
      ),
    );
  }

  Future<void> _trimTimelineClip(
    ProjectReadModel expected,
    ProjectTimelineClip clip,
    ProjectTimelineTrimEdge edge,
    ProjectRationalTime timelineTime,
  ) async {
    final session = _activeSession;
    final result = await _runProjectActionAtSnapshot(
      expected,
      (session, current) => widget.projectGateway.trimTimelineClip(
        session,
        current,
        clipId: clip.clipId,
        edge: edge,
        timelineTime: timelineTime,
      ),
    );
    if (result == null &&
        mounted &&
        session != null &&
        identical(session, _activeSession)) {
      await _refreshProjectState(session);
    }
  }

  Future<void> _splitTimelineClip(
    ProjectReadModel expected,
    ProjectTimelineClip clip,
    ProjectRationalTime timelineTime,
  ) async {
    await _runProjectActionAtSnapshot(
      expected,
      (session, current) => widget.projectGateway.splitTimelineClip(
        session,
        current,
        clipId: clip.clipId,
        timelineTime: timelineTime,
      ),
    );
  }

  Future<void> _rippleDeleteTimelineClip(
    ProjectReadModel expected,
    ProjectTimelineClip clip,
  ) async {
    await _runProjectActionAtSnapshot(
      expected,
      (session, current) => widget.projectGateway.rippleDeleteTimelineClip(
        session,
        current,
        clip.clipId,
      ),
    );
  }

  Future<void> _addTimelineMarker(
    ProjectReadModel expected,
    ProjectRationalTime timelineTime,
    String label,
  ) async {
    await _runProjectActionAtSnapshot(
      expected,
      (session, current) => widget.projectGateway.addTimelineMarker(
        session,
        current,
        timelineTime: timelineTime,
        label: label,
      ),
    );
  }

  Future<void> _moveTimelineMarker(
    ProjectReadModel expected,
    ProjectTimelineMarker marker,
    ProjectRationalTime timelineTime,
  ) async {
    await _runProjectActionAtSnapshot(
      expected,
      (session, current) => widget.projectGateway.moveTimelineMarker(
        session,
        current,
        markerId: marker.markerId,
        timelineTime: timelineTime,
      ),
    );
  }

  Future<void> _renameTimelineMarker(
    ProjectReadModel expected,
    ProjectTimelineMarker marker,
    String label,
  ) async {
    await _runProjectActionAtSnapshot(
      expected,
      (session, current) => widget.projectGateway.renameTimelineMarker(
        session,
        current,
        markerId: marker.markerId,
        label: label,
      ),
    );
  }

  Future<void> _deleteTimelineMarker(
    ProjectReadModel expected,
    ProjectTimelineMarker marker,
  ) async {
    await _runProjectActionAtSnapshot(
      expected,
      (session, current) => widget.projectGateway.deleteTimelineMarker(
        session,
        current,
        marker.markerId,
      ),
    );
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
      _queueMediaPreviews(session, next.items);
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

  Future<void> _refreshTimelineFromUi() async {
    final session = _activeSession;
    final project = _activeProject;
    if (session != null && project != null) {
      await _refreshTimeline(session, project: project);
    }
  }

  Future<void> _refreshTimeline(
    ProjectSessionHandle session, {
    required ProjectReadModel project,
  }) async {
    if (!mounted || !identical(session, _activeSession)) return;
    final generation = ++_timelineRefreshGeneration;
    final mediaGeneration = _mediaRefreshGeneration;
    var current = project;
    setState(() {
      _timelineTracks = null;
      _timelineClipPages.clear();
      _timelineMarkerPage = null;
      _timelineLoadingMoreTracks.clear();
      _timelineLoadingMoreMarkers = false;
      _timelineLoading = true;
      _timelineLoadError = null;
    });

    for (var attempt = 0; attempt < 2; attempt++) {
      var inconsistent = false;
      try {
        final tracks = await widget.projectGateway.listTimelineTracks(session);
        if (!_timelineRequestIsCurrent(session, generation)) return;
        if (!_matchesTimelineProject(tracks, current)) {
          inconsistent = true;
        } else {
          final pages = <String, ProjectTimelineClipPage>{};
          for (final track in tracks.items) {
            final page = track.clipCount == 0
                ? ProjectTimelineClipPage(
                    projectId: tracks.projectId,
                    projectInstanceId: tracks.projectInstanceId,
                    projectRevision: tracks.projectRevision,
                    trackId: track.trackId,
                    items: const [],
                    totalCount: 0,
                    offset: 0,
                    limit: _timelineClipPageSize,
                    nextOffset: null,
                  )
                : await widget.projectGateway.listTimelineClips(
                    session,
                    trackId: track.trackId,
                    offset: 0,
                    limit: _timelineClipPageSize,
                  );
            if (!_timelineRequestIsCurrent(session, generation)) return;
            if (!_matchesTimelineClipPage(
              page,
              current,
              trackId: track.trackId,
              offset: 0,
              clipCount: track.clipCount,
            )) {
              inconsistent = true;
              break;
            }
            pages[track.trackId] = page;
          }

          ProjectTimelineMarkerPage? markers;
          if (!inconsistent) {
            markers = await widget.projectGateway.listTimelineMarkers(
              session,
              offset: 0,
              limit: _timelineMarkerPageSize,
            );
            if (!_timelineRequestIsCurrent(session, generation)) return;
            if (!_matchesTimelineMarkerPage(markers, current, offset: 0)) {
              inconsistent = true;
            }
          }

          if (!inconsistent && markers != null) {
            setState(() {
              _activeProject = current;
              _timelineTracks = tracks;
              _timelineClipPages
                ..clear()
                ..addAll(pages);
              _timelineMarkerPage = markers;
              _timelineLoading = false;
              _timelineLoadError = null;
            });
            if (current.revision != project.revision) {
              await _refreshMediaPage(
                session,
                project: current,
                refreshGeneration: mediaGeneration,
              );
            }
            return;
          }
        }
      } on ProjectGatewayException catch (error) {
        if (_timelineRequestIsCurrent(session, generation)) {
          setState(() {
            _timelineLoading = false;
            _timelineLoadError = error.message;
          });
        }
        return;
      } catch (_) {
        if (_timelineRequestIsCurrent(session, generation)) {
          setState(() {
            _timelineLoading = false;
            _timelineLoadError = 'The timeline could not load.';
          });
        }
        return;
      }

      if (!inconsistent) return;
      if (attempt == 1) break;
      try {
        current = await widget.projectGateway.summary(session);
      } on Object {
        if (_timelineRequestIsCurrent(session, generation)) {
          setState(() {
            _timelineLoading = false;
            _timelineLoadError = 'The project summary could not be refreshed.';
          });
        }
        return;
      }
      if (!_timelineRequestIsCurrent(session, generation)) return;
      setState(() => _activeProject = current);
    }

    if (!_timelineRequestIsCurrent(session, generation)) return;
    setState(() {
      _activeProject = current;
      _timelineTracks = null;
      _timelineClipPages.clear();
      _timelineMarkerPage = null;
      _timelineLoading = false;
      _timelineLoadError = 'The timeline changed while it was refreshing.';
    });
    if (current.revision != project.revision) {
      await _refreshMediaPage(
        session,
        project: current,
        refreshGeneration: mediaGeneration,
      );
    }
  }

  bool _timelineRequestIsCurrent(
    ProjectSessionHandle session,
    int generation,
  ) =>
      mounted &&
      identical(session, _activeSession) &&
      generation == _timelineRefreshGeneration;

  Future<void> _loadMoreTimelineClips(String trackId) async {
    final session = _activeSession;
    final project = _activeProject;
    final tracks = _timelineTracks;
    final page = _timelineClipPages[trackId];
    final offset = page?.nextOffset;
    ProjectTimelineTrack? track;
    for (final candidate in tracks?.items ?? const <ProjectTimelineTrack>[]) {
      if (candidate.trackId == trackId) {
        track = candidate;
        break;
      }
    }
    final generation = _timelineRefreshGeneration;
    if (session == null ||
        project == null ||
        tracks == null ||
        page == null ||
        offset == null ||
        track == null ||
        _timelineLoading ||
        _timelineLoadingMoreTracks.contains(trackId)) {
      return;
    }

    setState(() => _timelineLoadingMoreTracks.add(trackId));
    try {
      final next = await widget.projectGateway.listTimelineClips(
        session,
        trackId: trackId,
        offset: offset,
        limit: _timelineClipPageSize,
      );
      if (!_timelineRequestIsCurrent(session, generation)) return;
      if (!_matchesTimelineClipPage(
            next,
            project,
            trackId: trackId,
            offset: offset,
            clipCount: track.clipCount,
          ) ||
          !_matchesTimelineProject(tracks, project) ||
          page.projectRevision != project.revision) {
        await _refreshProjectState(session);
        return;
      }
      setState(() {
        _timelineClipPages[trackId] = ProjectTimelineClipPage(
          projectId: page.projectId,
          projectInstanceId: page.projectInstanceId,
          projectRevision: page.projectRevision,
          trackId: page.trackId,
          items: [...page.items, ...next.items],
          totalCount: next.totalCount,
          offset: 0,
          limit: page.limit,
          nextOffset: next.nextOffset,
        );
        _timelineLoadError = null;
      });
    } on ProjectGatewayException catch (error) {
      if (_timelineRequestIsCurrent(session, generation)) {
        setState(() => _timelineLoadError = error.message);
      }
    } catch (_) {
      if (_timelineRequestIsCurrent(session, generation)) {
        setState(() => _timelineLoadError = 'The timeline could not load.');
      }
    } finally {
      if (mounted && identical(session, _activeSession)) {
        setState(() => _timelineLoadingMoreTracks.remove(trackId));
      }
    }
  }

  Future<void> _loadMoreTimelineMarkers() async {
    final session = _activeSession;
    final project = _activeProject;
    final page = _timelineMarkerPage;
    final offset = page?.nextOffset;
    final generation = _timelineRefreshGeneration;
    if (session == null ||
        project == null ||
        page == null ||
        offset == null ||
        _timelineLoading ||
        _timelineLoadingMoreMarkers) {
      return;
    }

    setState(() => _timelineLoadingMoreMarkers = true);
    try {
      final next = await widget.projectGateway.listTimelineMarkers(
        session,
        offset: offset,
        limit: _timelineMarkerPageSize,
      );
      if (!_timelineRequestIsCurrent(session, generation)) return;
      if (!_matchesTimelineMarkerPage(next, project, offset: offset) ||
          page.projectId != project.projectId ||
          page.projectInstanceId != project.projectInstanceId ||
          page.projectRevision != project.revision) {
        await _refreshProjectState(session);
        return;
      }
      setState(() {
        _timelineMarkerPage = ProjectTimelineMarkerPage(
          projectId: page.projectId,
          projectInstanceId: page.projectInstanceId,
          projectRevision: page.projectRevision,
          items: [...page.items, ...next.items],
          totalCount: next.totalCount,
          offset: 0,
          limit: page.limit,
          nextOffset: next.nextOffset,
        );
        _timelineLoadError = null;
      });
    } on ProjectGatewayException catch (error) {
      if (_timelineRequestIsCurrent(session, generation)) {
        setState(() => _timelineLoadError = error.message);
      }
    } catch (_) {
      if (_timelineRequestIsCurrent(session, generation)) {
        setState(() => _timelineLoadError = 'The timeline could not load.');
      }
    } finally {
      if (mounted && identical(session, _activeSession)) {
        setState(() => _timelineLoadingMoreMarkers = false);
      }
    }
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
        _timelineTracks = null;
        _timelineClipPages.clear();
        _timelineMarkerPage = null;
        _timelineLoadingMoreTracks.clear();
        _timelineLoadingMoreMarkers = false;
        _timelineLoading = true;
        _timelineLoadError = null;
      });
      await Future.wait([
        _refreshMediaPage(
          session,
          project: view,
          refreshGeneration: refreshGeneration,
        ),
        _refreshTimeline(session, project: view),
      ]);
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
          _queueMediaPreviews(session, page.items);
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

  void _queueMediaPreviews(
    ProjectSessionHandle session,
    List<ProjectMediaItem> items,
  ) {
    if (!mounted || !identical(session, _activeSession)) return;

    final visibleIds = items.map((item) => item.mediaId).toSet();
    final nextPreviews = Map<String, ProjectMediaPreview>.of(_mediaPreviews)
      ..removeWhere((mediaId, _) => !visibleIds.contains(mediaId));
    _mediaPreviewQueue.removeWhere(
      (item) => !visibleIds.contains(item.mediaId),
    );
    _queuedMediaPreviewIds.removeWhere(
      (mediaId) => !visibleIds.contains(mediaId),
    );

    for (final item in items) {
      final kind = _mediaPreviewKind(item);
      if (kind == null || nextPreviews[item.mediaId]?.kind == kind) continue;
      nextPreviews[item.mediaId] = ProjectMediaPreview(
        kind: kind,
        state: ProjectMediaArtifactRequestState.queued,
      );
      if (_queuedMediaPreviewIds.add(item.mediaId)) {
        _mediaPreviewQueue.addLast(item);
      }
    }

    setState(() {
      _mediaPreviews
        ..clear()
        ..addAll(nextPreviews);
    });
    _scheduleMediaPreviewRequests(session);
  }

  void _scheduleMediaPreviewRequests(ProjectSessionHandle session) {
    if (!mounted || !identical(session, _activeSession)) return;
    while (_activeMediaPreviewRequests < _maxActiveMediaPreviewRequests &&
        _mediaPreviewQueue.isNotEmpty) {
      final page = _mediaPage;
      if (page == null) return;
      final item = _mediaPreviewQueue.removeFirst();
      _queuedMediaPreviewIds.remove(item.mediaId);
      if (!page.items.any((visible) => visible.mediaId == item.mediaId)) {
        continue;
      }
      final kind = _mediaPreviewKind(item);
      if (kind == null) continue;
      _activeMediaPreviewRequests++;
      unawaited(_requestMediaPreview(session, item, kind));
    }
  }

  Future<void> _requestMediaPreview(
    ProjectSessionHandle session,
    ProjectMediaItem item,
    ProjectMediaArtifactKind kind,
  ) async {
    _mediaPreviewSubmissionsInFlight++;
    try {
      final request = switch (kind) {
        ProjectMediaArtifactKind.thumbnail =>
          await widget.projectGateway.requestMediaThumbnail(
            session,
            item.mediaId,
          ),
        ProjectMediaArtifactKind.waveform =>
          await widget.projectGateway.requestMediaWaveform(
            session,
            item.mediaId,
          ),
      };
      if (!mounted || !identical(session, _activeSession)) return;
      if (request.kind != kind) {
        _updateMediaPreview(
          session,
          item.mediaId,
          ProjectMediaPreview(
            kind: kind,
            state: ProjectMediaArtifactRequestState.failed,
            errorCode: 'INVALID_ARTIFACT_RESPONSE',
          ),
        );
        _finishMediaPreviewRequest(session);
        return;
      }

      final preview = ProjectMediaPreview(
        kind: kind,
        state: request.state,
        cacheKey: request.cacheKey,
        jobId: request.jobId,
        errorCode: request.errorCode,
      );
      _updateMediaPreview(session, item.mediaId, preview);
      switch (request.state) {
        case ProjectMediaArtifactRequestState.ready:
          final cacheKey = request.cacheKey;
          if (cacheKey == null) {
            _updateMediaPreview(
              session,
              item.mediaId,
              ProjectMediaPreview(
                kind: kind,
                state: ProjectMediaArtifactRequestState.failed,
                errorCode: 'INVALID_ARTIFACT_RESPONSE',
              ),
            );
          } else {
            await _readReadyMediaPreview(session, item.mediaId, kind, cacheKey);
          }
          _finishMediaPreviewRequest(session);
        case ProjectMediaArtifactRequestState.queued ||
            ProjectMediaArtifactRequestState.running:
          final cacheKey = request.cacheKey;
          final jobId = request.jobId;
          if (cacheKey == null || jobId == null) {
            _updateMediaPreview(
              session,
              item.mediaId,
              ProjectMediaPreview(
                kind: kind,
                state: ProjectMediaArtifactRequestState.failed,
                errorCode: 'INVALID_ARTIFACT_RESPONSE',
              ),
            );
            _finishMediaPreviewRequest(session);
          } else {
            final ticket = _MediaPreviewTicket(
              mediaId: item.mediaId,
              kind: kind,
              cacheKey: cacheKey,
              jobId: jobId,
            );
            _pendingMediaPreviewTickets.add(ticket);
            final earlyEvent =
                _earlyMediaArtifactEvents[_mediaArtifactEventKey(
                  kind,
                  cacheKey,
                  jobId,
                )];
            if (earlyEvent != null) {
              unawaited(_processMediaArtifactEvent(session, earlyEvent));
            }
          }
        case ProjectMediaArtifactRequestState.notApplicable ||
            ProjectMediaArtifactRequestState.failed:
          _finishMediaPreviewRequest(session);
      }
    } catch (_) {
      if (mounted && identical(session, _activeSession)) {
        _updateMediaPreview(
          session,
          item.mediaId,
          ProjectMediaPreview(
            kind: kind,
            state: ProjectMediaArtifactRequestState.failed,
            errorCode: 'PREVIEW_UNAVAILABLE',
          ),
        );
        _finishMediaPreviewRequest(session);
      }
    } finally {
      if (identical(session, _activeSession)) {
        _mediaPreviewSubmissionsInFlight--;
        if (_mediaPreviewSubmissionsInFlight == 0) {
          _earlyMediaArtifactEvents.clear();
        }
      }
    }
  }

  Future<void> _readReadyMediaPreview(
    ProjectSessionHandle session,
    String mediaId,
    ProjectMediaArtifactKind kind,
    String cacheKey,
  ) async {
    try {
      final artifact = await widget.projectGateway.readMediaArtifact(
        session,
        kind: kind,
        cacheKey: cacheKey,
      );
      if (!mounted || !identical(session, _activeSession)) return;
      final current = _mediaPreviews[mediaId];
      if (current == null || current.cacheKey != cacheKey) return;
      _updateMediaPreview(
        session,
        mediaId,
        ProjectMediaPreview(
          kind: kind,
          state: artifact?.mimeType == 'image/png'
              ? ProjectMediaArtifactRequestState.ready
              : ProjectMediaArtifactRequestState.failed,
          cacheKey: cacheKey,
          bytes: artifact?.mimeType == 'image/png' ? artifact?.bytes : null,
          errorCode: artifact == null ? 'ARTIFACT_NOT_FOUND' : null,
        ),
      );
    } catch (_) {
      if (!mounted || !identical(session, _activeSession)) return;
      final current = _mediaPreviews[mediaId];
      if (current == null || current.cacheKey != cacheKey) return;
      _updateMediaPreview(
        session,
        mediaId,
        ProjectMediaPreview(
          kind: kind,
          state: ProjectMediaArtifactRequestState.failed,
          cacheKey: cacheKey,
          errorCode: 'ARTIFACT_READ_FAILED',
        ),
      );
    }
  }

  void _updateMediaPreview(
    ProjectSessionHandle session,
    String mediaId,
    ProjectMediaPreview preview,
  ) {
    if (!mounted ||
        !identical(session, _activeSession) ||
        !_mediaPreviews.containsKey(mediaId)) {
      return;
    }
    setState(() => _mediaPreviews[mediaId] = preview);
  }

  void _finishMediaPreviewRequest(ProjectSessionHandle session) {
    if (!mounted || !identical(session, _activeSession)) return;
    if (_activeMediaPreviewRequests > 0) _activeMediaPreviewRequests--;
    _scheduleMediaPreviewRequests(session);
  }

  void _handleMediaArtifactEvent(
    ProjectSessionHandle session,
    ProjectMediaArtifactEvent event,
  ) {
    if (!mounted || !identical(session, _activeSession)) return;
    if (event.sequence <= _lastMediaArtifactSequence) return;
    _lastMediaArtifactSequence = event.sequence;
    _dispatchMediaArtifactEvent(session, event);
  }

  void _dispatchMediaArtifactEvent(
    ProjectSessionHandle session,
    ProjectMediaArtifactEvent event,
  ) {
    if (_mediaPreviewSubmissionsInFlight > 0) {
      _earlyMediaArtifactEvents[_mediaArtifactEventKey(
            event.kind,
            event.cacheKey,
            event.jobId,
          )] =
          event;
    }
    final matching = _pendingMediaPreviewTickets
        .where(
          (ticket) =>
              ticket.kind == event.kind &&
              ticket.cacheKey == event.cacheKey &&
              ticket.jobId == event.jobId,
        )
        .toList(growable: false);
    if (matching.isEmpty) {
      return;
    }
    unawaited(_processMediaArtifactEvent(session, event));
  }

  Future<void> _processMediaArtifactEvent(
    ProjectSessionHandle session,
    ProjectMediaArtifactEvent event,
  ) async {
    if (!mounted || !identical(session, _activeSession)) return;
    final matching = _pendingMediaPreviewTickets
        .where(
          (ticket) =>
              ticket.kind == event.kind &&
              ticket.cacheKey == event.cacheKey &&
              ticket.jobId == event.jobId,
        )
        .toList(growable: false);
    if (matching.isEmpty) return;
    _pendingMediaPreviewTickets.removeWhere(matching.contains);
    _activeMediaPreviewRequests = _activeMediaPreviewRequests >= matching.length
        ? _activeMediaPreviewRequests - matching.length
        : 0;

    final targetIds = matching.map((ticket) => ticket.mediaId).where((mediaId) {
      final preview = _mediaPreviews[mediaId];
      return preview != null &&
          preview.kind == event.kind &&
          preview.cacheKey == event.cacheKey &&
          preview.jobId == event.jobId;
    }).toSet();
    if (event.state != ProjectMediaArtifactEventState.succeeded) {
      setState(() {
        for (final mediaId in targetIds) {
          final current = _mediaPreviews[mediaId]!;
          _mediaPreviews[mediaId] = ProjectMediaPreview(
            kind: current.kind,
            state: ProjectMediaArtifactRequestState.failed,
            cacheKey: current.cacheKey,
            jobId: current.jobId,
            errorCode: event.errorCode ?? 'PREVIEW_UNAVAILABLE',
          );
        }
      });
      _scheduleMediaPreviewRequests(session);
      return;
    }

    _scheduleMediaPreviewRequests(session);
    if (targetIds.isEmpty) return;
    try {
      final artifact = await widget.projectGateway.readMediaArtifact(
        session,
        kind: event.kind,
        cacheKey: event.cacheKey,
      );
      if (!mounted || !identical(session, _activeSession)) return;
      final hasPng = artifact?.mimeType == 'image/png';
      setState(() {
        for (final mediaId in targetIds) {
          final current = _mediaPreviews[mediaId];
          if (current == null ||
              current.cacheKey != event.cacheKey ||
              current.jobId != event.jobId) {
            continue;
          }
          _mediaPreviews[mediaId] = ProjectMediaPreview(
            kind: current.kind,
            state: hasPng
                ? ProjectMediaArtifactRequestState.ready
                : ProjectMediaArtifactRequestState.failed,
            cacheKey: current.cacheKey,
            jobId: current.jobId,
            bytes: hasPng ? artifact?.bytes : null,
            errorCode: hasPng ? null : 'ARTIFACT_READ_FAILED',
          );
        }
      });
    } catch (_) {
      if (!mounted || !identical(session, _activeSession)) return;
      setState(() {
        for (final mediaId in targetIds) {
          final current = _mediaPreviews[mediaId];
          if (current == null || current.cacheKey != event.cacheKey) continue;
          _mediaPreviews[mediaId] = ProjectMediaPreview(
            kind: current.kind,
            state: ProjectMediaArtifactRequestState.failed,
            cacheKey: current.cacheKey,
            jobId: current.jobId,
            errorCode: 'ARTIFACT_READ_FAILED',
          );
        }
      });
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
    _exportPollTimer?.cancel();
    unawaited(_eventSubscription?.cancel());
    unawaited(_mediaArtifactSubscription?.cancel());
    _mediaPreviewQueue.clear();
    _queuedMediaPreviewIds.clear();
    _pendingMediaPreviewTickets.clear();
    _earlyMediaArtifactEvents.clear();
    _activeMediaPreviewRequests = 0;
    _mediaPreviewSubmissionsInFlight = 0;
    _lastMediaArtifactSequence = BigInt.zero;
    if (!mounted) return;
    setState(() {
      _eventSubscription = null;
      _activeSession = null;
      _activeProject = null;
      _mediaPage = null;
      _mediaPreviews.clear();
      _mediaLoading = false;
      _mediaLoadingMore = false;
      _mediaLoadError = null;
      _timelineTracks = null;
      _timelineClipPages.clear();
      _timelineMarkerPage = null;
      _timelineLoadingMoreTracks.clear();
      _timelineLoadingMoreMarkers = false;
      _timelineLoading = false;
      _timelineLoadError = null;
      _timelineRefreshGeneration++;
      _activeProjectPath = null;
      _projectNotice = null;
      _autosaveStatus = null;
      _autosaveFailed = false;
      _exportJob = null;
      _exportStatus = null;
      _exportFailed = false;
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
    if (path == null || path.isEmpty) return;
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

ProjectMediaArtifactKind? _mediaPreviewKind(ProjectMediaItem item) {
  if (item.videoDetails != null) return ProjectMediaArtifactKind.thumbnail;
  if (item.audioDetails != null) return ProjectMediaArtifactKind.waveform;
  return null;
}

String _mediaArtifactEventKey(
  ProjectMediaArtifactKind kind,
  String cacheKey,
  String jobId,
) => '${kind.name}\u0000$cacheKey\u0000$jobId';

class _MediaPreviewTicket {
  const _MediaPreviewTicket({
    required this.mediaId,
    required this.kind,
    required this.cacheKey,
    required this.jobId,
  });

  final String mediaId;
  final ProjectMediaArtifactKind kind;
  final String cacheKey;
  final String jobId;
}

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

bool _sameProjectIdentity(ProjectReadModel first, ProjectReadModel second) =>
    first.projectId == second.projectId &&
    first.projectInstanceId == second.projectInstanceId;

bool _matchesTimelineProject(
  ProjectTimelineTracks tracks,
  ProjectReadModel project,
) =>
    tracks.projectId == project.projectId &&
    tracks.projectInstanceId == project.projectInstanceId &&
    tracks.projectRevision == project.revision &&
    tracks.items.length <= 256;

bool _matchesTimelineClipPage(
  ProjectTimelineClipPage page,
  ProjectReadModel project, {
  required String trackId,
  required int offset,
  required int clipCount,
}) {
  final expectedItemCount = (clipCount - offset)
      .clamp(0, _timelineClipPageSize)
      .toInt();
  if (page.projectId != project.projectId ||
      page.projectInstanceId != project.projectInstanceId ||
      page.projectRevision != project.revision ||
      page.trackId != trackId ||
      page.offset != offset ||
      page.limit != _timelineClipPageSize ||
      page.totalCount != clipCount ||
      page.items.length != expectedItemCount) {
    return false;
  }
  final end = offset + page.items.length;
  return page.nextOffset == (end < clipCount ? end : null);
}

bool _matchesTimelineMarkerPage(
  ProjectTimelineMarkerPage page,
  ProjectReadModel project, {
  required int offset,
}) {
  final expectedItemCount = (page.totalCount - offset)
      .clamp(0, _timelineMarkerPageSize)
      .toInt();
  if (page.projectId != project.projectId ||
      page.projectInstanceId != project.projectInstanceId ||
      page.projectRevision != project.revision ||
      page.offset != offset ||
      page.limit != _timelineMarkerPageSize ||
      page.totalCount < offset ||
      page.items.length != expectedItemCount) {
    return false;
  }
  final end = offset + page.items.length;
  return page.nextOffset == (end < page.totalCount ? end : null);
}
