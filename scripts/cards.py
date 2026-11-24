"""Renders the share cards (Reddit, X): a hero card and a terminal card from a real transcript.

    python scripts/cards.py [transcript.txt]      (needs Pillow; Windows fonts by default)

Writes docs/assets/card-hero.png and, with a transcript, docs/assets/card-terminal.png.
"""

import os
import re
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "docs" / "assets"
SANS = os.environ.get("AA_FONT", "C:/Windows/Fonts/segoeuib.ttf")
SANS_REG = os.environ.get("AA_FONT_REGULAR", "C:/Windows/Fonts/segoeui.ttf")
ITALIC = os.environ.get("AA_FONT_ITALIC", "C:/Windows/Fonts/segoeuii.ttf")
MONO = os.environ.get("AA_MONO", "C:/Windows/Fonts/consola.ttf")
SYMBOL = os.environ.get("AA_SYMBOL", "C:/Windows/Fonts/seguisym.ttf")

K = 2  # draw at 2x, scale down
W, H = 1600, 1000
BG = (13, 15, 20)
PANEL = (22, 25, 33)
EDGE = (48, 54, 66)
TEXT = (236, 238, 244)
MUTED = (150, 156, 170)
VIOLET = (167, 112, 255)
PINK = (244, 114, 182)
GREEN = (63, 185, 80)
LINK = (110, 168, 254)


def font(path, size):
    return ImageFont.truetype(path, size * K)


def panel():
    img = Image.new("RGB", (W * K, H * K), BG)
    d = ImageDraw.Draw(img)
    m = 40 * K
    d.rounded_rectangle([m, m, W * K - m, H * K - m], radius=36 * K, fill=PANEL, outline=EDGE, width=2 * K)
    return img, d


def center_text(d, y, parts, f):
    """parts: [(text, rgb)] centred on one line at y."""
    total = sum(d.textlength(t, font=f) for t, _ in parts)
    x = (W * K - total) / 2
    for t, rgb in parts:
        d.text((x, y), t, font=f, fill=rgb)
        x += d.textlength(t, font=f)


def gradient_text(img, y, text, f, left, right):
    """One word filled with a horizontal gradient, centred, starting at x offset returned."""
    d = ImageDraw.Draw(img)
    w = d.textlength(text, font=f)
    return w


def badge(d, x, y, label, value, color):
    f = font(SANS_REG, 19)
    pad = 12 * K
    lw = d.textlength(label, font=f) + pad * 2
    vw = d.textlength(value, font=f) + pad * 2
    h = 34 * K
    d.rounded_rectangle([x, y, x + lw + vw, y + h], radius=6 * K, fill=(85, 85, 85))
    d.rounded_rectangle([x + lw, y, x + lw + vw, y + h], radius=6 * K, fill=color)
    d.rectangle([x + lw, y, x + lw + 8 * K, y + h], fill=color)
    d.text((x + pad, y + 5 * K), label, font=f, fill=(255, 255, 255))
    d.text((x + lw + pad, y + 5 * K), value, font=f, fill=(255, 255, 255))
    return lw + vw


