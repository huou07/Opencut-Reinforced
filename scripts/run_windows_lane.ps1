# The Windows desktop lane, shared by a real Windows host and the hosted job.
#
# Every step here is the same command the release gate runs, so a green local
# run and a green CI run mean the same thing. Build outputs, staging and the
# packaged ZIP are produced by the repository scripts, not by this driver.
[CmdletBinding()]
param(
  [ValidateSet('debug', 'release')][string]$Mode = 'debug',
  [string[]]$Steps = @('tests', 'flutter-static', 'probe', 'build', 'stage', 'journey', 'zip', 'installer'),
  [string]$RepoRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$Archive = ''
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'windows_lane_common.ps1')

# A single token such as `-Steps stage,zip` arrives as one string when the
# script is started through `pwsh -File`, which silently ran no steps at all
# and still reported PASS. Split it, and refuse a name that is not a step.
$Steps = @($Steps | ForEach-Object { $_ -split ',' } | ForEach-Object { $_.Trim() } | Where-Object { $_ })
$known = @('tests', 'flutter-static', 'probe', 'build', 'stage', 'journey', 'zip', 'installer')
$unknown = @($Steps | Where-Object { $known -notcontains $_ })
if ($Steps.Count -eq 0 -or $unknown.Count -gt 0) {
  throw "unknown lane step(s): $($unknown -join ', '); expected some of $($known -join ', ')"
}

# The hosted job puts the pinned FFmpeg bin directory on PATH, so media-probe
# tests find ffprobe.exe the same way they do in CI.
if ($env:FFMPEG_INSTALL_PREFIX) {
  $ffmpegBin = Join-Path $env:FFMPEG_INSTALL_PREFIX 'bin'
  if (Test-Path $ffmpegBin) { $env:PATH = "$ffmpegBin;$env:PATH" }
}

# ffmpeg-sys prefers FFMPEG_DIR and only falls back to pkg-config, which the
# hosted job does not install on Windows. The hook does not always inherit an
# exported value, so pin it where cargo always reads it, exactly as CI does.
if (-not $env:FFMPEG_DIR -and $env:FFMPEG_INSTALL_PREFIX) {
  $env:FFMPEG_DIR = $env:FFMPEG_INSTALL_PREFIX
}
if ($env:FFMPEG_INSTALL_PREFIX) {
  & (Join-Path $PSScriptRoot 'configure_windows_cargo_for_ffmpeg.ps1') -Prefix $env:FFMPEG_INSTALL_PREFIX -RepoRoot $RepoRoot | Out-Null
}

# bindgen opens libclang from LIBCLANG_PATH, and libclang in turn loads the other
# clang64 DLLs from its own directory. The hosted job gets that directory on PATH
# from the msys2 action, so the real host has to add it explicitly or every Rust
# build that runs bindgen fails with "LoadLibraryExW failed".
if ($env:LIBCLANG_PATH -and (Test-Path $env:LIBCLANG_PATH)) {
  $env:PATH = "$($env:LIBCLANG_PATH);$env:PATH"
}

Set-Location $RepoRoot

if ([string]::IsNullOrWhiteSpace($Archive)) {
  $Archive = Join-Path $RepoRoot "apps/or_app/build/windows/or-windows-x64-$Mode.zip"
}

function Invoke-Step([string]$Name, [scriptblock]$Body) {
  Write-Output "=== STEP $Name $(Get-Date -Format o)"
  & $Body
  if ($LASTEXITCODE -ne 0) { throw "step failed: $Name (exit $LASTEXITCODE)" }
  Write-Output "=== OK $Name"
}

if ($Steps -contains 'tests') {
  $cargoTests = @(
    @{ Name = 'or_core project storage'; Args = @('test', '-p', 'or_core', '--test', 'project_storage') },
    @{ Name = 'or_core project recovery'; Args = @('test', '-p', 'or_core', '--test', 'project_recovery') },
    @{ Name = 'or_core persistent cache'; Args = @('test', '-p', 'or_core', 'cache::tests') },
    @{ Name = 'or_core media artifacts'; Args = @('test', '-p', 'or_core', 'media_artifacts::tests') },
    @{ Name = 'or_ipc local transport'; Args = @('test', '-p', 'or_ipc', '--test', 'local_transport') },
    @{ Name = 'or_ipc live host'; Args = @('test', '-p', 'or_ipc', '--test', 'live_host') },
    @{ Name = 'or_cli semantic cli'; Args = @('test', '-p', 'or_cli', '--test', 'semantic_cli') },
    @{ Name = 'or_ipc endpoint access control'; Args = @('test', '-p', 'or_ipc', '--lib') }
  )
  foreach ($test in $cargoTests) {
    Invoke-Step "cargo $($test.Name)" { cargo @($test.Args) }
  }
}

