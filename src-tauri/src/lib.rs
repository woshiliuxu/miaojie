//! 秒接 —— 只有一件事：匹配成功时自动点“接受”。
//!
//! 数据来源全部是客户端本地 REST 接口（LCU），不注入、不读内存、不模拟键鼠。

mod lcu;

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, State};

const MAX_LOGS: usize = 60;
const POLL_MS: u64 = 800;
/// 胶囊窗口只在第一次进入胶囊模式时自动摆位，之后保留用户拖动过的位置
static CAPSULE_PLACED: AtomicBool = AtomicBool::new(false);
/// 召唤师信息（名字 / 等级 / 头像）的刷新间隔
const PROFILE_REFRESH_SECS: u64 = 60;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LogItem {
    time: String,
    text: String,
    kind: String,
}

/// 当前登录的召唤师。头像已经内联成 data URL。
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct Profile {
    name: String,
    level: i64,
    icon: String,
    /// 只用于判断头像是否变化，不必发给前端
    #[serde(skip)]
    icon_id: i64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    connected: bool,
    phase: String,
    enabled: bool,
    delay_ms: u64,
    accepted_today: u32,
    port: Option<u16>,
    logs: Vec<LogItem>,
    client_path: Option<String>,
    profile: Option<Profile>,
}

#[derive(Default)]
struct Inner {
    connected: bool,
    phase: String,
    enabled: bool,
    delay_ms: u64,
    accepted_today: u32,
    day: String,
    port: Option<u16>,
    client_path: Option<String>,
    logs: VecDeque<LogItem>,
    profile: Option<Profile>,
}

type Shared = Arc<Mutex<Inner>>;

fn now_time() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}

fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

fn push_log(state: &Shared, kind: &str, text: &str) {
    let time = now_time();

    if let Ok(mut guard) = state.lock() {
        guard.logs.push_front(LogItem {
            time: time.clone(),
            text: text.to_string(),
            kind: kind.to_string(),
        });
        while guard.logs.len() > MAX_LOGS {
            guard.logs.pop_back();
        }
    }

    // 界面上不再显示日志，但仍然落盘：出问题直接看 %APPDATA%\miaojie\app.log
    if let Some(dir) = log_dir() {
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("app.log"))
        {
            let _ = writeln!(file, "{time} [{kind}] {text}");
        }
    }
}

