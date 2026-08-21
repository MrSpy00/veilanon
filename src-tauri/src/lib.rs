//! veilanon — Privacy-first, open-source desktop communication platform
//! Copyright © 2026 aegisSoft — https://www.aegissoft.com.tr/
//! AGPL-3.0 License — https://github.com/MrSpy00/veilanon
//!
//! Security boundary: ALL cryptographic operations, key management,
//! local encrypted storage, and sensitive network logic MUST remain
//! in this Rust core. The UI (WebView) ONLY communicates via narrow,
//! schema-validated IPC commands.

mod commands;
mod config;
mod crypto;
mod db;
mod error;
mod models;
mod network;
mod secrets;
mod state;

use commands::{auth, crypto as crypto_cmd, discord, files, gifs, local_ai, logging, media, media_scrape, messages, mls, privacy_tools, settings, social, spaces, updater};
use state::AppState;
use tauri::{Emitter, Manager};
use tracing::info;
use tracing_subscriber::prelude::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load .env from multiple candidate locations (working directory, parent directory, executable directory).
    // Values are consumed by std::env::var readers; nothing is logged.
    dotenvy::dotenv().ok();
    dotenvy::from_path(std::path::Path::new("..").join(".env")).ok();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            dotenvy::from_path(parent.join(".env")).ok();
        }
    }

    init_logging();

    tauri::Builder::default()
        // ── Plugins ──────────────────────────────────────────────────────────
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .plugin(tauri_plugin_deep_link::init())
        // Tek örnek: ikinci bir başlatmada (örn. `veilanon://` deep link) yeni
        // pencere açılmaz; pencere öne getirilir ve URL mevcut örneğe iletilir.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
            for arg in argv {
                if arg.starts_with("veilanon://") || arg.contains("veilanon.com") {
                    let _ = app.emit("deep-link://new-url", vec![arg.clone()]);
                    let _ = app.emit("plugin:deep-link|new-url", vec![arg]);
                    break;
                }
            }
        }))
        // ── Application State ─────────────────────────────────────────────────
        .setup(|app| {
            let app_state = AppState::new(app.handle().clone())?;
            app.manage(app_state);
            info!("veilanon setup complete");

            // `register_all` Windows registry + Linux xdg-mime üzerinde
            // `veilanon://`'yu OS handler olarak kaydeder; sadece `init()` yetmez.
            #[cfg(any(windows, target_os = "linux"))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                if let Err(e) = app.deep_link().register_all() {
                    tracing::warn!(
                        "veilanon: failed to register veilanon:// scheme at runtime: {e}"
                    );
                } else {
                    info!("veilanon:// scheme registered for current user");
                }
            }

            // Şifreli secrets store (OS keychain korumalı) + bir kerelik .env migration'ı.
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            secrets::init(&data_dir);

            // İlk örnek bir deep link URL'siyle başlatıldıysa (Windows/Linux'ta
            // CLI argümanı olarak gelir) arayüze ilet — frontend onOpenUrl ile
            // yakalar. Kısa gecikme, WebView dinleyicisinin hazır olmasını
            // garanti eder (event kaçarsa kullanıcı bağlantıyı yine de kopyalar).
            let mut first_url: Option<String> = None;
            for arg in std::env::args().skip(1) {
                if arg.starts_with("veilanon://") || arg.contains("veilanon.com") {
                    first_url = Some(arg);
                    break;
                }
            }
            if let Some(url) = first_url {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
                    let _ = handle.emit("deep-link://new-url", vec![url.clone()]);
                    let _ = handle.emit("plugin:deep-link|new-url", vec![url]);
                });
            }

            // WebView2 media permissions (Windows): auto-allow microphone,
            // camera and screen capture so the WebView2 permission dialog never
            // appears — the app owns these choices via its settings UI.
            #[cfg(windows)]
            {
                if let Some(window) = app.get_webview_window("main") {
                    use webview2_com::Microsoft::Web::WebView2::Win32::{
                        COREWEBVIEW2_PERMISSION_KIND, COREWEBVIEW2_PERMISSION_KIND_CAMERA,
                        COREWEBVIEW2_PERMISSION_KIND_MICROPHONE,
                        COREWEBVIEW2_PERMISSION_STATE_ALLOW,
                        ICoreWebView2SetPermissionStateCompletedHandler, ICoreWebView2Profile6,
                        ICoreWebView2_13,
                    };
                    use webview2_com::PermissionRequestedEventHandler;
                    use windows_core::Interface;

                    // webview2-com-sys 0.38.2, ekran yakalama sabitlerini henüz
                    // üretmez (SDK 1.0.2792+); resmi CoreWebView2PermissionKind
                    // numaraları kullanılır: 15=SCREEN_CAPTURE, 16=WITH_AUDIO,
                    // 17=WITHOUT_AUDIO. Yeni SDK'larla eşleşir.
                    const COREWEBVIEW2_PERMISSION_KIND_SCREEN_CAPTURE: COREWEBVIEW2_PERMISSION_KIND =
                        COREWEBVIEW2_PERMISSION_KIND(15);
                    const COREWEBVIEW2_PERMISSION_KIND_SCREEN_CAPTURE_WITH_AUDIO: COREWEBVIEW2_PERMISSION_KIND =
                        COREWEBVIEW2_PERMISSION_KIND(16);
                    const COREWEBVIEW2_PERMISSION_KIND_SCREEN_CAPTURE_WITHOUT_AUDIO: COREWEBVIEW2_PERMISSION_KIND =
                        COREWEBVIEW2_PERMISSION_KIND(17);

                    let _ = window.with_webview(move |webview| {
                        let controller = webview.controller();
                        unsafe {
                            if let Ok(core) = controller.CoreWebView2() {
                                // Tarayıcı otomatik doldurma ("kaydedilmiş bilgiler")
                                // ve parola kaydı asla gösterilmez — uygulama kendi
                                // kimlik akışını kullanır.
                                if let Ok(core13) = core.cast::<ICoreWebView2_13>() {
                                    if let Ok(profile) = core13.Profile() {
                                        if let Ok(p6) = profile.cast::<ICoreWebView2Profile6>() {
                                            p6.SetIsGeneralAutofillEnabled(false).ok();
                                            p6.SetIsPasswordAutosaveEnabled(false).ok();
                                        }
                                        // İzinler uygulama orijini için kalıcı önceden tanımlanır.
                                        if let Ok(p7) =
                                            profile.cast::<webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Profile7>()
                                        {
                                            for origin in ["http://localhost:1420", "http://tauri.localhost"] {
                                                let _ = p7.SetPermissionState(
                                                    COREWEBVIEW2_PERMISSION_KIND_MICROPHONE,
                                                    &windows_core::HSTRING::from(origin),
                                                    COREWEBVIEW2_PERMISSION_STATE_ALLOW,
                                                    None::<&ICoreWebView2SetPermissionStateCompletedHandler>,
                                                );
                                                let _ = p7.SetPermissionState(
                                                    COREWEBVIEW2_PERMISSION_KIND_CAMERA,
                                                    &windows_core::HSTRING::from(origin),
                                                    COREWEBVIEW2_PERMISSION_STATE_ALLOW,
                                                    None::<&ICoreWebView2SetPermissionStateCompletedHandler>,
                                                );
                                            }
                                        }
                                    }
                                }
                                let handler = PermissionRequestedEventHandler::create(Box::new(
                                    |_, args| {
                                        let Some(args) = args else { return Ok(()) };
                                        let mut kind = COREWEBVIEW2_PERMISSION_KIND::default();
                                        args.PermissionKind(&mut kind)?;
                                        if kind == COREWEBVIEW2_PERMISSION_KIND_MICROPHONE
                                            || kind == COREWEBVIEW2_PERMISSION_KIND_CAMERA
                                            || kind == COREWEBVIEW2_PERMISSION_KIND_SCREEN_CAPTURE
                                            || kind == COREWEBVIEW2_PERMISSION_KIND_SCREEN_CAPTURE_WITH_AUDIO
                                            || kind == COREWEBVIEW2_PERMISSION_KIND_SCREEN_CAPTURE_WITHOUT_AUDIO
                                        {
                                            args.SetState(COREWEBVIEW2_PERMISSION_STATE_ALLOW)?;
                                        }
                                        Ok(())
                                    },
                                ));
                                let mut token: i64 = 0;
                                let _ = core.add_PermissionRequested(&handler, &mut token);
                            }
                        }
                    });
                }
            }

            // Background tasks: offline-queue flush, expired-message purge and
            // realtime connection. Setup is synchronous: tokio::spawn panics
            // here ("no reactor"); tauri::async_runtime::spawn runs on Tauri's
            // own Tokio runtime.
            let flush_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(20));
                loop {
                    interval.tick().await;
                    let Some(state) = flush_handle.try_state::<AppState>() else {
                        continue;
                    };
                    let Some(device_id) = ({
                        let identity = state.get_or_restore_identity().await;
                        identity.as_ref().map(|i| i.device_id)
                    }) else {
                        continue;
                    };
                    let (db, network) = {
                        let db = state.db.read().await;
                        let network = state.network.read().await;
                        (db, network)
                    };
                    // Lock order: identity → db → network (matches commands).
                    let _ = network.flush_offline_queue(&db, device_id).await;
                    drop(network);
                    drop(db);
                    messages::purge_expired(&state).await;
                    messages::flush_pending_dm_messages(&state).await;
                }
            });

            let rt_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let Some(state) = rt_handle.try_state::<AppState>() else {
                    return;
                };
                let realtime = state.network.read().await.realtime.clone();
                let _ = realtime.run(rt_handle).await;
            });

            // Token refresh loop: refresh Supabase JWT every 50 minutes
            let refresh_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(50 * 60));
                loop {
                    interval.tick().await;
                    let Some(state) = refresh_handle.try_state::<AppState>() else {
                        continue;
                    };
                    let token = {
                        let guard = state.supabase_refresh_token.read().await;
                        guard.clone()
                    };
                    let Some(refresh_token) = token else { continue };
                    let result = tokio::time::timeout(
                        std::time::Duration::from_secs(10),
                        async {
                            let network = state.network.read().await;
                            network.api.refresh_access_token(&refresh_token).await
                        },
                    )
                    .await;
                    if let Ok(Ok(auth)) = result {
                        let mut network = state.network.write().await;
                        network.api.set_access_token(auth.access_token.clone());
                        network.realtime.set_token(Some(auth.access_token));
                        network.realtime.force_reconnect();
                        drop(network);
                        *state.supabase_refresh_token.write().await = Some(auth.refresh_token.clone());
                        let db = state.db.read().await;
                        let identity = state.get_or_restore_identity().await;
                        if let Some(ref id) = identity {
                            let _ = db.save_supabase_refresh_token(&id.id, &auth.refresh_token);
                        }
                    }
                }
            });

            // ── Presence Heartbeat — sends "online" presence every 30s ────────
            // Without a heartbeat, if the Supabase realtime connection drops
            // and reconnects, other clients won't see our presence again until
            // the next message. This ensures presence is always fresh.
            let presence_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // Give the app a moment to authenticate before starting heartbeat
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
                loop {
                    interval.tick().await;
                    let Some(state) = presence_handle.try_state::<AppState>() else { continue };
                    let identity = state.get_or_restore_identity().await;
                    let Some(identity) = identity else { continue };

                    // Broadcast heartbeat via Supabase realtime presence channel
                    // Frontend handles the actual Supabase presence track() call via emit
                    let _ = presence_handle.emit("presence:heartbeat", serde_json::json!({
                        "userId": identity.id.to_string(),
                        "username": identity.username,
                        "status": "online",
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    }));
                }
            });

            // ── System Tray Icon Setup (Tauri v2) ──────────────────────────────
            use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

            let show_item = MenuItemBuilder::with_id("show", "veilanon'ı Aç").build(app)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let status_online = MenuItemBuilder::with_id("status_online", "🟢 Çevrimiçi").build(app)?;
            let status_away = MenuItemBuilder::with_id("status_away", "🟡 Boşta").build(app)?;
            let status_dnd = MenuItemBuilder::with_id("status_dnd", "🔴 Rahatsız Etmeyin").build(app)?;
            let status_invisible = MenuItemBuilder::with_id("status_invisible", "⚪ Görünmez").build(app)?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let mute_item = MenuItemBuilder::with_id("toggle_mute", "🎙 Mikrofonu Sustur / Aç").build(app)?;
            let deafen_item = MenuItemBuilder::with_id("toggle_deafen", "🎧 Kulaklığı Kapat / Aç").build(app)?;
            let leave_voice_item = MenuItemBuilder::with_id("leave_voice", "📞 Sesten Ayrıl").build(app)?;
            let settings_item = MenuItemBuilder::with_id("settings", "⚙️ Ayarlar").build(app)?;
            let sep3 = PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "❌ Çıkış").build(app)?;

            let tray_menu = MenuBuilder::new(app)
                .item(&show_item)
                .item(&sep1)
                .item(&status_online)
                .item(&status_away)
                .item(&status_dnd)
                .item(&status_invisible)
                .item(&sep2)
                .item(&mute_item)
                .item(&deafen_item)
                .item(&leave_voice_item)
                .item(&settings_item)
                .item(&sep3)
                .item(&quit_item)
                .build()?;

            let app_icon = app.default_window_icon().cloned().unwrap();
            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(app_icon)
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .tooltip("veilanon")
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                        "status_online" => {
                            let _ = app.emit("tray:set-presence", "online");
                        }
                        "status_away" => {
                            let _ = app.emit("tray:set-presence", "away");
                        }
                        "status_dnd" => {
                            let _ = app.emit("tray:set-presence", "dnd");
                        }
                        "status_invisible" => {
                            let _ = app.emit("tray:set-presence", "invisible");
                        }
                        "toggle_mute" => {
                            let _ = app.emit("tray:toggle-mute", ());
                        }
                        "toggle_deafen" => {
                            let _ = app.emit("tray:toggle-deafen", ());
                        }
                        "leave_voice" => {
                            let _ = app.emit("tray:leave-voice", ());
                        }
                        "settings" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                            let _ = app.emit("tray:open-settings", "account");
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let is_visible = window.is_visible().unwrap_or(false);
                            let is_minimized = window.is_minimized().unwrap_or(false);
                            if !is_visible || is_minimized {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            } else {
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        // ── IPC Commands (narrow surface — no raw key/plaintext exposure) ─────
        .invoke_handler(tauri::generate_handler![
            // Auth
            auth::create_identity,
            auth::login_with_credentials,
            auth::load_identity,
            auth::try_auto_unlock,
            auth::set_auto_unlock,
            auth::has_auto_unlock,
            auth::recover_identity,
            auth::get_identity_hint,
            auth::verify_passphrase,
            auth::sign_out,
            auth::get_recovery_code,
            auth::verify_recovery_code,
            auth::get_device_info,
            auth::list_sessions,
            auth::revoke_session,
            auth::update_profile,
            auth::check_username_available,
            auth::set_avatar,
            auth::set_banner,
            auth::get_avatar,
            auth::reset_identity,
            // Crypto
            crypto_cmd::generate_keypair,
            crypto_cmd::sign_message,
            crypto_cmd::verify_signature,
            crypto_cmd::get_public_key,
            crypto_cmd::fingerprint,
            // Messages
            messages::send_message,
            messages::load_messages,
            messages::sync_messages,
            messages::edit_message,
            messages::delete_message,
            messages::clear_channel_messages,
            messages::add_reaction,
            messages::remove_reaction,
            messages::pin_message,
            messages::unpin_message,
            messages::mark_as_read,
            messages::get_pinned_messages,
            messages::search_messages,
            // Media
            media::join_voice_channel,
            media::leave_voice_channel,
            media::get_livekit_token,
            media::start_screen_share,
            media::stop_screen_share,
            media::set_audio_device,
            media::set_video_device,
            media::toggle_mute,
            media::toggle_camera,
            media::broadcast_voice_state,
            media::read_image_as_base64,
            // Files
            files::upload_file,
            files::upload_bytes,
            files::download_file,
            files::delete_file,
            files::get_file_info,
            files::get_file_data_url,
            files::write_text_file_user,
            // Settings
            settings::get_settings,
            settings::update_settings,
            settings::get_about_info,
            settings::set_startup_behavior,
            settings::export_data,
            settings::import_data,
            settings::clear_local_data,
            // Spaces
            spaces::spaces_list,
            spaces::spaces_create,
            spaces::spaces_update,
            spaces::spaces_transfer_ownership,
            spaces::spaces_set_banner,
            spaces::spaces_set_icon,
            spaces::spaces_set_custom_link,
            spaces::spaces_delete,
            spaces::spaces_leave,
            spaces::channels_list,
            spaces::channels_create,
            spaces::channels_update,
            spaces::channels_delete,
            spaces::roles_list,
            spaces::roles_create,
            spaces::roles_update,
            spaces::roles_delete,
            spaces::roles_reorder,
            spaces::channels_update_overrides,
            spaces::channels_get_overrides,
            spaces::invites_create,
            spaces::invites_redeem,
            spaces::members_list,
            spaces::members_update,
            spaces::spaces_kick_member,
            spaces::spaces_ban_member,
            spaces::spaces_unban_member,
            spaces::spaces_timeout_member,
            spaces::spaces_bans_list,
            spaces::spaces_search_public,
            spaces::spaces_join_public,
            // Social
            social::friends_add,
            social::friends_accept,
            social::friends_reject,
            social::friends_cancel,
            social::friends_remove,
            social::friends_block,
            social::friends_unblock,
            social::friends_list,
            social::dm_open,
            social::dm_list,
            social::group_dm_create,
            social::presence_update,
            social::typing_set,
            social::get_user_profile,
            social::resolve_username,
            social::get_mutual_spaces,
            social::get_mutual_friends,
            // Diagnostics
            logging::log_client_error,
            logging::get_diagnostics,
            logging::get_log_directory,
            logging::open_log_folder,
            // GIF search
            gifs::gif_search,
            gifs::gif_trending,
            // Local AI (Ollama)
            local_ai::local_ai_chat,
            local_ai::local_ai_status,
            // MLS group E2EE
            mls::mls_init_channel,
            mls::mls_create_key_package,
            mls::mls_add_member,
            mls::mls_consume_welcome,
            mls::mls_call_key,
            // Discord bridge (webhook)
            discord::discord_set_webhook,
            discord::discord_clear_webhook,
            discord::discord_get_webhook,
            // In-app Updater
            updater::check_for_updates,
            updater::download_and_install_update,
            // Privacy Tools & Diagnostics
            privacy_tools::check_tor_status,
            privacy_tools::check_ip_leak,
            privacy_tools::check_doh_status,
            privacy_tools::check_multi_doh_status,
            privacy_tools::check_password_pwned,
            privacy_tools::scan_urlhaus,
            privacy_tools::fetch_link_preview,
            privacy_tools::generate_qr_svg,
            privacy_tools::get_network_asn_info,
            privacy_tools::generate_privacy_avatar,
            privacy_tools::detect_clock_skew,
            privacy_tools::test_proxy_connection,
            privacy_tools::detect_local_tor_services,
            privacy_tools::detect_system_vpn_services,
            privacy_tools::validate_wireguard_profile,
            privacy_tools::get_privacy_endpoints_and_relays,
            // Media Scrape
            media_scrape::scrape_url,
        ])
        .run(tauri::generate_context!())
        .expect("veilanon: fatal error while running application");
}

