@echo off
chcp 65001 >nul
setlocal enabledelayedexpansion
title veilanon build araci - aegisSoft
set "ROOT=%~dp0"
cd /d "%ROOT%"
set "SRC_TAURI=%ROOT%src-tauri"
set "LOG_DIR=%ROOT%logs"

rem ---- Rust PATH garantisi (rustup kullanicilari icin) ------------------------
where rustc >nul 2>&1 || (
    if exist "%USERPROFILE%\.cargo\bin\rustc.exe" set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
)

rem ---- Log dosyasi (her calistirmada yeni, logs/ klasorune) --------------------
if not exist "%LOG_DIR%" mkdir "%LOG_DIR%"
for /f "delims=" %%I in ('powershell -NoProfile -Command "(Get-Date -Format yyyyMMdd-HHmmss)"') do set "DT=%%I"
set "LOG=%LOG_DIR%\build-%DT%.log"
set /a STEP=0

call :log "veilanon build araci baslatildi"

rem ---- Parametre kontrolu ------------------------------------------------------
set "INTERACTIVE=0"
if "%~1"=="" (
    set "INTERACTIVE=1"
    goto :menu
)
set "CMD=%~1"
if /i "%CMD%"=="dev"      goto :dev
if /i "%CMD%"=="frontend" goto :frontend
if /i "%CMD%"=="debug"    goto :debug
if /i "%CMD%"=="release"  goto :release
if /i "%CMD%"=="setup"    goto :setup
if /i "%CMD%"=="test"     goto :test
if /i "%CMD%"=="audit"    goto :audit
if /i "%CMD%"=="clean"    goto :clean
if /i "%CMD%"=="update"   goto :update
if /i "%CMD%"=="sign"     goto :sign
if /i "%CMD%"=="env"      goto :check_env
if /i "%CMD%"=="help"     goto :help
echo Bilinmeyen parametre: %1   (ipucu: build.bat help)
if "%INTERACTIVE%"=="1" pause
exit /b 1

:menu
set "INTERACTIVE=1"
call :banner
echo.
echo  Bir islem secmek icin numaraya basip ENTER'a basin:
echo.
echo   [ 1]  Gelistirme modu      - uygulamayi canli acar (tauri dev)
echo   [ 2]  Frontend kontrolu    - tip kontrolu + derleme (hizli)
echo   [ 3]  Debug derleme        - hata ayiklama surumu (veilanon.exe)
echo   [ 4]  Release derleme      - optimize surum + NSIS + MSI paketleri
echo   [ 5]  Kurulum sihirbazi    - sadece setup.exe uretir (NSIS)
echo   [ 6]  Testler              - Rust birim testleri + Svelte kontrolu
echo   [ 7]  Guvenlik denetimi    - cargo-audit + npm audit + gitleaks
echo   [ 8]  Temizlik             - derleme ciktilarini siler
echo   [ 9]  Bagimlilik guncelle  - npm + cargo paketlerini gunceller
echo   [10]  Guncelleme anahtari  - updater imza anahtari uretir
echo   [11]  Ortam kontrolu       - rust/node/npm/tauri surumlerini gosterir
echo   [ 0]  Cikis
echo.
set /p "CHOICE=Seciminiz: "
if "%CHOICE%"=="1" goto :dev
if "%CHOICE%"=="2" goto :frontend
if "%CHOICE%"=="3" goto :debug
if "%CHOICE%"=="4" goto :release
if "%CHOICE%"=="5" goto :setup
if "%CHOICE%"=="6" goto :test
if "%CHOICE%"=="7" goto :audit
if "%CHOICE%"=="8" goto :clean
if "%CHOICE%"=="9" goto :update
if "%CHOICE%"=="10" goto :sign
if "%CHOICE%"=="11" goto :check_env
echo  Cikis yapildi. Iyi gunler!
pause
exit /b 0

:banner
cls
echo.
echo   veilanon  -  build araci  v0.0.1
echo   gizlilik - hiz - ozgurluk
echo   aegisSoft  -  github.com/MrSpy00/veilanon  -  AGPL-3.0
echo.
goto :eof

