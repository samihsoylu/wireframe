# SPDX-License-Identifier: GPL-3.0-only
# Copyright 2026 Wireframe contributors
"""Compose missing/corrected letters from the retained font's own components."""
import copy
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.otlLib.builder import (
    buildLigatureSubstSubtable, buildLookup, buildMarkGlyphSetsDef,
)


def _attachment(font, base, mark):
    offsets = set()
    for lookup in font['GPOS'].table.LookupList.Lookup:
        if lookup.LookupType != 4:
            continue
        for sub in lookup.SubTable:
            if mark not in sub.MarkCoverage.glyphs or base not in sub.BaseCoverage.glyphs:
                continue
            record = sub.MarkArray.MarkRecord[sub.MarkCoverage.glyphs.index(mark)]
            anchor = sub.BaseArray.BaseRecord[sub.BaseCoverage.glyphs.index(base)].BaseAnchor[record.Class]
            if anchor:
                offsets.add((anchor.XCoordinate - record.MarkAnchor.XCoordinate,
                             anchor.YCoordinate - record.MarkAnchor.YCoordinate))
    assert len(offsets) == 1, (base, mark, offsets)
    return offsets.pop()


def _inherit_positioning(font, aliases, bases):
    for lookup in font['GPOS'].table.LookupList.Lookup:
        for sub in lookup.SubTable:
            if lookup.LookupType == 4:
                for new, base in bases.items():
                    if base in sub.BaseCoverage.glyphs:
                        index = sub.BaseCoverage.glyphs.index(base)
                        sub.BaseCoverage.glyphs.append(new)
                        sub.BaseArray.BaseRecord.append(copy.deepcopy(sub.BaseArray.BaseRecord[index]))
            elif lookup.LookupType == 2 and sub.Format == 1:
                # Extend second-glyph pairs before cloning first-glyph sets.
                for pairs in sub.PairSet:
                    original = {record.SecondGlyph: record for record in pairs.PairValueRecord}
                    for new, old in aliases.items():
                        if old in original:
                            record = copy.deepcopy(original[old])
                            record.SecondGlyph = new
                            pairs.PairValueRecord.append(record)
                    pairs.PairValueRecord.sort(key=lambda record: font.getGlyphID(record.SecondGlyph))
                for new, old in aliases.items():
                    if old in sub.Coverage.glyphs:
                        index = sub.Coverage.glyphs.index(old)
                        sub.Coverage.glyphs.append(new)
                        sub.PairSet.append(copy.deepcopy(sub.PairSet[index]))
            elif lookup.LookupType == 2 and sub.Format == 2:
                for new, old in aliases.items():
                    if old in sub.Coverage.glyphs:
                        sub.Coverage.glyphs.append(new)
                    for definition in [sub.ClassDef1, sub.ClassDef2]:
                        if old in definition.classDefs:
                            definition.classDefs[new] = definition.classDefs[old]


def _inherit_contexts(font, aliases):
    # The pinned sources use chaining coverage for case-sensitive accents.
    # These coverage arrays have no parallel glyph-indexed substitution records.
    for lookup in font['GSUB'].table.LookupList.Lookup:
        if lookup.LookupType != 6:
            continue
        for sub in lookup.SubTable:
            if sub.Format != 3:
                continue
            for field in ['BacktrackCoverage', 'InputCoverage', 'LookAheadCoverage']:
                for coverage in getattr(sub, field, []):
                    for new, old in aliases.items():
                        if old in coverage.glyphs:
                            coverage.glyphs.append(new)


def _compose_letters(font, specifications):
    """Use final transformed anchors so composite and positioned marks agree."""
    repairs = []
    aliases = {}
    bases = {}
    for codepoint, base, mark, sibling in specifications:
        name = f'uni{codepoint:04X}'
        assert name not in font.getGlyphOrder()
        dx, dy = _attachment(font, base, mark)
        pen = TTGlyphPen(font.getGlyphSet())
        pen.addComponent(base, (1, 0, 0, 1, 0, 0))
        pen.addComponent(mark, (1, 0, 0, 1, dx, dy))
        glyph = pen.glyph()
        order = font.getGlyphOrder().copy()
        font['glyf'][name] = glyph
        font.setGlyphOrder(order + [name])
        glyph.recalcBounds(font['glyf'])
        font['hmtx'][name] = (font['hmtx'][base][0], glyph.xMin)
        font['GDEF'].table.GlyphClassDef.classDefs[name] = 1
        previous = font.getBestCmap().get(codepoint)
        for table in font['cmap'].tables:
            if table.isUnicode():
                table.cmap[codepoint] = name
        aliases[name] = sibling
        bases[name] = base
        repairs.append(dict(codepoint=codepoint, glyph=name, previous=previous,
                            components=[base, mark], mark_offset=[dx, dy],
                            positioning_sibling=sibling))
    _inherit_positioning(font, aliases, bases)
    _inherit_contexts(font, aliases)
    return repairs


def repair_cedilla_letters(font):
    return _compose_letters(font, [
        (0x0162, 'T', 'uni0327', 'uni021A'),
        (0x0163, 't', 'uni0327', 'uni021B'),
    ])


def add_breve_letters(font):
    # Uppercase contextual shaping selects the font's case-specific accent.
    # Both letters keep their base's pair positioning and mark anchors.
    repairs = _compose_letters(font, [
        (0x011E, 'G', 'uni0306.case', 'G'),
        (0x011F, 'g', 'uni0306', 'g'),
    ])
    # Canonical ordering can put a below-mark between the base and breve.
    # Compose before case-context rules, ignoring only the pinned fonts' below
    # marks. Other above-marks remain blockers, preserving their input order.
    definition = font['GDEF'].table
    existing = getattr(definition, 'MarkGlyphSetsDef', None)
    mark_sets = [set(c.glyphs) for c in existing.Coverage] if existing else []
    below = {'uni0327', 'uni0327.case', 'uni0328', 'uni0328.case'}
    mark_sets.append({name for name, kind in definition.GlyphClassDef.classDefs.items()
                      if kind == 3 and name not in below})
    definition.MarkGlyphSetsDef = buildMarkGlyphSetsDef(mark_sets, font.getReverseGlyphMap())
    definition.Version = max(definition.Version, 0x00010002)
    substitution = font['GSUB'].table
    # Shapers apply selected lookups in numeric order. Put composition first
    # and shift inherited references without changing their relative order.
    for feature in substitution.FeatureList.FeatureRecord:
        feature.Feature.LookupListIndex = [i + 1 for i in feature.Feature.LookupListIndex]
    for lookup in substitution.LookupList.Lookup:
        for sub in lookup.SubTable:
            if lookup.LookupType == 6:
                assert sub.Format == 3
                for record in sub.SubstLookupRecord:
                    record.LookupListIndex += 1
    substitution.LookupList.Lookup.insert(0, buildLookup([
        buildLigatureSubstSubtable({
            ('G', 'uni0306'): 'uni011E',
            ('g', 'uni0306'): 'uni011F',
        }),
    ], markFilterSet=len(mark_sets) - 1))
    features = [f.Feature for f in substitution.FeatureList.FeatureRecord
                if f.FeatureTag == 'ccmp']
    assert len(features) == 1
    features[0].LookupListIndex.insert(0, 0)
    return repairs