def hero():
    img, d = panel()
    logo = Image.open(OUT / "logo.png").convert("RGBA").resize((190 * K, 190 * K), Image.LANCZOS)
    glow = Image.new("RGBA", img.size, (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow)
    cx, cy = W * K // 2, 175 * K
    gd.ellipse([cx - 220 * K, cy - 160 * K, cx + 220 * K, cy + 160 * K], fill=(124, 58, 237, 70))
    img = Image.alpha_composite(img.convert("RGBA"), glow.filter(ImageFilter.GaussianBlur(70 * K)))
    img.alpha_composite(logo, (cx - logo.width // 2, 80 * K))
    d = ImageDraw.Draw(img)

    title = font(SANS, 104)
    center_text(d, 285 * K, [("anything", TEXT), ("-anything", VIOLET)], title)
    d.line([(330 * K, 445 * K), ((W - 330) * K, 445 * K)], fill=EDGE, width=2 * K)
    center_text(d, 470 * K, [("Literally anything.", TEXT)], font(ITALIC, 40))
    center_text(
        d,
        535 * K,
        [("Describe it in a line · your agent builds it · its own tests prove it works", MUTED)],
        font(SANS_REG, 25),
    )
    badges = [("version", "0.1.0", VIOLET), ("stacks", "5", (219, 39, 119)), ("works with", "any agent", VIOLET), ("tests", "passing", GREEN), ("license", "MIT", VIOLET)]
    f = font(SANS_REG, 19)
    widths = [d.textlength(a, font=f) + d.textlength(b, font=f) + 48 * K for a, b, _ in badges]
    gap = 14 * K
    x = (W * K - sum(widths) - gap * (len(badges) - 1)) / 2
    for (a, b, c), w in zip(badges, widths, strict=True):
        badge(d, x, 600 * K, a, b, c)
        x += w + gap
    center_text(
        d,
        680 * K,
        [("Rust", TEXT), (" · ", MUTED), ("Python", TEXT), (" · ", MUTED), ("TypeScript", TEXT), (" · ", MUTED), ("Web & games", TEXT), (" · ", MUTED), ("any stack", TEXT), ("  ·  ", MUTED), ("one command", PINK)],
        font(SANS, 30),
    )
    small = font(SANS_REG, 21)
    link = "github.com/EL4CTEO/anything-anything"
    rest = "   ·   CLI + MCP server + Claude Code plugin   ·   npm i -g @el4cteo/anything-anything"
    total = d.textlength(link, font=small) + d.textlength(rest, font=small)
    x = (W * K - total) / 2
    y = 800 * K
    d.text((x, y), link, font=small, fill=LINK)
    lw = d.textlength(link, font=small)
    d.line([(x, y + 30 * K), (x + lw, y + 30 * K)], fill=LINK, width=K)
    d.text((x + lw, y), rest, font=small, fill=MUTED)
    img.convert("RGB").resize((W, H), Image.LANCZOS).save(OUT / "card-hero.png")


COLORS = {"✓": GREEN, "✗": (248, 81, 73), "▸": VIOLET, "!": (210, 153, 34), "$": GREEN}


def terminal(transcript):
    lines = [re.sub(r"\x1b\[[0-9;]*m", "", l.rstrip()) for l in Path(transcript).read_text(encoding="utf-8").splitlines()]
    img, d = panel()
    m = 40 * K
    bar = 64 * K
    d.rounded_rectangle([m, m, W * K - m, m + bar], radius=36 * K, fill=(30, 34, 44))
    d.rectangle([m, m + bar - 36 * K, W * K - m, m + bar], fill=(30, 34, 44))
    for i, c in enumerate([(255, 95, 86), (255, 189, 46), (39, 201, 63)]):
        x = m + (34 + i * 30) * K
        d.ellipse([x, m + 24 * K, x + 16 * K, m + 40 * K], fill=c)
    tf = font(SANS_REG, 20)
    center_text(d, m + 18 * K, [("anything-anything", MUTED)], tf)
    mono = font(MONO, 23)
    sym = font(SYMBOL, 22)
    y = m + bar + 26 * K
    step = 31 * K
    for line in lines[: int((H * K - y - 70 * K) / step)]:
        x = m + 40 * K
        stripped = line.lstrip()
        color = TEXT
        if stripped[:1] in COLORS:
            color = COLORS[stripped[:1]]
        if stripped.startswith("$ "):
            d.text((x, y), "$", font=mono, fill=GREEN)
            d.text((x + d.textlength("$ ", font=mono), y), stripped[2:], font=mono, fill=TEXT)
        else:
            for ch in line:
                f = sym if ch in "✓✗▸✦·" else mono
                fill = color if ch in "✓✗▸✦!" else (MUTED if line.startswith("    ") else TEXT)
                if ch == "✦":
                    fill = VIOLET
                d.text((x, y - (6 * K if f is sym else 0)), ch, font=f, fill=fill)
                x += d.textlength("M", font=mono)
        y += step
    img.convert("RGB").resize((W, H), Image.LANCZOS).save(OUT / "card-terminal.png")


if __name__ == "__main__":
    hero()
    if len(sys.argv) > 1:
        terminal(sys.argv[1])
    print("wrote", ", ".join(p.name for p in sorted(OUT.glob("card-*.png"))))
