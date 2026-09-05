"""Derive every icon the project uses from OpenFlow's Touch glyph, darkened to near-black."""
import os, shutil
from PIL import Image

os.chdir(r"D:\CreateCompanion")
SRC = "assets/touch.png"  # copied from OpenFlow: frontend/public/modules/touch.png
os.makedirs("assets", exist_ok=True)

img = Image.open(SRC).convert("RGBA")
# The source is mid-grey (~#7e7b85); the icon should read as black. Keep the
# alpha (anti-aliased edges) and replace the colour with a near-black.
INK = (24, 24, 28)
r, g, b, a = img.split()
img = Image.merge("RGBA", (Image.new("L", img.size, INK[0]), Image.new("L", img.size, INK[1]), Image.new("L", img.size, INK[2]), a))
# Trim to the glyph, then pad to a square with ~8% margin so it sits well in a tray.
bbox = img.getbbox()
glyph = img.crop(bbox)
side = max(glyph.size)
pad = int(side * 0.08)
canvas = Image.new("RGBA", (side + 2 * pad, side + 2 * pad), (0, 0, 0, 0))
canvas.paste(glyph, (pad + (side - glyph.width) // 2, pad + (side - glyph.height) // 2))
canvas.save("assets/icon.png")

def sized(n):
    return canvas.resize((n, n), Image.LANCZOS)

# Tray: raw RGBA 32x32, embedded by tray.rs with include_bytes!.
open("assets/tray-32.rgba", "wb").write(sized(32).tobytes())

# Engine exe icon (Task Manager, Explorer, taskbar).
sized(256).save("assets/icon.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])

# Tauri window / bundle icons.
os.makedirs("ui/src-tauri/icons", exist_ok=True)
sized(32).save("ui/src-tauri/icons/32x32.png")
sized(128).save("ui/src-tauri/icons/128x128.png")
sized(256).save("ui/src-tauri/icons/128x128@2x.png")
sized(512).save("ui/src-tauri/icons/icon.png")
shutil.copyfile("assets/icon.ico", "ui/src-tauri/icons/icon.ico")

# Sidebar brand mark.
os.makedirs("ui/public", exist_ok=True)
sized(64).save("ui/public/icon.png")

print("source", img.size, "glyph bbox", bbox, "canvas", canvas.size)
for p in ["assets/icon.png", "assets/tray-32.rgba", "assets/icon.ico", "ui/src-tauri/icons/icon.ico", "ui/public/icon.png"]:
    print(f"{p}: {os.path.getsize(p)} bytes")
