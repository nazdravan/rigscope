"""Draws the rigscope icon from assets/theme.toml: a RAM module on a rounded badge.

Usage: python3 assets/icon.py            (writes into assets/ and docs/assets/)
Outputs: assets/icon.png (256), assets/rigscope.ico, assets/rigscope.icns,
         docs/assets/rigscope-mark.png (transparent), docs/assets/icon-{16,32}.png (checks).
"""
import pathlib
import tomllib

from PIL import Image, ImageDraw

ROOT = pathlib.Path(__file__).resolve().parent.parent
THEME = tomllib.loads((ROOT / "assets" / "theme.toml").read_text())


def rgb(hexstr):
    h = hexstr.lstrip("#")
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


def mix(a, b, t):
    return tuple(round(x + (y - x) * t) for x, y in zip(a, b))


N = 1024  # drawn large, scaled down
ACCENT, PRESS = rgb(THEME["accent"]), rgb(THEME["accent_press"])
LIGHT = rgb(THEME["bg_light"])
GOLD = rgb(THEME["warn"])


def badge():
    img = Image.new("RGBA", (N, N), (0, 0, 0, 0))
    grad = Image.new("RGBA", (N, N))
    gd = ImageDraw.Draw(grad)
    for y in range(N):
        gd.line([(0, y), (N, y)], fill=mix(ACCENT, PRESS, y / N) + (255,))
    mask = Image.new("L", (N, N), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, N - 1, N - 1], int(N * 0.22), fill=255)
    img.paste(grad, (0, 0), mask)
    return img


def module(d, x0, y0, w, h):
    """A RAM stick: light board, three dark chips, gold contacts, the key notch."""
    r = h * 0.08
    d.rounded_rectangle([x0, y0, x0 + w, y0 + h], r, fill=LIGHT + (255,))
    chip_w, gap = w * 0.24, w * 0.07
    cx = x0 + (w - (3 * chip_w + 2 * gap)) / 2
    for i in range(3):
        x = cx + i * (chip_w + gap)
        d.rounded_rectangle([x, y0 + h * 0.14, x + chip_w, y0 + h * 0.60], h * 0.05, fill=PRESS + (255,))
    pins, pin_gap = 9, w * 0.018
    pin_w = (w * 0.86 - (pins - 1) * pin_gap) / pins
    px = x0 + w * 0.07
    for i in range(pins):
        x = px + i * (pin_w + pin_gap)
        d.rectangle([x, y0 + h * 0.72, x + pin_w, y0 + h * 0.94], fill=GOLD + (255,))
    # The key notch, off-centre like on a real DIMM.
    nx = x0 + w * 0.58
    d.rectangle([nx, y0 + h * 0.70, nx + w * 0.05, y0 + h + 1], fill=(0, 0, 0, 0))


def mark(transparent):
    img = Image.new("RGBA", (N, N), (0, 0, 0, 0)) if transparent else badge()
    d = ImageDraw.Draw(img)
    module(d, N * 0.14, N * 0.30, N * 0.72, N * 0.40)
    return img


def main():
    assets, docs = ROOT / "assets", ROOT / "docs" / "assets"
    docs.mkdir(parents=True, exist_ok=True)
    img = mark(False)
    # The notch must show the badge through it, not transparency.
    base = badge()
    base.alpha_composite(img)
    icon = base.resize((256, 256), Image.LANCZOS)
    icon.save(assets / "icon.png", optimize=True)
    icon.save(assets / "rigscope.ico", sizes=[(s, s) for s in (16, 24, 32, 48, 64, 128, 256)])
    mac = Image.new("RGBA", (N, N), (0, 0, 0, 0))
    inner = round(N * 824 / 1024)  # Apple grid: badge at ~80 % of the square
    mac.paste(base.resize((inner, inner), Image.LANCZOS), ((N - inner) // 2, (N - inner) // 2))
    mac.save(assets / "rigscope.icns", sizes=[(s, s) for s in (16, 32, 64, 128, 256, 512, 1024)])
    mark(True).resize((512, 512), Image.LANCZOS).save(docs / "rigscope-mark.png", optimize=True)
    for s in (16, 32):
        base.resize((s, s), Image.LANCZOS).save(docs / f"icon-{s}.png")


if __name__ == "__main__":
    main()
