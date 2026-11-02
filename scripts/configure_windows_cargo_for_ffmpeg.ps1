# Pins the Windows Rust builds to the packaged FFmpeg prefix.
#
# Flutter's native-assets hook builds the bridge with its own environment, so an
# exported FFMPEG_DIR does not always reach it. A cargo config in the checkout
# does, and that is the same file the FFmpeg build script writes on CI.
[CmdletBinding()]
param(
  [string]$Prefix = $env:FFMPEG_INSTALL_PREFIX,
  [string]$RepoRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = 'Stop'

if ([string]::IsNullOrWhiteSpace($Prefix)) {
  throw 'usage: configure_windows_cargo_for_ffmpeg.ps1 -Prefix <ffmpeg-install-prefix>'
}
if (-not (Test-Path (Join-Path $Prefix 'lib'))) {
  throw "the FFmpeg prefix has no lib directory: $Prefix\lib"
}
$Prefix = (Resolve-Path $Prefix).Path

# TOML literal strings keep Windows backslashes intact; a basic string would
# treat \t or \f as escapes and cargo would refuse the whole file.
$lines = @(
  '[env]',
  "FFMPEG_DIR = { value = '$Prefix', force = true }",
  "PKG_CONFIG_PATH = { value = '$Prefix\lib\pkgconfig', force = true }"
)
if ($env:LIBCLANG_PATH -and (Test-Path $env:LIBCLANG_PATH)) {
  $libclang = (Resolve-Path $env:LIBCLANG_PATH).Path
  $lines += "LIBCLANG_PATH = { value = '$libclang', force = true }"
}

$cargoDir = Join-Path $RepoRoot '.cargo'
New-Item -ItemType Directory -Force -Path $cargoDir | Out-Null
Set-Content -Path (Join-Path $cargoDir 'config.toml') -Value ($lines -join "`n") -Encoding ascii

Write-Output "CARGO_FFMPEG_PREFIX=$Prefix"
