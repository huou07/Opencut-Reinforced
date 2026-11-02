# The Windows desktop lane, shared by a real Windows host and the hosted job.
#
# Every step here is the same command the release gate runs, so a green local
# run and a green CI run mean the same thing. Build outputs, staging and the
# packaged ZIP are produced by the repository scripts, not by this driver.
[CmdletBinding()]
param(
  [ValidateSet('debug', 'release')][string]$Mode = 'debug',
  [string[]]$Steps = @('tests', 'flutter-static', 'probe', 'build', 'stage', 'journey', 'zip'),
  [string]$RepoRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$Archive = ''
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'windows_lane_common.ps1')

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
  $env:OR_PACKAGED_HELPERS_DIRECTORY = $bundle
  if (-not $env:FFMPEG_DIR -and $env:FFMPEG_INSTALL_PREFIX) {
    $env:FFMPEG_DIR = $env:FFMPEG_INSTALL_PREFIX
  }
  Invoke-Step 'packaged product journey' { python (Join-Path $RepoRoot 'scripts/run_packaged_desktop_journey.py') }
}

if ($Steps -contains 'zip') {
  Invoke-Step 'desktop ZIP install, replacement and removal' {
    python (Join-Path $RepoRoot 'scripts/test_desktop_archive_install.py') $bundle --archive $Archive
  }
}

Write-Output 'WINDOWS_LANE=PASS'
