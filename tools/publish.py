# -*- coding: utf-8 -*-
"""把构建产物整理成可直接发给别人的发布包。

产物：
  发布/对局确认助手.exe
  发布/WebView2Loader.dll
  对局确认助手.zip        ← 发这个

为什么要带 dll：exe 的导入表里写死了 WebView2Loader.dll，
单独拿出 exe 到别的目录会直接报「找不到 WebView2Loader.dll」
（objdump -p 可以验证）。所以发布包必须是这两个文件一起。

用法：python tools/publish.py
"""
import shutil
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RELEASE = ROOT / "src-tauri" / "target" / "release"
OUT = ROOT / "发布"
NAME = "对局确认助手"

SRC_EXE = RELEASE / "lol-auto-accept.exe"
SRC_DLL = RELEASE / "WebView2Loader.dll"

README = """对局确认助手

【怎么用】
1. 把整个文件夹解压出来，别只拿 exe —— 旁边的 WebView2Loader.dll 必须和它待在同一个文件夹里。
2. 双击「对局确认助手.exe」。
3. 打开英雄联盟客户端，软件会自动连上，显示你的头像、名字和等级。

【如果 Windows 拦截】
第一次打开可能会提示「Windows 已保护你的电脑」。原因是个人开发的小工具没有购买数字签名证书，
不是病毒。点「更多信息」→「仍要运行」就可以。

如果仍然不放心，可以把 exe 传到 https://www.virustotal.com 查一下，多引擎扫一遍。

【它做什么】
只读英雄联盟客户端自带的本地接口（127.0.0.1），排到对局时替你点「接受」。
不读游戏内存、不注入 DLL、不模拟键鼠操作，不向任何服务器上传数据。

【环境要求】
Windows 10 / 11，系统里有 WebView2 运行时（装了 Edge 就有，一般无需额外安装）。
软件不需要登录，也不会问你要账号密码。
"""


def main() -> int:
    if not SRC_EXE.exists():
        print("找不到构建产物：", SRC_EXE)
        print("先执行：npm run build，然后")
        print("  cargo build --release --features tauri/custom-protocol")
        return 1

    OUT.mkdir(exist_ok=True)
    files = []
    for src, dst in ((SRC_EXE, OUT / f"{NAME}.exe"), (SRC_DLL, OUT / SRC_DLL.name)):
        if not src.exists():
            print("警告：缺少", src.name, "，发布包可能无法运行")
            continue
        shutil.copy2(src, dst)
        files.append(dst)
        print(f"  {dst.relative_to(ROOT)}  {dst.stat().st_size / 1024 / 1024:.2f} MB")

    readme = OUT / "使用说明.txt"
    readme.write_text(README, encoding="utf-8-sig")  # 带 BOM，老版记事本也不乱码
    files.append(readme)
    print(f"  {readme.relative_to(ROOT)}  {readme.stat().st_size / 1024:.1f} KB")

    zip_path = ROOT / f"{NAME}.zip"
    with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as zf:
        for dst in files:
            # 解压后是一个文件夹，避免文件散落在桌面
            zf.write(dst, f"{NAME}/{dst.name}")

    print(f"\n打包完成：{zip_path}  {zip_path.stat().st_size / 1024 / 1024:.2f} MB")
    return 0


if __name__ == "__main__":
    sys.exit(main())
