# veilanon Comprehensive Release Packaging Script
$ErrorActionPreference = "Stop"

Write-Host "`n[Packaging Release Artifacts Across All Platforms]" -ForegroundColor Cyan

if (-not (Test-Path "release")) {
    New-Item -ItemType Directory -Path "release" -Force | Out-Null
} else {
    Remove-Item "release\*" -Recurse -Force -ErrorAction SilentlyContinue
}

# 1. Windows Installers (NSIS and MSI)
if (Test-Path "ci-artifacts\veilanon-release-windows-latest\nsis\veilanon_0.0.1_x64-setup.exe") {
    Copy-Item "ci-artifacts\veilanon-release-windows-latest\nsis\veilanon_0.0.1_x64-setup.exe" -Destination "release\veilanon_0.0.1_x64-setup.exe" -Force
} elseif (Test-Path "src-tauri\target\release\bundle\nsis\veilanon_0.0.1_x64-setup.exe") {
    Copy-Item "src-tauri\target\release\bundle\nsis\veilanon_0.0.1_x64-setup.exe" -Destination "release\veilanon_0.0.1_x64-setup.exe" -Force
}

if (Test-Path "ci-artifacts\veilanon-release-windows-latest\msi\veilanon_0.0.1_x64_en-US.msi") {
    Copy-Item "ci-artifacts\veilanon-release-windows-latest\msi\veilanon_0.0.1_x64_en-US.msi" -Destination "release\veilanon_0.0.1_x64_en-US.msi" -Force
} elseif (Test-Path "src-tauri\target\release\bundle\msi\veilanon_0.0.1_x64_en-US.msi") {
    Copy-Item "src-tauri\target\release\bundle\msi\veilanon_0.0.1_x64_en-US.msi" -Destination "release\veilanon_0.0.1_x64_en-US.msi" -Force
}

# 2. Linux Artifacts (AppImage, Deb, Rpm)
if (Test-Path "ci-artifacts\veilanon-release-ubuntu-latest\appimage\veilanon_0.0.1_amd64.AppImage") {
    Copy-Item "ci-artifacts\veilanon-release-ubuntu-latest\appimage\veilanon_0.0.1_amd64.AppImage" -Destination "release\veilanon_0.0.1_amd64.AppImage" -Force
}
if (Test-Path "ci-artifacts\veilanon-release-ubuntu-latest\deb\veilanon_0.0.1_amd64.deb") {
    Copy-Item "ci-artifacts\veilanon-release-ubuntu-latest\deb\veilanon_0.0.1_amd64.deb" -Destination "release\veilanon_0.0.1_amd64.deb" -Force
}
if (Test-Path "ci-artifacts\veilanon-release-ubuntu-latest\rpm\veilanon-0.0.1-1.x86_64.rpm") {
    Copy-Item "ci-artifacts\veilanon-release-ubuntu-latest\rpm\veilanon-0.0.1-1.x86_64.rpm" -Destination "release\veilanon-0.0.1-1.x86_64.rpm" -Force
}

# 3. macOS Artifacts (DMG and app.tar.gz)
if (Test-Path "ci-artifacts\veilanon-release-macos-latest\dmg\veilanon_0.0.1_aarch64.dmg") {
    Copy-Item "ci-artifacts\veilanon-release-macos-latest\dmg\veilanon_0.0.1_aarch64.dmg" -Destination "release\veilanon_0.0.1_aarch64.dmg" -Force
}
if (Test-Path "ci-artifacts\veilanon-release-macos-latest\macos\veilanon.app.tar.gz") {
    Copy-Item "ci-artifacts\veilanon-release-macos-latest\macos\veilanon.app.tar.gz" -Destination "release\veilanon_0.0.1_aarch64.app.tar.gz" -Force
}

# 4. Sign Windows binaries if signing script exists
if (Test-Path "scripts\sign-windows.ps1") {
    & powershell -ExecutionPolicy Bypass -File scripts\sign-windows.ps1
}

# 5. Clean cert artifacts from release folder
Remove-Item "release\veilanon-ca.cer", "release\Trust-Certificate.bat" -Force -ErrorAction SilentlyContinue

# 6. Generate fresh unified SHA256SUMS.txt
$hashLines = @()
Get-ChildItem -Path "release\*" -Exclude "*.sha256", "SHA256SUMS.txt", "*.cer", "*.bat" | ForEach-Object {
    $h = Get-FileHash -Algorithm SHA256 -Path $_.FullName
    $hashLines += "$($h.Hash.ToLower())  $($_.Name)"
}
$hashLines | Out-File -FilePath "release\SHA256SUMS.txt" -Encoding utf8

Write-Host "[Release Packaging Completed Successfully]" -ForegroundColor Green
Get-ChildItem -Path "release\*" | Select-Object Name, Length, LastWriteTime
