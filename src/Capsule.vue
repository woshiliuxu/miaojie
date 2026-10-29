<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

interface Snapshot {
  connected: boolean;
  phase: string;
  enabled: boolean;
}

const connected = ref(false);
const phase = ref("None");
const enabled = ref(true);
const isTauri = ref(true);

const ready = computed(() => connected.value && phase.value === "ReadyCheck");

/** 胶囊上只讲一件事：现在点不点接受 */
const label = computed(() => {
  if (!enabled.value) return "已暂停";
  if (ready.value) return "正在确认";
  if (!connected.value) return "等待客户端";
  return "自动确认";
});

const dotClass = computed(() => {
  if (ready.value && enabled.value) return "run";
  if (!enabled.value) return "off";
  return connected.value ? "ok" : "idle";
});

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

async function openMain() {
  await call("set_window_mode", { mode: "main" });
}

async function startDrag(e: MouseEvent) {
  if (!isTauri.value || e.button !== 0) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().startDragging();
}

function applyAppearance(mode: string) {
  document.documentElement.dataset.appearance = mode === "dark" ? "dark" : "light";
}

let unlisten: UnlistenFn | null = null;
let unlistenTheme: UnlistenFn | null = null;

onMounted(async () => {
  applyAppearance(localStorage.getItem("appearance") ?? "light");

  isTauri.value = "__TAURI_INTERNALS__" in window;
  if (!isTauri.value) {
    connected.value = true;
    phase.value = "Lobby";
    enabled.value = true;
    return;
  }

  unlisten = await listen<Snapshot>("snapshot", (e) => {
    const s = e.payload;
    connected.value = s.connected;
    phase.value = s.phase;
    enabled.value = s.enabled;
  });
  unlistenTheme = await listen<string>("appearance", (e) => {
    applyAppearance(e.payload);
    localStorage.setItem("appearance", e.payload);
  });

  const first = await call<Snapshot>("get_snapshot");
  if (first) {
    connected.value = first.connected;
    phase.value = first.phase;
    enabled.value = first.enabled;
  }
});

onUnmounted(() => {
  unlisten?.();
  unlistenTheme?.();
});
</script>

<template>
  <div class="wrap" @mousedown="startDrag" @dblclick="openMain">
    <div class="pill" :class="{ off: !enabled, live: ready && enabled }">
      <span class="dot" :class="dotClass" />
      <span class="label">{{ label }}</span>
      <button
        class="mini-switch"
        :class="{ on: enabled }"
        :aria-pressed="enabled"
        :title="enabled ? '关闭自动确认' : '开启自动确认'"
        @mousedown.stop
        @click.stop="toggle"
      >
        <i />
      </button>
      <button class="expand" title="回到主界面（或双击胶囊）" @mousedown.stop @click.stop="openMain">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round">
          <path d="M14 4h6v6" />
          <path d="M20 4l-8.5 8.5" />
          <path d="M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5" />
        </svg>
      </button>
    </div>
  </div>
</template>
