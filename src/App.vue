<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

const VERSION = "1.1.0";

interface LogItem {
  time: string;
  text: string;
  kind: "info" | "ok" | "warn" | "err";
}

interface Snapshot {
  connected: boolean;
  phase: string;
  enabled: boolean;
  delayMs: number;
  acceptedToday: number;
  port: number | null;
  logs: LogItem[];
  clientPath: string | null;
  profile: Profile | null;
}

interface Profile {
  name: string;
  level: number;
  icon: string;
}

const PHASE_CN: Record<string, string> = {
  None: "空闲",
  Lobby: "大厅",
  Matchmaking: "匹配中",
  ReadyCheck: "等待确认",
  ChampSelect: "选人阶段",
  GameStart: "载入中",
  InProgress: "对局中",
  Reconnect: "重连中",
  WaitingForStats: "结算中",
  PreEndOfGame: "即将结束",
  EndOfGame: "对局结束",
  TerminatedInError: "异常结束"
};

const DELAYS = [0, 1, 2, 3];

const connected = ref(false);
const phase = ref("None");
const enabled = ref(true);
const delayMs = ref(0);
const acceptedToday = ref(0);
const port = ref<number | null>(null);
const clientPath = ref<string | null>(null);
const profile = ref<Profile | null>(null);
const logs = ref<LogItem[]>([]);
const dark = ref(false);
const isTauri = ref(true);

const phaseText = computed(() => {
  if (!connected.value) return "空闲";
  return PHASE_CN[phase.value] ?? (phase.value || "空闲");
});
const delaySec = computed(() => Math.round(delayMs.value / 1000));
const ready = computed(() => connected.value && phase.value === "ReadyCheck");

const statusClass = computed(() => {
  if (!connected.value) return "";
  return ready.value ? "run" : "ok";
});
const statusText = computed(() => (connected.value ? "已连接" : "未连接"));

const phaseHint = computed(() => {
  if (!connected.value) return "打开英雄联盟客户端后会自动连接";
  if (phase.value === "ReadyCheck") {
    return enabled.value ? `将在 ${delaySec.value} 秒后自动确认` : "自动确认已关闭，本次不点接受";
  }
  if (phase.value === "Matchmaking") return "正在匹配，等待对手";
  if (phase.value === "ChampSelect") return "已进入选人，本次不需要确认";
  if (phase.value === "InProgress") return "对局进行中";
  if (phase.value === "Lobby") return "在大厅等待开始匹配";
  return "等待匹配";
});

function applySnapshot(s: Snapshot) {
  connected.value = s.connected;
  phase.value = s.phase;
  enabled.value = s.enabled;
  delayMs.value = s.delayMs;
  acceptedToday.value = s.acceptedToday;
  port.value = s.port;
  clientPath.value = s.clientPath;
  profile.value = s.profile ?? null;
  logs.value = s.logs ?? [];
}

function mockSnapshot(): Snapshot {
  return {

    connected: true,
    phase: "Lobby",
    enabled: true,
    delayMs: 0,
    acceptedToday: 3,
    port: 61208,
    clientPath: "G:\\WeGameApps\\英雄联盟",
    profile: {
      name: "宇宙探索编辑部 #49135",
      level: 164,
      icon: ""
    },
    logs: [
      { time: "12:00:00", text: "正在查找英雄联盟客户端…", kind: "info" },
      { time: "12:00:01", text: "已连接客户端（端口 61208）", kind: "ok" },
      { time: "12:03:12", text: "阶段变化：匹配中", kind: "info" }
    ]
  };
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T | null> {
  if (!isTauri.value) return null;
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    console.error(cmd, e);
    return null;
  }
}

async function toggle() {
  const next = !enabled.value;
  enabled.value = next;
  await call("set_enabled", { enabled: next });
}

async function pickDelay(sec: number) {
  delayMs.value = sec * 1000;
  await call("set_delay", { ms: sec * 1000 });
}

async function clearLogs() {
  logs.value = [];
  await call("clear_logs");
}

async function toggleTheme() {
  dark.value = !dark.value;
  document.documentElement.dataset.appearance = dark.value ? "dark" : "light";
  localStorage.setItem("appearance", dark.value ? "dark" : "light");
  if (isTauri.value) {
    const { emitTo } = await import("@tauri-apps/api/event");
    await emitTo("capsule", "appearance", dark.value ? "dark" : "light");
  }
}

async function enterCapsule() {
  await call("set_window_mode", { mode: "capsule" });
}

async function startDrag(e: MouseEvent) {
  if (!isTauri.value || e.button !== 0) return;
  const target = e.target as HTMLElement | null;
  if (target && target.closest(".winbtns")) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().startDragging();
}

async function toggleMax() {
  if (!isTauri.value) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  const w = getCurrentWindow();
  if (await w.isMaximized()) await w.unmaximize();
  else await w.maximize();
}

