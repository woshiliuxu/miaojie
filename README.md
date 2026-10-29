# 对局确认助手

匹配成功时自动点「接受」，不用再盯着屏幕 —— 就这一个功能。

## 它做什么

- 监听客户端对局流程阶段（`/lol-gameflow/v1/gameflow-phase`）
- 只在 `ReadyCheck`（等待确认）阶段动手，其余阶段完全不碰
- 调 `/lol-matchmaking/v1/ready-check/accept` 点接受，每次匹配只点一次
- 可选 0-3 秒延迟（延迟后还会复查一次阶段，已经离开就不点）

## 它不做什么

- 不注入 DLL、不读游戏内存、不模拟键鼠输入
- 不做秒选、不做自动开局、不碰对局内任何数据
- 不联网、不上传、无账号

全部数据来自 `127.0.0.1` 上的客户端本地 REST 接口，和客户端自己点按钮走的是同一条路。

## 两种形态

| 形态 | 尺寸 | 用途 |
| --- | --- | --- |
| 主界面 | 400×520 | 看阶段、看今日次数、看运行记录、调延迟 |
| 胶囊模式 | 224×78（透明） | 挂在桌面上（置顶、不占任务栏），只做开 / 关 |

顶栏第一个按钮（胶囊图标）进入胶囊模式：主界面隐藏，桌面右下角出现一颗小胶囊。
胶囊上只有一个开关，外加右上角一个回主界面的小箭头（双击胶囊也行）；`●` 的颜色就是当前连接状态。

## 技术栈

Tauri 2 + Rust + Vue 3 + Vite（见 `~/.workbuddy/.../MEMORY.md` 选型记录）。

- Rust 侧：`src-tauri/src/lcu.rs`（凭证发现 + REST）、`lib.rs`（状态机 + 命令 + 双窗口切换）
- 前端：`src/App.vue`（主窗口）+ `src/Capsule.vue`（胶囊）+ `src/style.css` / `src/capsule.css`（Liquid Glass 主题）
- 双页入口：`index.html` / `capsule.html`，由 `vite.config.ts` 的 `rollupOptions.input` 指定
- 产物：单 exe，约 8-10MB

## 构建

```bash
npm install
npm run build
cd src-tauri && cargo build --release --features tauri/custom-protocol
python tools/create-shortcut.py   # 建桌面快捷方式
```

exe 输出在 `src-tauri/target/release/LolAutoAccept.exe`。

> 裸 `cargo build --release` 少了 `--features tauri/custom-protocol` 会按 dev 模式运行，
> 去连 `devUrl` 报 `ERR_CONNECTION_REFUSED`。

## 客户端凭证从哪来

国服 `lockfile` 经常是 0 字节，所以按顺序尝试：

1. `lockfile`（`进程名:pid:端口:密码:协议`）
2. `LeagueClient/*_LeagueClientUx.log` 里的 `--app-port` 与 `--remoting-auth-token`
3. 安装目录：注册表（腾讯 / Riot 各一套）+ 各盘符 `WeGameApps\英雄联盟`

客户端重启会换端口，所以未连接时每 5 秒重新扫一次，并且对每个候选都先用
`gameflow-phase` 验证，避免连到旧端口。

## 版本

- 1.1.0 —— 胶囊模式（桌面置顶小胶囊，只做开 / 关）+ 主界面整体重排
- 1.0.0 —— 首个可用版本

## 许可

仅供学习自用，与腾讯 / Riot 无关。