:check_env
call :step "Ortam araclari kontrol ediliyor"
where rustc >nul 2>&1 || (call :fail "rustc bulunamadi - https://rustup.rs" & goto :eof)
where cargo >nul 2>&1 || (call :fail "cargo bulunamadi" & goto :eof)
where node >nul 2>&1 || (call :fail "node bulunamadi - https://nodejs.org" & goto :eof)
where npm >nul 2>&1 || (call :fail "npm bulunamadi" & goto :eof)
for /f "delims=" %%V in ('rustc --version') do call :print info "%%V"
for /f "delims=" %%V in ('node --version') do call :print info "node %%V"
for /f "delims=" %%V in ('npm --version') do call :print info "npm %%V"
call npx --no-install tauri --version >nul 2>&1
if errorlevel 1 (
    call :print warn "tauri CLI kurulu degil - once: npm install"
) else (
    for /f "delims=" %%V in ('npx --no-install tauri --version') do call :print info "%%V"
)
if exist "%ROOT%.env" (
    call :print ok ".env dosyasi bulundu"
) else (
    call :print warn ".env yok - kopyalayin: copy .env.example .env"
)
call :done "Ortam hazir"
goto :eof

:dev
call :step "Gelistirme modu baslatiliyor (pencere acilacak)"
if not exist "%ROOT%node_modules" (call :print warn "node_modules yok, npm install yapiliyor" & call npm install || (call :fail "npm install" & goto :eof))
call :log "ADIM %STEP%: tauri dev baslatildi - tum konsol ciktisi log dosyasina da yazilir"
rem Tee-Object: cikti hem ekranda hem log dosyasinda gorunur
rem 2>&1 stderr'i ErrorRecord yapar ve PowerShell kirmizi gosterir; "%_" ile
rem duz metne cevrilir (icerik kaybolmaz, sadece yanlis "hata" gorunumu gider).
powershell -NoProfile -Command "cd '%ROOT%'; npm run tauri:dev 2>&1 | ForEach-Object { \"$_\" } | Tee-Object -FilePath '%LOG%' -Append"
if errorlevel 1 call :fail "tauri dev"
call :done "Gelistirme modu sonlandi"
goto :eof

:frontend
call :step "Frontend tip kontrolu ve derleme"
if not exist "%ROOT%node_modules" (call :print warn "node_modules yok, npm install yapiliyor" & call npm.cmd install || (call :fail "npm install" & goto :eof))
call :print info "Adim 1/2: svelte-check (tip hatalari)"
call npm.cmd run check
if errorlevel 1 (call :fail "npm run check" & goto :eof)
call :print info "Adim 2/2: vite build (uretim derlemesi)"
call npm.cmd run build
if errorlevel 1 (call :fail "npm run build" & goto :eof)
call :done "Frontend derlendi"
goto :eof

:debug
call :step "Debug derleme (frontend + Rust)"
if not exist "%ROOT%node_modules" (call :print warn "node_modules yok, npm install yapiliyor" & call npm.cmd install || (call :fail "npm install" & goto :eof))
call :print info "Adim 1/2: frontend derleme"
call npm.cmd run build
if errorlevel 1 (call :fail "frontend build" & goto :eof)
call :print info "Adim 2/2: cargo build (debug)"
call cargo build --manifest-path "%SRC_TAURI%\Cargo.toml"
if errorlevel 1 (call :fail "cargo build (debug)" & goto :eof)
call :print ok "Cikti: src-tauri\target\debug\veilanon.exe"
call :done "Debug derleme tamam"
goto :eof

:release
call :step "Release derleme (optimize surum + NSIS + MSI - uzun surebilir)"
if not exist "%ROOT%node_modules" (call :print warn "node_modules yok, npm install yapiliyor" & call npm.cmd install || (call :fail "npm install" & goto :eof))
call :print info "tauri build calisiyor - ilk derleme 10-20 dk surebilir"
powershell -NoProfile -Command "cd '%ROOT%'; npm run tauri:build 2>&1 | ForEach-Object { \"$_\" } | Tee-Object -FilePath '%LOG%' -Append"
if errorlevel 1 call :fail "tauri build (release)"
call :copy_artifacts
call :done "Release hazir"
goto :eof

