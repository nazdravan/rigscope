"""Draws the README banner from assets/theme.toml in the app's own style: the name next to the
Memory panel's slot strip (filled = module, outline = empty slot).

Usage: python3 assets/banner.py    ->  docs/assets/rigscope-banner.png (2430x420, transparent)
                                       docs/assets/rigscope-og.png (1200x630, social preview)
"""
import pathlib
import tomllib

from PIL import Image, ImageDraw, ImageFont

ROOT = pathlib.Path(__file__).resolve().parent.parent
THEME = tomllib.loads((ROOT / "assets" / "theme.toml").read_text())
FONT = ROOT / "assets" / "fonts" / "NotoSans-Bold.ttf"


def rgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


W, H = 2430, 420
ACCENT, PRESS, WARN = rgb(THEME["accent"]), rgb(THEME["accent_press"]), rgb(THEME["warn"])


def slot(d, x, y, w, h, filled, label, font):
    r = 18
    if filled:
        d.rounded_rectangle([x, y, x + w, y + h], r, fill=PRESS + (255,), outline=ACCENT + (255,), width=6)
        # gold contacts along the bottom edge, like the logo's module
        pins = 9
        pw = (w - 60) / pins
        for i in range(pins):
            px = x + 30 + i * pw
            d.rectangle([px + 4, y + h - 34, px + pw - 4, y + h - 14], fill=WARN + (255,))
        fill = (255, 255, 255, 255)
    else:
        d.rounded_rectangle([x, y, x + w, y + h], r, outline=ACCENT + (255,), width=6)
        fill = ACCENT + (255,)
    tw = d.textlength(label, font=font)
    d.text((x + (w - tw) / 2, y + h * 0.18), label, font=font, fill=fill)


def main():
    img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    name = ImageFont.truetype(str(FONT), 230)
    small = ImageFont.truetype(str(FONT), 64)
    d.text((40, 40), THEME["name"], font=name, fill=ACCENT + (255,))
    x0, y0, sw, sh, gap = 1270, 105, 250, 200, 30
    for i, (filled, label) in enumerate([(True, "16 GiB"), (True, "16 GiB"), (False, "Empty"), (False, "Empty")]):
        slot(d, x0 + i * (sw + gap), y0, sw, sh, filled, label, small)
    out = ROOT / "docs" / "assets" / "rigscope-banner.png"
    out.parent.mkdir(parents=True, exist_ok=True)
    img.save(out, optimize=True)


def og():
    """1200x630 social preview: the banner's name and slot strip on the dark background."""
    w, h = 1200, 630
    img = Image.new("RGBA", (w, h), rgb(THEME["bg_dark"]) + (255,))
    d = ImageDraw.Draw(img)
    name = ImageFont.truetype(str(FONT), 190)
    small = ImageFont.truetype(str(FONT), 44)
    d.text((70, 90), THEME["name"], font=name, fill=ACCENT + (255,))
    for i, (filled, label) in enumerate([(True, "16 GiB"), (True, "16 GiB"), (False, "Empty"), (False, "Empty")]):
        slot(d, 70 + i * 270, 380, 240, 170, filled, label, small)
    out = ROOT / "docs" / "assets" / "rigscope-og.png"
    img.convert("RGB").save(out, optimize=True)


if __name__ == "__main__":
    main()
    og()
