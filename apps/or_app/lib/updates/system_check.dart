import 'dart:io';

import '../support/host_platform.dart';

/// What this machine is, measured once, in the words the UI shows.
///
/// The numbers come from the operating system rather than from a build-time
/// assumption, because "8 GB and 40 GB free" is the difference between an
/// export that runs and one that fails at 90 %. Every field is nullable on
/// purpose: a host that refuses a command reports "unknown" instead of a guess.
class HostFacts {
  const HostFacts({
    required this.platformName,
    required this.osVersion,
    required this.cpuCount,
    this.memoryBytes,
    this.freeDiskBytes,
  });

  final String platformName;
  final String osVersion;
  final int cpuCount;
  final int? memoryBytes;
  final int? freeDiskBytes;

  /// A 1080p H.264 export of a short project, with the staging copy beside it.
  static const exportHeadroomBytes = 4 * 1024 * 1024 * 1024;

  bool get hasExportHeadroom =>
      freeDiskBytes == null || freeDiskBytes! >= exportHeadroomBytes;

  String get summary {
    final memory = memoryBytes == null ? null : formatBytes(memoryBytes!);
    final free = freeDiskBytes == null ? null : formatBytes(freeDiskBytes!);
    return [
      '$platformName $osVersion',
      '$cpuCount ${cpuCount == 1 ? 'core' : 'cores'}',
      if (memory != null) '$memory RAM',
      if (free != null) '$free free',
    ].join(' · ');
  }
}

/// Bytes in the units a person compares against a file size.
String formatBytes(int bytes) {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  var value = bytes.toDouble();
  var unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  final rounded = value >= 10 || unit == 0
      ? value.round().toString()
      : value.toStringAsFixed(1);
  return '$rounded ${units[unit]}';
}

/// Runs a command and returns its stdout, or null when the host refuses.
typedef CommandRunner = Future<String?> Function(
  String executable,
  List<String> arguments,
);

Future<String?> _run(String executable, List<String> arguments) async {
  try {
    final result = await Process.run(executable, arguments);
    if (result.exitCode != 0) return null;
    final out = result.stdout;
    return out is String ? out : null;
  } catch (_) {
    return null;
  }
}

/// Reads what this machine is, without failing the caller.
///
/// `volume` is the directory the app writes exports and caches into, so the
/// free-space row describes the disk that will actually hold the file.
Future<HostFacts> readHostFacts({
  required Directory volume,
  HostPlatform? platform,
  CommandRunner run = _run,
}) async {
  final host = platform ?? HostPlatform.current();
  // The two probes are independent processes (on Windows each is a PowerShell
  // launch), so they run together: sequentially the panel waited for two process
  // startups before it could name the machine at all. Neither probe throws — a
  // launch that fails answers null — so no timer is needed to bound the wait.
  final probes = await Future.wait([
    _memoryBytes(host, run),
    _freeDiskBytes(host, volume.path, run),
  ]);
  return HostFacts(
    platformName: host.name,
    osVersion: Platform.operatingSystemVersion,
    cpuCount: Platform.numberOfProcessors,
    memoryBytes: probes[0],
    freeDiskBytes: probes[1],
  );
}

Future<int?> _memoryBytes(HostPlatform host, CommandRunner run) async {
  if (host.isLinux) {
    try {
      final info = await File('/proc/meminfo').readAsString();
      final match = RegExp(
        r'^MemTotal:\s+(\d+) kB',
        multiLine: true,
      ).firstMatch(info);
      final kb = match == null ? null : int.tryParse(match.group(1)!);
      return kb == null ? null : kb * 1024;
    } catch (_) {
      return null;
    }
  }
  if (host.isMacOS) {
    final out = await run('sysctl', ['-n', 'hw.memsize']);
    return out == null ? null : int.tryParse(out.trim());
  }
  if (host.isWindows) {
    final out = await run('powershell', [
      '-NoProfile',
      '-Command',
      '(Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory',
    ]);
    return out == null ? null : int.tryParse(out.trim());
  }
  return null;
}

Future<int?> _freeDiskBytes(
  HostPlatform host,
  String path,
  CommandRunner run,
) async {
  if (host.isWindows) {
    final drive = path.length >= 2 && path[1] == ':'
        ? path.substring(0, 2)
        : null;
    if (drive == null) return null;
    final out = await run('powershell', [
      '-NoProfile',
      '-Command',
      '(Get-PSDrive -Name ${drive[0]}).Free',
    ]);
    return out == null ? null : int.tryParse(out.trim());
  }
  // `df -k` reports 1024-byte blocks on both Darwin and GNU coreutils, so the
  // parse is one multiplication and no unit table.
  final out = await run('df', ['-k', path]);
  if (out == null) return null;
  final lines = out.trim().split('\n');
  if (lines.length < 2) return null;
  final fields = lines.last.trim().split(RegExp(r'\s+'));
  if (fields.length < 4) return null;
  final available = int.tryParse(fields[3]);
  return available == null ? null : available * 1024;
}
