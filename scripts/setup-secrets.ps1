# ============================================================================
# veilanon — GitHub Secrets kurulum script'i
# .env dosyasındaki değerleri GitHub Secrets'a aktarır.
# Kullanim: powershell -ExecutionPolicy Bypass -File scripts\setup-secrets.ps1
# ============================================================================

$ErrorActionPreference = "Stop"
$envFile = Join-Path $PSScriptRoot "..\.env"

if (-not (Test-Path $envFile)) {
    Write-Host "[HATA] .env dosyasi bulunamadi: $envFile" -ForegroundColor Red
    exit 1
}

Write-Host "veilanon GitHub Secrets kurulumu baslatiliyor..." -ForegroundColor Cyan
Write-Host ""

$content = Get-Content $envFile -Raw
$secrets = @{}

foreach ($line in $content -split "`n") {
    $line = $line.Trim()
    if ($line -eq "" -or $line.StartsWith("#")) { continue }
    $eqIdx = $line.IndexOf("=")
    if ($eqIdx -lt 1) { continue }
    $key = $line.Substring(0, $eqIdx).Trim()
    $value = $line.Substring($eqIdx + 1).Trim()
    if ($key -and $value) {
        $secrets[$key] = $value
    }
}

$count = 0
foreach ($key in $secrets.Keys) {
    $value = $secrets[$key]
    Write-Host "  Ayarlaniyor: $key" -ForegroundColor Gray -NoNewline
    $result = gh secret set $key --body $value 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host " [OK]" -ForegroundColor Green
        $count++
    } else {
        Write-Host " [HATA]" -ForegroundColor Red
        Write-Host "    $result" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "Tamamlandi: $count secret ayarlandi." -ForegroundColor Green
Write-Host "Simdi yeni build tetiklemek icin: git tag -d v0.0.1 && git push origin :refs/tags/v0.0.1 && git tag v0.0.1 && git push origin v0.0.1" -ForegroundColor Yellow
