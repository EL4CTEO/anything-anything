"""Renders the anything-anything logo: tiles nested in tiles, each a little smaller and turned, spiralling
into a spark. The tool that makes the tools.

    python scripts/logo.py      (needs Pillow; Windows fonts by default, set AA_FONT elsewhere)

Writes docs/assets/: logo.png (1024), logo-256.png, mark.png (512), wordmark.png, social.png (1280x640),
and .claude-plugin/icon.png.
"""

import math
import os
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "docs" / "assets"
FONT = os.environ.get("AA_FONT", "C:/Windows/Fonts/segoeuib.ttf")
S = 4096  # draw big, scale down: smooth edges without antialiasing tricks

VIOLET = (124, 58, 237)
MAGENTA = (219, 39, 119)
INK = (17, 12, 34)


def lerp(a, b, t):
    return tuple(round(x + (y - x) * t) for x, y in zip(a, b, strict=True))


def rounded_square(cx, cy, size, angle, radius):
    """The outline of a rounded square, turned by angle, as a polygon."""
    half = size / 2
    r = min(radius, half)
    pts = []
    for qx, qy, start in ((1, 1, 0), (-1, 1, 90), (-1, -1, 180), (1, -1, 270)):
        ox, oy = qx * (half - r), qy * (half - r)
        for step in range(0, 91, 6):
            a = math.radians(start + step)
            pts.append((ox + r * math.cos(a), oy + r * math.sin(a)))
    ca, sa = math.cos(angle), math.sin(angle)
    return [(cx + x * ca - y * sa, cy + x * sa + y * ca) for x, y in pts]


def spark(draw, cx, cy, r, fill):
    pts = []
    for i in range(8):
        a = math.radians(i * 45 - 90)
        rr = r if i % 2 == 0 else r * 0.28
        pts.append((cx + rr * math.cos(a), cy + rr * math.sin(a)))
    draw.polygon(pts, fill=fill)


def mark(size):
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    c = S / 2
    tile = S * 0.94
    layers = 7
    shrink = 0.78
    turn = math.radians(9)
    for i in range(layers):
        t = i / (layers - 1)
        side = tile * shrink**i
        poly = rounded_square(c, c, side, turn * i, side * 0.24)
        fill = lerp(VIOLET, MAGENTA, t) if i % 2 == 0 else lerp(INK, (46, 16, 72), t)
        d.polygon(poly, fill=fill + (255,))
    # a soft glow behind the spark
    glow = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow)
    gr = tile * shrink ** (layers - 1) * 0.55
    gd.ellipse([c - gr, c - gr, c + gr, c + gr], fill=(255, 255, 255, 150))
    glow = glow.filter(ImageFilter.GaussianBlur(S * 0.02))
    img = Image.alpha_composite(img, glow)
    spark(ImageDraw.Draw(img), c, c, tile * shrink ** (layers - 1) * 0.42, (255, 255, 255, 255))
    return img.resize((size, size), Image.LANCZOS)


def text_image(text, height, fills):
    """Text in several colours: [(text, rgb)], at a cap height of height pixels."""
    font = ImageFont.truetype(FONT, int(height * 1.35))
    widths = [font.getlength(t) for t, _ in fills]
    img = Image.new("RGBA", (int(sum(widths)) + 8, int(height * 1.9)), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    x = 0
    for (t, rgb), w in zip(fills, widths, strict=True):
        d.text((x, 0), t, font=font, fill=rgb + (255,))
        x += w
    return img.crop(img.getbbox())


def fit(img, width):
    """Scaled down to fit width, if it doesn't."""
    if img.width <= width:
        return img
    return img.resize((width, round(img.height * width / img.width)), Image.LANCZOS)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    big = mark(1024)
    big.save(OUT / "logo.png")
    big.resize((256, 256), Image.LANCZOS).save(OUT / "logo-256.png")
    big.resize((512, 512), Image.LANCZOS).save(OUT / "mark.png")
    (ROOT / ".claude-plugin").mkdir(exist_ok=True)
    big.resize((256, 256), Image.LANCZOS).save(ROOT / ".claude-plugin" / "icon.png")

    # wordmark: mark + "anything-anything", the second word in the brand colour
    # one for dark backgrounds, one for light (README picks with prefers-color-scheme)
    for theme, first, dash, second in (
        ("dark", (240, 236, 255), (150, 140, 180), (192, 132, 252)),
        ("light", INK, (120, 110, 150), VIOLET),
    ):
        words = text_image("x", 120, [("anything", first), ("-", dash), ("anything", second)])
        m = big.resize((260, 260), Image.LANCZOS)
        wm = Image.new("RGBA", (m.width + 40 + words.width, 260), (0, 0, 0, 0))
        wm.alpha_composite(m, (0, 0))
        wm.alpha_composite(words, (m.width + 40, (260 - words.height) // 2))
        wm.save(OUT / f"wordmark-{theme}.png")

    # social card for GitHub
    card = Image.new("RGBA", (1280, 640), INK + (255,))
    halo = Image.new("RGBA", card.size, (0, 0, 0, 0))
    ImageDraw.Draw(halo).ellipse([-200, -260, 760, 700], fill=VIOLET + (90,))
    card = Image.alpha_composite(card, halo.filter(ImageFilter.GaussianBlur(120)))
    card.alpha_composite(big.resize((380, 380), Image.LANCZOS), (90, 130))
    room = 1280 - 530 - 70
    title = fit(text_image("x", 70, [("anything", (240, 236, 255)), ("-", (150, 140, 180)), ("anything", (192, 132, 252))]), room)
    card.alpha_composite(title, (530, 190))
    slogan = fit(text_image("x", 44, [("Literally anything.", (244, 114, 182))]), room)
    card.alpha_composite(slogan, (532, 190 + title.height + 34))
    sub = fit(text_image("x", 24, [("Describe it. Your agent builds it. Its own tests prove it works.", (190, 184, 214))]), room)
    card.alpha_composite(sub, (533, 190 + title.height + 34 + slogan.height + 30))
    card.convert("RGB").save(OUT / "social.png")
    print("wrote", ", ".join(p.name for p in sorted(OUT.glob("*.png"))))


if __name__ == "__main__":
    main()