if ($Steps -contains 'flutter-static') {
  Push-Location (Join-Path $RepoRoot 'apps/or_app')
  try {
    Invoke-Step 'flutter pub get' { flutter pub get }
    Invoke-Step 'flutter analyze' { flutter analyze }
    Invoke-Step 'flutter test' { flutter test }
  }
  finally {
    Pop-Location
  }
}

$configuration = if ($Mode -eq 'release') { 'Release' } else { 'Debug' }
$bundle = Join-Path $RepoRoot "apps/or_app/build/windows/x64/runner/$configuration"

if ($Steps -contains 'build') {
  Push-Location (Join-Path $RepoRoot 'apps/or_app')
  try {
    Invoke-Step "flutter build windows $Mode" { flutter build windows "--$Mode" }
  }
  finally {
    Pop-Location
  }
}

if ($Steps -contains 'probe') {
  Invoke-Step 'production media probe' {
    cargo build --locked --release --manifest-path (Join-Path $RepoRoot 'tools/ffmpeg-link-probe/Cargo.toml')
  }
}

if ($Steps -contains 'stage') {
  Invoke-Step 'stage packaged runtime' {
    & (Join-Path $PSScriptRoot 'stage_windows_bundle.ps1') -Bundle $bundle -Mode $Mode -RepoRoot $RepoRoot
  }
}

if ($Steps -contains 'journey') {
  # `flutter test -d windows` runs the Debug build, so the journey has to use the
  # Debug bundle even when the lane packages a release build: the app resolves
  # its FFmpeg helpers beside its own executable, and a helper staged only into
  # Release leaves the thumbnail job with nothing to spawn.
  $journeyBundle = Join-Path $RepoRoot 'apps/or_app/build/windows/x64/runner/Debug'
  Push-Location (Join-Path $RepoRoot 'apps/or_app')
  try {
    Invoke-Step 'flutter build windows debug' { flutter build windows --debug }
  }
  finally {
    Pop-Location
  }
  Invoke-Step 'stage journey runtime' {
    & (Join-Path $PSScriptRoot 'stage_windows_bundle.ps1') -Bundle $journeyBundle -Mode debug -RepoRoot $RepoRoot
  }
  $env:OR_PACKAGED_HELPERS_DIRECTORY = $journeyBundle
  $env:OR_PACKAGED_BRIDGE_DIRECTORY = $journeyBundle
  if (-not $env:FFMPEG_DIR -and $env:FFMPEG_INSTALL_PREFIX) {
    $env:FFMPEG_DIR = $env:FFMPEG_INSTALL_PREFIX
  }
  Invoke-Step 'packaged product journey' { python (Join-Path $RepoRoot 'scripts/run_packaged_desktop_journey.py') }
}

if ($Steps -contains 'installer') {
  $versionLine = Get-Content (Join-Path $RepoRoot 'apps/or_app/pubspec.yaml') |
    Where-Object { $_ -match '^version:' } | Select-Object -First 1
  $version = (($versionLine -split '\s+')[1] -split '\+')[0]
  $candidates = @()
  $command = Get-Command iscc.exe -ErrorAction SilentlyContinue
  if ($command) { $candidates += $command.Source }
  foreach ($root in @(${env:ProgramFiles(x86)}, $env:ProgramFiles)) {
    if ($root) { $candidates += (Join-Path $root 'Inno Setup 6\ISCC.exe') }
  }
  $iscc = $candidates | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1
  if (-not $iscc) {
    throw "Inno Setup 6 is not installed; install it with 'choco install innosetup -y' or set ISCC."
  }
  $installerOut = Join-Path $RepoRoot 'apps/or_app/build/windows'
  New-Item -ItemType Directory -Force -Path $installerOut | Out-Null
  $setup = Join-Path $installerOut "Opencut-Reinforced-$version-windows-x64-setup.exe"
  & $iscc "/DAppVersion=$version" "/DSourceDir=$Bundle" "/DOutputDir=$installerOut" "/DOutputBaseFilename=$([System.IO.Path]::GetFileNameWithoutExtension($setup))" (Join-Path $RepoRoot 'scripts/windows_installer.iss')
  if ($LASTEXITCODE -ne 0) { throw "Inno Setup failed with exit code $LASTEXITCODE" }
  if (-not (Test-Path $setup)) { throw "Inno Setup did not produce $setup" }
  Write-Output "WINDOWS_INSTALLER=$setup"
  Write-Output 'STEP_DONE=installer'
}

if ($Steps -contains 'zip') {
  Invoke-Step 'desktop ZIP install, replacement and removal' {
    python (Join-Path $RepoRoot 'scripts/test_desktop_archive_install.py') $bundle --archive $Archive
  }
}

Write-Output 'WINDOWS_LANE=PASS'
