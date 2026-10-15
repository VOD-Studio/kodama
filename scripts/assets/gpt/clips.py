"""Every companion action is its own short clip of hand-placed frames with uneven timing:
holds of 200-700 ms on rests, 60-100 ms on quick changes (blinks, startles).

A frame is `fr(ms, expr, **parts)`; parts are the options of compose.frame()."""
import props as P
from compose import frame

def fr(ms, expr='open', **kw):
    return (ms, dict(expr=expr, **kw))

def at(fn, *a, **k):
    return lambda C: fn(C, *a, **k)

TR = (55, 3)           # big symbol slot, top right of the head
CLIPS = {}

# ---- rest: breathing, tail and cowlick drift, an occasional blink -------------------
CLIPS['idle'] = [
    fr(700), fr(280, bob=1, ahoge=1, tail=1), fr(280, bob=1, tail=2), fr(240, ahoge=-1, tail=1),
    fr(360), fr(70, 'blink'), fr(60, 'closed'), fr(90, 'blink'), fr(650, bob=1, ahoge=1),
    fr(280, bob=1, tail=1), fr(280, ahoge=-1),
]
CLIPS['blink'] = [fr(200), fr(70, 'blink'), fr(60, 'closed'), fr(90, 'blink'), fr(300)]
CLIPS['look'] = [
    fr(300), fr(200, 'curious', dx=-1), fr(200, 'curious', dx=-1, bob=1), fr(300, 'curious'),
    fr(200, 'curious', dx=1), fr(200, 'curious', dx=1, bob=1), fr(300, 'open'), fr(120, 'blink'), fr(300),
]
CLIPS['roam'] = [
    fr(150, 'happy', dx=0, dy=0, wing=0, tail=0), fr(150, 'happy', dx=1, dy=-2, wing=-2, tail=1),
    fr(150, 'happy', dx=2, dy=-3, wing=-3, tail=2), fr(150, 'happy', dx=1, dy=-2, wing=-2, tail=1),
    fr(150, 'happy', dx=0, dy=0, wing=0), fr(150, 'happy', dx=-1, dy=-2, wing=-2, tail=-1),
    fr(150, 'happy', dx=-2, dy=-3, wing=-3, tail=-2), fr(150, 'happy', dx=-1, dy=-2, wing=-2, tail=-1),
]
CLIPS['eat'] = [
    fr(200, 'curious', props=[at(P.cookie, 56, 38)]), fr(160, 'curious', props=[at(P.cookie, 50, 41)]),
    fr(160, 'happy', props=[at(P.cookie, 43, 43)]), fr(140, 'open', props=[at(P.cookie, 38, 44)]),
    fr(140, 'happy', bob=1), fr(140, 'open'), fr(140, 'happy', bob=1), fr(140, 'open'),
    fr(160, 'happy', bob=1, props=[at(P.sparkle, 56, 8)]), fr(300, 'happy', props=[at(P.heart, 56, 6)]),
]
CLIPS['play'] = [
    fr(120, 'happy', props=[at(P.ball, 31, 41)]), fr(120, 'happy', dy=-2, wing=-2, tail=1, props=[at(P.ball, 31, 33)]),
    fr(120, 'happy', dy=-3, wing=-3, tail=2, props=[at(P.ball, 31, 26)]), fr(120, 'happy', dy=-2, wing=-2, props=[at(P.ball, 31, 33)]),
    fr(120, 'happy', props=[at(P.ball, 31, 41)]), fr(120, 'happy', dy=-2, wing=-2, tail=-1, props=[at(P.ball, 31, 33), at(P.note, 55, 4)]),
    fr(120, 'happy', dy=-3, wing=-3, tail=-2, props=[at(P.ball, 31, 26), at(P.note, 59, 8)]), fr(120, 'happy', dy=-2, props=[at(P.ball, 31, 33)]),
]
CLIPS['sleep'] = [
    fr(330, 'closed', props=[at(P.z, 54, 6)]), fr(330, 'closed', bob=1, props=[at(P.z, 54, 6), at(P.z, 58, 2, 5)]),
    fr(330, 'closed', bob=1, props=[at(P.z, 56, 4, 5)]), fr(330, 'closed', props=[at(P.z, 58, 1, 5)]),
    fr(330, 'closed', props=[at(P.z, 54, 6)]), fr(330, 'closed'),
]
CLIPS['startle'] = [
    fr(80, 'startled', dy=-5, wing=-2, props=[at(P.exclaim, 55, 2)]), fr(80, 'startled', dy=-4, wing=-2, props=[at(P.exclaim, 55, 2)]),
    fr(90, 'startled', dy=-2, wing=-1, props=[at(P.exclaim, 55, 2)]), fr(200, 'startled', dy=-1, props=[at(P.sweat, 60, 18)]),
    fr(200, 'startled', props=[at(P.sweat, 60, 20)]), fr(160, 'shy', props=[at(P.sweat, 60, 22)]),
    fr(200, 'shy'), fr(300, 'open'),
]
CLIPS['sick'] = [
    fr(300, 'focused', props=[at(P.forehead_lines, 36, 30), at(P.dizzy, 46, 3, 0)]),
    fr(300, 'shy', bob=1, props=[at(P.forehead_lines, 36, 30), at(P.dizzy, 46, 3, 1), at(P.sweat, 60, 18)]),
    fr(300, 'focused', props=[at(P.forehead_lines, 36, 30), at(P.dizzy, 46, 3, 0)]),
    fr(300, 'shy', bob=1, props=[at(P.forehead_lines, 36, 30), at(P.dizzy, 46, 3, 1), at(P.sweat, 60, 20)]),
]
CLIPS['evolve'] = [
    fr(120, 'open', props=[at(P.sparkle, 8, 30, False), at(P.sparkle, 60, 10, False)]),
    fr(120, 'focused', wing=-1, props=[at(P.sparkle, 6, 28), at(P.sparkle, 62, 12, False), at(P.sparkle, 30, 4, False)]),
    fr(120, 'happy', wing=-2, dy=-1, props=[at(P.sparkle, 5, 24), at(P.sparkle, 62, 8), at(P.sparkle, 18, 4), at(P.spark_lines, 50, 3)]),
    fr(120, 'happy', wing=-3, dy=-3, props=[at(P.spark_lines, 4, 20), at(P.sparkle, 60, 6), at(P.sparkle, 10, 6), at(P.sparkle, 34, 2), at(P.spark_lines, 56, 14)]),
    fr(120, 'happy', wing=-4, dy=-3, props=[at(P.sparkle, 3, 14), at(P.sparkle, 64, 4), at(P.spark_lines, 14, 3), at(P.sparkle, 40, 2, False), at(P.sparkle, 62, 20)]),
    fr(120, 'happy', wing=-3, dy=-1, props=[at(P.sparkle, 6, 10), at(P.sparkle, 60, 3, False), at(P.sparkle, 20, 2, False)]),
    fr(120, 'happy', wing=-1, props=[at(P.sparkle, 8, 6, False), at(P.sparkle, 58, 4, False)]),
    fr(400, 'happy', props=[at(P.sparkle, 56, 5)]),
]
CLIPS['end'] = [fr(1000, 'closed', props=[at(P.halo, 30, 5)])]
CLIPS['think'] = [
    fr(220, 'open', props=[at(P.bubble, 52, 10, 1)]), fr(220, 'focused', props=[at(P.bubble, 52, 10, 1), at(P.bubble, 55, 6, 2)]),
    fr(260, 'focused', dx=1, props=[at(P.bubble, 52, 10, 1), at(P.bubble, 55, 6, 2)]),
    fr(260, 'focused', dx=1, props=[at(P.bubble, 52, 10, 1), at(P.bubble, 55, 6, 2), at(P.bubble, 59, 0, 3)]),
    fr(260, 'focused', dx=1, bob=1, props=[at(P.bubble, 52, 10, 1), at(P.bubble, 55, 6, 2), at(P.bubble, 59, 0, 3)]),
    fr(300, 'focused', dx=1, props=[at(P.bubble, 52, 10, 1), at(P.bubble, 55, 6, 2), at(P.bubble, 59, 0, 3)]),
    fr(170, 'curious', props=[at(P.sparkle, 59, 3)]), fr(170, 'open', props=[at(P.sparkle, 59, 3, False)]),
    fr(300, 'open'),
]
CLIPS['work'] = [
    fr(150, 'focused', hands='type', props=[at(P.keyboard, 24, 58, 0)]),
    fr(170, 'focused', hands='type', props=[at(P.keyboard, 24, 58, 1), at(P.code, 51, 3, 3)]),
    fr(150, 'focused', hands='type', bob=1, props=[at(P.keyboard, 24, 58, 2), at(P.code, 51, 3, 7)]),
    fr(170, 'focused', hands='type', props=[at(P.keyboard, 24, 58, 3), at(P.code, 51, 3, 11)]),
    fr(150, 'focused', hands='type', props=[at(P.keyboard, 24, 58, 4), at(P.code, 51, 3, 11)]),
    fr(210, 'curious', hands='type', bob=1, props=[at(P.keyboard, 24, 58, 0), at(P.code, 51, 3, 11)]),
    fr(150, 'focused', hands='type', props=[at(P.keyboard, 24, 58, 1), at(P.code, 51, 3, 7)]),
    fr(170, 'focused', hands='type', props=[at(P.keyboard, 24, 58, 2), at(P.code, 51, 3, 3)]),
]
CLIPS['parallel'] = [
    fr(150, 'curious', dx=-1, props=[at(P.gear, 6, 6, 0), at(P.card, 58, 4)]),
    fr(150, 'curious', dx=-1, props=[at(P.gear, 6, 6, 1), at(P.card, 58, 4)]),
    fr(150, 'open', props=[at(P.gear, 10, 4, 0), at(P.card, 54, 6)]),
    fr(150, 'focused', dx=1, props=[at(P.gear, 14, 3, 1), at(P.card, 50, 7)]),
    fr(150, 'focused', dx=1, props=[at(P.gear, 14, 3, 0), at(P.card, 50, 7), at(P.sparkle, 60, 3, False)]),
    fr(150, 'open', props=[at(P.gear, 10, 4, 1), at(P.card, 54, 6)]),
]
CLIPS['build'] = [
    fr(100, 'focused', hands='up', props=[at(P.gear, 56, 4, 0)]),
    fr(100, 'focused', hands='up', bob=1, props=[at(P.gear, 56, 4, 1), at(P.spark_lines, 52, 9)]),
    fr(100, 'focused', hands='clasp', props=[at(P.gear, 56, 4, 0)]),
    fr(120, 'smug', hands='up', tail=1, props=[at(P.gear, 56, 4, 1), at(P.sparkle, 60, 10, False)]),
    fr(100, 'focused', hands='up', props=[at(P.gear, 56, 4, 0), at(P.spark_lines, 52, 9)]),
    fr(130, 'focused', hands='clasp', bob=1, props=[at(P.gear, 56, 4, 1)]),
]
CLIPS['error'] = [
    fr(80, 'startled', dx=2, props=[at(P.cross, 55, 3)]), fr(80, 'startled', dx=-2, props=[at(P.cross, 55, 3)]),
    fr(90, 'startled', dx=2, props=[at(P.cross, 55, 3), at(P.sweat, 60, 18)]),
    fr(200, 'shy', props=[at(P.cross, 55, 3), at(P.sweat, 60, 20), at(P.dizzy, 44, 4, 0)]),
    fr(200, 'shy', bob=1, props=[at(P.cross, 55, 3), at(P.sweat, 60, 22), at(P.dizzy, 44, 4, 1)]),
    fr(160, 'focused', props=[at(P.cross, 55, 3), at(P.sweat, 60, 24)]),
    fr(200, 'shy', props=[at(P.cross, 55, 3)]), fr(300, 'open', props=[at(P.cross, 55, 3)]),
]
CLIPS['complete'] = [
    fr(90, 'happy', wing=-2, tail=-2, props=[at(P.check, 55, 3)]),
    fr(100, 'happy', wing=-2, tail=-2, dy=-2, props=[at(P.check, 55, 3), at(P.sparkle, 8, 10), at(P.sparkle, 62, 20, False)]),
    fr(100, 'happy', wing=-2, tail=-2, dy=-2, props=[at(P.check, 55, 3), at(P.sparkle, 4, 14), at(P.sparkle, 64, 14), at(P.sparkle, 30, 3, False)]),
    fr(110, 'happy', wing=-1, tail=-1, props=[at(P.check, 55, 3), at(P.sparkle, 6, 18, False), at(P.sparkle, 62, 10, False)]),
    fr(120, 'happy', props=[at(P.check, 55, 3), at(P.sparkle, 10, 8, False)]),
    fr(200, 'happy', bob=1, props=[at(P.check, 55, 3)]), fr(200, 'happy', props=[at(P.check, 55, 3)]),
    fr(400, 'open', props=[at(P.check, 55, 3)]),
]
CLIPS['ask'] = [
    fr(200, 'curious', dx=1, props=[at(P.question, 56, 2)]), fr(200, 'curious', dx=1, ahoge=1, props=[at(P.question, 56, 3)]),
    fr(200, 'curious', dx=1, ahoge=-1, props=[at(P.question, 56, 2)]), fr(200, 'curious', dx=1, bob=1, props=[at(P.question, 56, 3)]),
    fr(110, 'blink', dx=1, props=[at(P.question, 56, 2)]), fr(200, 'curious', dx=1, props=[at(P.question, 56, 3)]),
    fr(200, 'curious', dx=1, ahoge=1, props=[at(P.question, 56, 2)]), fr(200, 'curious', dx=1, props=[at(P.question, 56, 3)]),
]
CLIPS['organize'] = [
    fr(100, 'focused', props=[at(P.card, 4, 36), at(P.card, 8, 30)]), fr(100, 'focused', dx=-1, props=[at(P.card, 8, 34), at(P.card, 12, 24)]),
    fr(100, 'focused', props=[at(P.card, 12, 30), at(P.card, 22, 14)]), fr(130, 'focused', bob=1, props=[at(P.card, 22, 22), at(P.card, 36, 8)]),
    fr(130, 'focused', dx=1, props=[at(P.card, 36, 14), at(P.card, 50, 4)]), fr(130, 'smug', props=[at(P.card, 56, 4), at(P.card, 56, 9)]),
    fr(200, 'smug', bob=1, props=[at(P.card, 56, 4), at(P.card, 56, 9), at(P.card, 56, 14), at(P.sparkle, 64, 6, False)]),
    fr(200, 'open', props=[at(P.card, 56, 4), at(P.card, 56, 9), at(P.card, 56, 14)]),
]
CLIPS['carry'] = [
    fr(140, 'smug', hands='hold', props=[at(P.box, 26, 49)]), fr(140, 'smug', hands='hold', dx=1, dy=-1, props=[at(P.box, 26, 49)]),
    fr(140, 'smug', hands='hold', dx=2, props=[at(P.box, 26, 49)]), fr(140, 'smug', hands='hold', dx=1, dy=-1, props=[at(P.box, 26, 49)]),
    fr(140, 'smug', hands='hold', props=[at(P.box, 26, 49)]), fr(140, 'smug', hands='hold', dx=-1, dy=-1, props=[at(P.box, 26, 49)]),
    fr(140, 'smug', hands='hold', dx=-2, props=[at(P.box, 26, 49)]), fr(140, 'smug', hands='hold', dx=-1, dy=-1, props=[at(P.box, 26, 49)]),
]
CLIPS['wake'] = [
    fr(420, 'closed', props=[at(P.z, 54, 6)]), fr(110, 'blink', props=[at(P.z, 56, 4)]), fr(120, 'closed'),
    fr(100, 'startled', props=[at(P.exclaim, 55, 3)]), fr(110, 'startled', dy=-2, props=[at(P.exclaim, 55, 3)]),
    fr(200, 'open', wing=-2, bob=1), fr(70, 'blink', wing=-2, bob=1), fr(80, 'open', wing=-1), fr(70, 'blink'), fr(500, 'open'),
]
CLIPS['poke_left'] = [
    fr(300, 'open'), fr(110, 'startled', dx=2, props=[at(P.finger, 22, 40, True)]), fr(90, 'startled', dx=3, props=[at(P.finger, 22, 40, True)]),
    fr(200, 'shy', dx=2, props=[at(P.finger, 24, 40, True)]), fr(140, 'shy', dx=1, props=[at(P.heart, 56, 14, False)]),
    fr(140, 'shy', props=[at(P.heart, 56, 10, False)]), fr(240, 'happy', props=[at(P.heart, 56, 7)]),
    fr(200, 'happy', props=[at(P.heart, 56, 4)]), fr(200, 'open', props=[at(P.heart, 56, 1)]), fr(400, 'open'),
]
CLIPS['poke_right'] = [
    fr(300, 'open'), fr(110, 'startled', dx=-2, props=[at(P.finger, 47, 40, False)]), fr(90, 'startled', dx=-3, props=[at(P.finger, 47, 40, False)]),
    fr(200, 'shy', dx=-2, props=[at(P.finger, 45, 40, False)]), fr(140, 'shy', dx=-1, props=[at(P.heart, 6, 14, False)]),
    fr(140, 'shy', props=[at(P.heart, 6, 10, False)]), fr(240, 'happy', props=[at(P.heart, 6, 7)]),
    fr(200, 'happy', props=[at(P.heart, 6, 4)]), fr(200, 'open', props=[at(P.heart, 6, 1)]), fr(400, 'open'),
]
CLIPS['tickle'] = [
    fr(90, 'happy', dx=-1, tail=1, props=[at(P.sparkle, 58, 8)]), fr(90, 'happy', dx=1, tail=-1, props=[at(P.heart, 56, 4)]),
    fr(90, 'happy', dx=-1, tail=1, props=[at(P.sparkle, 6, 8)]), fr(90, 'happy', dx=1, tail=-1, props=[at(P.sparkle, 58, 8, False)]),
    fr(90, 'happy', dx=-1, tail=1, props=[at(P.heart, 6, 4)]), fr(90, 'happy', dx=1, tail=-1, props=[at(P.sparkle, 60, 5)]),
    fr(200, 'happy', props=[at(P.heart, 56, 3)]), fr(400, 'open', props=[at(P.heart, 56, 1, False)]),
]
CLIPS['lift'] = [
    fr(90, 'startled', dy=-5, wing=-2, tail=1, props=[at(P.exclaim, 55, 2)]), fr(90, 'startled', dy=-6, wing=-2, tail=2, props=[at(P.exclaim, 55, 2), at(P.dust, 30, 61)]),
    fr(90, 'startled', dy=-6, wing=-3, tail=2, props=[at(P.dust, 30, 62), at(P.dust, 38, 63)]), fr(90, 'shy', dy=-5, wing=-2, tail=1, props=[at(P.dust, 34, 62)]),
    fr(90, 'shy', dy=-6, wing=-3, tail=2), fr(90, 'startled', dy=-5, wing=-2, tail=1),
]
CLIPS['praise'] = [
    fr(100, 'open', hands='up'), fr(100, 'happy', hands='up', props=[at(P.heart, 56, 8, False)]), fr(110, 'happy', hands='up', props=[at(P.heart, 56, 6)]),
    fr(120, 'happy', hands='up', bob=1, props=[at(P.heart, 56, 4), at(P.sparkle, 6, 12)]), fr(120, 'smug', hands='up', props=[at(P.heart, 56, 3), at(P.sparkle, 8, 8, False)]),
    fr(160, 'smug', hands='up', bob=1, props=[at(P.heart, 56, 2)]), fr(160, 'happy', hands='up', props=[at(P.sparkle, 58, 4)]),
    fr(100, 'smug', hands='up', props=[at(P.heart, 52, 5)]), fr(100, 'happy', hands='up', bob=1, props=[at(P.heart, 52, 3)]),
    fr(100, 'smug', hands='up', props=[at(P.heart, 56, 6, False)]), fr(120, 'happy', hands='up'), fr(100, 'open', hands='clasp'), fr(300, 'open'),
]
# ---- explicit expressions: short loops that hold a face, like the idle clips ---------
def hold(expr, *symbols):
    base = [fr(500, expr), fr(300, expr, bob=1, ahoge=1, tail=1), fr(300, expr, bob=1, tail=2), fr(300, expr, ahoge=-1, tail=1),
            fr(500, expr, ahoge=1), fr(300, expr, bob=1, tail=1)]
    marked = []
    for i, (ms, spec) in enumerate(base):
        spec = dict(spec); spec['props'] = [s for s in symbols] if i % 2 == 0 else []
        marked.append((ms, spec))
    return marked
