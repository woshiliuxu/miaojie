//! LCU（League Client Update）本地接口访问。
//!
//! 只走 127.0.0.1 上的客户端 REST 接口，不注入、不读内存、不模拟输入。

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use base64::Engine;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Debug)]
pub struct Cred {
    pub port: u16,
    pub token: String,
    pub root: PathBuf,
}

/// 当前召唤师（名字里带上 #tagLine）。
#[derive(Clone, Debug)]
pub struct SummonerInfo {
    pub name: String,
    pub icon_id: i64,
    pub level: i64,
}

/// 客户端安装目录候选：先查注册表，再按盘符扫常见路径。
pub fn candidate_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();

    for (key, name) in [
        ("HKCU\\Software\\Tencent\\LOL", "InstallPath"),
        ("HKLM\\SOFTWARE\\WOW6432Node\\Tencent\\LOL", "InstallPath"),
        ("HKLM\\SOFTWARE\\Tencent\\LOL", "InstallPath"),
        ("HKCU\\Software\\Riot Games\\League of Legends", "InstallLocation"),
        (
            "HKLM\\SOFTWARE\\WOW6432Node\\Riot Games\\League of Legends",
            "InstallLocation",
        ),
    ] {
        if let Some(path) = reg_value(key, name) {
            roots.push(PathBuf::from(path));
        }
    }

    for drive in ["C", "D", "E", "F", "G", "H"] {
        roots.push(PathBuf::from(format!("{drive}:\\WeGameApps\\英雄联盟")));
    }
    roots.push(PathBuf::from("C:\\Program Files\\WeGameApps\\英雄联盟"));
    roots.push(PathBuf::from("C:\\Riot Games\\League of Legends"));
    roots.push(PathBuf::from("D:\\Riot Games\\League of Legends"));
    roots.push(PathBuf::from("E:\\Riot Games\\League of Legends"));

    roots
}

#[cfg(windows)]
fn reg_value(key: &str, name: &str) -> Option<String> {
    let out = Command::new("reg")
        .args(["query", key, "/v", name])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        let line = line.trim();
        if !line.starts_with(name) {
            continue;
        }
        // 形如：InstallPath    REG_SZ    G:\WeGameApps\英雄联盟
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let value = parts[2..].join(" ");
            let value = value.trim().trim_end_matches(['\\', '/']);
            if value.len() > 2 {
                return Some(value.to_string());
            }
        }
    }
    None
}

#[cfg(not(windows))]
fn reg_value(_key: &str, _name: &str) -> Option<String> {
    None
}

/// 从运行中的 LeagueClientUx.exe 进程命令行直接取端口与令牌。
/// 这是唯一实时可靠的来源（参考 Frank 的实现）：客户端每次重启都换端口，
/// 注册表与日志都可能过期或读不出来。
fn from_process() -> Option<Cred> {
    use sysinfo::{ProcessExt, SystemExt};
    let sys = sysinfo::System::new_all();
    let proc = sys
        .processes()
        .values()
        .find(|p| p.name() == "LeagueClientUx.exe")?;
    let args = proc.cmd();

    let port = args
        .iter()
        .find_map(|a| a.strip_prefix("--app-port="))?
        .parse::<u16>()
        .ok()?;
    let token = args
        .iter()
        .find_map(|a| a.strip_prefix("--remoting-auth-token="))?
        .to_string();
    if token.is_empty() {
        return None;
    }

    // 进程命令行拿不到安装目录，从父目录兜底猜一个用于展示
    let root = candidate_roots()
        .into_iter()
        .find(|r| r.join("LeagueClient").is_dir())
        .unwrap_or_default();

    Some(Cred {
        port,
        token,
        root,
    })
}

/// 返回所有可能的凭证候选（按可靠性排序），由调用方逐个验证。
///
/// 客户端重启会换端口，旧日志还留在目录里，所以不能只取第一个。
pub fn find_candidates() -> Vec<Cred> {
    let mut out: Vec<Cred> = Vec::new();

    // 1. 进程命令行（最可靠）
    if let Some(cred) = from_process() {
        out.push(cred);
    }

    // 2. lockfile / 日志兜底（客户端在跑但进程枚举失败时用）
    for root in candidate_roots() {
        if !root.join("LeagueClient").is_dir() && !root.join("lockfile").is_file() {
            continue;
        }
        if let Some(cred) = from_lockfile(&root) {
            out.push(cred);
        }
        out.extend(from_logs(&root));
    }
    // 按端口去重，保留先出现的
    let mut seen = Vec::new();
    out.retain(|c| {
        if seen.contains(&c.port) {
            false
        } else {
            seen.push(c.port);
            true
        }
    });
    out
}

/// lockfile 内容：进程名:pid:端口:密码:协议
fn from_lockfile(root: &PathBuf) -> Option<Cred> {
    let text = fs::read_to_string(root.join("lockfile")).ok()?;
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let parts: Vec<&str> = text.split(':').collect();
    if parts.len() < 5 {
        return None;
    }
    let port: u16 = parts[2].parse().ok()?;
    let token = parts[3].to_string();
    if token.is_empty() {
        return None;
    }
    Some(Cred {
        port,
        token,
        root: root.clone(),
    })
}

