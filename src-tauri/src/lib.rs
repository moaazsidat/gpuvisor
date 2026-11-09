mod gpu;
mod settings;
mod tray;

use gpu::GpuMonitor;
use settings::Settings;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItem, MenuItemBuilder, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, WebviewWindowBuilder, WebviewUrl,
};

#[tauri::command]
fn get_settings(state: tauri::State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn save_settings(
    new_settings: Settings,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    new_settings.save(&app_handle)?;
    *state.settings.lock().unwrap() = new_settings;
    Ok(())
}

#[tauri::command]
fn set_pinned(pinned: bool, state: tauri::State<'_, AppState>) {
    state.pinned.store(pinned, Ordering::Relaxed);
}

#[tauri::command]
fn get_pinned(state: tauri::State<'_, AppState>) -> bool {
    state.pinned.load(Ordering::Relaxed)
}

struct AppState {
    settings: Mutex<Settings>,
    pinned: AtomicBool,
    stats_item: MenuItem<tauri::Wry>,
}

const PANEL_WIDTH: f64 = 420.0;
const PANEL_HEIGHT: f64 = 620.0;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let gpu_monitor = Arc::new(
        GpuMonitor::new().expect("Failed to initialize NVML. Is the NVIDIA driver installed?"),
    );

    tauri::Builder::default()
        .setup(move |app| {
            let settings = Settings::load(&app.handle());

            let stats_item = MenuItemBuilder::with_id("stats", "GPU: --")
                .enabled(false)
                .build(app)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let show = MenuItemBuilder::with_id("show", "Open GPUvisor").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
            let menu = MenuBuilder::new(app)
                .items(&[&stats_item, &separator, &show, &quit])
                .build()?;

            app.manage(AppState {
                settings: Mutex::new(settings.clone()),
                pinned: AtomicBool::new(false),
                stats_item,
            });

            let icon = Image::from_bytes(include_bytes!("../icons/32x32.png"))
                .expect("failed to load tray icon");

            let _tray = TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .tooltip("GPUvisor")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "quit" => std::process::exit(0),
                    "show" => toggle_panel(app),
                    _ => {}
                })
                .build(app)?;

            let app_handle = app.handle().clone();
            let monitor = gpu_monitor.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let interval = {
                        let state = app_handle.state::<AppState>();
                        let val = state.settings.lock().unwrap().poll_interval_ms;
                        val
                    };

                    match monitor.poll() {
                        Ok(data) => {
                            let state = app_handle.state::<AppState>();
                            let settings = state.settings.lock().unwrap().clone();
                            let label = tray::format_tray_title(&data, &settings);
                            state.stats_item.set_text(&label).ok();

                            if let Some(tray) = app_handle.tray_by_id("main-tray") {
                                tray.set_tooltip(Some(&label)).ok();
                            }

                            app_handle.emit("gpu-data", &data).ok();
                        }
                        Err(e) => {
                            eprintln!("GPU poll error: {}", e);
                        }
                    }

                    tokio::time::sleep(tokio::time::Duration::from_millis(interval)).await;
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_settings, save_settings, set_pinned, get_pinned])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn toggle_panel(app_handle: &tauri::AppHandle) {
    if let Some(window) = app_handle.get_webview_window("panel") {
        if window.is_visible().unwrap_or(false) {
            window.hide().ok();
        } else {
            position_panel(&window);
            window.show().ok();
            force_focus(&window);
        }
    } else {
        if let Ok(window) = WebviewWindowBuilder::new(
            app_handle,
            "panel",
            WebviewUrl::App("index.html".into()),
        )
        .title("GPUvisor")
        .inner_size(PANEL_WIDTH, PANEL_HEIGHT)
        .resizable(true)
        .decorations(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .visible(false)
        .build()
        {
            position_panel(&window);
            window.show().ok();

            let win = window.clone();
            let handle = app_handle.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::Focused(false) = event {
                    let pinned = handle.state::<AppState>().pinned.load(Ordering::Relaxed);
                    if !pinned {
                        win.hide().ok();
                    }
                }
            });

            force_focus(&window);
        }
    }
}

fn position_panel(window: &tauri::WebviewWindow) {
    if let Ok(Some(monitor)) = window.current_monitor() {
        let screen_width = monitor.size().width as f64;
        let scale = monitor.scale_factor();
        let x = screen_width / scale - PANEL_WIDTH - 12.0;
        let y = 32.0;
        window
            .set_position(tauri::PhysicalPosition::new(
                (x * scale) as i32,
                (y * scale) as i32,
            ))
            .ok();
    }
}

fn force_focus(window: &tauri::WebviewWindow) {
    let window = window.clone();
    let pid = std::process::id();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        window.set_focus().ok();
        let pid_str = pid.to_string();
        if let Ok(output) = std::process::Command::new("xdotool")
            .args(["search", "--pid", &pid_str])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(wid) = stdout.lines().last() {
                std::process::Command::new("xdotool")
                    .args(["windowactivate", "--sync", wid])
                    .output()
                    .ok();
            }
        }
    });
}
