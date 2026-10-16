#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright 2026 Wireframe contributors
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
R=Path(__file__).resolve().parents[1];P=R/'assets/pdf';P.mkdir(exist_ok=True)
names=['.notdef']+[f'g{i}'for i in range(1,513)]
f=FontBuilder(1000,isTTF=True);f.setupGlyphOrder(names);f.setupCharacterMap({32:'g1'})
p=TTGlyphPen(None);p.moveTo((0,-200));p.lineTo((1000,-200));p.lineTo((1000,800));p.lineTo((0,800));p.closePath();glyph=p.glyph()
f.setupGlyf({n:glyph for n in names});f.setupHorizontalMetrics({n:(1000,0)for n in names});f.setupHorizontalHeader(ascent=800,descent=-200)
f.setupNameTable({'familyName':'Wireframe PDF Semantic','styleName':'Regular','uniqueFontIdentifier':'Wireframe PDF Semantic','fullName':'Wireframe PDF Semantic','psName':'WireframePDFSemantic'})
f.setupOS2(sTypoAscender=800,sTypoDescender=-200,usWinAscent=800,usWinDescent=200);f.setupPost();f.setupMaxp();f.font['head'].created=f.font['head'].modified=0;f.save(P/'semantic.ttf')
