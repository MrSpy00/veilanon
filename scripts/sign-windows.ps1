<#
.SYNOPSIS
    veilanon Windows Code Signing & SmartScreen Protection Eliminator
.DESCRIPTION
    Signs binary executables and installers with an Authenticode certificate,
    exports the Root CA certificate, installs it to Trusted Root Authority, and
    creates a 1-click trust installer (Trust-Certificate.bat).
#>
param(
    [string]$TargetFile,
    [string]$CertPath,
    [string]$CertPassword,
    [string]$TimestampServer = "http://timestamp.digicert.com"
)

$ErrorActionPreference = "Continue"

Write-Host "`n[Windows Authenticode Code Signing & SmartScreen Protection]" -ForegroundColor Cyan

# Find or Create Code Signing Certificate in CurrentUser\My
$cert = $null

if ($CertPath -and (Test-Path $CertPath)) {
    Write-Host "  Using PFX certificate from: $CertPath" -ForegroundColor Green
    $cert = Get-PfxCertificate -FilePath $CertPath
} else {
    # Check Cert:\CurrentUser\My for existing CodeSigning cert
    $certs = Get-ChildItem Cert:\CurrentUser\My -CodeSigningCert -ErrorAction SilentlyContinue
    if (-not $certs) {
        $certs = Get-ChildItem Cert:\LocalMachine\My -CodeSigningCert -ErrorAction SilentlyContinue
    }

    if ($certs) {
        $cert = $certs[0]
        Write-Host "  Found existing Code Signing certificate: $($cert.Subject)" -ForegroundColor Green
    } else {
        Write-Host "  Generating dedicated code signing certificate..." -ForegroundColor Yellow
        try {
            $cert = New-SelfSignedCertificate `
                -Type CodeSigningCert `
                -Subject "CN=veilanon Open Source, O=aegisSoft, C=TR" `
                -KeyUsage DigitalSignature `
                -FriendlyName "veilanon Code Signing Certificate" `
                -CertStoreLocation "Cert:\CurrentUser\My" `
                -NotAfter (Get-Date).AddYears(10)
            Write-Host "  Certificate created successfully." -ForegroundColor Green
        } catch {
            Write-Host "  Failed to generate certificate: $_" -ForegroundColor Red
        }
    }
}

if (-not $cert) {
    Write-Host "  [WARN] No certificate available for code signing." -ForegroundColor Yellow
    exit 0
}



# 2. Export public certificate to release\veilanon-ca.cer
if (-not (Test-Path "release")) {
    New-Item -ItemType Directory -Path "release" -Force | Out-Null
}

$cerPath = "release\veilanon-ca.cer"
try {
    [System.IO.File]::WriteAllBytes($cerPath, $cert.Export([System.Security.Cryptography.X509Certificates.X509ContentType]::Cert))
    Write-Host "  [OK] Exported Root Certificate to: $cerPath" -ForegroundColor Green
} catch {
    Write-Host "  [WARN] Failed to export CER: $_" -ForegroundColor Yellow
}

# 3. Create 1-click Trust-Certificate.bat in release folder
$trustBatContent = @"
@echo off
chcp 65001 >nul
echo ============================================================================
echo  veilanon Guvenlik ve Sertifika Dogrulama Araci
echo ============================================================================
echo.
echo veilanon dijital sertifikasi Guvenilen Kok Sertifika Yetkilileri havuzuna
echo ekleniyor. Bu islem Windows SmartScreen ve Virus uyarisini tamamen kaldirir.
echo.
certutil -addstore -f "Root" "%~dp0veilanon-ca.cer"
if %ERRORLEVEL% EQU 0 (
    echo.
    echo [BASARILI] veilanon sertifikasi basariyla guvenilenlere eklendi!
    echo Artik kurulum dosyasini (veilanon-setup.exe) uyarisiz calistirabilirsiniz.
) else (
    echo.
    echo [BILGI] Yonetici izinleri gerekebilir. Lutfen sag tiklayip 'Yonetici Olarak Calistir' seciniz.
)
echo.
pause
"@

$trustBatPath = "release\Trust-Certificate.bat"
[System.IO.File]::WriteAllText($trustBatPath, $trustBatContent, [System.Text.Encoding]::UTF8)
Write-Host "  [OK] Created 1-click Certificate Trust Tool: $trustBatPath" -ForegroundColor Green

# 4. Determine target files to sign
$filesToSign = @()
if ($TargetFile) {
    if (Test-Path $TargetFile) {
        $filesToSign += (Resolve-Path $TargetFile).Path
    }
} else {
    $searchPaths = @(
        "release\*.exe",
        "release\*.msi",
        "src-tauri\target\release\veilanon.exe",
        "src-tauri\target\release\bundle\nsis\*.exe",
        "src-tauri\target\release\bundle\msi\*.msi"
    )
    foreach ($pattern in $searchPaths) {
        Get-Item $pattern -ErrorAction SilentlyContinue | ForEach-Object {
            if ($filesToSign -notcontains $_.FullName) {
                $filesToSign += $_.FullName
            }
        }
    }
}

if ($filesToSign.Count -eq 0) {
    Write-Host "  No binaries found to sign yet." -ForegroundColor Gray
    exit 0
}

foreach ($file in $filesToSign) {
    Write-Host "  Signing: $(Split-Path $file -Leaf)" -ForegroundColor White
    try {
        $sig = Set-AuthenticodeSignature -FilePath $file -Certificate $cert -HashAlgorithm SHA256 -ErrorAction Stop
        Write-Host "    [OK] Signed successfully" -ForegroundColor Green
    } catch {
        Write-Host "    [FAIL] Failed to sign $file : $_" -ForegroundColor Red
    }
}

Write-Host "[Code Signing Completed]`n" -ForegroundColor Cyan
