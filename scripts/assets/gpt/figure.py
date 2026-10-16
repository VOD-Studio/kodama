"""Half-body figure of the GPT white dragon girl, 49x48, crafted to the concept study
in assets/preview/companions/studies/gpt/gpt-adult.png:
curved segmented ivory-lavender horns, top ahoge curl, swept silver-lavender bangs,
gently tilted face with luminous violet eyes, spiral knot rosette with silver diamond
tassel, pointed ear, silver diamond collar clasp, bare right shoulder, clasped hands,
small bat-like dragon wings and curled scaled tail."""
from lib import Img

BASE = [
    ".............................OOO.................",  #  0
    "............................ObcO.................",  #  1
    "...........................ObcO..................",  #  2
    ".................OOOO......ObbO..................",  #  3
    ".........OO.....OaaabO....OcabdO.................",  #  4
    ".........OcO...Oab.OcO....OcbbbO.................",  #  5
    ".........ObcO..Oac.........OcaacOO...............",  #  6
    ".........OccO...ObcOO..OOOOOdacaacO..............",  #  7
    ".........OaadO.OObdcbaabcdOOOdbabcdOO............",  #  8
    "........OdcbO.OeddccdcbbaacdOdbcbaadO............",  #  9
    ".......OcaadOOdcbbcededabaabdOdbaaacdO...........",  # 10
    "......OdbabdOdcacecaacddabbcbOcbccbbdO...........",  # 11
    "......OcbccOdcacecbabaadcaacdOdcbaccO............",  # 12
    ".....OdaaddOcbaebbaaabaacbaabOcabbcdO............",  # 13
    ".....OccccOdbadcbcaaabbaacaaadbbaccdO............",  # 14
    ".....OdbacebaacabcaababaaabaacdcccddO............",  # 15
    ".....OdacadbaacabcabbabaaacaddPdddPO.............",  # 16
    ".....OddbbdaaacabcbaaabcabcbPdaaaadPO............",  # 17
    ".....OddcebaabbbabcaabceabbcPaaPaaadO............",  # 18
    "......OdceaaacababbcbbaedabcPaPaPaadO............",  # 19
    ".......OeababcadaacaccbdddbdPaaPaaadOO...........",  # 20
    ".......OOababcabccccdddbdbaccPdaaadPccO..........",  # 21
    ".......OdcabbdbacbccdcceddaaadPdddPdcOO..........",  # 22
    "......OdddabadbbaccddssccdaaabPngPOOOOO..........",  # 23
    "......OdcbbbadcbbccsssssssbabaPgagOssstOO........",  # 24
    "......OdcbbbadcbbcsssssssssacaPngPOssstO.........",  # 25
    "......OdcabaadcbcssssssssssddbcPgPOsttO..........",  # 26
    ".......OdcbbbdbcsssssssssssdccPngnPOOO...........",  # 27
    ".......ObecdcdcssssssssssssddcPgagPdcO...........",  # 28
    "......OOeceecccssssssssssssddcPngnPdcOO..........",  # 29
    "......OOedeecdcssssssssssstddbPabaPdcaO..........",  # 30
    ".......OdeeeeceOttssssssstOOcaPcbcPdcaO..........",  # 31
    "......OOdeeeeedeOOOtsssstOOedacPcdPdcOO..........",  # 32
    ".......OdededeedeeOOOOOOOeeebeOOcbdOOO...........",  # 33
    ".......OOcdcdedcdeOPaPOedddcdeecbcdOdO...........",  # 34
    ".......OOOccdecdOOagngaOOssOOceeOccddOO..........",  # 35
    ".....OObOcbddecOcssgngssOsstOsbedcecbcdOecOO.....",  # 36
    "....ObdcObddddcOtssOPOsstsstOddddeeOcbcdddcbOO...",  # 37
    "...ObddcOdeedbeOtsstOtsstOOddggOddObcdbOdOdbbdO..",  # 38
    "..OaddcdcOebbeeeOOtOOOtOObaOggedddbccddaOdOccbO..",  # 39
    ".OaccccddbbcceeOabOOOOOabcaOdendedcddcbcbdOdbacO.",  # 40
    ".OccbccdccdceeOabcaabaaabccaOcnOeceddcbcbOOdcbacO",  # 41
    "ObcbbbdObdddeOabcaaaaaaabcccaOgOebdddcbbcOOOccbcO",  # 42
    "OccOOOOOdcdeeaacaaababOccbabbceedccdcbbbcbO.OabcO",  # 43
    "OO.....OObddbabcaaabaacOdbabbddOddaedOObcbO.OabaO",  # 44
    "......OObcdOaadababacaceObbacdccedbdOOeOcbO.OcbbO",  # 45
    "....OObbcddebbdbaabbdbddOababcOOeddbdcdOOcOOccacO",  # 46
    "....OeddddcdbbdaaabbccdeeaacaaeeeecaddcbOOOOcbacc",  # 47
]

LEFT_EYE, RIGHT_EYE, MOUTH = (14, 25), (22, 23), (20, 30)

