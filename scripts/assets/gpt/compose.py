"""Places the figure on the 69x66 canvas and applies the per-frame part motions."""
from lib import Img
from figure import build
import props as P

CANVAS_W, CANVAS_H = 69, 66
FIG_X, FIG_Y = 13, 12          # same placement as the original companion's 49x48 figure
SEAM = FIG_Y + 33              # canvas row where breathing stretches the body


def shift_region(g, x0, y0, x1, y1, dx, dy, copy=True):
    if not (dx or dy):
        return
    region = [row[x0:x1] for row in g[y0:y1]]
    for y in range(y0, y1):
        for x in range(x0, x1):
            g[y][x] = '.'
    h, w = len(region), len(region[0])
    for y in range(h):
        for x in range(w):
            ch = region[y][x]
            ny, nx = y + dy, x + dx
            if ch != '.' and 0 <= ny < h and 0 <= nx < w:
                g[y0 + ny][x0 + nx] = ch
    if copy:
        for y in range(h):
            for x in range(w):
                if region[y][x] != '.' and g[y0 + y][x0 + x] == '.':
                    g[y0 + y][x0 + x] = region[y][x]


def frame(expr='open', hands='clasp', bob=0, ahoge=0, tail=0, wing=0, dx=0, dy=0, props=()):
    fig = build(expr, hands)
    C = Img(CANVAS_W, CANVAS_H)
    for y in range(fig.h):
        for x in range(fig.w):
            ch = fig.g[y][x]
            if ch != '.':
                C.g[FIG_Y + y][FIG_X + x] = ch
    g = C.g
    if bob:
        g.insert(SEAM, list(g[SEAM]))
        del g[0]
    lift = 1 if bob else 0
    shift_region(g, 35, 10 - lift, 47, 21 - lift, ahoge, 0, copy=False)      # cowlick sways
    shift_region(g, 54, 42, 63, 54, 0, tail)                                  # tail tip wags
    shift_region(g, 13, 39, 23, 51, 0, wing)                                  # wings flap
    shift_region(g, 52, 39, 62, 49, 0, wing)
    if dx or dy:
        moved = [['.'] * CANVAS_W for _ in range(CANVAS_H)]
        for y in range(CANVAS_H):
            for x in range(CANVAS_W):
                ch = g[y][x]
                ny, nx = y + dy, x + dx
                if ch != '.' and 0 <= ny < CANVAS_H and 0 <= nx < CANVAS_W:
                    moved[ny][nx] = ch
        C.g = g = moved
    for draw in props:
        draw(C)
    return C