/// Initialize tracing: console output plus a rolling daily file log in the
/// app data directory. No PII, keys, tokens or plaintext content is logged.
fn init_logging() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "veilanon=debug,tauri=warn,tokio=warn".into());
    let log_dir = default_log_dir();
    // Rolling daily file appender producing `veilanon.YYYY-MM-DD.log`; the file
    // layer must be ANSI-free so escape codes never corrupt the log file.
    if std::fs::create_dir_all(&log_dir).is_ok() {
        if let Ok(file_appender) = tracing_appender::rolling::RollingFileAppender::builder()
            .rotation(tracing_appender::rolling::Rotation::DAILY)
            .filename_prefix("veilanon")
            .filename_suffix("log")
            .build(&log_dir)
        {
            let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
            let file_layer = tracing_subscriber::fmt::layer()
                .with_writer(file_writer)
                .with_target(true)
                .with_ansi(false)
                .compact();
            let console_layer = tracing_subscriber::fmt::layer().with_target(false).compact();
            tracing_subscriber::registry()
                .with(console_layer)
                .with(file_layer)
                .with(env_filter)
                .init();
            // Keep the non-blocking writer alive for the whole process lifetime.
            std::mem::forget(guard);
            info!(
                "veilanon logging initialized — log file: {}/veilanon.YYYY-MM-DD.log",
                log_dir.display()
            );
            return;
        }
    }
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .compact()
        .init();
}

pub(crate) fn default_log_dir() -> std::path::PathBuf {
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return std::path::PathBuf::from(appdata)
                .join("com.aegissoft.veilanon")
                .join("logs");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return std::path::PathBuf::from(home)
            .join(".config")
            .join("veilanon")
            .join("logs");
    }
    std::path::PathBuf::from("logs")
}
