"""Half-body figure of the GPT white dragon girl, 49x48, drawn to the concept study:
lavender-white curved horns with rings, a swept fringe with a long front strand, a slightly
turned face with large violet eyes and a tiny mouth, a white spiral hairpin with a silver
tassel, a pointed ear, wide white sleeves, a silver diamond brooch and clasped hands."""
from lib import *


def blob(I, spans, tone, outline='O'):
    cells = set()
    for y, x0, x1 in spans:
        for x in range(x0, x1 + 1):
            cells.add((x, y))
    for (x, y) in cells:
        I.put(x, y, tone(x, y))
    if outline:
        for (x, y) in list(cells):
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                if (x + dx, y + dy) not in cells and I.get(x + dx, y + dy) == '.':
                    I.put(x + dx, y + dy, outline)
    return cells


def mirror(spans, w=W):
    return [(y, w - 1 - x1, w - 1 - x0) for y, x0, x1 in spans]


def line(I, x0, y0, x1, y1, c, only=None):
    n = max(abs(x1 - x0), abs(y1 - y0)) or 1
    for i in range(n + 1):
        x = round(x0 + (x1 - x0) * i / n)
        y = round(y0 + (y1 - y0) * i / n)
        if only is None or I.get(x, y) in only:
            I.put(x, y, c)


def curve(I, p0, p1, p2, c, only='ba', n=40):
    for i in range(n + 1):
        t = i / n
        x = round((1 - t) ** 2 * p0[0] + 2 * (1 - t) * t * p1[0] + t * t * p2[0])
        y = round((1 - t) ** 2 * p0[1] + 2 * (1 - t) * t * p1[1] + t * t * p2[1])
        if I.get(x, y) in only:
            I.put(x, y, c)


LEFT_EYE, RIGHT_EYE, MOUTH = (13, 26), (24, 25), (22, 31)
# (left eye rows, right eye rows) per expression; both eyes always change together.
EYES = {
    'open':    (["OOOOOO", "OwiiiO", "OiiIIO", "OIIFFO", ".sFFs."], ["OOOOOO", "OiiiwO", "OIIIiO", "OFFIIO", ".sFFs."]),
    'curious': (["OOOOOO", "OwwiiO", "OiiIIO", "OIFFFO", ".sFFs."], ["OOOOOO", "OiiwwO", "OIIIiO", "OFFFIO", ".sFFs."]),
    'focused': (["OOOOOO", "OiiiiO", "OIIIIO", "sFFFFs", "......"], ["OOOOOO", "OiiiiO", "OIIIIO", "sFFFFs", "......"]),
    'shy':     (["......", "OOOOOO", "OiIIiO", "OIwFIO", ".OOOO."], ["......", "OOOOOO", "OiIIiO", "OIFwIO", ".OOOO."]),
    'happy':   (["......", "..OO..", ".O..O.", "O....O", "......"], ["......", "..OO..", ".O..O.", "O....O", "......"]),
    'startled': (["OOOOOO", "OwwwwO", "OwiiwO", "OwiiwO", ".OwwO."], ["OOOOOO", "OwwwwO", "OwiiwO", "OwiiwO", ".OwwO."]),
    'smug':    (["OOOOOO", "OOOOOO", ".OIIO.", "......", "......"], ["......", "..OO..", ".O..O.", "O....O", "......"]),
    'closed':  (["......", "......", "OOOOOO", ".OOOO.", "......"], ["......", "......", "OOOOOO", ".OOOO.", "......"]),
    'blink':   (["......", "OOOOOO", "OIIIIO", ".OOOO.", "......"], ["......", "OOOOOO", "OIIIIO", ".OOOO.", "......"]),
}
MOUTHS = {
    'open': ["mm"], 'curious': ["mm"], 'focused': ["m"], 'shy': ["m"], 'happy': [".mm.", "mMMm"],
    'startled': ["mm", "MM"], 'smug': ["mmm"], 'closed': ["mm"], 'blink': ["mm"],
}


