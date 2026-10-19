"""Fixed-grid sampling, shared colour reduction and source-anchor placement."""
from collections import Counter
import hashlib

from PIL import Image, __version__ as PILLOW_VERSION

from manifest import require


def digest(data):
    return hashlib.sha256(data).hexdigest()


def remove_matte(image, settings):
    """Resolve the declared work matte before sampling; never fill a source region."""
    key = settings.get("chroma_key")
    margin = settings.get("green_dominance", 32)
    threshold = settings["alpha_threshold"]
    pixels = []
    for r, g, b, alpha in image.convert("RGBA").get_flattened_data():
        background = key is not None and (
            max(abs(c - k) for c, k in zip((r, g, b), key)) <= 16
            or (g >= 64 and g - max(r, b) >= margin)
        )
        if alpha < threshold or background:
            pixels.append((0, 0, 0, 0))
        else:
            pixels.append((r, min(g, max(r, b)) if key is not None else g, b, 255))
    cleaned = Image.new("RGBA", image.size)
    cleaned.putdata(pixels)
    return cleaned


def native_pixel(image, crop, origin, cell, size, x, y, settings):
    phase = settings.get("phase", [0, 0])
    sx = origin[0] + (2 * x + 1) * cell[0] // (2 * size[0]) + phase[0]
    sy = origin[1] + (2 * y + 1) * cell[1] // (2 * size[1]) + phase[1]
    inside = crop[0] <= sx < crop[2] and crop[1] <= sy < crop[3] and 0 <= sx < image.width and 0 <= sy < image.height
    return image.getpixel((sx, sy)) if inside else (0, 0, 0, 0)


def sample_poses(data, root):
    size = tuple(data["sampling"]["native_cell"])
    phase = data["sampling"].get("phase", [0, 0])
    sheets, source_info = {}, {}
    for key, sheet in data["sheets"].items():
        path = (root / sheet["path"]).resolve()
        require(path.is_relative_to(root.resolve()), f"{key}: source resolves outside repository")
        raw = path.read_bytes()
        require(digest(raw) == sheet["sha256"], f"{key}: source SHA-256 mismatch")
        image = Image.open(path)
        require(image.mode == "RGBA" or data["sampling"].get("chroma_key") is not None, f"{key}: RGB sources require an explicit work-matte chroma key")
        columns, rows = sheet["columns"], sheet["rows"]
        require(image.width % columns == 0 and image.height % rows == 0, f"{key}: sheet does not divide into declared cells")
        image_cell = image.width // columns, image.height // rows
        cell = tuple(sheet.get("sampling_cell", image_cell))
        origin = sheet.get("sampling_origin", [0, 0])
        require(cell[0] * size[1] == cell[1] * size[0], f"{key}: cell aspect ratio would stretch the artwork")
        sheets[key] = remove_matte(image, data["sampling"]), cell, origin
        source_info[key] = {"path": sheet["path"], "sha256": digest(raw), "size": list(image.size), "layout": [columns, rows], "image_cell": list(image_cell), "cell": list(cell), "origin": origin}
    poses, info = {}, {}
    for key, pose in data["poses"].items():
        image, cell, base_origin = sheets[pose["sheet"]]
        col, row = pose["cell"]
        origin = [base_origin[0] + col * cell[0], base_origin[1] + row * cell[1]]
        crop = pose.get("crop", [origin[0], origin[1], origin[0] + cell[0], origin[1] + cell[1]])
        anchors = {}
        for anchor, point in pose["anchors"].items():
            require(crop[0] <= point[0] < crop[2] and crop[1] <= point[1] < crop[3], f"{key}: {anchor} lies outside its declared crop")
            anchors[anchor] = [(2 * (point[n] - origin[n] - phase[n]) + 1) * size[n] // (2 * cell[n]) for n in range(2)]
        placement = data["placement_groups"][pose["placement"]]
        dx, dy = placement["head_x"] - anchors["head"][0], placement["feet_y"] - anchors["feet"][1]
        head_y, feet_x = anchors["head"][1] + dy, anchors["feet"][0] + dx
        require(placement["head_y"][0] <= head_y <= placement["head_y"][1], f"{key}: head height drifts outside its placement group; do not auto-scale")
        require(placement["feet_x"][0] <= feet_x <= placement["feet_x"][1], f"{key}: feet drift outside their placement group; do not auto-scale")
        canvas = [(0, 0, 0, 0)] * (69 * 66)
        # Crop changes extraction bounds only. Sampling keeps the same cell-anchored
        # rational grid, including crops extending into padding outside the sheet.
        x0 = (crop[0] - origin[0] - phase[0]) * size[0] // cell[0]
        x1 = -((origin[0] + phase[0] - crop[2]) * size[0] // cell[0])
        y0 = (crop[1] - origin[1] - phase[1]) * size[1] // cell[1]
        y1 = -((origin[1] + phase[1] - crop[3]) * size[1] // cell[1])
        for y in range(y0, y1):
            for x in range(x0, x1):
                pixel = native_pixel(image, crop, origin, cell, size, x, y, data["sampling"])
                if pixel[3]:
                    tx, ty = x + dx, y + dy
                    require(0 <= tx < 69 and 0 <= ty < 66, f"{key}: aligned artwork would clip; fix source padding or anchors")
                    canvas[ty * 69 + tx] = pixel
        require(any(pixel[3] for pixel in canvas), f"{key}: source pose is empty after sampling")
        poses[key] = canvas
        visible = [(i % 69, i // 69) for i, pixel in enumerate(canvas) if pixel[3]]
        bounds = [min(p[0] for p in visible), min(p[1] for p in visible), max(p[0] for p in visible) + 1, max(p[1] for p in visible) + 1]
        info[key] = {"sheet": pose["sheet"], "cell": pose["cell"], "crop": crop, "sampling_origin": origin, "source_anchors": pose["anchors"], "native_offset": [dx, dy], "native_head": [placement["head_x"], head_y], "native_feet": [feet_x, placement["feet_y"]], "native_bbox": bounds, "opaque_pixels": len(visible), "rgba_sha256": digest(bytes(channel for pixel in canvas for channel in pixel))}
    return poses, {"sheets": source_info, "poses": info, "pillow": PILLOW_VERSION, "matte_policy": {"max_chroma_distance": 16, "minimum_green": 64, "despill": "G=min(G,max(R,B))", "transparent_rgba": [0, 0, 0, 0]}}


def shared_palette(poses, limit):
    visible = [pixel[:3] for name in sorted(poses) for pixel in poses[name] if pixel[3]]
    histogram = Counter(visible)
    if len(histogram) <= limit:
        colors = sorted(histogram)
    else:
        # One deterministic atlas-wide reduction. No per-frame palettes or dithering.
        image = Image.new("RGB", (len(visible), 1))
        image.putdata(visible)
        reduced = image.quantize(colors=limit, method=Image.Quantize.FASTOCTREE, dither=Image.Dither.NONE)
        palette = reduced.getpalette()
        colors = sorted({tuple(palette[3 * i:3 * i + 3]) for i in set(reduced.get_flattened_data())})
    nearest = {rgb: min(range(len(colors)), key=lambda i: (sum((rgb[n] - colors[i][n]) ** 2 for n in range(3)), colors[i])) + 1 for rgb in histogram}
    indexed = {name: bytes(nearest[pixel[:3]] if pixel[3] else 0 for pixel in pixels) for name, pixels in poses.items()}
    return colors, indexed, {"source_rgb_count": len(histogram), "palette_rgb_count": len(colors), "method": "shared octree then nearest RGB; no dithering"}