CLIPS['exp_curious'] = hold('curious', at(P.question, 56, 3))
CLIPS['exp_focused'] = hold('focused')
CLIPS['exp_shy'] = hold('shy', at(P.heart, 56, 6, False))
CLIPS['exp_happy'] = hold('happy', at(P.sparkle, 58, 6))
CLIPS['exp_startled'] = hold('startled', at(P.exclaim, 55, 3), at(P.sweat, 60, 18))
CLIPS['exp_smug'] = hold('smug', at(P.sparkle, 58, 6, False))

# companion action name -> clip (mirrors CompanionAction in kodama-term)
ACTION_CLIP = {
    'idle': 'idle', 'blink': 'blink', 'move': 'roam', 'observe': 'look', 'eat': 'eat', 'play': 'play',
    'sleep': 'sleep', 'startled': 'startle', 'sick': 'sick', 'evolve': 'evolve', 'end': 'end', 'think': 'think',
    'work': 'work', 'parallel': 'parallel', 'build': 'build', 'error': 'error', 'complete': 'complete',
    'await_confirmation': 'ask', 'organize': 'organize', 'carry': 'carry', 'wake': 'wake',
    'poke_left': 'poke_left', 'poke_right': 'poke_right', 'tickle': 'tickle', 'lift': 'lift', 'praise': 'praise',
}

def render_clip(name):
    return [(ms, frame(**spec)) for ms, spec in CLIPS[name]]
