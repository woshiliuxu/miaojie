# -*- coding: utf-8 -*-
"""在桌面创建「对局确认助手」快捷方式。

用法：python tools/create-shortcut.py
"""
import os
import sys
import winreg
from pathlib import Path

import pythoncom
import win32com.client

ROOT = Path(__file__).resolve().parent.parent
# 指向发布目录的中文名 exe（与 WebView2Loader.dll 同目录，缺 dll 会启动失败）
EXE = ROOT / "发布" / "对局确认助手.exe"
ICON = ROOT / "src-tauri" / "icons" / "icon.ico"
NAME = "对局确认助手.lnk"


def desktop_dir() -> Path:
    with winreg.OpenKey(
        winreg.HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders",
    ) as key:
        raw, _ = winreg.QueryValueEx(key, "Desktop")
    return Path(os.path.expandvars(raw))


def main() -> int:
    if not EXE.exists():
        print("找不到 exe：", EXE)
        print("先跑 npm run build 与 cargo build --release")
        return 1

    target = desktop_dir() / NAME
    pythoncom.CoInitialize()
    shell = win32com.client.Dispatch("WScript.Shell")
    shortcut = shell.CreateShortCut(str(target))
    shortcut.TargetPath = str(EXE)
    shortcut.WorkingDirectory = str(EXE.parent)
    if ICON.exists():
        shortcut.IconLocation = f"{ICON},0"
    shortcut.Description = "对局确认助手 · 匹配成功自动接受，只读本地接口"
    shortcut.Save()
    print("created:", target)
    return 0


if __name__ == "__main__":
    sys.exit(main())
