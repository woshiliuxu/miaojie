# -*- coding: utf-8 -*-
"""按窗口相对坐标点击：找到目标窗口 -> 置前 -> 把相对坐标换算成屏幕坐标再点。
用法: python tools/wclick.py <窗口标题关键字> <x> <y> [double]
x/y 是相对窗口可见内容区左上角的坐标（已补偿无边框窗口约 8px 的隐形边框）。
"""
import ctypes
import ctypes.wintypes as wt
import sys
import time

u = ctypes.windll.user32


def find(title_key):
    hits = []

    @ctypes.WINFUNCTYPE(ctypes.c_bool, wt.HWND, wt.LPARAM)
    def cb(h, _):
        n = u.GetWindowTextLengthW(h)
        if n:
            b = ctypes.create_unicode_buffer(n + 1)
            u.GetWindowTextW(h, b, n + 1)
            if title_key in b.value and u.IsWindowVisible(h):
                hits.append((h, b.value))
        return True

    u.EnumWindows(cb, 0)
    return hits


def main():
    key, rx, ry = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
    dbl = len(sys.argv) > 4 and sys.argv[4] == "double"
    hits = find(key)
    if not hits:
        print("未找到窗口:", key)
        return 1
    # 优先精确标题，其次取第一个
    hwnd = next((h for h, t in hits if t == key), hits[0][0])
    u.ShowWindow(hwnd, 9)  # SW_RESTORE
    u.SetForegroundWindow(hwnd)
    time.sleep(0.45)

    r = wt.RECT()
    u.GetWindowRect(hwnd, ctypes.byref(r))
    w, h = r.right - r.left, r.bottom - r.top
    # 坐标以截图为准：截图就是从 GetWindowRect 区域裁的，所以直接用原始偏移
    ax, ay = r.left + rx, r.top + ry
    print(f"窗口 ({r.left},{r.top}) {w}x{h} -> 点击 ({ax},{ay})")
    u.SetCursorPos(ax, ay)
    ev = 0x02, 0x04
    if dbl:
        for e in ev * 2:
            u.mouse_event(e, 0, 0, 0, 0)
            time.sleep(0.06)
    else:
        for e in ev:
            u.mouse_event(e, 0, 0, 0, 0)
            time.sleep(0.05)
    return 0


if __name__ == "__main__":
    sys.exit(main())
