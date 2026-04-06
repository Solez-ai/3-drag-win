Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $scriptRoot
$cargoTomlPath = Join-Path $projectRoot "Cargo.toml"
$readmePath = Join-Path $projectRoot "README.md"
$licensePath = Join-Path $projectRoot "LICENSE"
$logoPath = Join-Path $projectRoot "logo.png"
$issPath = Join-Path $projectRoot "installer\3-win-drag.iss"
$releaseExePath = Join-Path $projectRoot "target\x86_64-pc-windows-gnu\release\3-win-drag.exe"
$distRoot = Join-Path $projectRoot "dist"
$assetRoot = Join-Path $distRoot "installer-assets"
$installerRoot = Join-Path $distRoot "installer"
$guidePath = Join-Path $assetRoot "INSTALLER_README.txt"
$sideBitmapPath = Join-Path $assetRoot "wizard-side.bmp"
$topBitmapPath = Join-Path $assetRoot "wizard-top.bmp"
$installerIconPath = Join-Path $assetRoot "3-win-drag.ico"

function Get-AppVersion {
    $content = Get-Content $cargoTomlPath -Raw
    $match = [regex]::Match($content, '(?m)^version\s*=\s*"([^"]+)"')
    if (-not $match.Success) {
        throw "Failed to read the application version from Cargo.toml."
    }

    return $match.Groups[1].Value
}

function New-Directory {
    param([string]$Path)

    if (-not (Test-Path $Path)) {
        $null = New-Item -ItemType Directory -Path $Path -Force
    }
}

function Write-InstallerGuide {
    $intro = @"
3-win-drag Installer Guide

This was made by Samin Yeasar
This is an open source project on GitHub - https://github.com/Solez-ai/3-drag-win
MIT Licensed

Important:
Before continuing with the installation, open Windows Settings, go to Bluetooth & devices > Touchpad > Three-finger gestures, and disable the built-in Windows three-finger actions or set them to Nothing. This prevents conflicts with 3-win-drag.

3-win-drag installs with Drag And Drop as the default profile so browser tabs, files, images, downloads, and other native drag operations work immediately after installation.

README
======

"@

    $readme = Get-Content $readmePath -Raw
    $payload = $intro + $readme
    Set-Content -Path $guidePath -Value $payload -Encoding UTF8
}

function Get-PngDimension {
    param([byte[]]$Bytes, [int]$Offset)

    return (($Bytes[$Offset] -shl 24) -bor ($Bytes[$Offset + 1] -shl 16) -bor ($Bytes[$Offset + 2] -shl 8) -bor $Bytes[$Offset + 3])
}

function Write-IconFromPng {
    param(
        [string]$PngPath,
        [string]$IcoPath
    )

    $pngBytes = [System.IO.File]::ReadAllBytes($PngPath)
    if ($pngBytes.Length -lt 24) {
        throw "logo.png is not a valid PNG file."
    }

    $width = Get-PngDimension -Bytes $pngBytes -Offset 16
    $height = Get-PngDimension -Bytes $pngBytes -Offset 20
    $widthEntry = if ($width -ge 256) { 0 } else { [byte]$width }
    $heightEntry = if ($height -ge 256) { 0 } else { [byte]$height }

    $stream = [System.IO.File]::Open($IcoPath, [System.IO.FileMode]::Create, [System.IO.FileAccess]::Write)
    try {
        $writer = New-Object System.IO.BinaryWriter($stream)
        $writer.Write([UInt16]0)
        $writer.Write([UInt16]1)
        $writer.Write([UInt16]1)
        $writer.Write([byte]$widthEntry)
        $writer.Write([byte]$heightEntry)
        $writer.Write([byte]0)
        $writer.Write([byte]0)
        $writer.Write([UInt16]1)
        $writer.Write([UInt16]32)
        $writer.Write([UInt32]$pngBytes.Length)
        $writer.Write([UInt32]22)
        $writer.Write($pngBytes)
        $writer.Flush()
    }
    finally {
        $stream.Dispose()
    }
}

