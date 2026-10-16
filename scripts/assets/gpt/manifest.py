"""The source-image and animation contract; no character drawing lives here."""
import json
import re
from pathlib import Path

ACTION_CLIPS = {
    "idle": "idle", "blink": "blink", "move": "roam", "observe": "look",
    "eat": "eat", "play": "play", "sleep": "sleep", "startled": "startle",
    "sick": "sick", "evolve": "evolve", "end": "end", "think": "think",
    "work": "work", "parallel": "parallel", "build": "build", "error": "error",
    "complete": "complete", "await_confirmation": "ask", "organize": "organize",
    "carry": "carry", "wake": "wake", "poke_left": "poke_left",
    "poke_right": "poke_right", "tickle": "tickle", "lift": "lift", "praise": "praise",
}
EXPRESSIONS = ("curious", "focused", "shy", "happy", "startled", "smug")
PALETTE_KEYS = tuple(chr(i) for i in range(33, 127) if chr(i) != ".")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def pair(value):
    return isinstance(value, list) and len(value) == 2 and all(type(v) is int for v in value)


def read_manifest(path, allow_pending=False):
    data = json.loads(Path(path).read_text())
    require(data["version"] == 1, "Unsupported manifest version")
    require(data["canvas"] == [69, 66], "Runtime requires a 69x66 canvas")
    size = data["sampling"]["native_cell"]
    require(pair(size) and 1 <= size[0] <= 69 and 1 <= size[1] <= 66, "Invalid native_cell")
    require(type(data["sampling"]["alpha_threshold"]) is int and 1 <= data["sampling"]["alpha_threshold"] <= 255, "Invalid alpha threshold")
    require(data["sampling"].get("filter", "nearest") == "nearest", "Only fixed-grid nearest sampling is supported")
    phase = data["sampling"].get("phase", [0, 0])
    require(pair(phase) and all(-3 <= value <= 3 for value in phase), "Invalid global source-pixel phase")
    key = data["sampling"].get("chroma_key")
    require(key is None or key == [0, 255, 0], "Only the explicitly authored pure-green work matte is supported")
    require(type(data["sampling"].get("green_dominance", 32)) is int and 1 <= data["sampling"].get("green_dominance", 32) <= 255, "Invalid green-dominance margin")
    require(type(data["palette"]["max_colors"]) is int and 1 <= data["palette"]["max_colors"] <= len(PALETTE_KEYS), "Palette exceeds runtime single-byte keys")
    pending = []
    for name, sheet in data["sheets"].items():
        require((sheet["columns"], sheet["rows"]) in ((3, 2), (1, 1)), f"{name}: expected a 3x2 atlas or one complete pose")
        if "sampling_cell" in sheet or "sampling_origin" in sheet:
            require((sheet["columns"], sheet["rows"]) == (1, 1), f"{name}: only a standalone source may declare its fixed sampling grid")
            require(pair(sheet.get("sampling_cell")) and min(sheet["sampling_cell"]) > 0, f"{name}: invalid sampling_cell")
            require(pair(sheet.get("sampling_origin")), f"{name}: invalid sampling_origin")
        source = Path(sheet["path"])
        require(not source.is_absolute() and ".." not in source.parts, f"{name}: source must be repository-relative")
        digest = sheet.get("sha256")
        if digest is None:
            pending.append(f"{name}.sha256")
        else:
            require(isinstance(digest, str) and re.fullmatch(r"[0-9a-f]{64}", digest), f"{name}: invalid SHA-256")
    for name, group in data["placement_groups"].items():
        require(type(group["head_x"]) is int and 0 <= group["head_x"] < 69, f"{name}: invalid head_x")
        require(type(group["feet_y"]) is int and 0 <= group["feet_y"] < 66, f"{name}: invalid feet_y")
        for axis, maximum in [("head_y", 66), ("feet_x", 69)]:
            require(pair(group[axis]) and 0 <= group[axis][0] <= group[axis][1] < maximum, f"{name}: invalid {axis} range")
    cells = set()
    for name, pose in data["poses"].items():
        require(pose["sheet"] in data["sheets"], f"{name}: unknown sheet")
        sheet = data["sheets"][pose["sheet"]]
        require(pair(pose["cell"]) and 0 <= pose["cell"][0] < sheet["columns"] and 0 <= pose["cell"][1] < sheet["rows"], f"{name}: invalid cell")
        cell = (pose["sheet"], *pose["cell"])
        require(cell not in cells, f"{name}: duplicate source cell; reference the existing pose instead")
        cells.add(cell)
        crop = pose.get("crop")
        require(crop is None or (isinstance(crop, list) and len(crop) == 4 and all(type(v) is int for v in crop) and crop[0] < crop[2] and crop[1] < crop[3]), f"{name}: invalid source crop")
        require(pose["placement"] in data["placement_groups"], f"{name}: unknown placement group")
        for key in ("head", "feet"):
            value = pose["anchors"].get(key)
            if value is None:
                pending.append(f"{name}.{key}")
            else:
                require(pair(value) and min(value) >= 0, f"{name}: invalid {key} source anchor")
    require(set(data["animations"]) == set(ACTION_CLIPS), "Manifest must cover exactly 26 actions")
    require(set(data["expressions"]) == set(EXPRESSIONS), "Manifest must cover exactly six expressions")
    for name, clip in clips(data):
        frames = clip["frames"]
        require(isinstance(clip["loop"], bool) and isinstance(frames, list) and frames, f"{name}: invalid clip")
        require(clip["loop"] or name == "end", f"{name}: the current runtime loops every non-End clip")
        for frame in frames:
            require(set(frame) == {"pose", "ms"}, f"{name}: frames may only select source poses and durations")
            require(frame["pose"] in data["poses"], f"{name}: unknown source pose")
            ms = frame["ms"]
            require(type(ms) is int and 100 <= ms <= 65500 and ms % 100 == 0, f"{name}: duration must be a positive 100ms multiple")
        if clip["loop"]:
            require(frames[0]["pose"] == frames[-1]["pose"], f"{name}: loop must settle back on its first source pose")
        if name == "end":
            require(not clip["loop"] and len(frames) == 1, "End must hold one static source pose")
    require(allow_pending or not pending, "Source manifest is not ready: " + ", ".join(pending))
    return data, pending


def clips(data):
    for action, runtime_name in ACTION_CLIPS.items():
        yield runtime_name, data["animations"][action]
    for expression in EXPRESSIONS:
        yield "exp_" + expression, data["expressions"][expression]
