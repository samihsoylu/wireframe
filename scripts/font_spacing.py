# SPDX-License-Identifier: GPL-3.0-only
# Copyright 2026 Wireframe contributors
"""Drawing-font advance design and math outlines from retained OFL sources."""
import hashlib
import string
import unicodedata

import pathops
from fontTools.ttLib import TTFont
from fontTools.pens.recordingPen import DecomposingRecordingPen
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.pens.cu2quPen import Cu2QuPen


ADVANCES = {
    'Regular': (1.07, 1.04, 1.11, .95),
    'Bold': (1.07, 1.06, 1.13, 1.02),
    'Italic': (1.08, 1.04, 1.06, .96),
    'BoldItalic': (1.08, 1.06, 1.08, 1.02),
}


def space_letters(font, style):
    """Set advances without moving outlines or inherited positioning anchors."""
    scales = ADVANCES[style]
    changes, conflicts = {}, set()
    for codepoint, name in font.getBestCmap().items():
        char = chr(codepoint)
        base = unicodedata.normalize('NFD', char)[0]
        if char in ' \u00a0':
            factor = .75
        elif base in string.ascii_uppercase:
            factor = scales[0]
        elif base in string.ascii_lowercase:
            factor = scales[1]
        elif char in string.digits:
            factor = scales[2]
        elif char in ',.;:!?-()':
            factor = scales[3]
        else:
            factor = 1
        if name in changes and changes[name] != factor:
            conflicts.add(name)
        changes[name] = factor
    # An aliased glyph must not acquire contradictory category treatment.
    for name in conflicts:
        changes[name] = 1
    for name, factor in changes.items():
        advance, bearing = font['hmtx'][name]
        font['hmtx'][name] = (round(advance * factor), bearing)
    return dict(uppercase=scales[0], lowercase=scales[1], digits=scales[2],
                punctuation=scales[3], space=.75, alias_conflicts=sorted(conflicts))


def draw_operators(font, style, sources):
    """Fit licensed math outlines to x-height and a shared 0.65-em advance."""
    source_path = sources / f"ComicNeue-{'Italic' if 'Italic' in style else 'Regular'}.ttf"
    source = TTFont(source_path, recalcTimestamp=False)
    source_cmap, cmap = source.getBestCmap(), font.getBestCmap()
    glyphset = source.getGlyphSet()
    stroke = 32 if 'Bold' in style else 0
    advance = round(.65 * font['head'].unitsPerEm)
    x = font['glyf'][cmap[ord('x')]]
    x.recalcBounds(font['glyf'])
    changes = []
    for char in '+×':
        glyph = source['glyf'][source_cmap[ord(char)]]
        glyph.recalcBounds(source['glyf'])
        scale = (x.yMax - stroke) / (glyph.yMax - glyph.yMin)
        dx = (advance - (glyph.xMax - glyph.xMin) * scale) / 2 - glyph.xMin * scale
        dy = stroke / 2 - glyph.yMin * scale
        recording = DecomposingRecordingPen(glyphset)
        glyphset[source_cmap[ord(char)]].draw(recording)
        path = pathops.Path()
        recording.replay(TransformPen(path.getPen(), (scale, 0, 0, scale, dx, dy)))
        if stroke:
            outline = pathops.Path()
            outline.addPath(path)
            outline.stroke(stroke, pathops.LineCap.ROUND_CAP, pathops.LineJoin.ROUND_JOIN, 4.)
            outline.convertConicsToQuads(.25)
            path = pathops.op(path, outline, pathops.PathOp.UNION)
        pen = TTGlyphPen(None)
        path.draw(Cu2QuPen(pen, max_err=.5, reverse_direction=True))
        name = cmap[ord(char)]
        font['glyf'][name] = pen.glyph()
        font['glyf'][name].recalcBounds(font['glyf'])
        font['hmtx'][name] = (advance, font['glyf'][name].xMin)
        changes.append(dict(character=char, glyph=name, source_glyph=source_cmap[ord(char)],
                            scale=scale, translation=[dx, dy], stroke=stroke, advance=advance))
    return dict(source=source_path.name,
                source_sha256=hashlib.sha256(source_path.read_bytes()).hexdigest(), changes=changes)