def draw_hands(I, pose):
    def sleeves(y0, rows=3):
        for k in range(rows):
            I.P(y0 + k, 16 - (k > 0), "XXXXX"[: 4 + (k > 0)])
            I.P(y0 + k, 29, "XXXXX"[: 4 + (k > 0)])
    if pose == 'clasp':
        for y, x0, t in [(39, 18, "PPPPPPPPP"), (40, 17, "PXXsssXXXP"), (41, 17, "PXXPstXXXP"), (42, 18, "PXXXXXXXP"), (43, 19, "PPPPPPP")]:
            I.P(y, x0, t)
    elif pose == 'up':       # left hand at the chest, right hand raised with thumb up
        for y, x0, t in [(40, 17, "PPPPP"), (41, 16, "PsssstP"), (42, 17, "PssssP"), (43, 18, "PPPP")]:
            I.P(y, x0, t)
        for y, x0, t in [(33, 35, "PP"), (34, 34, "PssP"), (35, 34, "PssP"), (36, 33, "PPssssP"), (37, 33, "PsssssP"), (38, 34, "PsssstP"), (39, 34, "PPPPPP")]:
            I.P(y, x0, t)
        I.P(36, 36, "ss")
    elif pose == 'type':     # both hands low and apart, resting on a keyboard prop
        for y, x0, t in [(43, 13, "PPPPP"), (44, 12, "PsssstP"), (45, 12, "PsssssP"), (46, 13, "PPPPP")]:
            I.P(y, x0, t)
        for y, x0, t in [(43, 31, "PPPPP"), (44, 30, "PtssssP"), (45, 30, "PsssssP"), (46, 31, "PPPPP")]:
            I.P(y, x0, t)
    elif pose == 'hold':     # both arms forward, hands under a carried box
        for y, x0, t in [(43, 13, "PsssP"), (44, 13, "PsssP"), (45, 14, "PPP")]:
            I.P(y, x0, t)
            I.P(y, 35, t)


