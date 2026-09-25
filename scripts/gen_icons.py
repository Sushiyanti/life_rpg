"""
Generate the application icon set.

Kept as a tiny dependency-free script (stdlib zlib + struct only) so icon
regeneration never requires a network install or an image toolchain. Tauri's
bundler needs these files to exist for `cargo tauri build`.

Run:  python3 scripts/gen_icons.py
"""

from __future__ import annotations

import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parents[1] / "src-tauri" / "icons"

# Palette (RGBA) — matches src/styles/tokens.css
BACKGROUND = (15, 17, 22, 255)
GOLD = (224, 179, 74, 255)
PURPLE = (155, 107, 214, 255)


def _png(width: int, height: int, pixels: bytes) -> bytes:
    """Encode raw RGBA rows as a PNG."""
    raw = b"".join(
        b"\x00" + pixels[y * width * 4 : (y + 1) * width * 4] for y in range(height)
    )
    compressed = zlib.compress(raw, 9)

    def chunk(tag: bytes, data: bytes) -> bytes:
        return (
            struct.pack(">I", len(data))
            + tag
            + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", compressed)
        + chunk(b"IEND", b"")
    )


def render(size: int) -> bytes:
    """A rounded dark tile with a gold/purple diamond sigil."""
    pixels = bytearray()
    cx = cy = (size - 1) / 2
    radius = size / 2
    diamond = size * 0.42

    for y in range(size):
        for x in range(size):
            # Rounded-square tile mask.
            corner = max(abs(x + 0.5 - cx), abs(y + 0.5 - cy))
            in_tile = corner <= radius * 0.94

            dx = abs(x + 0.5 - cx) + abs(y + 0.5 - cy)
            color = BACKGROUND

            if in_tile and dx <= diamond:
                # Vertical gold -> purple gradient across the sigil.
                t = (y + 0.5) / size
                color = tuple(
                    round(GOLD[i] * (1 - t) + PURPLE[i] * t) for i in range(3)
                ) + (255,)
            elif not in_tile:
                color = (0, 0, 0, 0)

            pixels.extend(color)

    return _png(size, size, bytes(pixels))


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    targets = {
        "32x32.png": 32,
        "128x128.png": 128,
        "128x128@2x.png": 256,
        "icon.png": 512,
    }
    for name, size in targets.items():
        path = OUT / name
        path.write_bytes(render(size))
        print(f"wrote {path.relative_to(OUT.parents[1])} ({size}x{size})")


if __name__ == "__main__":
    main()