function New-WizardBitmap {
    param(
        [string]$OutputPath,
        [int]$Width,
        [int]$Height,
        [string]$Title,
        [string]$Subtitle,
        [int]$LogoSize
    )

    Add-Type -AssemblyName System.Drawing

    $bitmap = New-Object System.Drawing.Bitmap($Width, $Height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $logo = [System.Drawing.Image]::FromFile($logoPath)

    try {
        $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
        $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $graphics.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::ClearTypeGridFit

        $background = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
            ([System.Drawing.Rectangle]::new(0, 0, $Width, $Height)),
            ([System.Drawing.ColorTranslator]::FromHtml("#F4F0E8")),
            ([System.Drawing.ColorTranslator]::FromHtml("#E2ECE1")),
            45.0
        )
        $graphics.FillRectangle($background, 0, 0, $Width, $Height)
        $background.Dispose()

        $accentBrush = New-Object System.Drawing.SolidBrush([System.Drawing.ColorTranslator]::FromHtml("#1F5F45"))
        $mutedBrush = New-Object System.Drawing.SolidBrush([System.Drawing.ColorTranslator]::FromHtml("#4A4A44"))
        $linePen = New-Object System.Drawing.Pen([System.Drawing.ColorTranslator]::FromHtml("#D8CEC1"), 2)

        $logoX = [int](($Width - $LogoSize) / 2)
        $logoY = 24
        $graphics.DrawImage($logo, $logoX, $logoY, $LogoSize, $LogoSize)
        $graphics.DrawLine($linePen, 18, $logoY + $LogoSize + 18, $Width - 18, $logoY + $LogoSize + 18)

        $titleFont = New-Object System.Drawing.Font("Segoe UI Semibold", 16, [System.Drawing.FontStyle]::Bold)
        $bodyFont = New-Object System.Drawing.Font("Segoe UI", 9)
        $titleTop = $logoY + $LogoSize + 28
        $bodyTop = $logoY + $LogoSize + 82
        $contentWidth = $Width - 36
        $bodyHeight = $Height - $bodyTop - 14
        $titleRect = New-Object System.Drawing.RectangleF(18, $titleTop, $contentWidth, 46)
        $bodyRect = New-Object System.Drawing.RectangleF(18, $bodyTop, $contentWidth, $bodyHeight)
        $format = New-Object System.Drawing.StringFormat
        $format.Alignment = [System.Drawing.StringAlignment]::Center

        $graphics.DrawString($Title, $titleFont, $accentBrush, $titleRect, $format)
        $graphics.DrawString($Subtitle, $bodyFont, $mutedBrush, $bodyRect, $format)

        $format.Dispose()
        $titleFont.Dispose()
        $bodyFont.Dispose()
        $accentBrush.Dispose()
        $mutedBrush.Dispose()
        $linePen.Dispose()

        $bitmap.Save($OutputPath, [System.Drawing.Imaging.ImageFormat]::Bmp)
    }
    finally {
        $logo.Dispose()
        $graphics.Dispose()
        $bitmap.Dispose()
    }
}

function Get-InnoSetupCompiler {
    $command = Get-Command ISCC.exe -ErrorAction SilentlyContinue
    if ($command) {
        return $command.Source
    }

    $fallbacks = @(
        "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
        "C:\Program Files\Inno Setup 6\ISCC.exe",
        (Join-Path $env:LOCALAPPDATA "Programs\Inno Setup 6\ISCC.exe")
    )

    foreach ($fallback in $fallbacks) {
        if (Test-Path $fallback) {
            return $fallback
        }
    }

    throw "Inno Setup Compiler (ISCC.exe) was not found."
}

function Stop-RunningBuildBinary {
    $processes = Get-Process -Name "3-win-drag" -ErrorAction SilentlyContinue
    foreach ($process in $processes) {
        try {
            if ($process.Path -and ([System.StringComparer]::OrdinalIgnoreCase.Equals($process.Path, $releaseExePath))) {
                Stop-Process -Id $process.Id -Force -ErrorAction Stop
            }
        }
        catch {
            continue
        }
    }
}

$version = Get-AppVersion

New-Directory -Path $distRoot
New-Directory -Path $assetRoot
New-Directory -Path $installerRoot

Push-Location $projectRoot
try {
    Stop-RunningBuildBinary

    Write-Host "Building release executable..."
    cargo build --release

    if (-not (Test-Path $releaseExePath)) {
        throw "The release executable was not produced at $releaseExePath."
    }

    Write-Host "Generating installer assets..."
    Write-InstallerGuide
    Write-IconFromPng -PngPath $logoPath -IcoPath $installerIconPath
    New-WizardBitmap -OutputPath $sideBitmapPath -Width 164 -Height 314 -Title "3-win-drag" -Subtitle "Three-finger drag and drop for Windows 10 and Windows 11." -LogoSize 92
    New-WizardBitmap -OutputPath $topBitmapPath -Width 55 -Height 55 -Title "" -Subtitle "" -LogoSize 36

    $compiler = Get-InnoSetupCompiler

    Write-Host "Compiling installer with Inno Setup..."
    & $compiler "/DAppVersion=$version" $issPath
    if ($LASTEXITCODE -ne 0) {
        throw "Inno Setup compilation failed with exit code $LASTEXITCODE."
    }

    $installerPath = Join-Path $installerRoot ("3-win-drag-setup-{0}.exe" -f $version)
    if (-not (Test-Path $installerPath)) {
        throw "Expected installer output was not found at $installerPath."
    }

    Write-Host ""
    Write-Host "Installer build completed:"
    Write-Host $installerPath
}
finally {
    Pop-Location
}
