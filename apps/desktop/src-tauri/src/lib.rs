use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    fs::OpenOptions,
    collections::HashMap,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
    thread,
    time::Duration,
};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, WindowEvent,
};
use tungstenite::{connect, Message};

const BACKEND_STATUS_EVENT: &str = "desktop://backend-status";
const REALTIME_EVENT: &str = "desktop://realtime-event";
const REALTIME_STATUS_EVENT: &str = "desktop://realtime-status";
const REALTIME_WS_URL: &str = "ws://127.0.0.1:8080/ws/events.json";
const REALTIME_RECONNECT_DELAY_MS: u64 = 3000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopStatus {
    tauri_available: bool,
    backend_running: bool,
    backend_pid: Option<u32>,
    togospace_root: Option<String>,
    app_version: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopRealtimeStatus {
    state: String,
    detail: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopHttpRequest {
    url: String,
    method: String,
    headers: Option<HashMap<String, String>>,
    body: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopHttpResponse {
    status: u16,
    content_type: Option<String>,
    headers: HashMap<String, String>,
    body: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct FrontendDebugEvent {
    session_id: String,
    run_id: String,
    hypothesis_id: String,
    location: String,
    msg: String,
    data: serde_json::Value,
    ts: u64,
}

#[derive(Clone)]
struct RealtimeBridgeHandle {
    stop: Arc<AtomicBool>,
}

static REALTIME_BRIDGE_STATE: OnceLock<Mutex<Option<RealtimeBridgeHandle>>> = OnceLock::new();

fn realtime_bridge_slot() -> &'static Mutex<Option<RealtimeBridgeHandle>> {
    REALTIME_BRIDGE_STATE.get_or_init(|| Mutex::new(None))
}

fn packaged_togospace_root(app: &AppHandle) -> Option<PathBuf> {
    let resource_root = app.path().resource_dir().ok()?;
    let candidate = resource_root.join("extensions/TogoSpace");
    is_togospace_root(&candidate).then_some(candidate)
}

fn workspace_parent_togospace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../")
}

fn candidate_togospace_roots(app: Option<&AppHandle>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(app) = app {
        if let Some(packaged_root) = packaged_togospace_root(app) {
            candidates.push(packaged_root);
        }
    }

    if let Ok(path) = env::var("TOGOSPACE_ROOT") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            candidates.push(PathBuf::from(trimmed));
        }
    }

    candidates.push(workspace_parent_togospace_root());
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../external/TogoSpace"));

    candidates
}

fn is_togospace_root(path: &Path) -> bool {
    path.join("scripts/start_backend.sh").exists() && path.join("src/backend_main.py").exists()
}

fn resolve_togospace_root(app: Option<&AppHandle>) -> Option<PathBuf> {
    candidate_togospace_roots(app)
        .into_iter()
        .find(|path| is_togospace_root(path))
        .and_then(|path| path.canonicalize().ok().or(Some(path)))
}

fn sibling_python_candidates(root: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(parent_1) = root.parent() {
        if let Some(parent_2) = parent_1.parent() {
            if let Some(parent_3) = parent_2.parent() {
                candidates.push(parent_3.join("TogoSpace/.venv/bin/python"));
                candidates.push(parent_3.join("TogoSpace/.venv/bin/python3"));
            }
        }
    }

    candidates
}

fn candidate_python_commands(root: &Path) -> Vec<String> {
    let mut commands = Vec::new();

    if let Ok(path) = env::var("TOGOSPACE_PYTHON") {
        commands.push(path);
    }

    let path_candidates = [
        root.join(".venv/bin/python"),
        root.join(".venv/bin/python3"),
        workspace_parent_togospace_root().join(".venv/bin/python"),
        workspace_parent_togospace_root().join(".venv/bin/python3"),
    ];

    for path in path_candidates
        .into_iter()
        .chain(sibling_python_candidates(root).into_iter())
    {
        if path.exists() {
            commands.push(path.to_string_lossy().to_string());
        }
    }

    commands.push("python3".to_string());
    commands.push("python".to_string());
    commands
}

fn backend_pid(storage_root: &Path) -> Option<u32> {
    let pid_file = backend_pid_file(storage_root);
    let content = fs::read_to_string(pid_file).ok()?;
    content.trim().parse::<u32>().ok()
}

