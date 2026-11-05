# Stage the packaged Windows runtime beside the built application.
#
# The hosted Windows job and a real Windows host both run this script, so a
# packaged build is assembled exactly one way: the Rust bridge DLL, the FFmpeg
# shared libraries, ffmpeg.exe/ffprobe.exe and the production media probe all
# land beside the app, and each is proven to load from those files alone.
[CmdletBinding()]
param(
  [string]$Bundle = '',
  [ValidateSet('debug', 'release')][string]$Mode = 'debug',
  [string]$RepoRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$FfmpegPrefix = $env:FFMPEG_INSTALL_PREFIX
)

$ErrorActionPreference = 'Stop'

if ([string]::IsNullOrWhiteSpace($Bundle)) {
  $configuration = if ($Mode -eq 'release') { 'Release' } else { 'Debug' }
  $Bundle = Join-Path $RepoRoot "apps/or_app/build/windows/x64/runner/$configuration"
}
if (-not (Test-Path $Bundle -PathType Container)) {
  throw "Windows app bundle directory not found: $Bundle"
}
if ([string]::IsNullOrWhiteSpace($FfmpegPrefix)) {
  throw 'FFMPEG_INSTALL_PREFIX is not set; run scripts/build_windows_ffmpeg.sh first'
}

$bridge = @(Get-ChildItem -Path (Join-Path $RepoRoot 'apps/or_app/build/windows') -Filter '*or_app_bridge*.dll' -Recurse) | Select-Object -First 1
if ($null -eq $bridge) { throw 'Packaged or_app_bridge DLL not found' }
$runtimeDir = $bridge.Directory.FullName
if ($env:GITHUB_ENV) {
  Add-Content -Path $env:GITHUB_ENV -Value "OR_PACKAGED_BRIDGE_DIRECTORY=$runtimeDir"
} else {
  $env:OR_PACKAGED_BRIDGE_DIRECTORY = $runtimeDir
}