:setup
call :step "Kurulum sihirbazi derleniyor (NSIS setup.exe)"
if not exist "%ROOT%node_modules" (call :print warn "node_modules yok, npm install yapiliyor" & call npm.cmd install || (call :fail "npm install" & goto :eof))
call :print info "tauri build --bundles nsis - ilk derleme 10-20 dk surebilir"
powershell -NoProfile -Command "cd '%ROOT%'; npm run tauri:build -- --bundles nsis 2>&1 | ForEach-Object { \"$_\" } | Tee-Object -FilePath '%LOG%' -Append"
if errorlevel 1 call :fail "tauri build (nsis)"
call :copy_artifacts
call :done "setup.exe hazir"
goto :eof

:test
call :step "Testler calistiriliyor (Rust + Svelte)"
call :print info "Adim 1/2: cargo test"
call cargo test --manifest-path "%SRC_TAURI%\Cargo.toml" -- --test-threads=1
if errorlevel 1 (call :fail "cargo test" & goto :eof)
call :print info "Adim 2/2: npm run check"
if exist "%ROOT%node_modules" (
    call npm.cmd run check
    if errorlevel 1 (call :fail "npm run check" & goto :eof)
)
call :done "Testler gecti"
goto :eof

:audit
call :step "Guvenlik denetimleri (cargo-audit + npm audit + gitleaks)"
if exist "%SRC_TAURI%\Cargo.lock" (
    call :print info "cargo audit (Rust guvenlik taramasi)"
    call cargo audit --file "%SRC_TAURI%\Cargo.lock" 2>nul || call :print warn "cargo-audit kurulu degil: cargo install cargo-audit"
)
if exist "%ROOT%package-lock.json" (
    call :print info "npm audit (JS bagimlilik taramasi)"
    call npm.cmd audit --audit-level=high || call :print warn "npm audit onerileri var - 'npm audit' ile detay"
)
where gitleaks >nul 2>&1 && (
    call :print info "gitleaks (gizli anahtar taramasi)"
    call gitleaks git --no-banner || call :print warn "gitleaks bulgulari var"
) || call :print warn "gitleaks kurulu degil - atlandi"
call :done "Denetim tamamlandi"
goto :eof

:clean
call :print warn "Bu islem derleme ciktilarini siler!"
set /p "CONF=Devam etmek istiyor musunuz? (e/h): "
if /i not "%CONF%"=="e" (echo  Iptal edildi. & pause & goto :eof)
call :step "Temizlik basladi"
if exist "%SRC_TAURI%\target" (call cargo clean --manifest-path "%SRC_TAURI%\Cargo.toml")
if exist "%ROOT%build" (rmdir /s /q "%ROOT%build")
if exist "%ROOT%.svelte-kit" (rmdir /s /q "%ROOT%.svelte-kit")
if exist "%ROOT%release" (rmdir /s /q "%ROOT%release")
set /p "CONF2=node_modules da silinsin mi? (e/h): "
if /i "%CONF2%"=="e" (if exist "%ROOT%node_modules" rmdir /s /q "%ROOT%node_modules")
call :done "Temizlendi"
goto :eof

:update
call :step "Bagimliliklar guncelleniyor (npm update + cargo update)"
call npm.cmd update
if errorlevel 1 (call :fail "npm update" & goto :eof)
call cargo update --manifest-path "%SRC_TAURI%\Cargo.toml"
if errorlevel 1 (call :fail "cargo update" & goto :eof)
call :print warn "Guncelleme sonrasi test edin: build.bat test"
call :done "Bagimliliklar guncellendi"
goto :eof

