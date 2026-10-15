#!/usr/bin/env python3
"""Import the original Deepy PNG sheets without redrawing or quantizing pixels.

Requires Pillow and rustfmt. Source URLs and hashes are pinned in SOURCES.json.
python3 scripts/assets/import-deepy.py --source-dir /tmp/deepy --download
Add --check to compare regenerated data without modifying the checked-in files.
"""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import urllib.request

from PIL import Image


def flat_pixels(image):
    return getattr(image, "get_flattened_data", image.getdata)()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-dir", type=Path, required=True)
    parser.add_argument("--download", action="store_true")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    out = root / "crates/core/kodama-term/src/companions/deepy"
    manifest = json.loads((out / "SOURCES.json").read_text())
    width, height = manifest["source_frame_width"], manifest["source_frame_height"]
    args.source_dir.mkdir(parents=True, exist_ok=True)
    sheets = {}
    for animation in manifest["animations"]:
        path = args.source_dir / (animation["key"] + ".png")
        if args.download and not path.exists():
            request = urllib.request.Request(animation["url"], headers={
                "User-Agent": "Mozilla/5.0", "Referer": manifest["source_page"],
            })
            with urllib.request.urlopen(request, timeout=30) as response:
                path.write_bytes(response.read())
        if hashlib.sha256(path.read_bytes()).hexdigest() != animation["sha256"]:
            raise ValueError(f"Source hash mismatch: {path}")
        image = Image.open(path).convert("RGBA")
        sheet = animation["sheet"]
        if image.size != (sheet["cols"] * width, sheet["rows"] * height):
            raise ValueError(f"Source dimensions mismatch: {path}")
        sheets[animation["key"]] = image

    colors = sorted({rgba for image in sheets.values()
                     for _, rgba in image.getcolors(maxcolors=1 << 24)
                     if rgba != (0, 0, 0, 0)})
    if any(rgba[3] == 0 for rgba in colors):
        raise ValueError("Unexpected hidden RGB in a transparent source pixel")
    palette = [(0, 0, 0, 0)] + colors
    ids = {rgba: index for index, rgba in enumerate(palette)}
    if len(palette) > 65536:
        raise ValueError("Source exceeds the 16-bit lossless color table")

    blob, tables = bytearray(), []
    for animation in manifest["animations"]:
        image, cols = sheets[animation["key"]], animation["sheet"]["cols"]
        frames, source_hash, decoded_hash = [], hashlib.sha256(), hashlib.sha256()
        for frame in range(animation["frame_count"]):
            x, y = (frame % cols) * width, (frame // cols) * height
            original = image.crop((x, y, x + width, y + height))
            source_hash.update(original.tobytes())
            pixels = [ids[rgba] for rgba in flat_pixels(original)]
            display_colors = {pixels[(y * 5 + 2) * width + x * 5 + 2]
                              for y in range(52) for x in range(52)}
            if len(display_colors - {0}) > 93:
                raise ValueError("Display frame exceeds printable one-byte keys")
            offset, last, run = len(blob), pixels[0], 0
            for color in pixels:
                if color == last and run < 255:
                    run += 1
                else:
                    blob.extend((run, last & 255, last >> 8))
                    last, run = color, 1
            blob.extend((run, last & 255, last >> 8))
            decoded = []
            for count, low, high in zip(blob[offset::3], blob[offset + 1::3], blob[offset + 2::3]):
                color = low | (high << 8)
                decoded.extend([color] * count)
            if decoded != pixels:
                raise ValueError("RLE round-trip changed source indices")
            decoded_hash.update(bytes(channel for color in decoded for channel in palette[color]))
            frames.append((animation["duration_ms"], offset, len(blob) - offset))
        if source_hash.hexdigest() != animation["frames_rgba_sha256"]:
            raise ValueError(f"Source RGBA hash mismatch: {animation['key']}")
        if source_hash.digest() != decoded_hash.digest():
            raise ValueError(f"RLE changed source RGBA: {animation['key']}")
        tables.append((animation["key"], frames))

    lines = ["// Generated from original RGBA PNGs; see SOURCES.json and scripts/assets/import-deepy.py.",
             'pub(super) const DATA: &[u8] = include_bytes!("frames.bin");',
             "pub(super) const COLORS: &[[u8; 4]] = &["]
    lines += [f"    [{r}, {g}, {b}, {a}]," for r, g, b, a in palette]
    lines += ["];",
              "pub(super) type Frame = (u16, usize, usize);",
              "pub(super) const ANIMATIONS: &[(&str, &[Frame])] = &["]
    for name, frames in tables:
        lines.append(f'    ("{name}", &[')
        lines += ["        (" + ", ".join(map(str, frame)) + ")," for frame in frames]
        lines.append("    ]),")
    lines.append("];")
    formatted = subprocess.run(
        ["rustfmt", "--emit", "stdout", "--config-path", str(root / "rustfmt.toml")],
        input="\n".join(lines) + "\n", text=True, capture_output=True, check=True,
    ).stdout.encode()
    outputs = {"frames.bin": bytes(blob), "data.rs": formatted}
    for name, expected in outputs.items():
        path = out / name
        if args.check:
            if path.read_bytes() != expected:
                raise ValueError(f"Generated file is stale: {path}")
        else:
            path.write_bytes(expected)
    verb = "Verified" if args.check else "Imported"
    print(f"{verb} {len(tables)} original animations / {sum(len(t[1]) for t in tables)} frames / {len(blob)} RLE bytes; exact RGBA round-trip")


if __name__ == "__main__":
    main()