fn log_dir() -> Option<std::path::PathBuf> {
    let base = std::env::var("APPDATA").ok()?;
    let dir = std::path::PathBuf::from(base).join("miaojie");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn snapshot_of(state: &Shared) -> Snapshot {
    let guard = match state.lock() {
        Ok(g) => g,
        Err(_) => return Snapshot {
            connected: false,
            phase: String::new(),
            enabled: true,
            delay_ms: 0,
            accepted_today: 0,
            port: None,
            logs: Vec::new(),
            client_path: None,
            profile: None,
        },
    };
    Snapshot {
        connected: guard.connected,
        phase: guard.phase.clone(),
        enabled: guard.enabled,
        delay_ms: guard.delay_ms,
        accepted_today: guard.accepted_today,
        port: guard.port,
        logs: guard.logs.iter().cloned().collect(),
        client_path: guard.client_path.clone(),
        profile: guard.profile.clone(),
    }
}

fn emit(app: &tauri::AppHandle, state: &Shared) {
    let _ = app.emit("snapshot", snapshot_of(state));
}

#[tauri::command]
fn get_snapshot(state: State<'_, Shared>) -> Snapshot {
    snapshot_of(&state)
}

#[tauri::command]
fn set_enabled(app: tauri::AppHandle, state: State<'_, Shared>, enabled: bool) {
    if let Ok(mut g) = state.lock() {
        g.enabled = enabled;
    }
    push_log(&state, "info", if enabled { "自动确认已开启" } else { "自动确认已关闭" });
    // 胶囊窗口要立刻看到开关变化，不能等下一轮轮询
    emit(&app, &state);
}

#[tauri::command]
fn set_delay(app: tauri::AppHandle, state: State<'_, Shared>, ms: u64) {
    if let Ok(mut g) = state.lock() {
        g.delay_ms = ms;
    }
    emit(&app, &state);
}

/// 主界面 ↔ 桌面胶囊。两者的显示/隐藏都在后端做，前端只报一个目标模式。
#[tauri::command]
fn set_window_mode(app: tauri::AppHandle, mode: String) {
    let main = app.get_webview_window("main");
    let capsule = app.get_webview_window("capsule");

    if mode == "capsule" {
        if let Some(c) = &capsule {
            if !CAPSULE_PLACED.swap(true, Ordering::Relaxed) {
                place_capsule(c);
            }
            let _ = c.show();
            let _ = c.set_focus();
        }
        if let Some(m) = &main {
            let _ = m.hide();
        }
    } else {
        if let Some(c) = &capsule {
            let _ = c.hide();
        }
        if let Some(m) = &main {
            let _ = m.unminimize();
            let _ = m.show();
            let _ = m.set_focus();
        }
    }
}

/// 摆到主屏右下角，避开任务栏与屏幕边缘。
fn place_capsule(win: &tauri::WebviewWindow) {
    let Ok(Some(monitor)) = win.primary_monitor() else {
        return;
    };
    let size = win
        .outer_size()
        .unwrap_or_else(|_| PhysicalSize::new(224, 78));
    let area = monitor.size();
    let origin = monitor.position();
    let x = origin.x + area.width as i32 - size.width as i32 - 28;
    let y = origin.y + area.height as i32 - size.height as i32 - 96;
    let _ = win.set_position(PhysicalPosition::new(x, y));
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn clear_logs(state: State<'_, Shared>) {
    if let Ok(mut g) = state.lock() {
        g.logs.clear();
    }
}

async fn run_loop(app: tauri::AppHandle, state: Shared) {
    let mut client: Option<lcu::Lcu> = None;
    let mut handled_ready_check = false;
    let mut last_phase = String::new();
    let mut last_discovery: Option<std::time::Instant> = None;
    let mut last_profile: Option<std::time::Instant> = None;

    push_log(&state, "info", "正在查找英雄联盟客户端…");
    emit(&app, &state);

    loop {
        // 跨天清零
        let d = today();
        if let Ok(mut g) = state.lock() {
            if g.day != d {
                g.day = d;
                g.accepted_today = 0;
            }
        }

        // 未连接时每 5 秒扫一次（扫一次要跑 reg + 读日志，不能每轮都做）
        let should_discover = client.is_none()
            && last_discovery.map_or(true, |t| t.elapsed() >= Duration::from_secs(5));

        if should_discover {
            last_discovery = Some(std::time::Instant::now());
            for cred in lcu::find_candidates() {
                let candidate = lcu::Lcu::new(cred);
                if candidate.phase().await.is_ok() {
                    let port = candidate.cred.port;
                    let root = candidate.cred.root.display().to_string();
                    if let Ok(mut g) = state.lock() {
                        g.connected = true;
                        g.port = Some(port);
                        g.client_path = Some(root);
                    }
                    push_log(&state, "ok", &format!("已连接客户端（端口 {port}）"));
                    client = Some(candidate);
                    break;
                }
            }
        }

        let mut dropped = false;
        if let Some(c) = &client {
            // 召唤师信息：连上立刻拉一次，之后每分钟刷新（换头像、升级都能跟上）
            let need_profile = last_profile
                .map_or(true, |t| t.elapsed() >= Duration::from_secs(PROFILE_REFRESH_SECS));
            if need_profile {
                last_profile = Some(std::time::Instant::now());
                refresh_profile(c, &state).await;
            }

            match c.phase().await {
                Ok(phase) => {
                    if phase != last_phase {
                        if !last_phase.is_empty() || phase != "None" {
                            push_log(&state, "info", &format!("阶段变化：{}", phase_cn(&phase)));
                        }
                        last_phase = phase.clone();
                    }

                    if let Ok(mut g) = state.lock() {
                        g.phase = phase.clone();
                        g.connected = true;
                    }

                    if phase == "ReadyCheck" {
                        if !handled_ready_check {
                            handled_ready_check = true;
                            handle_ready_check(c, &state).await;
                        }
                    } else {
                        handled_ready_check = false;
                    }
                }
                Err(err) => {
                    dropped = true;
                    if let Ok(mut g) = state.lock() {
                        g.connected = false;
                        g.phase.clear();
                    }
                    push_log(&state, "warn", &format!("与客户端断开：{err}"));
                }
            }
        }

        if dropped {
            client = None;
            last_phase.clear();
            handled_ready_check = false;
            last_profile = None;
        }

        emit(&app, &state);
        tokio::time::sleep(Duration::from_millis(POLL_MS)).await;
    }
}

/// 拉取当前召唤师的名字、等级与头像。
///
/// 失败时保留旧值（头像尤其重要：取不到就沿用上一条，避免界面上头像闪没）。
async fn refresh_profile(c: &lcu::Lcu, state: &Shared) {
    let info = match c.summoner().await {
        Ok(i) => i,
        Err(_) => return,
    };

    let old = match state.lock() {
        Ok(g) => g.profile.clone(),
        Err(_) => None,
    };
    // 头像 id 没变就复用，省一次 19KB 的下载
    let icon = match &old {
        Some(p) if !p.icon.is_empty() && p.icon_id == info.icon_id => p.icon.clone(),
        _ => c
            .icon_data_url(info.icon_id)
            .await
            .unwrap_or_else(|_| old.as_ref().map(|p| p.icon.clone()).unwrap_or_default()),
    };

    if let Ok(mut g) = state.lock() {
        g.profile = Some(Profile {
            name: info.name,
            level: info.level,
            icon,
            icon_id: info.icon_id,
        });
    }
}

async fn handle_ready_check(c: &lcu::Lcu, state: &Shared) {
    let (enabled, delay_ms) = match state.lock() {
        Ok(g) => (g.enabled, g.delay_ms),
        Err(_) => return,
    };

    if !enabled {
        push_log(state, "info", "匹配成功，但自动确认处于关闭状态");
        return;
    }

    push_log(state, "warn", "匹配成功，准备确认");

    if delay_ms > 0 {
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    }

    // 延迟之后状态可能已经变了，确认一次再动手
    match c.phase().await {
        Ok(p) if p == "ReadyCheck" => {
            match c.post_empty("/lol-matchmaking/v1/ready-check/accept").await {
                Ok(()) => {
                    if let Ok(mut g) = state.lock() {
                        g.accepted_today += 1;
                    }
                    push_log(state, "ok", "已自动确认，进入选人");
                }
                Err(e) => push_log(state, "err", &format!("确认失败：{e}")),
            }
        }
        Ok(other) => {
            push_log(state, "info", &format!("已离开确认阶段（{}），跳过", phase_cn(&other)));
        }
        Err(e) => push_log(state, "err", &format!("确认前复查失败：{e}")),
    }
}

fn phase_cn(phase: &str) -> String {
    let cn = match phase {
        "None" => "空闲",
        "Lobby" => "大厅",
        "Matchmaking" => "匹配中",
        "ReadyCheck" => "等待确认",
        "ChampSelect" => "选人阶段",
        "GameStart" => "载入中",
        "InProgress" => "对局中",
        "Reconnect" => "重连中",
        "WaitingForStats" => "结算中",
        "PreEndOfGame" => "即将结束",
        "EndOfGame" => "对局结束",
        "TerminatedInError" => "异常结束",
        other => return other.to_string(),
    };
    cn.to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 这台机器的 GPU 栈对 Chromium 系（Electron / WebView2）的独立 GPU 进程都有问题：
    // Electron 上实测唯一有效的方案是 in-process-gpu，WebView2 同样适用。
    // 必须在任何 WebView 环境创建之前设置。
    #[cfg(windows)]
    if std::env::var_os("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").is_none() {
        std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", "--in-process-gpu");
    }

    tauri::Builder::default()
        .setup(|app| {
            let state: Shared = Arc::new(Mutex::new(Inner {
                enabled: true,
                delay_ms: 0,
                day: today(),
                ..Default::default()
            }));
            app.manage(state.clone());

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(run_loop(handle, state));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            set_enabled,
            set_delay,
            clear_logs,
            set_window_mode,
            quit_app
        ])
        // 主窗口是「应用本体」：关掉它就退出，别让它关完只剩一个胶囊漂在桌面上
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    window.app_handle().exit(0);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("应用启动失败");
}