:sign
call :step "Guncelleme (updater) imza anahtari uretiliyor"
if not exist "%ROOT%keys" mkdir "%ROOT%keys"
call npx tauri signer generate --force "%ROOT%keys/veilanon.key" || (call :fail "tauri signer generate" & goto :eof)
call :print ok "Ozel anahtar: %ROOT%keys\veilanon.key  (ASLA git'e eklemeyin)"
call :print info "Genel anahtari (pubkey) tauri.conf.json ^> plugins ^> updater ^> pubkey alanina yapistirin"
call :print info "ve updater.active = true yapin. Sonra: build.bat setup"
call :done "Anahtar uretildi"
goto :eof

:copy_artifacts
if not exist "%ROOT%release" mkdir "%ROOT%release"
set "COPIED=0"
for /r "%SRC_TAURI%\target\release\bundle\nsis" %%F in (*setup.exe) do (
    copy /y "%%F" "%ROOT%release\%%~nxF" >nul 2>&1
    call :print ok "Kurulum paketi kopyalandi: release\%%~nxF"
    set "COPIED=1"
)
if "%COPIED%"=="0" call :print warn "setup.exe bulunamadi - bundle klasorunu kontrol edin"
for /r "%SRC_TAURI%\target\release\bundle\msi" %%F in (*.msi) do (
    copy /y "%%F" "%ROOT%release\" >nul 2>&1
    call :print ok "MSI paketi kopyalandi: release\%%~nxF"
)
if exist "%ROOT%scripts\sign-windows.ps1" (
    call :print info "Windows Authenticode kod imzalamasi yapiliyor..."
    powershell -NoProfile -ExecutionPolicy Bypass -File "%ROOT%scripts\sign-windows.ps1"
)
goto :eof

:step
set /a STEP+=1
call :print step "[Adim %STEP%] %~1"
call :log "ADIM %STEP%: %~1"
goto :eof

:done
call :log "[OK] %~1"
call :print ok "%~1"
echo.
echo  ------------------------------------------------------------
echo   Log dosyasi: %LOG%
echo  ------------------------------------------------------------
echo.
if "%INTERACTIVE%"=="1" pause
goto :eof

:fail
call :log "[HATA] %~1"
call :print err "%~1"
echo.
echo  Log dosyasi: %LOG%
echo.
if "%INTERACTIVE%"=="1" pause
exit /b 1

:print
if "%1"=="ok"    powershell -NoProfile -Command "Write-Host '  [OK]    %~2' -ForegroundColor Green"
if "%1"=="warn"  powershell -NoProfile -Command "Write-Host '  [UYARI]  %~2' -ForegroundColor Yellow"
if "%1"=="err"   powershell -NoProfile -Command "Write-Host '  [HATA]   %~2' -ForegroundColor Red"
if "%1"=="info"  powershell -NoProfile -Command "Write-Host '  [~]      %~2' -ForegroundColor Cyan"
if "%1"=="step"  powershell -NoProfile -Command "Write-Host '  [>>>]    %~2' -ForegroundColor White"
goto :eof

:log
rem Locale-independent ISO timestamp (cmd %DATE%/%TIME% vary by locale and
rem can render as empty/"null" on some systems)
for /f "delims=" %%T in ('powershell -NoProfile -Command "(Get-Date -Format 'yyyy-MM-dd HH:mm:ss.fff')"') do echo %%T  %~1 >> "%LOG%"
goto :eof

:help
echo  Kullanim: build.bat [komut]
echo.
echo  Komutlar ve ne yaptiklari:
echo    dev       - gelistirme modu (uygulama penceresi acilir)
echo    frontend  - svelte-check + vite build
echo    debug     - debug surumu derler
echo    release   - release + NSIS + MSI paketleri
echo    setup     - sadece kurulum sihirbazi (setup.exe)
echo    test      - cargo test + npm run check
echo    audit     - cargo-audit + npm audit + gitleaks
echo    clean     - derleme ciktilarini siler
echo    update    - bagimliliklari gunceller
echo    sign      - updater imza anahtari uretir
echo    env       - ortam surumlerini gosterir
echo    help      - bu yardim metni
echo.
echo  Parametresiz calistirirsaniz (cift tik) etkilesimli menu acilir.
if "%INTERACTIVE%"=="1" pause
exit /b 0
