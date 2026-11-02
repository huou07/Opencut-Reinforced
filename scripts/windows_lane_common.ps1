# Shared helpers for the Windows desktop lane.
#
# Kept deliberately small: the lane driver and the staging script are the only
# consumers, and each helper exists to make the same command work on a real host
# and on the hosted runner.

function Get-RepoBundle([string]$RepoRoot, [ValidateSet('debug', 'release')][string]$Mode) {
  $configuration = if ($Mode -eq 'release') { 'Release' } else { 'Debug' }
  return Join-Path $RepoRoot "apps/or_app/build/windows/x64/runner/$configuration"
}
