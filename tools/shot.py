"""开发用截图脚本：按窗口标题抓图，方便核对界面像素。

用法：python tools/shot.py [标题关键字] [输出文件]
"""
import ctypes
import ctypes.wintypes as wt
import sys

from PIL import ImageGrab

user32 = ctypes.windll.user32


def find_window(keyword: str):
    """按标题关键字找可见窗口，返回 (hwnd, title)。"""
    found = []

    @ctypes.WINFUNCTYPE(ctypes.c_bool, wt.HWND, wt.LPARAM)
    def callback(hwnd, _):
        if not user32.IsWindowVisible(hwnd):
            return True
        length = user32.GetWindowTextLengthW(hwnd)
        if length == 0:
            return True
        buf = ctypes.create_unicode_buffer(length + 1)
        user32.GetWindowTextW(hwnd, buf, length + 1)
        if keyword in buf.value:
            found.append((hwnd, buf.value, user32.GetWindowRect))
        return True

    user32.EnumWindows(callback, 0)
    return found


def main():
    keyword = sys.argv[1] if len(sys.argv) > 1 else "对局确认助手"
    out = sys.argv[2] if len(sys.argv) > 2 else "_shot/shot.png"
    wins = find_window(keyword)
    if not wins:
        print("未找到窗口:", keyword)
        return 1
    hwnd, title, _ = wins[0]
    rect = wt.RECT()
    user32.GetWindowRect(hwnd, ctypes.byref(rect))
    box = (rect.left, rect.top, rect.right, rect.bottom)
    img = ImageGrab.grab(bbox=box, all_screens=True)
    img.save(out)
    print(f"{title} | {box} | {img.size} -> {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
