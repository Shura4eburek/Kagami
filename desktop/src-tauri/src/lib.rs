use std::process::{Child, Command};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tauri::{
    tray::{MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, PhysicalPosition, Rect, WindowEvent,
};

/// Запущенные процессы приёмника: uxplay + mDNS-рефлектор.
#[derive(Default)]
struct Procs {
    uxplay: Option<Child>,
    reflector: Option<Child>,
}
#[derive(Default)]
struct Receiver(Mutex<Procs>);

/// Корень репо (где build/uxplay.exe и reflector.py). Dev cwd = desktop/src-tauri.
fn kagami_home() -> String {
    std::env::var("KAGAMI_HOME").unwrap_or_else(|_| "../..".into())
}
/// Каталог с DLL GStreamer (MSYS2 UCRT64).
fn gst_bin() -> String {
    std::env::var("KAGAMI_GST_BIN").unwrap_or_else(|_| "C:\\msys64\\ucrt64\\bin".into())
}

#[tauri::command]
fn start_receiver(app: AppHandle, name: String) -> Result<String, String> {
    let state = app.state::<Receiver>();
    let mut p = state.0.lock().map_err(|e| e.to_string())?;
    if p.uxplay.is_some() {
        return Ok("already running".into());
    }
    let home = kagami_home();
    // GStreamer-DLL должны быть в PATH, иначе uxplay не поднимет пайплайн.
    let path = format!(
        "{};{}",
        gst_bin(),
        std::env::var("PATH").unwrap_or_default()
    );
    let uxplay = Command::new(format!("{home}/build/uxplay.exe"))
        .args(["-n", &name, "-nohold", "-vsync", "no", "-fps", "60"])
        .current_dir(&home)
        .env("PATH", &path)
        .spawn()
        .map_err(|e| format!("uxplay не запустился: {e}"))?;
    p.uxplay = Some(uxplay);
    drop(p);

    // Рефлектор — после паузы, чтобы uxplay успел анонсировать сервисы.
    let app2 = app.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(5));
        let st = app2.state::<Receiver>();
        let Ok(mut p) = st.0.lock() else { return };
        if p.uxplay.is_none() {
            return; // приёмник уже остановили
        }
        match Command::new("py")
            .args(["-3.13", "-u", "reflector.py"])
            .current_dir(kagami_home())
            .spawn()
        {
            Ok(c) => p.reflector = Some(c),
            Err(e) => eprintln!("[kagami] reflector не запустился: {e}"),
        }
    });

    Ok("started".into())
}

#[tauri::command]
fn stop_receiver(app: AppHandle) -> Result<String, String> {
    let state = app.state::<Receiver>();
    let mut p = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(mut r) = p.reflector.take() {
        let _ = r.kill();
    }
    if let Some(mut u) = p.uxplay.take() {
        let _ = u.kill();
        return Ok("stopped".into());
    }
    Ok("not running".into())
}

#[tauri::command]
fn receiver_status(app: AppHandle) -> bool {
    app.state::<Receiver>()
        .0
        .lock()
        .map(|p| p.uxplay.is_some())
        .unwrap_or(false)
}

// ---- команды для кастомного трей-флайаута ----
#[tauri::command]
fn open_main(app: AppHandle) {
    hide_flyout(&app);
    show_main(&app);
}

#[tauri::command]
fn open_settings(app: AppHandle) {
    hide_flyout(&app);
    show_main(&app);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.emit("nav", "settings");
    }
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    // погасим дочерние процессы перед выходом
    if let Some(state) = app.try_state::<Receiver>() {
        if let Ok(mut p) = state.0.lock() {
            if let Some(mut r) = p.reflector.take() {
                let _ = r.kill();
            }
            if let Some(mut u) = p.uxplay.take() {
                let _ = u.kill();
            }
        }
    }
    app.exit(0);
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn hide_flyout(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("tray") {
        let _ = w.hide();
    }
}

/// Показать флайаут, прижав его к иконке в трее (rect из события клика).
fn show_flyout(app: &AppHandle, icon: Rect) {
    let Some(fly) = app.get_webview_window("tray") else {
        return;
    };
    let scale = fly.scale_factor().unwrap_or(1.0);
    let gap = (8.0 * scale) as i32;
    let (w, h) = match fly.outer_size() {
        Ok(s) => (s.width as i32, s.height as i32),
        Err(_) => ((272.0 * scale) as i32, (292.0 * scale) as i32),
    };

    let ip = icon.position.to_physical::<i32>(scale);
    let is = icon.size.to_physical::<i32>(scale);

    let mut x = ip.x + is.width - w;
    let mut y = ip.y - h - gap;

    if let Ok(Some(mon)) = fly.current_monitor() {
        let ms = mon.size();
        let max_x = ms.width as i32 - w - (6.0 * scale) as i32;
        x = x.clamp((6.0 * scale) as i32, max_x.max(0));
        y = y.max((6.0 * scale) as i32);
    }

    let _ = fly.set_position(PhysicalPosition::new(x, y));
    let _ = fly.show();
    let _ = fly.set_focus();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Receiver::default())
        .invoke_handler(tauri::generate_handler![
            start_receiver,
            stop_receiver,
            receiver_status,
            open_main,
            open_settings,
            quit_app
        ])
        .setup(|app| {
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Kagami")
                .on_tray_icon_event(|tray, event| {
                    // Флайаут по отпусканию любой кнопки (Windows не шлёт Left-клик стабильно).
                    if let TrayIconEvent::Click {
                        button_state: MouseButtonState::Up,
                        rect,
                        ..
                    } = event
                    {
                        show_flyout(tray.app_handle(), rect);
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } if window.label() == "main" => {
                let _ = window.hide();
                api.prevent_close();
            }
            WindowEvent::Focused(false) if window.label() == "tray" => {
                let _ = window.hide();
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