fn backend_running(storage_root: &Path) -> bool {
    let Some(pid) = backend_pid(storage_root) else {
        return false;
    };

    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn desktop_status(app: &AppHandle) -> DesktopStatus {
    let togospace_root = resolve_togospace_root(Some(app));
    let storage_root = desktop_backend_storage_root(app).ok();
    let is_backend_running = storage_root
        .as_deref()
        .map(backend_running)
        .unwrap_or(false);
    let backend_pid = storage_root
        .as_deref()
        .and_then(|root| if is_backend_running { backend_pid(root) } else { None });

    DesktopStatus {
        tauri_available: true,
        backend_running: is_backend_running,
        backend_pid,
        togospace_root: togospace_root.map(|path| path.to_string_lossy().to_string()),
        app_version: app.package_info().version.to_string(),
    }
}

fn emit_backend_status(app: &AppHandle) {
    let _ = app.emit(BACKEND_STATUS_EVENT, desktop_status(app));
}

fn emit_realtime_status(app: &AppHandle, state: &str, detail: Option<String>) {
    let _ = app.emit(
        REALTIME_STATUS_EVENT,
        DesktopRealtimeStatus {
            state: state.to_string(),
            detail,
        },
    );
}

fn emit_realtime_payload(app: &AppHandle, payload: &str) {
    let _ = app.emit(REALTIME_EVENT, payload.to_string());
}

fn desktop_backend_storage_root(app: &AppHandle) -> Result<PathBuf, String> {
    if cfg!(debug_assertions) {
        return Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.desktop-dev-storage"));
    }

    app.path()
        .app_data_dir()
        .map(|path| path.join("togospace-backend"))
        .map_err(|error| format!("无法解析桌面端数据目录: {error}"))
}

fn backend_stdout_log(storage_root: &Path) -> PathBuf {
    storage_root.join("logs/desktop-backend-stdout.log")
}

fn backend_pid_file(storage_root: &Path) -> PathBuf {
    storage_root.join("run/backend.pid")
}

fn workspace_debug_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.dbg")
}

fn frontend_debug_log_path(session_id: &str) -> PathBuf {
    workspace_debug_dir().join(format!("trae-debug-log-{session_id}.ndjson"))
}

fn prepare_backend_dirs(storage_root: &Path) -> Result<(), String> {
    fs::create_dir_all(storage_root.join("logs"))
        .map_err(|error| format!("无法创建日志目录 {}: {error}", storage_root.join("logs").display()))?;
    fs::create_dir_all(storage_root.join("run"))
        .map_err(|error| format!("无法创建运行目录 {}: {error}", storage_root.join("run").display()))?;
    Ok(())
}

fn tail_log_excerpt(path: &Path) -> String {
    let Ok(content) = fs::read_to_string(path) else {
        return String::new();
    };

    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(20);
    lines[start..].join("\n")
}

fn spawn_backend_process(app: &AppHandle, root: &Path) -> Result<String, String> {
    let storage_root = desktop_backend_storage_root(app)?;
    prepare_backend_dirs(&storage_root)?;

    let log_path = backend_stdout_log(&storage_root);
    let python_commands = candidate_python_commands(root);
    let mut spawn_errors = Vec::new();

    for python_command in python_commands {
        let stdout = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|error| format!("无法打开日志文件 {}: {error}", log_path.display()))?;
        let stderr = stdout
            .try_clone()
            .map_err(|error| format!("无法复制日志文件句柄 {}: {error}", log_path.display()))?;

        let mut command = Command::new(&python_command);
        command
            .arg("backend_main.py")
            .current_dir(root.join("src"))
            .env("PYTHONUNBUFFERED", "1")
            .env("STORAGE_ROOT", &storage_root)
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));

        match command.spawn() {
            Ok(_child) => return Ok(python_command),
            Err(error) => spawn_errors.push(format!("{python_command}: {error}")),
        }
    }

    Err(format!(
        "未找到可用的 Python 解释器。请设置 TOGOSPACE_PYTHON，或为 TogoSpace 准备 .venv。\n{}",
        spawn_errors.join("\n")
    ))
}

fn stop_backend_process(storage_root: &Path) -> Result<(), String> {
    let Some(pid) = backend_pid(storage_root) else {
        return Ok(());
    };

    let status = Command::new("kill")
        .arg(pid.to_string())
        .status()
        .map_err(|error| format!("无法停止后端进程 {pid}: {error}"))?;

    if !status.success() {
        return Err(format!("停止后端进程失败，PID={pid}"));
    }

    for _ in 0..10 {
        if !backend_running(storage_root) {
            break;
        }
        std::thread::sleep(Duration::from_millis(300));
    }

    let _ = fs::remove_file(backend_pid_file(storage_root));
    Ok(())
}

