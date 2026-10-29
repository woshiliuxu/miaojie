# -*- coding: utf-8 -*-
"""从 exe 里把图标抠出来，确认编译进去的是不是新图标。

只读操作：ExtractIconExW 取 HICON，GetDIBits 拿像素。

用法：<venv>/Scripts/python tools/verify-icon.py <exe路径> <输出png>
"""
import ctypes
import sys
from ctypes import wintypes
from pathlib import Path

from PIL import Image

user32 = ctypes.windll.user32
gdi32 = ctypes.windll.gdi32
shell32 = ctypes.windll.shell32


class ICONINFO(ctypes.Structure):
    _fields_ = [
        ("fIcon", wintypes.BOOL),
        ("xHotspot", wintypes.DWORD),
        ("yHotspot", wintypes.DWORD),
        ("hbmMask", ctypes.c_void_p),
        ("hbmColor", ctypes.c_void_p),
    ]


class BITMAP(ctypes.Structure):
    _fields_ = [
        ("bmType", wintypes.LONG),
        ("bmWidth", wintypes.LONG),
        ("bmHeight", wintypes.LONG),
        ("bmWidthBytes", wintypes.LONG),
        ("bmPlanes", wintypes.WORD),
        ("bmBitsPixel", wintypes.WORD),
        ("bmBits", ctypes.c_void_p),
    ]


class BITMAPINFOHEADER(ctypes.Structure):
    _fields_ = [
        ("biSize", wintypes.DWORD),
        ("biWidth", wintypes.LONG),
        ("biHeight", wintypes.LONG),
        ("biPlanes", wintypes.WORD),
        ("biBitCount", wintypes.WORD),
        ("biCompression", wintypes.DWORD),
        ("biSizeImage", wintypes.DWORD),
        ("biXPelsPerMeter", wintypes.LONG),
        ("biYPelsPerMeter", wintypes.LONG),
        ("biClrUsed", wintypes.DWORD),
        ("biClrImportant", wintypes.DWORD),
    ]


def grab(exe: str) -> Image.Image:
    large = (ctypes.c_void_p * 1)()
    small = (ctypes.c_void_p * 1)()
    got = shell32.ExtractIconExW(exe, 0, large, small, 1)
    if got == 0 or not large[0]:
        raise RuntimeError("exe 里没有图标资源")
    hicon = large[0]

    info = ICONINFO()
    if not user32.GetIconInfo(hicon, ctypes.byref(info)):
        raise RuntimeError("GetIconInfo 失败")

    bm = BITMAP()
    gdi32.GetObjectW(info.hbmColor, ctypes.sizeof(bm), ctypes.byref(bm))
    w, h = bm.bmWidth, bm.bmHeight

    bi = BITMAPINFOHEADER()
    bi.biSize = ctypes.sizeof(bi)
    bi.biWidth = w
    bi.biHeight = -h
    bi.biPlanes = 1
    bi.biBitCount = 32
    bi.biCompression = 0

    buf = ctypes.create_string_buffer(w * h * 4)
    hdc = user32.GetDC(0)
    gdi32.GetDIBits(hdc, info.hbmColor, 0, h, buf, ctypes.byref(bi), 0)
    user32.ReleaseDC(0, hdc)

    return Image.frombuffer("RGBA", (w, h), buf, "raw", "BGRA", 0, 1).copy()


if __name__ == "__main__":
    exe = sys.argv[1]
    out = Path(sys.argv[2])
    img = grab(exe)
    print("提取到", img.width, "x", img.height)
    scale = max(1, 128 // img.width)
    img.resize((img.width * scale, img.height * scale), Image.NEAREST).save(out)
    print("已保存", out)
