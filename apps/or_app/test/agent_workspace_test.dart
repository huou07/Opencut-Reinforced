import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:or_app/assistant/agent_workspace.dart';

void main() {
  AgentWorkspaceBundle bundle(int version, Map<String, String> files) =>
      AgentWorkspaceBundle(version: version, files: files);

  test('a bundle refuses paths that escape the workspace', () {
    for (final path in const [
      '/etc/passwd',
      '../outside.md',
      'docs/../../outside.md',
      r'docs\windows.md',
      'docs//empty.md',
      '',
    ]) {
      expect(
        () => AgentWorkspaceBundle.fromJson({
          'version': 1,
          'files': {path: 'x'},
        }),
        throwsFormatException,
        reason: path,
      );
    }
    expect(isSafeWorkspacePath('docs/guide.md'), isTrue);
    expect(
      isSafeWorkspacePath('.agents/skills/or-timeline-edit/SKILL.md'),
      isTrue,
    );
  });

  test(
    'install writes the framework files and the agent work folders',
    () async {
      final root = await Directory.systemTemp.createTemp('or-agent-workspace-');
      addTearDown(() => root.delete(recursive: true));
      final workspace = await AgentWorkspace.install(
        directory: root,
        bundle: bundle(1, {
          'AGENTS.md': '# index\n',
          'docs/guide.md': '# guide\n',
        }),
      );

      expect(File('${root.path}/AGENTS.md').readAsStringSync(), '# index\n');
      expect(
        File('${root.path}/docs/guide.md').readAsStringSync(),
        '# guide\n',
      );
      expect(Directory('${root.path}/work').existsSync(), isTrue);
      expect(Directory('${root.path}/scenes').existsSync(), isTrue);
      expect(workspace.path, root.path);

      final manifest = jsonDecode(
        File('${root.path}/$agentWorkspaceManifestName').readAsStringSync(),
      );
      expect(manifest['version'], 1);
    },
  );

  test(
    'a same-version install keeps edits and restores a deleted file',
    () async {
      final root = await Directory.systemTemp.createTemp('or-agent-workspace-');
      addTearDown(() => root.delete(recursive: true));
      final files = {'AGENTS.md': '# index\n', 'docs/guide.md': '# guide\n'};
      await AgentWorkspace.install(directory: root, bundle: bundle(1, files));
      File('${root.path}/AGENTS.md').writeAsStringSync('# edited by agent\n');
      File('${root.path}/docs/guide.md').deleteSync();

      await AgentWorkspace.install(directory: root, bundle: bundle(1, files));

      expect(
        File('${root.path}/AGENTS.md').readAsStringSync(),
        '# edited by agent\n',
      );
      expect(File('${root.path}/docs/guide.md').existsSync(), isTrue);
    },
  );

  test(
    'a new bundle version refreshes framework files but not agent work',
    () async {
      final root = await Directory.systemTemp.createTemp('or-agent-workspace-');
      addTearDown(() => root.delete(recursive: true));
      await AgentWorkspace.install(
        directory: root,
        bundle: bundle(1, {'AGENTS.md': '# old\n'}),
      );
      File('${root.path}/AGENTS.md').writeAsStringSync('# edited by agent\n');
      File('${root.path}/work/notes.md').writeAsStringSync('keep me\n');

      await AgentWorkspace.install(
        directory: root,
        bundle: bundle(2, {'AGENTS.md': '# new\n'}),
      );

      expect(File('${root.path}/AGENTS.md').readAsStringSync(), '# new\n');
      expect(
        File('${root.path}/work/notes.md').readAsStringSync(),
        'keep me\n',
      );
    },
  );

  test('runtime files bind the workspace to the app', () async {
    final root = await Directory.systemTemp.createTemp('or-agent-workspace-');
    addTearDown(() => root.delete(recursive: true));
    final workspace = await AgentWorkspace.install(
      directory: root,
      bundle: bundle(1, {'AGENTS.md': '# index\n'}),
    );

    await workspace.writeProjectMarker(
      projectName: 'Demo',
      projectPath: '/tmp/demo.orproj',
      descriptorPath: '/tmp/demo.or-session',
    );
    final marker = jsonDecode(
      File('${root.path}/project.json').readAsStringSync(),
    );
    expect(marker['project_name'], 'Demo');
    expect(marker['project_path'], '/tmp/demo.orproj');
    expect(marker['app_support_path'], root.parent.path);
    // The CLI resolves the app-level motion library from this marker, so the
    // project must carry the folder the app actually uses.
    expect(
      marker['motion_library_path'],
      '${root.parent.path}${Platform.pathSeparator}motion-library',
    );

    final cli = '/tmp/or build/or';
    await workspace.writeOrCli(cli);
    final wrapper = File('${root.path}/tools/or').readAsStringSync();
    expect(wrapper, startsWith('#!/bin/sh\n'));
    expect(wrapper, contains(cli));
    expect(wrapper, contains(r'"$@"'));
  });

  test('a CLI path with spaces and quotes is shell-quoted', () async {
    final root = await Directory.systemTemp.createTemp('or-agent-workspace-');
    addTearDown(() => root.delete(recursive: true));
    final workspace = await AgentWorkspace.install(
      directory: root,
      bundle: bundle(1, {'AGENTS.md': '# index\n'}),
    );

    await workspace.writeOrCli("/tmp/vibecode projects/it's or");
    final wrapper = File('${root.path}/tools/or').readAsStringSync();
    expect(wrapper, contains(r"'/tmp/vibecode projects/it'\''s or'"));
  });

  test('the bundled asset carries the index, guides and skills', () async {
    TestWidgetsFlutterBinding.ensureInitialized();
    final bundle = await loadAgentWorkspaceBundle();
    expect(bundle.version, greaterThanOrEqualTo(1));
    expect(bundle.files['AGENTS.md'], contains('Routing'));
    expect(bundle.files['docs/timeline-editing.md'], isNotNull);
    expect(
      bundle.files['.agents/skills/or-timeline-edit/SKILL.md'],
      contains('name: or-timeline-edit'),
    );
    expect(bundle.files['tools/or'], isNull);
  });
}
