#!/usr/bin/env python3
"""Generate Elin app icons and Windows installer bitmaps from the droplet mark."""

from __future__ import annotations

import math
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ICONS = ROOT / "src-tauri" / "icons"
WINDOWS = ROOT / "src-tauri" / "windows"
PUBLIC = ROOT / "public"
DOCS = ROOT / "docs"
FONTS_CSS_DIR = ROOT / "src" / "assets"


def lerp(a: float, b: float, t: float) -> float:
    return a + (b - a) * t


def mix(c0: tuple[float, float, float], c1: tuple[float, float, float], t: float) -> tuple[float, float, float]:
    t = max(0.0, min(1.0, t))
    return (lerp(c0[0], c1[0], t), lerp(c0[1], c1[1], t), lerp(c0[2], c1[2], t))


def smin(a: float, b: float, k: float) -> float:
    h = max(k - abs(a - b), 0.0) / k
    return min(a, b) - h * h * k * 0.25


def sd_circle(x: float, y: float, cx: float, cy: float, r: float) -> float:
    return math.hypot(x - cx, y - cy) - r


def sd_round_box(x: float, y: float, cx: float, cy: float, hx: float, hy: float, r: float) -> float:
    dx = abs(x - cx) - hx + r
    dy = abs(y - cy) - hy + r
    ox, oy = max(dx, 0.0), max(dy, 0.0)
    return math.hypot(ox, oy) + min(max(dx, dy), 0.0) - r


def sd_segment(x: float, y: float, ax: float, ay: float, bx: float, by: float, r: float) -> float:
    pax, pay = x - ax, y - ay
    bax, bay = bx - ax, by - ay
    h = max(0.0, min(1.0, (pax * bax + pay * bay) / (bax * bax + bay * bay + 1e-9)))
    return math.hypot(pax - bax * h, pay - bay * h) - r


def sd_droplet(x: float, y: float) -> float:
    """Normalized coords: icon content box 0..1, y down."""
    bulb = sd_circle(x, y, 0.50, 0.64, 0.255)
    cone = sd_segment(x, y, 0.50, 0.16, 0.50, 0.58, 0.018 + 0.22 * max(0.0, (y - 0.16) / 0.48))
    return smin(bulb, cone, 0.08)


def sd_highlight(x: float, y: float) -> float:
    return sd_circle(x, y, 0.58, 0.36, 0.055)


INK = (11 / 255, 10 / 255, 18 / 255)
PANEL = (26 / 255, 16 / 255, 40 / 255)
VIOLET = (196 / 255, 181 / 255, 253 / 255)
DEEP = (124 / 255, 58 / 255, 237 / 255)
GOLD = (253 / 255, 230 / 255, 138 / 255)
ROSE = (244 / 255, 63 / 255, 94 / 255)
MIST = (246 / 255, 243 / 255, 255 / 255)


def sample_icon(u: float, v: float, size: int) -> tuple[int, int, int, int]:
    # u,v in 0..1 of the square
    pad = 0.06
    x = (u - pad) / (1 - 2 * pad)
    y = (v - pad) / (1 - 2 * pad)
    box = sd_round_box(u, v, 0.5, 0.5, 0.5, 0.5, 0.22)
    drop = sd_droplet(x, y)
    hi = sd_highlight(x, y)
    inner = sd_droplet(0.5 + (x - 0.5) * 1.35, 0.08 + y * 0.92)

    aa = 1.5 / size
    cover = max(0.0, min(1.0, 0.5 - box / aa))
    if cover <= 0:
        return (0, 0, 0, 0)

    # panel fill
    gy = max(0.0, min(1.0, y))
    bg = mix(PANEL, INK, gy * 0.55)
    # faint rose glow top-right
    glow = math.exp(-((u - 0.82) ** 2 + (v - 0.18) ** 2) * 18)
    bg = mix(bg, ROSE, glow * 0.12)

    drop_c = max(0.0, min(1.0, 0.5 - drop / aa))
    grad = mix(VIOLET, DEEP, max(0.0, min(1.0, (y - 0.18) / 0.7)))
    rgb = mix(bg, grad, drop_c)

    inner_c = max(0.0, min(1.0, 0.5 - inner / aa)) * 0.22
    rgb = mix(rgb, MIST, inner_c)

    hi_c = max(0.0, min(1.0, 0.5 - hi / aa))
    rgb = mix(rgb, GOLD, hi_c * 0.92)

    a = int(round(cover * 255))
    return (
        int(round(rgb[0] * 255)),
        int(round(rgb[1] * 255)),
        int(round(rgb[2] * 255)),
        a,
    )


