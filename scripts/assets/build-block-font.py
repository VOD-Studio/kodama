#!/usr/bin/env python3
"""Build the four mathematical block glyphs used by the browser's text preview.

This does not draw pets or replace terminal output. It avoids browser font
leading/antialias seams when displaying the actual ANSI frame's Unicode text.
Requires fonttools and brotli; there is no runtime Python/font dependency in SDK.
"""
from pathlib import Path

from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / "assets/preview/companions/block-grid.woff2"


def rectangle(bottom, top):
    pen = TTGlyphPen(None)
    if top > bottom:
        pen.moveTo((0, bottom))
        pen.lineTo((500, bottom))
        pen.lineTo((500, top))
        pen.lineTo((0, top))
        pen.closePath()
    return pen.glyph()


font = FontBuilder(1000, isTTF=True)
order = [".notdef", "space", "upper", "lower", "full"]
font.setupGlyphOrder(order)
font.setupCharacterMap({32: "space", 0x2580: "upper", 0x2584: "lower", 0x2588: "full"})
font.setupGlyf({
    ".notdef": rectangle(0, 0),
    "space": rectangle(0, 0),
    "upper": rectangle(500, 1000),
    "lower": rectangle(0, 500),
    "full": rectangle(0, 1000),
})
font.setupHorizontalMetrics({name: (500, 0) for name in order})
font.setupHorizontalHeader(ascent=1000, descent=0, lineGap=0)
font.setupNameTable({
    "familyName": "Kodama Block Grid",
    "styleName": "Regular",
    "uniqueFontIdentifier": "KodamaBlockGrid-Regular-1",
    "fullName": "Kodama Block Grid Regular",
    "psName": "KodamaBlockGrid-Regular",
    "version": "Version 1.0",
    "licenseDescription": "MIT; four original mathematical rectangle glyphs.",
})
font.setupOS2(
    version=4,
    sTypoAscender=1000, sTypoDescender=0, sTypoLineGap=0,
    usWinAscent=1000, usWinDescent=0, fsSelection=0xC0,
    xAvgCharWidth=500, sxHeight=500, sCapHeight=1000,
)
font.setupPost(isFixedPitch=1)
font.setupMaxp()
font.font["head"].created = font.font["head"].modified = 2082844800
font.font.recalcTimestamp = False
font.font.flavor = "woff2"
font.save(OUTPUT)
print(f"Built {OUTPUT.relative_to(ROOT)} ({OUTPUT.stat().st_size} bytes)")
