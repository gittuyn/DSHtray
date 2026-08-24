$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Push-Location $root
try {
    pnpm tauri build --no-bundle --ci
    $portable = Join-Path $root "artifacts\portable\DSHtray"
    New-Item -ItemType Directory -Force -Path $portable | Out-Null
    Copy-Item "src-tauri\target\release\dshtray.exe" (Join-Path $portable "DSHtray.exe") -Force
    Copy-Item "README.md" (Join-Path $portable "README.md") -Force
    $hash = Get-FileHash (Join-Path $portable "DSHtray.exe") -Algorithm SHA256
    "{0}  {1}" -f $hash.Hash.ToLowerInvariant(), $hash.Path | Set-Content (Join-Path $portable "SHA256SUMS.txt") -Encoding utf8
    Write-Host "Portable package: $portable"
} finally {
    Pop-Location
}
