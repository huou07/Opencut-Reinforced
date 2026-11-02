# Build the pinned static zlib the Windows FFmpeg helper executables link.
#
# Hosted CI and a real Windows host run this same script so the helper binaries
# are the same in both places. Parameters:
#   -BuildRoot  scratch directory (CI passes $env:RUNNER_TEMP)
#   -Prefix     install prefix (defaults to <BuildRoot>/zlib-1.3.2-install)
# In CI it appends the environment the later steps need to $env:GITHUB_ENV;
# locally it also writes <BuildRoot>/zlib-build-env.txt for the caller to import.
param(
  [Parameter(Mandatory = $true)][string]$BuildRoot,
  [string]$Prefix = ''
)

$ErrorActionPreference = 'Stop'
if (-not $Prefix) { $Prefix = Join-Path $BuildRoot 'zlib-1.3.2-install' }
$archive = Join-Path $BuildRoot 'zlib-1.3.2.tar.gz'
$source = Join-Path $BuildRoot 'zlib-1.3.2'
$build = Join-Path $BuildRoot 'zlib-1.3.2-build'
$envFile = Join-Path $BuildRoot 'zlib-build-env.txt'
Set-Content -Path $envFile -Value '' -Encoding utf8

function Record-Env([string]$line) {
  Add-Content -Path $envFile -Value $line -Encoding utf8
  if ($env:GITHUB_ENV) { Add-Content -Path $env:GITHUB_ENV -Value $line -Encoding utf8 }
}

if (-not (Test-Path $archive)) {
  Invoke-WebRequest -Uri 'https://github.com/madler/zlib/releases/download/v1.3.2/zlib-1.3.2.tar.gz' -OutFile $archive
}
$actualHash = (Get-FileHash -Algorithm SHA256 $archive).Hash.ToLowerInvariant()
if ($actualHash -ne 'bb329a0a2cd0274d05519d61c667c062e06990d72e125ee2dfa8de64f0119d16') {
  throw "zlib source hash mismatch: $actualHash"
}
if (-not (Test-Path $source)) { tar -xzf $archive -C $BuildRoot }
cmake -S $source -B $build -A x64 `
  "-DCMAKE_INSTALL_PREFIX=$Prefix" `
  -DZLIB_BUILD_SHARED=OFF -DZLIB_BUILD_STATIC=ON -DZLIB_BUILD_TESTING=OFF
if ($LASTEXITCODE -ne 0) { throw "zlib configure failed: $LASTEXITCODE" }
cmake --build $build --config Release
if ($LASTEXITCODE -ne 0) { throw "zlib build failed: $LASTEXITCODE" }
cmake --install $build --config Release
if ($LASTEXITCODE -ne 0) { throw "zlib install failed: $LASTEXITCODE" }
$staticLibrary = Join-Path $Prefix 'lib/zs.lib'
if (-not (Test-Path $staticLibrary -PathType Leaf)) { throw 'static MSVC zlib library was not installed' }
Copy-Item $staticLibrary (Join-Path $Prefix 'lib/zlib.lib') -Force
Copy-Item (Join-Path $source 'LICENSE') (Join-Path $BuildRoot 'ZLIB-LICENSE.txt') -Force
Record-Env "OR_ZLIB_PREFIX=$Prefix"
Record-Env "INCLUDE=$Prefix/include;$env:INCLUDE"
Record-Env "LIB=$Prefix/lib;$env:LIB"
Write-Output "zlib installed at $Prefix"