async function minimizeWin() {
  if (!isTauri.value) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().minimize();
}

async function quitApp() {
  if (!isTauri.value) return;
  await call("quit_app");
}

let unlisten: UnlistenFn | null = null;

onMounted(async () => {
  const saved = localStorage.getItem("appearance");
  dark.value = saved === "dark";
  document.documentElement.dataset.appearance = dark.value ? "dark" : "light";

  // 非 Tauri 环境（纯 vite dev）下走 mock，方便调样式
  isTauri.value = "__TAURI_INTERNALS__" in window;
  if (!isTauri.value) {
    applySnapshot(mockSnapshot());
    return;
  }

  unlisten = await listen<Snapshot>("snapshot", (e) => applySnapshot(e.payload));
  const first = await call<Snapshot>("get_snapshot");
  if (first) applySnapshot(first);
});

onUnmounted(() => unlisten?.());
</script>

<template>
  <header class="topbar">
    <div class="dragzone" @mousedown="startDrag" @dblclick="toggleMax">
      <div class="brand">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round">
          <path d="M20 6 9 17l-5-5" />
        </svg>
      </div>
      <div class="titles">
        <h1>对局确认助手</h1>
        <p>匹配成功时自动点接受</p>
      </div>
    </div>
    <div class="winbtns">
      <button class="winbtn" title="胶囊模式（挂桌面）" @click="enterCapsule">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round">
          <rect x="2.5" y="8" width="19" height="8" rx="4" />
          <path d="M7.5 8v8" />
        </svg>
      </button>
      <button class="winbtn" :title="dark ? '切换到浅色' : '切换到深色'" @click="toggleTheme">
        <svg v-if="!dark" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8Z" />
        </svg>
        <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="4" />
          <path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" />
        </svg>
      </button>
      <button class="winbtn" title="最小化" @click="minimizeWin">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round">
          <path d="M5 12h14" />
        </svg>
      </button>
      <button class="winbtn close" title="关闭" @click="quitApp">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round">
          <path d="M6 6l12 12M18 6L6 18" />
        </svg>
      </button>
    </div>
  </header>

  <div class="body">
    <section class="card hero" :class="{ live: ready }">
      <div class="who">
        <div class="avatar">
          <img v-if="profile && profile.icon" :src="profile.icon" alt="" />
          <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="M20 21v-1.5A5.5 5.5 0 0 0 14.5 14h-5A5.5 5.5 0 0 0 4 19.5V21" />
            <circle cx="12" cy="7.5" r="3.8" />
          </svg>
        </div>
        <div class="pinfo">
          <span class="pname">{{ profile ? profile.name : "未登录" }}</span>
          <span v-if="profile && profile.level" class="plv">Lv.{{ profile.level }}</span>
        </div>
        <span class="status" :class="statusClass">{{ statusText }}</span>
      </div>

      <div class="phaserow">
        <span class="ph">{{ phaseText }}</span>
        <span class="chip">今日 <b>{{ acceptedToday }}</b> 次</span>
      </div>

      <div class="hintrow">
        <span class="hint">{{ phaseHint }}</span>
        <span v-if="port" class="port">端口 {{ port }}</span>
      </div>
    </section>

    <section class="card">
      <div class="mainrow">
        <div class="txt">
          <strong>自动确认对局</strong>
          <span>只在「等待确认」阶段点一次接受</span>
        </div>
        <button class="switch" :class="{ on: enabled }" :aria-pressed="enabled" @click="toggle">
          <i />
        </button>
      </div>
      <div class="segrow">
        <span class="k">确认延迟</span>
        <div class="seg">
          <button v-for="d in DELAYS" :key="d" :class="{ on: delaySec === d }" @click="pickDelay(d)">
            {{ d === 0 ? "立即" : d + " 秒" }}
          </button>
        </div>
      </div>
    </section>

    <section class="card logcard">
      <div class="loghead">
        <strong>运行记录</strong>
        <span class="spacer" />
        <button v-if="logs.length" class="link" @click="clearLogs">清空</button>
      </div>
      <div v-if="logs.length" class="logs">
        <div v-for="(l, i) in logs" :key="i" class="logrow" :class="l.kind">
          <span class="t">{{ l.time }}</span>
          <span class="m">{{ l.text }}</span>
        </div>
      </div>
      <div v-else class="empty">
        <p>暂无记录</p>
        <p class="dim">每次自动确认都会记在这里</p>
      </div>
    </section>

    <div class="foot">
      <span>v{{ VERSION }} · 只读本地接口</span>
      <span class="spacer" />
      <span class="ellipsis" :title="clientPath ?? ''">{{ clientPath ? "已识别客户端" : "未找到客户端" }}</span>
    </div>
  </div>
</template>