def build(expr='open', hands='clasp'):
    I = Img()
    # ---- wings (behind the hair, peeking out at the shoulders) ----
    wing = [(27, 2, 3), (28, 1, 4), (29, 0, 5), (30, 0, 6), (31, 0, 7), (32, 0, 8), (33, 1, 9), (34, 1, 9), (35, 2, 9), (36, 3, 9), (37, 5, 9), (38, 6, 9)]
    wt = lambda x, y: 'V' if y < 31 else ('U' if y < 35 else 'T')
    blob(I, wing, wt, 'P')
    blob(I, mirror(wing), wt, 'P')
    for (x, y) in [(1, 29), (2, 30), (3, 31), (4, 32), (5, 33), (6, 34), (7, 35)]:
        I.put(x, y, 'L')
        I.put(48 - x, y, 'L')
    for (x, y) in [(3, 36), (5, 37), (2, 35)]:
        I.put(x, y, '.')
        I.put(48 - x, y, '.')
    # ---- long wavy locks behind the shoulders ----
    left = [(y, 7, 15) for y in range(30, 43)] + [(43, 8, 15), (44, 9, 15), (45, 10, 14), (46, 11, 14)]
    right = [(y, 33, 41) for y in range(30, 43)] + [(43, 33, 40), (44, 33, 39), (45, 34, 38), (46, 35, 37)]

    def lt(x, y):
        k = x - 7
        if k <= 1:
            return 'd'
        if k in (3, 4):
            return 'c'
        if k == 6 and y < 40:
            return 'a'
        return 'c' if k == 5 else 'b'
    blob(I, left, lt, 'P')
    blob(I, right, lambda x, y: lt(48 - x, y), 'P')
    # ---- scaly tail, in front of the lock and behind the dress ----
    def ttone(x, y):
        v = (x - 38) * 0.7 + (y - 40) * 0.5
        return 'S' if v < -2 else ('R' if v < 3 else 'Q')
    I.stroke([(35, 46), (40, 45), (44, 42), (46, 38), (45, 34)], [3.0, 3.2, 2.8, 2.4, 1.4], 'R', 'P', ttone)
    for (x, y) in [(41, 44), (43, 41), (45, 38), (40, 46), (44, 36), (46, 34)]:
        if I.get(x, y) in 'SRQ':
            I.put(x, y, 'N')
    for (x, y, c) in [(45, 31, 'P'), (44, 32, 'P'), (46, 32, 'P'), (45, 32, 'S'), (43, 33, 'P'), (47, 33, 'P'), (44, 33, 'S'), (45, 33, 'R'), (46, 33, 'R')]:
        I.put(x, y, c)
    # ---- white dress with wide sleeves ----
    dress = [(35, 19, 29), (36, 17, 31), (37, 15, 33), (38, 14, 34), (39, 13, 35)] + [(y, 12, 36) for y in range(40, 44)] + [(44, 11, 37), (45, 11, 37), (46, 10, 38), (47, 10, 38)]

    def dt(x, y):
        if x <= 15 or x >= 33:
            return 'X'
        if y >= 46:
            return 'Y'
        if x in (19, 29) and y >= 41:
            return 'Y'
        return 'W'
    blob(I, dress, dt, 'P')
    for y in range(39, 48):                     # wide hanging sleeves
        w = 3 + (y - 39) // 3
        I.span(y, 12 - w + 3, 16, 'X' if y % 2 else 'W')
        I.span(y, 32, 36 + w - 3, 'X' if y % 2 else 'W')
        I.put(12 - w + 3, y, 'P')
        I.put(36 + w - 3, y, 'P')
    for (y, x0, x1) in [(33, 20, 24), (34, 20, 24), (35, 20, 24)]:
        I.span(y, x0, x1, 's')
    I.span(35, 20, 24, 't')
    I.P(36, 19, "XXWWWXX")
    I.P(36, 15, "PPPP")
    I.P(36, 29, "PPPP")
    # silver diamond brooch and chain
    I.put(24, 36, 'g'); I.P(37, 23, "gwg"); I.put(24, 38, 'g')
    for (x, y) in [(31, 41), (32, 44), (32, 47)]:
        I.put(x, y, 'g')
    draw_hands(I, hands)
    # ---- head hair mass ----
    xl = {5: 20, 6: 16, 7: 13, 8: 11, 9: 9, 10: 8, 11: 7, 12: 6, 13: 5}
    for y in range(14, 31):
        xl[y] = 4 if y not in (14, 15) else 5
    xr = {k: 48 - v for k, v in xl.items()}
    for y in range(5, 31):
        a, b = xl[y], xr[y]
        I.span(y, a, b, 'b')
        I.put(a, y, 'P')
        I.put(b, y, 'P')
    I.span(5, xl[5], xr[5], 'P')
    for y in range(6, 16):                      # soft highlights on the crown
        for x in range(12, 36):
            if I.get(x, y) == 'b' and ((x - 19) ** 2 / 70 + (y - 9) ** 2 / 9 < 1 or (x - 31) ** 2 / 14 + (y - 11) ** 2 / 5 < 1):
                I.put(x, y, 'a')
    for p2, ctrl in [((7, 25), (9, 12)), ((10, 28), (12, 14)), ((13, 27), (15, 15)), ((36, 26), (33, 13)), ((39, 24), (37, 12)), ((41, 22), (40, 10))]:
        curve(I, (28, 6), ctrl, p2, 'c', only='ba')
    for p2, ctrl in [((8, 27), (10, 13)), ((38, 26), (36, 12))]:
        curve(I, (29, 7), ctrl, p2, 'd', only='bac')
    for y in range(8, 30):
        for x in list(range(5, 9)) + list(range(40, 44)):
            if I.get(x, y) == 'b':
                I.put(x, y, 'c' if (x + y) % 3 else 'd')
    for y in range(24, 31):
        for x in list(range(5, 8)) + list(range(41, 44)):
            if I.get(x, y) in 'bc':
                I.put(x, y, 'd')
    # ---- face: slightly turned, centred a little left of the head ----
    face = [(22, 14, 30)] + [(y, 12, 32) for y in range(23, 30)] + [(30, 13, 31), (31, 15, 30), (32, 17, 28), (33, 19, 26)]
    blob(I, face, lambda x, y: 't' if (y == 22 or y >= 32) else 's', 'P')
    I.span(22, 14, 30, 't')
    # pointed ear on the right, slipping out of the hair
    I.stroke([(32, 28), (35, 27), (38, 25)], [1.7, 1.2, 0.5], 's', 'O', lambda x, y: 's' if y < 27 else 't')
    # eyes, mouth, blush
    L, R = EYES[expr]
    for dy, row in enumerate(L):
        for dx, ch in enumerate(row):
            if ch != '.':
                I.put(LEFT_EYE[0] + dx, LEFT_EYE[1] + dy, ch)
    for dy, row in enumerate(R):
        for dx, ch in enumerate(row):
            if ch != '.':
                I.put(RIGHT_EYE[0] + dx, RIGHT_EYE[1] + dy, ch)
    for dy, row in enumerate(MOUTHS[expr]):
        for dx, ch in enumerate(row):
            if ch != '.':
                I.put(MOUTH[0] + dx, MOUTH[1] + dy, ch)
    cheeks = [(12, 30), (13, 30), (12, 29), (30, 30), (29, 30), (30, 29)] if expr in ('shy', 'happy') else [(13, 29), (29, 29)]
    for (x, y) in cheeks:
        I.put(x, y, 'k')
    if expr == 'startled':
        for (x, y) in [(31, 22), (31, 23), (32, 24)]:
            I.put(x, y, 'F')
    # ---- fringe: swept from the parting at the upper right to the lower left ----
    def lock(xa, xb, y0, tip, y1):
        for y in range(y0, y1 + 1):
            f = (y - y0) / max(1, (y1 - y0))
            l = round(xa + (tip - xa) * f)
            r = round(xb + (tip - xb) * f)
            for x in range(l, r + 1):
                I.put(x, y, 'b')
            I.put(l, y, 'c')
            I.put(r, y, 'c')
            if r - l >= 2 and f < 0.55:
                I.put(l + 1, y, 'a')
        I.put(tip, y1, 'd')
        I.put(tip, y1 - 1, 'c')
    for y in range(13, 22):
        I.span(y, 7, 41, 'b')
    lock(6, 14, 11, 8, 32)      # long front strand down the left of the face
    lock(12, 21, 12, 13, 28)
    lock(18, 26, 12, 20, 25)
    lock(24, 32, 12, 26, 23)
    lock(30, 37, 12, 33, 26)
    lock(35, 42, 12, 39, 29)
    # ---- spiral hairpin with a silver diamond chain and white tassel ----
    for y, x0, t in [(13, 36, "BBB"), (14, 35, "BaaaB"), (15, 34, "BaaBBaB"), (16, 34, "BaBaaaB"), (17, 34, "BaaBaaB"), (18, 35, "BaaaB"), (19, 36, "BBB")]:
        I.P(y, x0, t)
    for y, x0, t in [(20, 38, "g"), (21, 37, "ngn"), (22, 38, "g"), (23, 38, "n"), (24, 37, "ngn"), (25, 38, "g"), (26, 37, "aaa"), (27, 37, "aaa"), (28, 37, "cac"), (29, 38, "c")]:
        I.P(y, x0, t)
    # ---- horns: lavender crescents with dark outline and ring ridges ----
    def htone(mirror):
        def f(x, y):
            xx = 48 - x if mirror else x
            v = xx * 0.9 - y * 0.55
            return 'H' if v < 1.6 else ('J' if v < 5.2 else 'K')
        return f
    I.stroke([(13, 11), (8, 9), (4, 5), (5, 1)], [3.0, 2.8, 2.2, 0.9], 'J', 'O', htone(False))
    I.stroke([(35, 11), (41, 9), (44, 5), (42, 1)], [3.4, 3.0, 2.4, 0.9], 'J', 'O', htone(True))
    for (x, y) in [(5, 3), (6, 6), (8, 8), (10, 10), (43, 3), (42, 6), (40, 8), (38, 10)]:
        for dx in (0, 1):
            if I.get(x + dx, y) in 'HJK':
                I.put(x + dx, y, 'B')
    # ---- cowlick curl above the forehead, left of centre ----
    I.stroke([(16, 6), (15, 3), (17, 1), (20, 2), (20, 4), (18, 4)], [0.5] * 6, 'c', 'P')
    return I