def render_rgba(w: int, h: int, sampler) -> bytearray:
    buf = bytearray(w * h * 4)
    i = 0
    for y in range(h):
        v = (y + 0.5) / h
        for x in range(w):
            u = (x + 0.5) / w
            r, g, b, a = sampler(u, v, max(w, h))
            buf[i : i + 4] = bytes((r, g, b, a))
            i += 4
    return buf


def write_png(path: Path, w: int, h: int, rgba: bytes) -> None:
    def chunk(tag: bytes, data: bytes) -> bytes:
        crc = zlib.crc32(tag + data) & 0xFFFFFFFF
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", crc)

    raw = bytearray()
    stride = w * 4
    for y in range(h):
        raw.append(0)
        raw.extend(rgba[y * stride : (y + 1) * stride])
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n")
        f.write(chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)))
        f.write(chunk(b"IDAT", zlib.compress(bytes(raw), 9)))
        f.write(chunk(b"IEND", b""))


def write_bmp24(path: Path, w: int, h: int, rgb_top_down: bytes) -> None:
    """24-bit BMP, BGR, bottom-up rows, 4-byte padded. No alpha."""
    row_stride = (w * 3 + 3) & ~3
    pixel_size = row_stride * h
    header = 54
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("wb") as f:
        f.write(b"BM")
        f.write(struct.pack("<IHHI", header + pixel_size, 0, 0, header))
        f.write(struct.pack("<IIIHHIIIIII", 40, w, h, 1, 24, 0, pixel_size, 2835, 2835, 0, 0))
        pad = b"\x00" * (row_stride - w * 3)
        for y in range(h - 1, -1, -1):
            row = bytearray()
            o = y * w * 3
            for x in range(w):
                r, g, b = rgb_top_down[o + x * 3 : o + x * 3 + 3]
                row.extend((b, g, r))
            f.write(row)
            if pad:
                f.write(pad)


def flatten_rgb(rgba: bytes, w: int, h: int, bg: tuple[int, int, int]) -> bytes:
    out = bytearray(w * h * 3)
    for i in range(w * h):
        r, g, b, a = rgba[i * 4 : i * 4 + 4]
        t = a / 255.0
        out[i * 3] = int(round(r * t + bg[0] * (1 - t)))
        out[i * 3 + 1] = int(round(g * t + bg[1] * (1 - t)))
        out[i * 3 + 2] = int(round(b * t + bg[2] * (1 - t)))
    return bytes(out)


GLYPHS = {
    "A": ["01110", "10001", "11111", "10001", "10001"],
    "B": ["11110", "10001", "11110", "10001", "11110"],
    "C": ["01111", "10000", "10000", "10000", "01111"],
    "D": ["11110", "10001", "10001", "10001", "11110"],
    "E": ["11111", "10000", "11110", "10000", "11111"],
    "F": ["11111", "10000", "11110", "10000", "10000"],
    "G": ["01111", "10000", "10111", "10001", "01110"],
    "H": ["10001", "10001", "11111", "10001", "10001"],
    "I": ["11111", "00100", "00100", "00100", "11111"],
    "J": ["00111", "00010", "00010", "10010", "01100"],
    "K": ["10001", "10010", "11100", "10010", "10001"],
    "L": ["10000", "10000", "10000", "10000", "11111"],
    "M": ["10001", "11011", "10101", "10001", "10001"],
    "N": ["10001", "11001", "10101", "10011", "10001"],
    "O": ["01110", "10001", "10001", "10001", "01110"],
    "P": ["11110", "10001", "11110", "10000", "10000"],
    "Q": ["01110", "10001", "10001", "10010", "01101"],
    "R": ["11110", "10001", "11110", "10010", "10001"],
    "S": ["01111", "10000", "01110", "00001", "11110"],
    "T": ["11111", "00100", "00100", "00100", "00100"],
    "U": ["10001", "10001", "10001", "10001", "01110"],
    "V": ["10001", "10001", "10001", "01010", "00100"],
    "W": ["10001", "10001", "10101", "11011", "10001"],
    "X": ["10001", "01010", "00100", "01010", "10001"],
    "Y": ["10001", "01010", "00100", "00100", "00100"],
    "Z": ["11111", "00010", "00100", "01000", "11111"],
    "-": ["00000", "00000", "11111", "00000", "00000"],
    ".": ["000", "000", "000", "000", "010"],
    " ": ["000", "000", "000", "000", "000"],
}


