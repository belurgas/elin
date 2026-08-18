#!/usr/bin/env python3
"""Generate Elin app icons and installer bitmaps from docs/logo.png."""

from __future__ import annotations

import io
import struct
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ICONS = ROOT / "src-tauri" / "icons"
WINDOWS = ROOT / "src-tauri" / "windows"
PUBLIC = ROOT / "public"
DOCS = ROOT / "docs"
LOGO = DOCS / "logo.png"

INK = (11, 10, 18)
PANEL = (26, 16, 40)
VIOLET = (196, 181, 253)


def require_pil():
    try:
        from PIL import Image, ImageDraw, ImageFilter  # noqa: F401
    except ImportError:
        sys.stderr.write("Pillow is required. Install with: python -m pip install Pillow\n")
        sys.exit(1)
    from PIL import Image, ImageDraw, ImageFilter

    return Image, ImageDraw, ImageFilter


def load_logo(Image):
    if not LOGO.exists():
        sys.stderr.write(f"Missing {LOGO}\n")
        sys.exit(1)
    img = Image.open(LOGO).convert("RGBA")
    return img


def cover_square(Image, src, size: int, resample):
    """Scale the mark to fill a square, then center-crop."""
    w, h = src.size
    side = min(w, h)
    left = (w - side) // 2
    top = (h - side) // 2
    cropped = src.crop((left, top, left + side, top + side))
    return cropped.resize((size, size), resample)


def write_png_file(path: Path, img) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    img.save(path, "PNG", optimize=True)
    print(f"  {path.relative_to(ROOT)} {img.size[0]}x{img.size[1]}")


def png_bytes(img) -> bytes:
    buf = io.BytesIO()
    img.save(buf, "PNG", optimize=True)
    return buf.getvalue()


def write_ico(path: Path, master, Image, sizes: tuple[int, ...]) -> None:
    frames = [cover_square(Image, master, s, Image.Resampling.LANCZOS) for s in sizes]
    path.parent.mkdir(parents=True, exist_ok=True)
    frames[0].save(
        path,
        format="ICO",
        sizes=[(s, s) for s in sizes],
        append_images=frames[1:],
    )
    print(f"  {path.relative_to(ROOT)} ico {sizes}")


def write_icns(path: Path, master, Image) -> None:
    """PNG-in-ICNS, the format modern macOS actually reads."""
    kinds = (
        ("ic11", 32),
        ("ic12", 64),
        ("ic07", 128),
        ("ic08", 256),
        ("ic13", 256),
        ("ic09", 512),
        ("ic14", 512),
        ("ic10", 1024),
    )
    chunks = bytearray()
    for tag, size in kinds:
        payload = png_bytes(cover_square(Image, master, size, Image.Resampling.LANCZOS))
        chunks.extend(tag.encode("ascii"))
        chunks.extend(struct.pack(">I", 8 + len(payload)))
        chunks.extend(payload)
    body = bytes(chunks)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("wb") as f:
        f.write(b"icns")
        f.write(struct.pack(">I", 8 + len(body)))
        f.write(body)
    print(f"  {path.relative_to(ROOT)} icns")


def fill_rgb(w: int, h: int, color: tuple[int, int, int]) -> bytearray:
    buf = bytearray(w * h * 3)
    r, g, b = color
    for i in range(w * h):
        buf[i * 3 : i * 3 + 3] = bytes((r, g, b))
    return buf


def write_bmp24(path: Path, w: int, h: int, rgb_top_down: bytes) -> None:
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
    print(f"  {path.relative_to(ROOT)} bmp {w}x{h}")


def paste_mark(Image, canvas, mark, box: tuple[int, int, int, int]) -> None:
    x0, y0, x1, y1 = box
    w, h = x1 - x0, y1 - y0
    fitted = cover_square(Image, mark, max(w, h), Image.Resampling.LANCZOS)
    if fitted.size != (w, h):
        fitted = fitted.resize((w, h), Image.Resampling.LANCZOS)
    canvas.paste(fitted, (x0, y0), fitted)


