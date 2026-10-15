"""Small symbols drawn beside the figure, in canvas coordinates (69x66).

Like the original companion's thought bubbles, check badges and hearts, every symbol
is a tiny hand-placed bitmap that lives in the empty margin around the head.
Letters index PALETTE in lib.py; `.` and space are transparent.
"""


def bmp(C, x, y, rows):
    for dy, row in enumerate(rows):
        for dx, ch in enumerate(row):
            if ch not in ". ":
                C.put(x + dx, y + dy, ch)


def bubble(C, x, y, r):
    if r <= 1:
        C.put(x, y, "7")
    elif r == 2:
        bmp(C, x, y, [".7.", "7.7", ".7."])
    else:
        bmp(C, x, y, [".777.", "7w..7", "7...7", "7...7", ".777."])


def code(C, x, y, shown=11):
    """`</>` built left to right; `shown` is how many columns are visible."""
    glyphs = [
        ["..1", ".1.", "1..", ".1.", "..1"],
        ["..5", "..5", ".5.", "5..", "5.."],
        ["1..", ".1.", "..1", ".1.", "1.."],
    ]
    for i, glyph in enumerate(glyphs):
        if i * 4 < shown:
            bmp(C, x + i * 4, y, glyph)


def check(C, x, y):
    bmp(C, x, y, [".22222.", "1111111", "11111w1", "1w11w11", "11ww111", "1111111", ".11111."])


def cross(C, x, y):
    bmp(C, x, y, [".33333.", "3w333w3", "33w3w33", "333w333", "33w3w33", "3w333w3", ".44444."])


def heart(C, x, y, big=True):
    bmp(C, x, y, [".6.6.", "66666", ".666.", "..6.."] if big else ["6.6", ".6."])


def sparkle(C, x, y, big=True):
    if big:
        bmp(C, x, y, [".G.", "GGG", ".G."])
    else:
        C.put(x, y, "G")


def exclaim(C, x, y):
    bmp(C, x, y, ["33", "33", "33", "..", "33"])


def question(C, x, y):
    bmp(C, x, y, [".555.", "5...5", "...5.", "..5..", "..5..", ".....", "..5.."])


def z(C, x, y, size=3):
    if size >= 5:
        bmp(C, x, y, ["77777", "...7.", "..7..", ".7...", "77777"])
    else:
        bmp(C, x, y, ["777", ".7.", "777"])


def sweat(C, x, y):
    bmp(C, x, y, ["5", "5", "w"])


def note(C, x, y):
    bmp(C, x, y, ["..66", "..6.", ".66.", "666.", ".6.."])


def gear(C, x, y, phase=0):
    if phase % 2 == 0:
        bmp(C, x, y, [".7.7.", "77777", "7.8.7", "77777", ".7.7."])
    else:
        bmp(C, x, y, ["7.7.7", ".777.", "78.87", ".777.", "7.7.7"])


def spark_lines(C, x, y):
    bmp(C, x, y, ["G.G", ".G.", "G.G"])


def dizzy(C, x, y, phase=0):
    if phase % 2 == 0:
        bmp(C, x, y, ["7777.", "7...7", "7.7.7", "7.77.", ".7..."])
    else:
        bmp(C, x, y, [".777.", "7...7", "7.7.7", ".77.7", "...7."])


def keyboard(C, x, y, pressed=0):
    bmp(C, x, y, ["888888888888888888888", "877777777777777777778", "87w7w7w7w7w7w7w7w7778", "877777777777777777778", "888888888888888888888"])
    if pressed:
        C.put(x + 3 + (pressed % 5) * 3, y + 2, "8")


def box(C, x, y, w=29, h=7):
    for dy in range(h):
        for dx in range(w):
            edge = dx in (0, w - 1) or dy in (0, h - 1)
            tape = abs(dx - w // 2) <= 1
            C.put(x + dx, y + dy, "9" if edge else ("y" if tape else "0"))


def ball(C, x, y):
    bmp(C, x, y, [".666.", "66w66", "66666", "66566", ".666."])


def cookie(C, x, y):
    bmp(C, x, y, [".00.", "0y00", ".00."])


def card(C, x, y, tone="5"):
    bmp(C, x, y, ["wwwww", "w" + tone * 3 + "w", "wwwww", "w" + tone * 3 + "w", "wwwww", "77777"])


def halo(C, x, y):
    bmp(C, x, y, [".GGGGGGG.", "G.......G", ".GGGGGGG."])


def finger(C, x, y, from_left=True):
    bmp(C, x, y, ["PPPPP.", "PssssP", "PPPPP."] if from_left else [".PPPPP", "PssssP", ".PPPPP"])


def dust(C, x, y):
    bmp(C, x, y, ["7.7.7", ".7.7."])


def forehead_lines(C, x, y):
    for dx in (0, 2, 4):
        for dy in range(3):
            C.put(x + dx, y + dy, "5")