def blit_text(rgb: bytearray, w: int, h: int, text: str, x0: int, y0: int, scale: int, color: tuple[int, int, int]) -> None:
    x = x0
    for ch in text:
        g = GLYPHS.get(ch, GLYPHS[" "])
        gw = len(g[0])
        for gy, row in enumerate(g):
            for gx, bit in enumerate(row):
                if bit != "1":
                    continue
                for oy in range(scale):
                    for ox in range(scale):
                        px, py = x + gx * scale + ox, y0 + gy * scale + oy
                        if 0 <= px < w and 0 <= py < h:
                            o = (py * w + px) * 3
                            rgb[o : o + 3] = bytes(color)
        x += (gw + 1) * scale


def sample_sidebar(u: float, v: float, w: int, h: int) -> tuple[int, int, int]:
    # Dark aurora panel, droplet upper-center
    gy = v
    bg = mix(INK, PANEL, 0.35 + 0.25 * (1 - gy))
    glow_v = math.exp(-((u - 0.5) ** 2 * 6 + (v - 0.38) ** 2 * 4))
    bg = mix(bg, DEEP, glow_v * 0.45)
    glow_r = math.exp(-((u - 0.78) ** 2 * 10 + (v - 0.12) ** 2 * 8))
    bg = mix(bg, ROSE, glow_r * 0.18)
    # droplet in upper 62%
    dx = (u - 0.5) / 0.72 + 0.5
    dy = v / 0.62
    drop = sd_droplet(dx, dy)
    aa = 1.8 / min(w, h)
    drop_c = max(0.0, min(1.0, 0.5 - drop / aa))
    grad = mix(VIOLET, DEEP, max(0.0, min(1.0, dy)))
    rgb = mix(bg, grad, drop_c)
    hi = sd_highlight(dx, dy)
    rgb = mix(rgb, GOLD, max(0.0, min(1.0, 0.5 - hi / aa)) * 0.9)
    return (int(rgb[0] * 255), int(rgb[1] * 255), int(rgb[2] * 255))


def sample_og(u: float, v: float, w: int, h: int) -> tuple[int, int, int]:
    bg = mix(INK, PANEL, 0.22 + 0.45 * v)
    g1 = math.exp(-((u - 0.22) ** 2 * 7 + (v - 0.48) ** 2 * 4.2))
    bg = mix(bg, DEEP, g1 * 0.55)
    g2 = math.exp(-((u - 0.82) ** 2 * 9 + (v - 0.18) ** 2 * 7))
    bg = mix(bg, ROSE, g2 * 0.16)
    g3 = math.exp(-((u - 0.7) ** 2 * 5 + (v - 0.72) ** 2 * 6))
    bg = mix(bg, VIOLET, g3 * 0.12)
    side = 0.42
    dx = (u - 0.06) / side
    dy = (v - 0.14) / 0.72
    drop = sd_droplet(dx, dy)
    aa = 1.4 / min(w, h)
    drop_c = max(0.0, min(1.0, 0.5 - drop / aa)) if -0.05 <= dx <= 1.08 else 0.0
    grad = mix(VIOLET, DEEP, max(0.0, min(1.0, dy)))
    rgb = mix(bg, grad, drop_c)
    hi = sd_highlight(dx, dy)
    rgb = mix(rgb, GOLD, max(0.0, min(1.0, 0.5 - hi / aa)) * 0.88 * drop_c)
    return (int(rgb[0] * 255), int(rgb[1] * 255), int(rgb[2] * 255))


def rgb_to_rgba(rgb: bytes, w: int, h: int) -> bytes:
    out = bytearray(w * h * 4)
    for i in range(w * h):
        out[i * 4 : i * 4 + 3] = rgb[i * 3 : i * 3 + 3]
        out[i * 4 + 3] = 255
    return bytes(out)


def _segoe(size: int, bold: bool = False):
    from PIL import ImageFont

    names = ("segoeuib.ttf", "segoeui.ttf") if bold else ("segoeui.ttf", "segoeuib.ttf")
    for name in names:
        path = Path(r"C:\Windows\Fonts") / name
        if path.exists():
            return ImageFont.truetype(str(path), size)
    return ImageFont.load_default()


