"""Render the README banner (dark and light) from the app icon and the wordmark.

    python tools/make_banner.py

Writes docs/media/banner-dark.png and docs/media/banner-light.png. Uses Segoe UI
when present (Windows); falls back to Pillow's bundled font otherwise.
"""
import os
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
os.chdir(ROOT)

W, H = 1600, 520
ICON = Image.open("assets/icon.png").convert("RGBA")


def font(size: int, bold: bool = False) -> ImageFont.FreeTypeFont:
    for name in (["segoeuib.ttf", "seguisb.ttf"] if bold else ["segoeui.ttf"]):
        p = Path(r"C:\Windows\Fonts") / name
        if p.exists():
            return ImageFont.truetype(str(p), size)
    return ImageFont.load_default(size=size)


def tinted(icon: Image.Image, rgb: tuple[int, int, int], size: int) -> Image.Image:
    a = icon.split()[3]
    out = Image.new("RGBA", icon.size, rgb + (0,))
    out.putalpha(a)
    return out.resize((size, size), Image.LANCZOS)


def render(theme: str) -> None:
    if theme == "dark":
        bg, ink, muted, accent = (17, 17, 20), (245, 245, 247), (160, 160, 170), (255, 122, 26)
    else:
        bg, ink, muted, accent = (250, 250, 252), (24, 24, 28), (100, 100, 110), (224, 96, 0)
    img = Image.new("RGB", (W, H), bg)
    d = ImageDraw.Draw(img)

    # Icon on the left, tinted to the theme's ink.
    size = 260
    icon = tinted(ICON, ink, size)
    ix, iy = 140, (H - size) // 2
    img.paste(icon, (ix, iy), icon)

    # Wordmark and tagline.
    x = ix + size + 80
    f_title = font(112, bold=True)
    f_tag = font(40)
    f_small = font(30)
    d.text((x, 118), "Create Companion", font=f_title, fill=ink)
    d.text((x, 262), "Your Naya Create modules, one profile per app.", font=f_tag, fill=muted)
    # Accent rule and platform line.
    d.rounded_rectangle((x, 336, x + 90, 342), radius=3, fill=accent)
    d.text((x, 366), "Windows  ·  macOS  ·  Tune and Touch", font=f_small, fill=muted)

    out = Path("docs/media") / f"banner-{theme}.png"
    img.save(out, optimize=True)
    print(out, os.path.getsize(out), "bytes")


for t in ("dark", "light"):
    render(t)