# (left eye rows, right eye rows) per expression; both eyes always change together.
EYES = {
    'open': (
        [".OOOO", "Owwii", ".wiiI", ".iFFI", "..FF."],
        ["OOOO.", "iiwwO", "Iiiw.", "IFFi.", ".FF.."],
    ),
    'curious': (
        [".OOOO", "Owwii", "OwiII", ".IFFF", "..FF."],
        ["OOOO.", "iiwwO", "IIiwO", "FFFI.", ".FF.."],
    ),
    'focused': (
        [".....", "OOOOO", ".iwII", ".iFFI", "..FF."],
        [".....", "OOOOO", "IIwi.", "IFFi.", ".FF.."],
    ),
    'shy': (
        [".....", ".OOOO", "OiiIw", ".iFFF", "..FF."],
        [".....", "OOOO.", "wIiiO", "FFFi.", ".FF.."],
    ),
    'happy': (
        [".....", ".OOO.", "O...O", ".....", "....."],
        [".....", ".OOO.", "O...O", ".....", "....."],
    ),
    'startled': (
        [".OOO.", "OwwwO", "OwiwO", "OwwwO", ".OOO."],
        [".OOO.", "OwwwO", "OwiwO", "OwwwO", ".OOO."],
    ),
    'smug': (
        [".....", "OOOOO", ".iIIw", "..FF.", "....."],
        [".....", ".OOO.", "O...O", ".....", "....."],
    ),
    'closed': (
        [".....", ".....", "O...O", ".OOO.", "....."],
        [".....", ".....", "O...O", ".OOO.", "....."],
    ),
    'blink': (
        [".....", ".....", "OOOOO", ".sFs.", "....."],
        [".....", ".....", "OOOOO", ".sFs.", "....."],
    ),
}

MOUTHS = {
    'open': ["m"],
    'curious': ["mm"],
    'focused': ["m"],
    'shy': [".m"],
    'happy': ["mm", "MM"],
    'startled': ["mm", "MM"],
    'smug': ["mm"],
    'closed': ["m"],
    'blink': ["m"],
}


def draw_hands(I, pose):
    if pose == 'clasp':
        return
    elif pose == 'up':       # left hand at chest, right hand raised with thumb up
        rows = [
            (35, 29, ".OO.."),
            (36, 19, "gngcbO..OssO."),
            (37, 19, "OPOdbcbOOssssO"),
            (38, 19, "OdbcbacOOssttO"),
            (39, 18, "tOOcbcaaaOOOO."),
            (40, 18, "OOabcaabcaO"),
        ]
        for y, x0, s in rows:
            I.P(y, x0, s)
    elif pose == 'type':     # both hands low and apart, resting on a keyboard prop
        rows = [
            (36, 15, "OcbcagngcbOsstO"),
            (37, 15, "OdbcbOPOdcbsstO"),
            (38, 15, "OdcbbabcbdbOOdd"),
            (39, 15, "eOdcbbaaabcbbaO"),
            (40, 15, "Oabcbbaaaabcbca"),
            (42, 13, ".OOOO.....OOOO."),
            (43, 12, "OssssO...OssssO"),
            (44, 12, "OssttO...OttssO"),
            (45, 13, "OOOO.....OOOO."),
        ]
        for y, x0, s in rows:
            I.P(y, x0, s)
    elif pose == 'hold':     # both arms forward, hands under a carried box
        rows = [
            (36, 15, "OcbcagngcbOsstO"),
            (37, 15, "OdbcbOPOdcbsstO"),
            (38, 15, "OdcbbabcbdbOOdd"),
            (39, 15, "eOdcbbaaabcbbaO"),
            (40, 15, "Oabcbbaaaabcbca"),
            (41, 13, ".OOO.......OOO."),
            (42, 12, "OsssO.....OsssO"),
            (43, 12, "OsstO.....OtssO"),
            (44, 13, "OOO.......OOO."),
        ]
        for y, x0, s in rows:
            I.P(y, x0, s)


def build(expr='open', hands='clasp'):
    I = Img()
    for y, row in enumerate(BASE):
        I.P(y, 0, row)
    L, R = EYES[expr]
    for dy, row in enumerate(L):
        I.P(LEFT_EYE[1] + dy, LEFT_EYE[0], row)
    for dy, row in enumerate(R):
        I.P(RIGHT_EYE[1] + dy, RIGHT_EYE[0], row)
    # Central swept bang wisp dipping gently between the eyes
    I.P(23, 19, "bcd")
    I.P(24, 19, "ccd")
    I.P(25, 20, "cd")
    I.put(20, 26, 'd')
    # Cheek blush
    cheeks = [(14, 30), (15, 30), (25, 28), (26, 28)]
    if expr in ('shy', 'happy'):
        cheeks += [(16, 30), (24, 28)]
    for x, y in cheeks:
        I.put(x, y, 'k')
    if expr == 'startled':
        for x, y in [(27, 22), (27, 23), (28, 23)]:
            I.put(x, y, '5')
    for dy, row in enumerate(MOUTHS[expr]):
        I.P(MOUTH[1] + dy, MOUTH[0], row)
    draw_hands(I, hands)
    return I
