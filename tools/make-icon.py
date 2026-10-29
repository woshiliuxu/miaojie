# -*- coding: utf-8 -*-
"""生成「计时环」图标源图（1024×1024 PNG）。

再做一组小尺寸预览，方便肉眼确认 16/32px 下是否还认得出。

用法：<venv>/Scripts/python tools/make-icon.py
"""
import math
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "tools"

S = 4096  # 超采样 4 倍，缩小后边缘才干净
RED = (213, 45, 74, 255)
WHITE = (255, 255, 255, 255)
RING_ALPHA = 195


def build() -> Image.Image:
    card = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(card)
    d.rounded_rectangle([0, 0, S - 1, S - 1], radius=int(S * 0.226), fill=RED)

    cx = cy = S // 2
    r = int(S * 0.3027)
    w = int(S * 0.0625)
    bbox = [cx - r, cy - r, cx + r, cy + r]

    ring = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    dr = ImageDraw.Draw(ring)
    # 缺口留在右上角，像倒计时还没走完
    dr.arc(bbox, 350, 640, fill=(255, 255, 255, RING_ALPHA), width=w)
    for ang in (350.0, 280.0):
        x = cx + r * math.cos(math.radians(ang))
        y = cy + r * math.sin(math.radians(ang))
        dr.ellipse([x - w / 2, y - w / 2, x + w / 2, y + w / 2], fill=(255, 255, 255, RING_ALPHA))
    card = Image.alpha_composite(card, ring)

    d = ImageDraw.Draw(card)
    pts = [(S * 0.326, S * 0.516), (S * 0.441, S * 0.629), (S * 0.678, S * 0.348)]
    lw = int(S * 0.0859)
    d.line(pts, fill=WHITE, width=lw, joint="curve")
    for p in pts:
        d.ellipse([p[0] - lw / 2, p[1] - lw / 2, p[0] + lw / 2, p[1] + lw / 2], fill=WHITE)

    return card.resize((1024, 1024), Image.LANCZOS)


def preview(src: Image.Image) -> None:
    sizes = [256, 128, 64, 48, 32, 24, 16]
    gap = 24
    width = sum(sizes) + gap * (len(sizes) + 1)
    height = max(sizes) + gap * 2
    sheet = Image.new("RGBA", (width, height), (242, 244, 247, 255))
    x = gap
    for s in sizes:
        small = src.resize((s, s), Image.LANCZOS)
        top = gap + (max(sizes) - s) // 2
        sheet.alpha_composite(small, (x, top))
        x += s + gap
    sheet.save(OUT / "icon-sizes.png")


if __name__ == "__main__":
    icon = build()
    icon.save(OUT / "app-icon.png")
    preview(icon)
    print("已生成", OUT / "app-icon.png")
    print("已生成", OUT / "icon-sizes.png")
