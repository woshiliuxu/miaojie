"""开发用辅助脚本：模拟鼠标点击屏幕坐标（核对界面交互用）。

用法：python tools/click.py X Y [双击间隔秒]
"""
import ctypes
import sys
import time

user32 = ctypes.windll.user32


def click(x: int, y: int, double: bool = False):
    user32.SetCursorPos(int(x), int(y))
    time.sleep(0.05)
    user32.mouse_event(0x0002, 0, 0, 0, 0)  # LEFTDOWN
    user32.mouse_event(0x0004, 0, 0, 0, 0)  # LEFTUP
    if double:
        time.sleep(0.08)
        user32.mouse_event(0x0002, 0, 0, 0, 0)
        user32.mouse_event(0x0004, 0, 0, 0, 0)


if __name__ == "__main__":
    x, y = int(sys.argv[1]), int(sys.argv[2])
    double = len(sys.argv) > 3 and sys.argv[3] == "double"
    click(x, y, double)
    print(f"clicked {x},{y} double={double}")