# A caller may redirect Cargo's output (the local lane reuses one target
# directory), so look in the redirected release folder before the default one.
$probeCandidates = @(
  (Join-Path $RepoRoot 'tools/ffmpeg-link-probe/target/release/ffmpeg-link-probe.exe')
)
if (-not [string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
  $probeCandidates += (Join-Path $env:CARGO_TARGET_DIR 'release/ffmpeg-link-probe.exe')
}
$probeSource = $probeCandidates | Where-Object { Test-Path $_ -PathType Leaf } | Select-Object -First 1
if ($null -eq $probeSource) { throw 'Production or_media FFmpeg probe was not built' }
Copy-Item -Path $probeSource -Destination (Join-Path $runtimeDir 'ffmpeg-link-probe.exe') -Force

$destinations = @($runtimeDir)
if ($Bundle -ne $runtimeDir) { $destinations += $Bundle }
foreach ($pattern in @('avcodec*.dll', 'avformat*.dll', 'avutil*.dll', 'swresample*.dll', 'swscale*.dll', 'avfilter*.dll')) {
  $libraries = Get-ChildItem -Path (Join-Path $FfmpegPrefix 'bin') -Filter $pattern -File
  if ($libraries.Count -eq 0) { throw "FFmpeg runtime library missing: $pattern" }
  foreach ($destination in $destinations) {
    Copy-Item -Path $libraries.FullName -Destination $destination -Force
  }
}

# 9B1 packaged media helpers live beside the app binary; zlib is statically
# linked into these helper executables.
foreach ($helper in @('ffmpeg.exe', 'ffprobe.exe')) {
  $source = Join-Path (Join-Path $FfmpegPrefix 'bin') $helper
  if (-not (Test-Path $source -PathType Leaf)) { throw "FFmpeg helper missing: $helper" }
  Copy-Item -Path $source -Destination $Bundle -Force
}

# Speech runtime and the `or` CLI come from the shared stager, so the Windows
# package carries the same helpers as macOS and Linux.
$stager = Join-Path $RepoRoot 'scripts/stage_desktop_runtime.py'
# An App Execution Alias named python3.exe can sit first on PATH and only
# opens the Store. Pick the first candidate that actually starts Python: the
# stager is mandatory, so a stub must not be mistaken for an interpreter.
$python = $null
$pythonArgs = @()
if ($env:OR_PYTHON -and (Test-Path $env:OR_PYTHON -PathType Leaf)) {
  # An explicit interpreter always wins: a machine whose PATH is dominated by
  # Store aliases can name the real one without editing the machine.
  $python = [pscustomobject]@{ Source = (Resolve-Path $env:OR_PYTHON).Path }
}
# A Store alias named python3.exe or python.exe can sit first on PATH and only
# opens the Store. It reports failure through a native stderr record, and this
# script runs with ErrorActionPreference = Stop, so ask each candidate with the
# preference relaxed and trust the exit code. The py launcher is tried last
# because its real interpreter is the one that actually ships Python.
foreach ($candidate in @(
    @{ Name = 'python3'; Args = @() },
    @{ Name = 'python'; Args = @() },
    @{ Name = 'py'; Args = @('-3') })) {
  $command = Get-Command $candidate.Name -ErrorAction SilentlyContinue
  if ($null -eq $command -or $null -ne $python) { continue }
  $previous = $ErrorActionPreference
  $ErrorActionPreference = 'Continue'
  try {
    $probe = & $command.Source @($candidate.Args) -c "import sys" 2>&1
    $ok = ($LASTEXITCODE -eq 0) -and ($probe -notmatch 'App execution alias')
  }
  catch {
    $ok = $false
  }
  finally {
    $ErrorActionPreference = $previous
  }
  if ($ok) {
    $python = $command
    $pythonArgs = @($candidate.Args)
    break
  }
}
if ($null -eq $python) { throw 'Python 3 is required to stage the runtime; set OR_PYTHON to its executable' }
$arch = if ($env:PROCESSOR_ARCHITECTURE -match 'ARM64') { 'arm64' } else { 'amd64' }
$required = ''
$download = @()
if ($env:OR_REQUIRE_RUNTIME -eq '1') {
  $required = 'whisper,provider,or'
  $download = @('--download-whisper')
}
# An empty --require loses its value when PowerShell unrolls the array, so the
# arguments are built explicitly instead of passed positionally.
$stagerArgs = @(
  '--os', 'windows',
  '--arch', $arch,
  '--runtime-dir', $runtimeDir,
  '--repo-root', $RepoRoot
)
# The stager defaults --require to "whisper,provider", so an unchecked local
# package has to say "nothing is required" explicitly; a bare --require with an
# empty value is dropped by PowerShell before the process starts.
$stagerArgs += @("--require=$required")
$stagerArgs += $download
& $python.Source @pythonArgs $stager @stagerArgs
if ($LASTEXITCODE -ne 0) { throw 'the shared runtime stager failed' }

# A staged engine that cannot start is a broken package, so prove it runs on
# the libraries that travel with it. Verification builds without the optional
# runtime skip this because there is nothing to run.
$stagedWhisper = Join-Path $runtimeDir 'whisper-cli.exe'
if (Test-Path $stagedWhisper -PathType Leaf) {
  & $stagedWhisper --help | Out-Null
  if ($LASTEXITCODE -ne 0) { throw "the staged speech engine failed to start (exit $LASTEXITCODE)" }
}

$dumpbin = (Get-Command dumpbin.exe -ErrorAction Stop).Source
& $dumpbin /dependents (Join-Path $runtimeDir 'ffmpeg-link-probe.exe')
$helperDependencies = & $dumpbin /dependents (Join-Path $Bundle 'ffmpeg.exe') | Out-String
if ($helperDependencies -match '(?i)zlib.*\.dll') { throw 'ffmpeg.exe unexpectedly depends on a zlib DLL' }

$originalPath = $env:PATH
try {
  $env:PATH = "$runtimeDir;$Bundle;$env:SystemRoot\System32;$env:SystemRoot"
  $bridgeHandle = [System.Runtime.InteropServices.NativeLibrary]::Load($bridge.FullName)
  [System.Runtime.InteropServices.NativeLibrary]::Free($bridgeHandle)
  & (Join-Path $runtimeDir 'ffmpeg-link-probe.exe')
  if ($LASTEXITCODE -ne 0) { throw 'the production media probe failed against the staged runtime' }

  # The staged helpers must run on bundled libraries alone.
  $env:PATH = "$Bundle;$env:SystemRoot\System32;$env:SystemRoot"
  & (Join-Path $Bundle 'ffprobe.exe') -hide_banner -version
  if ($LASTEXITCODE -ne 0) { throw 'staged ffprobe did not run on bundled libraries' }
  $encoders = & (Join-Path $Bundle 'ffmpeg.exe') -hide_banner -encoders
  if ($LASTEXITCODE -ne 0) { throw 'staged ffmpeg did not run on bundled libraries' }
  if (-not ($encoders | Select-String ' png ' -SimpleMatch)) { throw 'staged ffmpeg lacks the png encoder' }
  if (-not ($encoders | Select-String ' mpeg4 ' -SimpleMatch)) { throw 'staged ffmpeg lacks the mpeg4 encoder' }
}
finally {
  $env:PATH = $originalPath
}

Write-Output "WINDOWS_BUNDLE=$Bundle"
Write-Output "OR_PACKAGED_BRIDGE_DIRECTORY=$runtimeDir"
Write-Output 'STEP_DONE=stage'