/// 国服 lockfile 经常是 0 字节，日志才是可靠来源。
fn from_logs(root: &PathBuf) -> Vec<Cred> {
    let dir = root.join("LeagueClient");
    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut logs: Vec<(std::time::SystemTime, PathBuf)> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.ends_with("_LeagueClientUx.log"))
                .unwrap_or(false)
        })
        .filter_map(|p| {
            let mtime = fs::metadata(&p).ok()?.modified().ok()?;
            Some((mtime, p))
        })
        .collect();

    logs.sort_by(|a, b| b.0.cmp(&a.0));

    let mut out = Vec::new();
    for (_, path) in logs.iter().take(4) {
        // 国服日志混有 GBK 字节，read_to_string（严格 UTF-8）会整份失败，
        // 必须读 bytes 再做有损转换。
        let content = match fs::read(path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(_) => continue,
        };
        let port = extract_arg(&content, "--app-port=");
        let token = extract_arg(&content, "--remoting-auth-token=");
        if let (Some(port), Some(token)) = (port, token) {
            if let Ok(port) = port.parse::<u16>() {
                out.push(Cred {
                    port,
                    token,
                    root: root.clone(),
                });
            }
        }
    }
    out
}

fn extract_arg(content: &str, key: &str) -> Option<String> {
    let start = content.find(key)? + key.len();
    let rest = &content[start..];
    let end = rest
        .find(|c: char| c == ' ' || c == '"' || c == '\'' || c == '\n' || c == '\r' || c == '\t')
        .unwrap_or(rest.len());
    let value = &rest[..end];
    if value.is_empty() || !value.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return None;
    }
    Some(value.to_string())
}

pub struct Lcu {
    pub cred: Cred,
    http: reqwest::Client,
    auth: String,
}

impl Lcu {
    pub fn new(cred: Cred) -> Self {
        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            // 关键：系统代理（http_proxy 等环境变量）会把发往 127.0.0.1 的请求劫持走代理，
            // 客户端本地接口必须直连。
            .no_proxy()
            .timeout(Duration::from_millis(2500))
            .build()
            .expect("创建 HTTP 客户端失败");
        let raw = format!("riot:{}", cred.token);
        let auth = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(raw)
        );
        Self { cred, http, auth }
    }

    fn url(&self, path: &str) -> String {
        format!("https://127.0.0.1:{}{}", self.cred.port, path)
    }

    pub async fn get(&self, path: &str) -> Result<String, String> {
        let resp = self
            .http
            .get(self.url(path))
            .header("Authorization", &self.auth)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("连接失败: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(format!("HTTP {}", status.as_u16()));
        }
        resp.text().await.map_err(|e| format!("读取响应失败: {e}"))
    }

    pub async fn post_empty(&self, path: &str) -> Result<(), String> {
        let resp = self
            .http
            .post(self.url(path))
            .header("Authorization", &self.auth)
            .header("Content-Length", "0")
            .send()
            .await
            .map_err(|e| format!("连接失败: {e}"))?;

        let status = resp.status();
        if status.is_success() {
            Ok(())
        } else {
            Err(format!("HTTP {}", status.as_u16()))
        }
    }

    /// 当前对局流程阶段，例如 "ReadyCheck" / "InProgress"。
    pub async fn phase(&self) -> Result<String, String> {
        let raw = self.get("/lol-gameflow/v1/gameflow-phase").await?;
        Ok(raw.trim().trim_matches('"').to_string())
    }

    /// 取原始字节（头像等二进制资源）。
    pub async fn get_bytes(&self, path: &str) -> Result<Vec<u8>, String> {
        let resp = self
            .http
            .get(self.url(path))
            .header("Authorization", &self.auth)
            .send()
            .await
            .map_err(|e| format!("连接失败: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(format!("HTTP {}", status.as_u16()));
        }
        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| format!("读取响应失败: {e}"))
    }

    /// 当前召唤师信息。
    ///
    /// 国服 `displayName` 经常是空的，真正的名字在 `gameName` + `tagLine`。
    pub async fn summoner(&self) -> Result<SummonerInfo, String> {
        let raw = self.get("/lol-summoner/v1/current-summoner").await?;
        let v: serde_json::Value =
            serde_json::from_str(&raw).map_err(|e| format!("解析召唤师信息失败: {e}"))?;

        let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
        let n = |k: &str| v.get(k).and_then(|x| x.as_i64()).unwrap_or(0);

        let display = s("displayName");
        let game = s("gameName");
        let tag = s("tagLine");
        let name = if !game.is_empty() {
            if tag.is_empty() {
                game
            } else {
                format!("{game} #{tag}")
            }
        } else if !display.is_empty() {
            display
        } else {
            s("internalName")
        };

        Ok(SummonerInfo {
            name,
            icon_id: n("profileIconId"),
            level: n("summonerLevel"),
        })
    }

    /// 头像转成 data URL，前端可直接放进 img src。
    ///
    /// 头像走的是普通 HTTP 资源，但需要 Basic 鉴权，img 标签没法带 header，
    /// 所以在 Rust 侧取回来内联成 base64。
    pub async fn icon_data_url(&self, icon_id: i64) -> Result<String, String> {
        if icon_id <= 0 {
            return Err("无效的头像 id".to_string());
        }
        let bytes = self
            .get_bytes(&format!("/lol-game-data/assets/v1/profile-icons/{icon_id}.jpg"))
            .await?;
        if bytes.is_empty() {
            return Err("头像内容为空".to_string());
        }
        Ok(format!(
            "data:image/jpeg;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        ))
    }
}
