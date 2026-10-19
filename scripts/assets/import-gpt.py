#!/usr/bin/env python3
"""Import actual raster key poses, never draw a character or invent animation parts.

python3 scripts/assets/import-gpt.py --validate-manifest
python3 scripts/assets/import-gpt.py --preview /tmp/gpt-native.png
python3 scripts/assets/import-gpt.py --animation-preview /tmp/gpt-clips
python3 scripts/assets/import-gpt.py --write       # explicit runtime-data replacement
python3 scripts/assets/import-gpt.py --check       # reproduce without writing

Requires Pillow only while building assets. Images, hashes, source-cell anchors,
fixed-grid sampling and 100ms animation timing live in gpt/manifest.json.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent / "gpt"))

from atlas import sample_poses, shared_palette  # noqa: E402
from manifest import PALETTE_KEYS, clips, read_manifest, require  # noqa: E402


def sha(data):
    return hashlib.sha256(data).hexdigest()


def rle(pixels):
    require(len(pixels) == 69 * 66, "Frame is not 69x66")
    output = bytearray()
    previous, count = pixels[0], 0
    for value in pixels:
        if value == previous and count < 255:
            count += 1
        else:
            output.extend((count, previous))
            previous, count = value, 1
    output.extend((count, previous))
    decoded = bytes(value for count, value in zip(output[::2], output[1::2]) for _ in range(count))
    require(decoded == pixels, "RLE round-trip changed a native pixel")
    return bytes(output)


def encode(data, poses, colors):
    blob, stored, animations = bytearray(), {}, []
    for name, clip in clips(data):
        frames = []
        for frame in clip["frames"]:
            packed = rle(poses[frame["pose"]])
            if packed not in stored:
                stored[packed] = len(blob), len(packed)
                blob.extend(packed)
            offset, length = stored[packed]
            frames.append((frame["ms"], offset, length))
        animations.append((name, frames))
    lines = [
        "// Generated from raster source poses by scripts/assets/import-gpt.py; see SOURCES.json.",
        "pub(super) const WIDTH: usize = 69;", "pub(super) const HEIGHT: usize = 66;",
        'pub(super) const DATA: &[u8] = include_bytes!("frames.bin");',
        "/// Colour n is COLORS[n - 1]; 0 is transparent.",
        "pub(super) const COLORS: &[(char, [u8; 3])] = &[",
    ]
    for key, (r, g, b) in zip(PALETTE_KEYS, colors):
        escaped = key.replace("\\", "\\\\").replace("'", "\\'")
        lines.append(f"    ('{escaped}', [{r}, {g}, {b}]),")
    lines.extend(["];", "pub(super) type Frame = (u16, usize, usize);", "pub(super) const ANIMATIONS: &[(&str, &[Frame])] = &["])
    for name, frames in animations:
        lines.append(f'    ("{name}", &[')
        lines.extend(f"        ({ms}, {offset}, {length})," for ms, offset, length in frames)
        lines.append("    ]),")
    lines.append("];")
    formatted = subprocess.run(
        ["rustfmt", "--emit", "stdout", "--config-path", str(ROOT / "rustfmt.toml")],
        input="\n".join(lines) + "\n", text=True, capture_output=True, check=True,
    ).stdout.encode()
    return formatted, bytes(blob), len(stored)


def preview(path, poses, colors):
    from PIL import Image
    names = list(poses)
    columns = min(6, len(names))
    image = Image.new("RGBA", (69 * columns, 66 * ((len(names) + columns - 1) // columns)))
    for index, name in enumerate(names):
        frame = Image.new("RGBA", (69, 66))
        frame.putdata([(*colors[value - 1], 255) if value else (0, 0, 0, 0) for value in poses[name]])
        image.paste(frame, ((index % columns) * 69, (index // columns) * 66))
    image.save(path)


def animation_preview(directory, data, poses, colors):
    """Native-size GIFs and frame strips made from the same indexed runtime poses."""
    from PIL import Image
    directory.mkdir(parents=True, exist_ok=True)
    palette = [0, 0, 0, *(channel for rgb in colors for channel in rgb)]
    palette += [0] * (768 - len(palette))
    for name, clip in clips(data):
        frames = []
        strip = Image.new("RGBA", (69 * len(clip["frames"]), 66))
        for index, step in enumerate(clip["frames"]):
            frame = Image.frombytes("P", (69, 66), poses[step["pose"]])
            frame.putpalette(palette)
            frame.info["transparency"] = 0
            frames.append(frame)
            strip.paste(frame.copy().convert("RGBA"), (69 * index, 0))
        options = {"loop": 0} if clip["loop"] else {}
        frames[0].save(directory / f"{name}.gif", save_all=True, append_images=frames[1:], duration=[step["ms"] for step in clip["frames"]], transparency=0, disposal=2, optimize=False, **options)
        strip.save(directory / f"{name}-frames.png")
    (directory / "clips.json").write_text(json.dumps(dict(clips(data)), indent=2) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=ROOT / "scripts/assets/gpt/manifest.json")
    parser.add_argument("--output-dir", type=Path, default=ROOT / "crates/core/kodama-term/src/companions/gpt")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--validate-manifest", action="store_true")
    parser.add_argument("--preview", type=Path)
    parser.add_argument("--animation-preview", type=Path)
    parser.add_argument("--poses", nargs="+", help="Preview only; never writes a partial runtime atlas")
    args = parser.parse_args()
    data, pending = read_manifest(args.manifest, allow_pending=args.validate_manifest or bool(args.poses))
    if args.validate_manifest:
        print(json.dumps({"poses": len(data["poses"]), "actions": 26, "expressions": 6, "pending": pending}, indent=2))
        return
    if args.poses:
        require(args.preview and not args.write and not args.check and not args.animation_preview, "--poses requires --preview and cannot write or check runtime data")
        require(all(name in data["poses"] for name in args.poses), "Unknown preview pose")
        data["poses"] = {name: data["poses"][name] for name in args.poses}
        used_sheets = {pose["sheet"] for pose in data["poses"].values()}
        data["sheets"] = {name: sheet for name, sheet in data["sheets"].items() if name in used_sheets}
        require(all(sheet["sha256"] for sheet in data["sheets"].values()), "Preview source sheet hashes are not pinned")
        require(all(all(point is not None for point in pose["anchors"].values()) for pose in data["poses"].values()), "Preview source anchors are not specified")
    rgba, sources = sample_poses(data, ROOT)
    colors, poses, color_info = shared_palette(rgba, data["palette"]["max_colors"])
    if args.poses:
        preview(args.preview, poses, colors)
        report = {"sampling": data["sampling"], "sources": sources, "palette": color_info, "pose_order": list(poses), "wrote_runtime_data": False}
        args.preview.with_suffix(".json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
        print(json.dumps(report, indent=2))
        return
    rust, blob, unique = encode(data, poses, colors)
    source = {
        "version": 1, "source_kind": "AI-generated raster key poses", "quality_status": "raster_key_pose_adaptation",
        "manifest": str(args.manifest.resolve().relative_to(ROOT)), "manifest_sha256": sha(args.manifest.read_bytes()),
        "importer_sha256": {str(p.relative_to(ROOT)): sha(p.read_bytes()) for p in [Path(__file__), *sorted((Path(__file__).parent / "gpt").glob("*.py"))] if not p.name.startswith("test_")},
        "sampling": data["sampling"], "placement_groups": data["placement_groups"], "palette": color_info,
        "sources": sources, "animations": data["animations"], "expressions": data["expressions"],
        "pose_order": list(poses), "unique_frames": unique,
        "artifact_sha256": {"data.rs": sha(rust), "frames.bin": sha(blob)},
    }
    artifacts = {"data.rs": rust, "frames.bin": blob, "SOURCES.json": (json.dumps(source, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode()}
    if args.check:
        for name, content in artifacts.items():
            require((args.output_dir / name).read_bytes() == content, f"{name} differs; regenerate explicitly after reviewing source artwork")
    elif args.write:
        args.output_dir.mkdir(parents=True, exist_ok=True)
        for name, content in artifacts.items():
            (args.output_dir / name).write_bytes(content)
    if args.preview:
        preview(args.preview, poses, colors)
    if args.animation_preview:
        animation_preview(args.animation_preview, data, poses, colors)
    print(json.dumps({"poses": len(poses), "clips": 32, "unique_frames": unique, "colors": len(colors), "rle_bytes": len(blob), "wrote_runtime_data": args.write}, indent=2))


if __name__ == "__main__":
    try:
        main()
    except (KeyError, ValueError, FileNotFoundError) as error:
        raise SystemExit(str(error)) from error