fn escape_applescript(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn send_system_notification(title: &str, body: &str) {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display notification \"{}\" with title \"{}\"",
            escape_applescript(body),
            escape_applescript(title)
        );
        let _ = Command::new("osascript").arg("-e").arg(script).status();
    }

    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("notify-send").arg(title).arg(body).status();
    }
}

fn open_path_in_finder(path: &Path) -> Result<(), String> {
    if path.is_dir() {
        Command::new("open")
            .arg(path)
            .status()
            .map_err(|error| format!("无法在 Finder 中打开目录 {}: {error}", path.display()))?;
        return Ok(());
    }

    if path.exists() {
        Command::new("open")
            .arg("-R")
            .arg(path)
            .status()
            .map_err(|error| format!("无法在 Finder 中定位文件 {}: {error}", path.display()))?;
        return Ok(());
    }

    let parent = path
        .parent()
        .filter(|candidate| candidate.exists())
        .ok_or_else(|| format!("路径不存在，且无法找到可打开的父目录: {}", path.display()))?;

    Command::new("open")
        .arg(parent)
        .status()
        .map_err(|error| format!("无法在 Finder 中打开目录 {}: {error}", parent.display()))?;
    Ok(())
}

fn append_frontend_debug_log(event: &FrontendDebugEvent) -> Result<(), String> {
    let debug_dir = workspace_debug_dir();
    fs::create_dir_all(&debug_dir)
        .map_err(|error| format!("无法创建调试目录 {}: {error}", debug_dir.display()))?;

    let log_path = frontend_debug_log_path(&event.session_id);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|error| format!("无法写入调试日志 {}: {error}", log_path.display()))?;

    let line = serde_json::to_string(event)
        .map_err(|error| format!("无法序列化调试日志: {error}"))?;
    use std::io::Write as _;
    writeln!(file, "{line}")
        .map_err(|error| format!("无法追加调试日志 {}: {error}", log_path.display()))?;
    Ok(())
}

fn append_debug_probe(
    hypothesis_id: &str,
    location: &str,
    msg: &str,
    data: serde_json::Value,
) {
    let _ = append_frontend_debug_log(&FrontendDebugEvent {
        session_id: "drag-path-missing".to_string(),
        run_id: "pre-fix".to_string(),
        hypothesis_id: hypothesis_id.to_string(),
        location: location.to_string(),
        msg: format!("[DEBUG] {msg}"),
        data,
        ts: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or(0),
    });
}

fn append_tmp_probe(line: &str) {
    let path = PathBuf::from("/tmp/togoclient-drag-probe.txt");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        use std::io::Write as _;
        let _ = writeln!(file, "{line}");
    }
}

