"""Debug contact sheet: python3 sheet.py out.png clip [clip ...]  (needs Pillow)."""
import sys
from PIL import Image, ImageDraw
from lib import PAL
import clips

def draw(names, path, scale=3, per=10, crop=(0, 0, 69, 66)):
    x0, y0, x1, y1 = crop
    tw, th = (x1 - x0) * scale, (y1 - y0) * scale
    im = Image.new('RGB', (per * (tw + 4), len(names) * (th + 16)), (255, 255, 255))
    d = ImageDraw.Draw(im)
    for r, name in enumerate(names):
        frames = clips.render_clip(name)
        step = max(1, len(frames) // per + (1 if len(frames) % per else 0))
        d.text((2, r * (th + 16) + 2), f"{name}: {len(frames)} frames, {[ms for ms, _ in frames][:per]}", fill=(0, 0, 0))
        for i, (ms, C) in enumerate(frames[::step][:per]):
            ox, oy = i * (tw + 4), r * (th + 16) + 14
            d.rectangle([ox, oy, ox + tw, oy + th], fill=(236, 233, 226))
            for y in range(y0, y1):
                for x in range(x0, x1):
                    ch = C.g[y][x]
                    if ch != '.':
                        v = PAL[ch]
                        d.rectangle([ox + (x - x0) * scale, oy + (y - y0) * scale, ox + (x - x0 + 1) * scale - 1, oy + (y - y0 + 1) * scale - 1],
                                    fill=(int(v[1:3], 16), int(v[3:5], 16), int(v[5:7], 16)))
    im.save(path)

if __name__ == '__main__':
    draw(sys.argv[2:], sys.argv[1])
