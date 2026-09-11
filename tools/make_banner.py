"""Render the README banners (dark and light) and the social preview card from
the app icon and the wordmark.

    python tools/make_banner.py

Writes docs/media/banner-dark.png, banner-light.png (1600x520) and
social-preview.png (1280x640, the size GitHub's Social preview setting wants).
Uses Segoe UI when present (Windows); falls back to Pillow's bundled font.
"""
import os
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
os.chdir(ROOT)

TAGLINE = ("Your Modules. Your Apps. Dynamic Gestures.", "For whatever you're working on.")
PLATFORMS = "Windows  -  macOS  -  Naya Create Tune & Touch"

ICON = Image.open("assets/icon.png").convert("RGBA")
THEMES = {
    "dark": dict(bg=(17, 17, 20), ink=(245, 245, 247), muted=(160, 160, 170), accent=(255, 122, 26)),
    "light": dict(bg=(250, 250, 252), ink=(24, 24, 28), muted=(100, 100, 110), accent=(224, 96, 0)),
}


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


def render(out: Path, w: int, h: int, theme: str, scale: float = 1.0, extra: str = "") -> None:
    c = THEMES[theme]
    img = Image.new("RGB", (w, h), c["bg"])
    d = ImageDraw.Draw(img)

    size = int(250 * scale)
    icon = tinted(ICON, c["ink"], size)
    ix, iy = int(120 * scale), (h - size) // 2
    img.paste(icon, (ix, iy), icon)

    x = ix + size + int(70 * scale)
    f_title = font(int(96 * scale), bold=True)
    f_tag = font(int(36 * scale))
    f_small = font(int(27 * scale))
    top = (h - int(330 * scale)) // 2
    d.text((x, top), "Create Companion", font=f_title, fill=c["ink"])
    # Tagline: the three-beat line, the accent rule right under it, then the
    # second line, then the platforms.
    y = top + int(122 * scale)
    d.text((x, y), TAGLINE[0], font=f_tag, fill=c["ink"])
    y += int(56 * scale)
    d.rounded_rectangle((x, y, x + int(90 * scale), y + int(6 * scale)), radius=3, fill=c["accent"])
    y += int(22 * scale)
    d.text((x, y), TAGLINE[1], font=f_tag, fill=c["muted"])
    y += int(66 * scale)
    d.text((x, y), PLATFORMS + extra, font=f_small, fill=c["muted"])

    img.save(out, optimize=True)
    print(out, os.path.getsize(out), "bytes")


render(Path("docs/media/banner-dark.png"), 1600, 520, "dark", scale=1.12)
render(Path("docs/media/banner-light.png"), 1600, 520, "light", scale=1.12)
render(Path("docs/media/social-preview.png"), 1280, 640, "dark", scale=0.86, extra="  -  MIT")
