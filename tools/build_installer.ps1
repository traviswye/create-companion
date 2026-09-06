<#
.SYNOPSIS
  Build the Windows installer: engine + configuration UI in one NSIS setup.

.DESCRIPTION
  1. cargo build --release -p create-companion          (the engine)
  2. copy it to ui/src-tauri/binaries/ with the target-triple suffix Tauri
     expects for a sidecar (create-companion-x86_64-pc-windows-msvc.exe)
  3. npm run tauri build                                  (frontend, UI exe, NSIS)
  Output: target\release\bundle\nsis\Create Companion_<version>_x64-setup.exe

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File D:\CreateCompanion\tools\build_installer.ps1
#>
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $root
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path" }

$triple = (rustc -vV | Select-String '^host: (.*)$').Matches[0].Groups[1].Value
Write-Host "target triple: $triple"

Write-Host "== engine ==" -ForegroundColor Cyan
cargo build --release -p create-companion
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$bin = Join-Path $root "ui\src-tauri\binaries"
New-Item -ItemType Directory -Force $bin | Out-Null
Copy-Item (Join-Path $root "target\release\create-companion.exe") (Join-Path $bin "create-companion-$triple.exe") -Force

Write-Host "== installer ==" -ForegroundColor Cyan
Set-Location (Join-Path $root "ui")
if (-not (Test-Path node_modules)) { npm ci; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE } }
npm run tauri build
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Set-Location $root
Get-ChildItem "target\release\bundle\nsis\*.exe" | ForEach-Object {
  $h = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower()
  "$h  $($_.Name)" | Out-File "$($_.FullName).sha256" -Encoding ascii
  Write-Host ("{0}  {1:N1} MB" -f $_.Name, ($_.Length / 1MB)) -ForegroundColor Green
}
