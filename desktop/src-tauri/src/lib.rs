use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{
    tray::{MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, PhysicalPosition, Rect, WindowEvent,
};

/// Состояние одной сессии (одного инстанса uxplay), из его вывода.
#[derive(Default)]
struct SessionState {
    slot: i32,
    sockets: i32,
    name: String,
    model: String,
}

/// Читает вывод рефлектора: ловит момент готовности анонса / ошибку.
fn spawn_reflector_reader<R: std::io::Read + Send + 'static>(app: AppHandle, reader: R) {
    thread::spawn(move || {
        for line in BufReader::new(reader).lines() {
            let Ok(line) = line else { break };
            if line.contains("Анонс активен") {
                let _ = app.emit("status", "ready");
            } else if line.contains("ОШИБКА") {
                let _ = app.emit("status", "error");
            }
        }
    });
}

/// Читает поток (stdout/stderr) uxplay построчно и шлёт события в UI.
fn spawn_log_reader<R: std::io::Read + Send + 'static>(
    app: AppHandle,
    sess: Arc<Mutex<SessionState>>,
    reader: R,
) {
    thread::spawn(move || {
        for line in BufReader::new(reader).lines() {
            let Ok(line) = line else { break };
            handle_log_line(&app, &sess, &line);
        }
    });
}

/// Парсит одну строку лога uxplay: готовность / подключение / закрытие сокетов.
fn handle_log_line(app: &AppHandle, sess: &Mutex<SessionState>, line: &str) {
    // uxplay поднял серверные сокеты — приёмник готов принимать
    if line.contains("Initialized server socket") {
        let _ = app.emit("status", "ready");
        return;
    }
    // "connection request from NAME (MODEL) with deviceID = ID"
    if let Some(idx) = line.find("connection request from ") {
        let rest = &line[idx + "connection request from ".len()..];
        if let Some(end) = rest.find(" with deviceID") {
            let nm = rest[..end].trim();
            let (name, model) = match nm.rfind(" (") {
                Some(p) => (
                    nm[..p].to_string(),
                    nm[p + 2..].trim_end_matches(')').to_string(),
                ),
                None => (nm.to_string(), String::new()),
            };
            let slot = {
                let mut s = match sess.lock() {
                    Ok(s) => s,
                    Err(_) => return,
                };
                s.name = name.clone();
                s.model = model.clone();
                s.slot
            };
            let _ = app.emit("status", "ready");
            let _ = app.emit(
                "device",
                serde_json::json!({ "slot": slot, "connected": true, "name": name, "model": model }),
            );
        }
        return;
    }
    // "Accepted IPv4 client on socket N" — новый сокет
    if line.contains(" client on socket") && line.contains("Accepted ") {
        if let Ok(mut s) = sess.lock() {
            s.sockets += 1;
        }
        return;
    }
    // "Connection closed on socket N" — сокет закрыт; 0 сокетов = устройство ушло
    if line.contains("Connection closed on socket") {
        let slot = {
            let mut s = match sess.lock() {
                Ok(s) => s,
                Err(_) => return,
            };
            s.sockets = (s.sockets - 1).max(0);
            if s.sockets != 0 {
                return;
            }
            s.name.clear();
            s.model.clear();
            s.slot
        };
        let _ = app.emit("device", serde_json::json!({ "slot": slot, "connected": false }));
    }
}

/// Запущенные процессы приёмника: N инстансов uxplay + mDNS-рефлектор.
#[derive(Default)]
struct Procs {
    children: Vec<Child>,
    reflector: Option<Child>,
}
#[derive(Default)]
struct Receiver(Mutex<Procs>);

/// Сколько одновременных сессий (инстансов uxplay) поднимать.
const SESSIONS: usize = 2;

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
    if !p.children.is_empty() {
        return Ok("already running".into());
    }
    let home = kagami_home();
    // GStreamer-DLL должны быть в PATH, иначе uxplay не поднимет пайплайн.
    let path = format!(
        "{};{}",
        gst_bin(),
        std::env::var("PATH").unwrap_or_default()
    );
    // N инстансов: разные имена, второй+ с уникальным MAC (-m), чтобы не конфликтовали.
    for slot in 0..SESSIONS {
        let iname = if slot == 0 {
            name.clone()
        } else {
            format!("{name} {}", slot + 1)
        };
        let mut cmd = Command::new(format!("{home}/build/uxplay.exe"));
        cmd.args(["-n", &iname, "-nohold", "-vsync", "no", "-fps", "60"]);
        if slot > 0 {
            cmd.arg("-m");
        }
        let mut child = cmd
            .current_dir(&home)
            .env("PATH", &path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("uxplay #{} не запустился: {e}", slot + 1))?;

        let sess = Arc::new(Mutex::new(SessionState {
            slot: slot as i32,
            ..Default::default()
        }));
        if let Some(out) = child.stdout.take() {
            spawn_log_reader(app.clone(), sess.clone(), out);
        }
        if let Some(err) = child.stderr.take() {
            spawn_log_reader(app.clone(), sess.clone(), err);
        }
        p.children.push(child);
    }
    drop(p);
    let _ = app.emit("status", "starting");

    // Рефлектор — после паузы, чтобы uxplay успел анонсировать сервисы.
    let app2 = app.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(3));
        let st = app2.state::<Receiver>();
        let Ok(mut p) = st.0.lock() else { return };
        if p.children.is_empty() {
            return; // приёмник уже остановили
        }
        let spawned = Command::new("py")
            .args(["-3.13", "-u", "reflector.py"])
            .current_dir(kagami_home())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        match spawned {
            Ok(mut c) => {
                if let Some(out) = c.stdout.take() {
                    spawn_reflector_reader(app2.clone(), out);
                }
                if let Some(err) = c.stderr.take() {
                    spawn_reflector_reader(app2.clone(), err);
                }
                p.reflector = Some(c);
            }
            Err(e) => {
                eprintln!("[kagami] reflector не запустился: {e}");
                let _ = app2.emit("status", "error");
            }
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
    if p.children.is_empty() {
        return Ok("not running".into());
    }
    for mut c in p.children.drain(..) {
        let _ = c.kill();
    }
    Ok("stopped".into())
}

#[tauri::command]
fn receiver_status(app: AppHandle) -> bool {
    app.state::<Receiver>()
        .0
        .lock()
        .map(|p| !p.children.is_empty())
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
            for mut c in p.children.drain(..) {
                let _ = c.kill();
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
