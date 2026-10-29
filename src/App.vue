<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

const VERSION = "2.0.0";

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
const dark = ref(false);
const detail = ref(false);
const isTauri = ref(true);

const delaySec = computed(() => Math.round(delayMs.value / 1000));
const ready = computed(() => connected.value && phase.value === "ReadyCheck");

const phaseText = computed(() => {
  if (!connected.value) return "等待客户端";
  if (phase.value === "ReadyCheck") {
    if (!enabled.value) return "等待你确认";
    return delaySec.value === 0 ? "正在接受对局" : `${delaySec.value} 秒后自动接受`;
  }
  return PHASE_CN[phase.value] ?? (phase.value || "空闲");
});

const phaseHint = computed(() => {
  if (!connected.value) return "打开英雄联盟后自动连接";
  if (phase.value === "ReadyCheck") {
    return enabled.value ? "已为你排到对局" : "自动接受已关闭，这次要自己点";
  }
  if (phase.value === "Matchmaking") return "正在匹配，等待对手";
  if (phase.value === "ChampSelect") return "已进入选人，不需要确认";
  if (phase.value === "InProgress") return "对局进行中";
  if (phase.value === "Lobby") return "在大厅，等开始匹配";
  return "等待匹配";
});

const stageTone = computed(() => {
  if (!connected.value) return "idle";
  if (ready.value) return "hot";
  return "live";
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
    profile: { name: "宇宙探索编辑部", level: 164, icon: "" },
    logs: []
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
  if (target && target.closest(".btns")) return;
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
  <div class="lol" :class="stageTone" aria-hidden="true">LOL</div>

  <header class="bar">
    <div class="drag" @mousedown="startDrag" @dblclick="toggleMax">
      <img v-if="profile && profile.icon" class="tav" :src="profile.icon" alt="" />
      <span v-else-if="profile" class="tav">{{ profile.name.slice(0, 1) }}</span>
      <span class="name" :title="profile ? profile.name + (profile.level ? ' · Lv.' + profile.level : '') : '秒接'">
        {{ profile ? profile.name : "秒接" }}
      </span>
    </div>
    <div class="btns">
      <button class="wbtn" title="胶囊模式（挂桌面）" @click="enterCapsule">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round">
          <rect x="2.5" y="8" width="19" height="8" rx="4" />
          <path d="M7.5 8v8" />
        </svg>
      </button>
      <button class="wbtn" :title="dark ? '切换到浅色' : '切换到深色'" @click="toggleTheme">
        <svg v-if="!dark" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8Z" />
        </svg>
        <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="4" />
          <path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" />
        </svg>
      </button>
      <button class="wbtn" title="最小化" @click="minimizeWin">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round">
          <path d="M5 12h14" />
        </svg>
      </button>
      <button class="wbtn close" title="关闭" @click="quitApp">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round">
          <path d="M6 6l12 12M18 6L6 18" />
        </svg>
      </button>
    </div>
  </header>

  <main class="body">
    <section class="row">
      <div class="stat">
        <p class="stline" :class="stageTone"><i class="dot"></i>{{ phaseText }}</p>
        <p class="stsub">{{ phaseHint }}</p>
      </div>
      <div class="ctrl">
        <button class="switch big" :class="{ on: enabled }" :aria-pressed="enabled" @click="toggle">
          <i />
        </button>
        <span class="clabel">自动接受</span>
      </div>
    </section>

    <div class="foot">
      <span>今天 <b>{{ acceptedToday }}</b> 次</span>
      <button class="link" @click="detail = !detail">{{ detail ? "收起" : "详情" }}</button>
    </div>
  </main>

  <div class="mask" :class="{ show: detail }" @click="detail = false"></div>
  <section class="sheet" :class="{ show: detail }">
    <div class="sheet-head">
      <strong>设置</strong>
      <button class="wbtn" title="收起" @click="detail = false">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round">
          <path d="M6 6l12 12M18 6L6 18" />
        </svg>
      </button>
    </div>
    <div class="srow">
      <span class="k">账号</span>
      <span class="v">{{ profile ? profile.name + (profile.level ? " · Lv." + profile.level : "") : "未登录" }}</span>
    </div>
    <div class="srow">
      <span class="k">确认延迟</span>
      <div class="seg">
        <button v-for="d in DELAYS" :key="d" :class="{ on: delaySec === d }" @click="pickDelay(d)">
          {{ d === 0 ? "立即" : d + " 秒" }}
        </button>
      </div>
    </div>
    <div class="srow">
      <span class="k">客户端</span>
      <span class="v">{{ connected ? "已连接 · 端口 " + port : "未找到" }}</span>
    </div>
    <div class="srow">
      <span class="k">版本</span>
      <span class="v">v{{ VERSION }} · 只读本地接口</span>
    </div>
  </section>
</template>
