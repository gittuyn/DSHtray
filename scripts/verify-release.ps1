$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$exe = Join-Path $root "src-tauri\target\release\dshtray.exe"
$installer = Join-Path $root "src-tauri\target\release\bundle\nsis\DSHtray_0.1.0_x64-setup.exe"
foreach ($path in @($exe, $installer)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing artifact: $path" }
    $file = Get-Item -LiteralPath $path
    if ($file.Length -lt 102400) { throw "Artifact is unexpectedly small: $path" }
    $hash = Get-FileHash -LiteralPath $path -Algorithm SHA256
    Write-Host ("{0}  {1}  {2} bytes" -f $hash.Hash.ToLowerInvariant(), $path, $file.Length)
}
$signature = Get-AuthenticodeSignature -LiteralPath $exe
Write-Host ("Executable signature status: {0}" -f $signature.Status)
if ($signature.Status -notin @("Valid", "NotSigned")) { throw "Unexpected executable signature status: $($signature.Status)" }
Write-Host "Release artifact verification passed."
