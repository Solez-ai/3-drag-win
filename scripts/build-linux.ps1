# ═══════════════════════════════════════════════════════════════════════
# build-linux.ps1 — Cross-compile 3-win-drag for Linux from Windows
#
# Prerequisites:
#   1. Docker Desktop (running)
#   2. Rust toolchain
#   3. cargo install cross
#
# Usage:
#   .\scripts\build-linux.ps1                          # x86_64 glibc
#   .\scripts\build-linux.ps1 -Target "aarch64-unknown-linux-gnu"  # ARM64
# ═══════════════════════════════════════════════════════════════════════

param(
    [string]$Target = "x86_64-unknown-linux-gnu",
    [switch]$Musl
)

if ($Musl) {
    $Target = "x86_64-unknown-linux-musl"
}

$ProjectDir = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$BinName = "3-win-drag"
$PkgName = "${BinName}-${Target}"

Write-Host "═══ Building ${BinName} for ${Target} ═══" -ForegroundColor Cyan

# Check for cross
$crossInstalled = Get-Command "cross" -ErrorAction SilentlyContinue
if (-not $crossInstalled) {
    Write-Host "Installing cross ..." -ForegroundColor Yellow
    cargo install cross
}

# Build
Set-Location $ProjectDir
cross build --target $Target --release

if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed!" -ForegroundColor Red
    exit 1
}

# Package
Write-Host "═══ Packaging ${PkgName} ═══" -ForegroundColor Cyan

$PkgDir = Join-Path "target" $PkgName
if (Test-Path $PkgDir) {
    Remove-Item -Recurse -Force $PkgDir
}
New-Item -ItemType Directory -Path $PkgDir | Out-Null

Copy-Item (Join-Path "target" $Target "release" $BinName) $PkgDir
Copy-Item "README.md" $PkgDir
Copy-Item "LICENSE" $PkgDir

# Create tar.gz archive
$arch = "${PkgName}.tar.gz"
Set-Location target
tar czf $arch $PkgName
Write-Host "═══ Done → target/$arch ═══" -ForegroundColor Green
