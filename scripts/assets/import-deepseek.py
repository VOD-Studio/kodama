#!/usr/bin/env python3
"""Rebuild the lossless reference atlas. Requires Pillow; never runs at runtime.

python3 scripts/assets/import-deepseek.py --source-dir /tmp/whale-frames --download
Without --download, all 22 original sheets must already exist in source-dir.
The checked-in source manifest pins each PNG by SHA-256.
"""

import argparse
import hashlib
import json
from pathlib import Path
import urllib.request

from PIL import Image


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-dir", required=True, type=Path)
    parser.add_argument("--download", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    out = root / "crates/core/kodama-term/src/companions/deepseek"
    manifest = json.loads((out / "SOURCES.json").read_text())
    args.source_dir.mkdir(parents=True, exist_ok=True)
    sheets = {}
    for animation in manifest["animations"]:
        file = args.source_dir / (animation["key"] + ".png")
        if args.download and not file.exists():
            request = urllib.request.Request(animation["url"], headers={
                "User-Agent": "Mozilla/5.0", "Referer": manifest["source_page"],
            })
            with urllib.request.urlopen(request, timeout=30) as response:
                file.write_bytes(response.read())
        if hashlib.sha256(file.read_bytes()).hexdigest() != animation["sha256"]:
            raise ValueError(f"Source hash mismatch: {file}")
        image = Image.open(file).convert("RGBA")
        if image.size != (69 * animation["frame_count"], 66):
            raise ValueError(f"Source dimensions mismatch: {file}")
        sheets[animation["key"]] = image

    colors = sorted({rgba[:3] for image in sheets.values()
                     for rgba in image.getdata() if rgba[3]})
    ids = {rgb: index + 1 for index, rgb in enumerate(colors)}
    keys = [ord(".")] + [key for key in range(33, 127) if key != ord(".")]
    if len(colors) >= len(keys):
        raise ValueError("The source exceeds the single-byte palette contract")
    blob, tables = bytearray(), []
    for animation in manifest["animations"]:
        image = sheets[animation["key"]]
        frames = []
        for frame, duration in enumerate(animation["durations_ms"]):
            pixels = []
            for y in range(66):
                for x in range(69):
                    rgba = image.getpixel((frame * 69 + x, y))
                    if rgba[3] not in (0, 255):
                        raise ValueError("Partial alpha cannot be represented losslessly")
                    pixels.append(ids[rgba[:3]] if rgba[3] else 0)
            offset, last, run = len(blob), pixels[0], 0
            for color in pixels:
                if color == last and run < 255:
                    run += 1
                else:
                    blob.extend((run, last))
                    last, run = color, 1
            blob.extend((run, last))
            decoded = []
            for count, color in zip(blob[offset::2], blob[offset + 1::2]):
                decoded.extend([color] * count)
            if decoded != pixels:
                raise ValueError("RLE round-trip changed source pixels")
            dx, dy = animation["head_offsets"][frame]
            frames.append((duration, offset, len(blob) - offset, dx, dy))
        tables.append((animation["key"], frames))

    lines = ["// Generated losslessly from source PNGs; see SOURCES.json and scripts/assets/import-deepseek.py.",
             'pub(super) const DATA: &[u8] = include_bytes!("frames.bin");',
             "pub(super) const COLORS: &[[u8; 3]] = &["]
    lines += [f"    [{r}, {g}, {b}]," for r, g, b in [(0, 0, 0)] + colors]
    lines += ["];", "pub(super) const KEYS: &[u8] = &[" +
              ",".join(str(key) for key in keys[:len(colors) + 1]) + "];",
              "pub(super) type Frame = (u16, usize, usize, i8, i8);",
              "pub(super) const ANIMATIONS: &[(&str, &[Frame])] = &["]
    for name, frames in tables:
        lines.append(f'    ("{name}", &[')
        lines += ["        (" + ", ".join(str(value) for value in frame) + ")," for frame in frames]
        lines.append("    ]),")
    lines.append("];")
    (out / "frames.bin").write_bytes(blob)
    (out / "data.rs").write_text("\n".join(lines) + "\n")
    print(f"Rebuilt {len(tables)} animations / {sum(len(t[1]) for t in tables)} frames / {len(blob)} bytes")


if __name__ == "__main__":
    main()