def installer_panel(Image, ImageDraw, ImageFilter, w: int, h: int, mark, accent: bool) -> object:
    img = Image.new("RGB", (w, h), INK)
    overlay = Image.new("RGB", (w, h), PANEL)
    img = Image.blend(img, overlay, 0.35)
    glow = Image.new("L", (w, h), 0)
    draw = ImageDraw.Draw(glow)
    if accent:
        draw.ellipse((-w * 0.2, -h * 0.3, w * 1.1, h * 0.9), fill=90)
    else:
        draw.ellipse((int(w * 0.15), int(-h * 0.4), int(w * 1.2), int(h * 0.85)), fill=70)
    glow = glow.filter(ImageFilter.GaussianBlur(radius=max(8, min(w, h) // 8)))
    violet = Image.new("RGB", (w, h), (88, 40, 170))
    img = Image.composite(violet, img, glow)
    return img


def bmp_from_image(path: Path, img) -> None:
    rgb = img.convert("RGB")
    w, h = rgb.size
    raw = rgb.tobytes()
    write_bmp24(path, w, h, raw)


def write_noise(path: Path, n: int = 128) -> None:
    buf = bytearray(n * n * 4)
    s = 0xC0FFEE
    for i in range(n * n):
        s ^= (s << 13) & 0xFFFFFFFF
        s ^= s >> 17
        s ^= (s << 5) & 0xFFFFFFFF
        g = s & 255
        buf[i * 4 : i * 4 + 4] = bytes((g, g, g, 255))

    def chunk(tag: bytes, data: bytes) -> bytes:
        crc = zlib.crc32(tag + data) & 0xFFFFFFFF
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", crc)

    raw = bytearray()
    stride = n * 4
    for y in range(n):
        raw.append(0)
        raw.extend(buf[y * stride : (y + 1) * stride])
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n")
        f.write(chunk(b"IHDR", struct.pack(">IIBBBBB", n, n, 8, 6, 0, 0, 0)))
        f.write(chunk(b"IDAT", zlib.compress(bytes(raw), 9)))
        f.write(chunk(b"IEND", b""))
    print(f"  {path.relative_to(ROOT)} {n}x{n}")


def main() -> None:
    Image, ImageDraw, ImageFilter = require_pil()
    ICONS.mkdir(parents=True, exist_ok=True)
    WINDOWS.mkdir(parents=True, exist_ok=True)
    PUBLIC.mkdir(parents=True, exist_ok=True)

    logo = load_logo(Image)
    master = cover_square(Image, logo, 1024, Image.Resampling.LANCZOS)

    print("App icons")
    write_png_file(ICONS / "icon-1024.png", master)
    write_png_file(ICONS / "icon.png", master)
    for size in (16, 32, 64, 128, 256, 512):
        write_png_file(
            ICONS / f"{size}x{size}.png",
            cover_square(Image, master, size, Image.Resampling.LANCZOS),
        )
    write_png_file(ICONS / "128x128@2x.png", cover_square(Image, master, 256, Image.Resampling.LANCZOS))
    write_png_file(DOCS / "mark.png", cover_square(Image, master, 256, Image.Resampling.LANCZOS))
    write_png_file(PUBLIC / "elin.png", cover_square(Image, master, 256, Image.Resampling.LANCZOS))
    write_png_file(PUBLIC / "elin-32.png", cover_square(Image, master, 32, Image.Resampling.LANCZOS))

    print("Windows Store tiles")
    for size in (30, 44, 71, 89, 107, 142, 150, 284, 310):
        write_png_file(
            ICONS / f"Square{size}x{size}Logo.png",
            cover_square(Image, master, size, Image.Resampling.LANCZOS),
        )
    write_png_file(ICONS / "StoreLogo.png", cover_square(Image, master, 50, Image.Resampling.LANCZOS))

    print("ICO / ICNS")
    write_ico(ICONS / "icon.ico", master, Image, (16, 24, 32, 48, 64, 128, 256))
    write_ico(PUBLIC / "favicon.ico", master, Image, (16, 32, 48))
    write_icns(ICONS / "icon.icns", master, Image)

    print("NSIS / WiX bitmaps")
    sidebar = installer_panel(Image, ImageDraw, ImageFilter, 164, 314, master, True)
    paste_mark(Image, sidebar, master, (18, 36, 146, 164))
    bmp_from_image(WINDOWS / "nsis-sidebar.bmp", sidebar)

    header = installer_panel(Image, ImageDraw, ImageFilter, 150, 57, master, False)
    paste_mark(Image, header, master, (8, 6, 56, 51))
    bmp_from_image(WINDOWS / "nsis-header.bmp", header)

    wix_banner = installer_panel(Image, ImageDraw, ImageFilter, 493, 58, master, False)
    paste_mark(Image, wix_banner, master, (10, 5, 58, 53))
    bmp_from_image(WINDOWS / "wix-banner.bmp", wix_banner)

    wix_dialog = installer_panel(Image, ImageDraw, ImageFilter, 493, 312, master, True)
    paste_mark(Image, wix_dialog, master, (170, 48, 322, 200))
    bmp_from_image(WINDOWS / "wix-dialog.bmp", wix_dialog)

    print("Noise tile")
    write_noise(PUBLIC / "grain.png")

    print("Done.")


if __name__ == "__main__":
    main()
