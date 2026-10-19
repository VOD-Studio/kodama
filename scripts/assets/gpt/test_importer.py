"""Build-time regression tests; fixtures are synthetic, never character artwork."""
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import tempfile
import unittest

from PIL import Image
from atlas import remove_matte, sample_poses, shared_palette
from manifest import ACTION_CLIPS, EXPRESSIONS, PALETTE_KEYS, read_manifest

ROOT = Path(__file__).resolve().parents[3]
SPEC = importlib.util.spec_from_file_location("gpt_import", ROOT / "scripts/assets/import-gpt.py")
IMPORTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(IMPORTER)


class ImporterTests(unittest.TestCase):
    def manifest(self):
        return json.loads((Path(__file__).parent / "manifest.json").read_text())

    def validate(self, data, allow_pending=True):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "manifest.json"
            path.write_text(json.dumps(data))
            return read_manifest(path, allow_pending)

    def test_clock_never_skips_a_sub_tick_source_pose(self):
        data = self.manifest()
        for duration in [0, 60, 99, 150, 65536]:
            data["animations"]["idle"]["frames"][0]["ms"] = duration
            with self.assertRaisesRegex(ValueError, "100ms"):
                self.validate(data)

    def test_loop_and_end_are_explicit_source_pose_contracts(self):
        data = self.manifest()
        data["animations"]["idle"]["frames"][-1]["pose"] = "work"
        with self.assertRaisesRegex(ValueError, "settle"):
            self.validate(data)
        data = self.manifest()
        data["animations"]["end"]["frames"] *= 2
        with self.assertRaisesRegex(ValueError, "static"):
            self.validate(data)

    def test_pending_sources_cannot_rewrite_runtime_assets(self):
        data = self.manifest()
        data["sheets"]["expressions"]["sha256"] = None
        self.validate(data)
        with self.assertRaisesRegex(ValueError, "not ready"):
            self.validate(data, False)

    def test_only_actual_pose_selection_is_allowed(self):
        data = self.manifest()
        data["animations"]["work"]["frames"][0]["overlay"] = "keyboard"
        with self.assertRaisesRegex(ValueError, "only select source poses"):
            self.validate(data)

    def test_persistent_tasks_never_drop_their_source_props(self):
        data = self.manifest()
        for action, base, allowed in [
            ("work", "work", {"work", "work_press", "work_release"}),
            ("parallel", "work", {"work", "work_press", "work_release"}),
            ("carry", "carry", {"carry", "carry_shift"}),
            ("organize", "organize", {"organize", "organize_place"}),
        ]:
            with self.subTest(action=action):
                frames = data["animations"][action]["frames"]
                poses = [frame["pose"] for frame in frames]
                self.assertTrue(set(poses) <= allowed, "Persistent tasks must retain their authored prop in every frame")
                self.assertEqual((poses[0], poses[-1]), (base, base))
                self.assertGreater(len(set(poses)), 1, "Retain real follow-through, not a static prop hold")
                self.assertTrue(all(frame["ms"] >= 100 and frame["ms"] % 100 == 0 for frame in frames))

    def test_green_spill_and_transparent_rgb_are_removed_before_sampling(self):
        image = Image.new("RGBA", (5, 1))
        image.putdata([(0, 255, 0, 255), (20, 180, 50, 255), (200, 210, 220, 100), (240, 250, 245, 255), (190, 130, 90, 255)])
        cleaned = list(remove_matte(image, {"chroma_key": [0, 255, 0], "alpha_threshold": 224, "green_dominance": 32}).get_flattened_data())
        self.assertEqual(cleaned[:3], [(0, 0, 0, 0)] * 3)
        self.assertEqual(cleaned[3], (240, 245, 245, 255))
        self.assertEqual(cleaned[4], (190, 130, 90, 255))

    def test_expanding_crop_never_rescales_or_repairs_the_figure(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = Image.new("RGB", (48, 32), (0, 255, 0))
            for y in range(3, 11):
                for x in range(3, 10):
                    source.putpixel((x, y), (100, 50 + x, 150))
            path = root / "source.png"
            source.save(path)
            data = {"sampling": {"native_cell": [8, 8], "alpha_threshold": 224, "chroma_key": [0, 255, 0]}, "sheets": {"source": {"path": "source.png", "sha256": hashlib.sha256(path.read_bytes()).hexdigest(), "columns": 3, "rows": 2}}, "placement_groups": {"test": {"head_x": 34, "feet_y": 64, "head_y": [50, 65], "feet_x": [20, 50]}}, "poses": {"test": {"sheet": "source", "cell": [0, 0], "placement": "test", "anchors": {"head": [6, 4], "feet": [6, 12]}}}}
            for phase in [[0, 0], [1, -1]]:
                data["sampling"]["phase"] = phase
                data["poses"]["test"]["crop"] = [0, 0, 16, 16]
                normal, _ = sample_poses(data, root)
                data["poses"]["test"]["crop"] = [-4, -4, 20, 20]
                padded, _ = sample_poses(data, root)
                self.assertEqual(normal, padded, phase)
                self.assertTrue(all(pixel == (0, 0, 0, 0) or pixel[3] == 255 for pixel in padded["test"]))

    def test_global_palette_and_encoding_are_reproducible(self):
        samples = {"a": [(30 + i % 150, 10 + i % 130, 80 + i % 140, 255) for i in range(69 * 66)], "b": [(0, 0, 0, 0)] * (69 * 66)}
        colors, frames, _ = shared_palette(samples, 80)
        again, reordered, _ = shared_palette(dict(reversed(list(samples.items()))), 80)
        self.assertEqual((colors, frames), (again, reordered))
        self.assertLessEqual(len(colors), 80)
        data = self.manifest()
        for clip in [*data["animations"].values(), *data["expressions"].values()]:
            clip["frames"] = [{"pose": "a", "ms": 100}]
        first = IMPORTER.encode(data, frames, colors)
        self.assertEqual(first, IMPORTER.encode(data, frames, colors))
        self.assertEqual(first[2], 1, "reused poses are stored once")
        rust, blob, _ = first
        self.assertEqual(len(bytes(value for count, value in zip(blob[::2], blob[1::2]) for _ in range(count))), 69 * 66)
        names = set(re.findall(rb'\("([^\"]+)",', rust))
        self.assertEqual(names, {name.encode() for name in ACTION_CLIPS.values()} | {("exp_" + name).encode() for name in EXPRESSIONS})
        self.assertEqual(len(PALETTE_KEYS), 93)
        self.assertNotIn(".", PALETTE_KEYS)

    def test_standalone_pose_uses_its_declared_whole_cell_scale(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            image = Image.new("RGB", (16, 16), (0, 255, 0))
            for y in range(4, 12):
                for x in range(4, 12):
                    image.putpixel((x, y), (80, 50, 120))
            path = root / "single.png"
            image.save(path)
            data = {"sampling": {"native_cell": [8, 8], "alpha_threshold": 224, "chroma_key": [0, 255, 0]}, "sheets": {"single": {"path": "single.png", "sha256": hashlib.sha256(path.read_bytes()).hexdigest(), "columns": 1, "rows": 1}}, "placement_groups": {"test": {"head_x": 34, "feet_y": 64, "head_y": [50, 65], "feet_x": [20, 50]}}, "poses": {"single": {"sheet": "single", "cell": [0, 0], "placement": "test", "anchors": {"head": [8, 4], "feet": [8, 12]}}}}
            sampled, report = sample_poses(data, root)
            self.assertEqual(report["sheets"]["single"]["cell"], [16, 16])
            self.assertEqual(report["poses"]["single"]["native_bbox"], [32, 60, 36, 64])
            self.assertEqual(sum(pixel[3] == 255 for pixel in sampled["single"]), 16)

    def test_gif_checks_preserve_binary_alpha_and_real_frame_timing(self):
        data = self.manifest()
        for clip in [*data["animations"].values(), *data["expressions"].values()]:
            clip["frames"] = [{"pose": "a", "ms": 100}]
        data["animations"]["work"]["frames"] = [{"pose": "a", "ms": 100}, {"pose": "b", "ms": 200}, {"pose": "a", "ms": 300}]
        a, b = bytearray(69 * 66), bytearray(69 * 66)
        a[70], b[71] = 1, 2
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            IMPORTER.animation_preview(directory, data, {"a": bytes(a), "b": bytes(b)}, [(70, 40, 100), (210, 150, 120)])
            with Image.open(directory / "work.gif") as image:
                self.assertEqual(image.n_frames, 3)
                for index, (duration, source) in enumerate([(100, a), (200, b), (300, a)]):
                    image.seek(index)
                    self.assertEqual(image.info["duration"], duration)
                    self.assertEqual(list(image.convert("RGBA").getchannel("A").get_flattened_data()), [255 if pixel else 0 for pixel in source])
            with Image.open(directory / "end.gif") as image:
                self.assertNotIn("loop", image.info)


if __name__ == "__main__":
    unittest.main()