def write_readme_banner(w: int = 2560, h: int = 840) -> None:
    """Wide README art: SDF droplet + real Segoe text (not the 5x5 bitmap font)."""
    from PIL import Image, ImageDraw

    rgb = render_rgb(w, h, sample_og)
    img = Image.frombytes("RGB", (w, h), bytes(rgb))
    draw = ImageDraw.Draw(img, "RGBA")
    sx, sy = w / 1280, h / 420

    title = _segoe(int(92 * sy), bold=True)
    sub = _segoe(int(28 * sy), bold=True)
    pill = _segoe(int(15 * sy), bold=True)
    tx, ty = int(430 * sx), int(168 * sy)
    draw.text((tx, ty), "Elin", font=title, fill=(246, 243, 255), anchor="ls")
    draw.text((int(434 * sx), int(218 * sy)), "Elixir on Windows", font=sub, fill=(196, 181, 253), anchor="ls")

    labels = (("Installer", 434, 108), ("Studio", 554, 92), ("CLI", 658, 72))
    for label, x, width in labels:
        x0, y0 = int(x * sx), int(252 * sy)
        x1, y1 = x0 + int(width * sx), y0 + int(32 * sy)
        draw.rounded_rectangle(
            (x0, y0, x1, y1),
            radius=int(8 * sy),
            fill=(124, 58, 237, 56),
            outline=(167, 139, 250, 115),
            width=max(1, int(sy)),
        )
        draw.text(((x0 + x1) / 2, (y0 + y1) / 2), label, font=pill, fill=(221, 214, 254), anchor="mm")

    dest = DOCS / "banner.png"
    dest.parent.mkdir(parents=True, exist_ok=True)
    img.save(dest, "PNG", optimize=True)
    print(f"  {dest} {w}x{h}")


def sample_header(u: float, v: float, w: int, h: int) -> tuple[int, int, int]:
    bg = mix(INK, PANEL, 0.4)
    # small droplet on the left
    dx = (u - 0.02) / (57 / 150 * 0.9) * (h / w) * (w / h)
    # map a square on the left
    side = h / w
    dx = u / side
    dy = v
    drop = sd_droplet(dx, dy)
    aa = 1.6 / h
    drop_c = max(0.0, min(1.0, 0.5 - drop / aa)) if 0 <= dx <= 1 else 0.0
    grad = mix(VIOLET, DEEP, v)
    rgb = mix(bg, grad, drop_c)
    return (int(rgb[0] * 255), int(rgb[1] * 255), int(rgb[2] * 255))


def render_rgb(w: int, h: int, sampler) -> bytearray:
    buf = bytearray(w * h * 3)
    i = 0
    for y in range(h):
        v = (y + 0.5) / h
        for x in range(w):
            u = (x + 0.5) / w
            r, g, b = sampler(u, v, w, h)
            buf[i : i + 3] = bytes((r, g, b))
            i += 3
    return buf


def render_noise(n: int = 128) -> bytearray:
    buf = bytearray(n * n * 4)
    # xorshift-ish
    s = 0xC0FFEE
    for i in range(n * n):
        s ^= (s << 13) & 0xFFFFFFFF
        s ^= s >> 17
        s ^= (s << 5) & 0xFFFFFFFF
        g = s & 255
        buf[i * 4 : i * 4 + 4] = bytes((g, g, g, 255))
    return buf


def main() -> None:
    ICONS.mkdir(parents=True, exist_ok=True)
    WINDOWS.mkdir(parents=True, exist_ok=True)
    PUBLIC.mkdir(parents=True, exist_ok=True)

    print("Rendering 1024 icon")
    rgba = render_rgba(1024, 1024, sample_icon)
    write_png(ICONS / "icon-1024.png", 1024, 1024, rgba)
    write_png(ICONS / "icon.png", 1024, 1024, rgba)

    for size in (32, 128, 256):
        print(f"Rendering {size}")
        write_png(ICONS / f"{size}x{size}.png", size, size, render_rgba(size, size, sample_icon))
    write_png(ICONS / "128x128@2x.png", 256, 256, render_rgba(256, 256, sample_icon))

    print("NSIS sidebar 164x314")
    side = render_rgb(164, 314, sample_sidebar)
    blit_text(side, 164, 314, "ELIN", 46, 268, 3, (196, 181, 253))
    write_bmp24(WINDOWS / "nsis-sidebar.bmp", 164, 314, side)

    print("NSIS header 150x57")
    header = render_rgb(150, 57, sample_header)
    blit_text(header, 150, 57, "ELIN", 62, 18, 4, (232, 226, 247))
    write_bmp24(WINDOWS / "nsis-header.bmp", 150, 57, header)

    print("WiX banner 493x58")
    banner = render_rgb(493, 58, sample_header)
    blit_text(banner, 493, 58, "ELIN", 70, 18, 4, (232, 226, 247))
    write_bmp24(WINDOWS / "wix-banner.bmp", 493, 58, banner)

    print("WiX dialog 493x312")
    dialog = render_rgb(493, 312, sample_sidebar)
    blit_text(dialog, 493, 312, "ELIN", 190, 268, 4, (196, 181, 253))
    write_bmp24(WINDOWS / "wix-dialog.bmp", 493, 312, dialog)

    print("Noise tile")
    write_png(PUBLIC / "grain.png", 128, 128, render_noise(128))

    print("README banner")
    write_readme_banner()

    print("Done.")


if __name__ == "__main__":
    main()