fn start_realtime_bridge_thread(app: AppHandle, token: Option<String>, stop: Arc<AtomicBool>) {
    thread::spawn(move || {
        let ws_url = env::var("TOGOCLIENT_REALTIME_WS_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| REALTIME_WS_URL.to_string());
        if let Err(error) = url::Url::parse(&ws_url) {
            emit_realtime_status(&app, "disconnected", Some(format!("实时地址无效: {error}")));
            return;
        }
        let mut first_attempt = true;

        loop {
            if stop.load(Ordering::Relaxed) {
                emit_realtime_status(&app, "disconnected", None);
                break;
            }

            emit_realtime_status(
                &app,
                if first_attempt { "connecting" } else { "reconnecting" },
                None,
            );

            match connect(ws_url.as_str()) {
                Ok((mut socket, _)) => {
                    if let Some(token) = token.clone().filter(|value| !value.trim().is_empty()) {
                        let auth_payload =
                            serde_json::json!({ "type": "auth", "token": token }).to_string();
                        if let Err(error) = socket.send(Message::Text(auth_payload.into())) {
                            emit_realtime_status(
                                &app,
                                "reconnecting",
                                Some(format!("实时鉴权消息发送失败: {error}")),
                            );
                            first_attempt = false;
                            thread::sleep(Duration::from_millis(REALTIME_RECONNECT_DELAY_MS));
                            continue;
                        }
                    }

                    emit_realtime_status(&app, "connected", None);

                    loop {
                        if stop.load(Ordering::Relaxed) {
                            let _ = socket.close(None);
                            emit_realtime_status(&app, "disconnected", None);
                            break;
                        }

                        match socket.read() {
                            Ok(Message::Text(text)) => emit_realtime_payload(&app, text.as_ref()),
                            Ok(Message::Ping(payload)) => {
                                let _ = socket.send(Message::Pong(payload));
                            }
                            Ok(Message::Close(frame)) => {
                                emit_realtime_status(
                                    &app,
                                    "reconnecting",
                                    frame.map(|item| format!("实时连接已关闭: {}", item.reason)),
                                );
                                break;
                            }
                            Ok(_) => {}
                            Err(error) => {
                                emit_realtime_status(
                                    &app,
                                    "reconnecting",
                                    Some(format!("实时连接已断开: {error}")),
                                );
                                break;
                            }
                        }
                    }
                }
                Err(error) => {
                    emit_realtime_status(
                        &app,
                        "reconnecting",
                        Some(format!("实时连接失败: {error}")),
                    );
                }
            }

            first_attempt = false;

            for _ in 0..(REALTIME_RECONNECT_DELAY_MS / 200) {
                if stop.load(Ordering::Relaxed) {
                    emit_realtime_status(&app, "disconnected", None);
                    break;
                }
                thread::sleep(Duration::from_millis(200));
            }
        }

        if let Ok(mut guard) = realtime_bridge_slot().lock() {
            if guard
                .as_ref()
                .map(|handle| Arc::ptr_eq(&handle.stop, &stop))
                .unwrap_or(false)
            {
                *guard = None;
            }
        }
    });
}

fn reveal_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open_item = MenuItem::with_id(app, "open", "显示窗口", true, None::<&str>)?;
    let start_item = MenuItem::with_id(app, "start_backend", "启动 TogoSpace 后端", true, None::<&str>)?;
    let stop_item = MenuItem::with_id(app, "stop_backend", "停止 TogoSpace 后端", true, None::<&str>)?;
    let open_root_item = MenuItem::with_id(app, "open_root", "打开 TogoSpace 目录", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[&open_item, &start_item, &stop_item, &open_root_item, &separator, &quit_item],
    )?;

    let mut tray_builder = TrayIconBuilder::new().menu(&menu).show_menu_on_left_click(false);
    if let Some(icon) = app.default_window_icon() {
        tray_builder = tray_builder.icon(icon.clone());
    }

    tray_builder
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => reveal_main_window(app),
            "start_backend" => {
                let _ = start_togospace_backend(app.clone());
            }
            "stop_backend" => {
                let _ = stop_togospace_backend(app.clone());
            }
            "open_root" => {
                let _ = open_togospace_root(app.clone());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

#[tauri::command]
fn get_desktop_status(app: AppHandle) -> DesktopStatus {
    desktop_status(&app)
}

#[tauri::command]
fn start_togospace_backend(app: AppHandle) -> Result<DesktopStatus, String> {
    let root = resolve_togospace_root(Some(&app)).ok_or("未找到 TogoSpace 根目录".to_string())?;
    let storage_root = desktop_backend_storage_root(&app)?;

    if backend_running(&storage_root) {
        let status = desktop_status(&app);
        emit_backend_status(&app);
        return Ok(status);
    }

    let python_command = spawn_backend_process(&app, &root)?;

    for _ in 0..20 {
        if backend_running(&storage_root) {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    let status = desktop_status(&app);
    emit_backend_status(&app);

    if status.backend_running {
        send_system_notification("TogoClient", "TogoSpace 后端已启动");
        Ok(status)
    } else {
        let log_path = backend_stdout_log(&storage_root);
        let log_excerpt = tail_log_excerpt(&log_path);
        Err(format!(
            "已尝试使用 `{python_command}` 启动 TogoSpace，但未检测到运行中的后端进程。\n日志文件: {}\n{}",
            log_path.display(),
            log_excerpt
        ))
    }
}

#[tauri::command]
fn stop_togospace_backend(app: AppHandle) -> Result<DesktopStatus, String> {
    let storage_root = desktop_backend_storage_root(&app)?;

    if backend_running(&storage_root) {
        stop_backend_process(&storage_root)?;
    }

    let status = desktop_status(&app);
    emit_backend_status(&app);
    send_system_notification("TogoClient", "TogoSpace 后端已停止");
    Ok(status)
}

#[tauri::command]
fn open_togospace_root(app: AppHandle) -> Result<(), String> {
    let root = resolve_togospace_root(Some(&app)).ok_or("未找到 TogoSpace 根目录".to_string())?;

    open_path_in_finder(&root)?;

    emit_backend_status(&app);
    Ok(())
}

#[tauri::command]
fn reveal_path_in_finder(path: String) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("路径不能为空".to_string());
    }

    open_path_in_finder(Path::new(trimmed))
}

#[tauri::command]
fn append_desktop_debug_event(event: FrontendDebugEvent) -> Result<(), String> {
    append_tmp_probe(&format!(
        "append_desktop_debug_event hypothesis={} location={} msg={}",
        event.hypothesis_id, event.location, event.msg
    ));
    append_debug_probe(
        "D",
        "src-tauri/src/lib.rs",
        "append_desktop_debug_event invoked",
        serde_json::json!({
            "location": event.location,
            "hypothesisId": event.hypothesis_id,
            "message": event.msg,
        }),
    );
    append_frontend_debug_log(&event)
}

#[tauri::command]
fn desktop_http_request(request: DesktopHttpRequest) -> Result<DesktopHttpResponse, String> {
    let parsed_url = url::Url::parse(&request.url)
        .map_err(|error| format!("无效请求地址 {}: {error}", request.url))?;

    match parsed_url.scheme() {
        "http" | "https" => {}
        other => return Err(format!("不支持的请求协议: {other}")),
    }

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(3))
        .timeout_read(Duration::from_secs(15))
        .timeout_write(Duration::from_secs(15))
        .build();

    let method = request.method.trim().to_uppercase();
    let mut req = agent.request_url(&method, &parsed_url);

    if let Some(headers) = request.headers.as_ref() {
        for (key, value) in headers {
            req = req.set(key, value);
        }
    }

    let response = match request.body {
        Some(body) => match req.send_string(&body) {
            Ok(response) => response,
            Err(ureq::Error::Status(_, response)) => response,
            Err(ureq::Error::Transport(error)) => {
                return Err(format!("请求转发失败: {error}"));
            }
        },
        None => match req.call() {
            Ok(response) => response,
            Err(ureq::Error::Status(_, response)) => response,
            Err(ureq::Error::Transport(error)) => {
                return Err(format!("请求转发失败: {error}"));
            }
        },
    };

    let status = response.status();
    let content_type = response.header("content-type").map(ToString::to_string);
    let mut headers = HashMap::new();
    for name in response.headers_names() {
        if let Some(value) = response.header(&name) {
            headers.insert(name.to_ascii_lowercase(), value.to_string());
        }
    }

    let body = response.into_string().unwrap_or_default();

    Ok(DesktopHttpResponse {
        status,
        content_type,
        headers,
        body,
    })
}

#[tauri::command]
fn start_desktop_realtime_bridge(app: AppHandle, token: Option<String>) -> Result<(), String> {
    let mut guard = realtime_bridge_slot()
        .lock()
        .map_err(|_| "无法获取实时桥状态".to_string())?;

    if let Some(handle) = guard.as_ref() {
        if !handle.stop.load(Ordering::Relaxed) {
            return Ok(());
        }
    }

    let stop = Arc::new(AtomicBool::new(false));
    *guard = Some(RealtimeBridgeHandle { stop: stop.clone() });
    drop(guard);

    start_realtime_bridge_thread(app, token, stop);
    Ok(())
}

#[tauri::command]
fn stop_desktop_realtime_bridge() -> Result<(), String> {
    let mut guard = realtime_bridge_slot()
        .lock()
        .map_err(|_| "无法获取实时桥状态".to_string())?;

    if let Some(handle) = guard.take() {
        handle.stop.store(true, Ordering::Relaxed);
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            append_tmp_probe(&format!(
                "native setup reached version={}",
                app.package_info().version
            ));
            append_debug_probe(
                "D",
                "src-tauri/src/lib.rs",
                "native setup reached",
                serde_json::json!({
                    "appVersion": app.package_info().version.to_string(),
                }),
            );
            build_tray(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_desktop_status,
            open_togospace_root,
            reveal_path_in_finder,
            append_desktop_debug_event,
            start_togospace_backend,
            stop_togospace_backend,
            desktop_http_request,
            start_desktop_realtime_bridge,
            stop_desktop_realtime_bridge
        ])
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                send_system_notification("TogoClient", "窗口已最小化到托盘");
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
