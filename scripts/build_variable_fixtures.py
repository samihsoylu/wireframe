# SPDX-License-Identifier: GPL-3.0-only
# Copyright 2026 Wireframe contributors
# Original diagnostic outlines; no third-party font data.
from pathlib import Path
import json,hashlib
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.designspaceLib import DesignSpaceDocument,AxisDescriptor,SourceDescriptor
from fontTools.varLib import build
from fontTools.varLib.instancer import instantiateVariableFont
from fontTools.ttLib import TTFont
R=Path(__file__).resolve().parents[1];F=R/'tests/fixtures/text/variable';F.mkdir(parents=True,exist_ok=True);M=F/'masters';M.mkdir(exist_ok=True)
axes=[('Weight','wght',100,400,900),('Width','wdth',75,100,125),('Optical','opsz',8,14,72)]
locations=[dict(Weight=400,Width=100,Optical=14)]
for name,tag,lo,default,hi in axes:
 for value in [lo,hi]:locations.append(dict(locations[0],**{name:value}))
doc=DesignSpaceDocument()
for name,tag,lo,default,hi in axes:
 a=AxisDescriptor();a.name=name;a.tag=tag;a.minimum=lo;a.default=default;a.maximum=hi;doc.addAxis(a)
for index,loc in enumerate(locations):
 fb=FontBuilder(1000,isTTF=True);order=['.notdef','space','A','B','V'];fb.setupGlyphOrder(order);fb.setupCharacterMap({32:'space',65:'A',66:'B',86:'V'})
 width=round(600*loc['Width']/100+(loc['Weight']-400)/10);thick=round(60+(loc['Weight']-100)/8+(loc['Optical']-14)/3)
 glyphs={}
 for k,name in enumerate(order):
  pen=TTGlyphPen(None)
  if name!='space':
   pen.moveTo((30,0));pen.lineTo((width-50,0));pen.lineTo((width-50,thick+k*10));pen.lineTo((30,thick+k*10));pen.closePath()
   pen.moveTo((30,thick+k*10));pen.lineTo((30+thick,thick+k*10));pen.lineTo((30+thick,700-k*15));pen.lineTo((30,700-k*15));pen.closePath()
  glyphs[name]=pen.glyph()
 fb.setupGlyf(glyphs);fb.setupHorizontalMetrics({n:(300 if n=='space' else width,30 if n!='space' else 0)for n in order});fb.setupHorizontalHeader(ascent=800,descent=-200)
 fb.setupNameTable({'familyName':'Variable Probe','styleName':'Regular','uniqueFontIdentifier':f'VariableProbe{index}','fullName':'Variable Probe','psName':'VariableProbe-Regular','version':'Version 1.000'});fb.setupOS2(sTypoAscender=800,sTypoDescender=-200,usWinAscent=800,usWinDescent=200,usWeightClass=400);fb.setupPost();fb.setupMaxp();fb.font['head'].created=fb.font['head'].modified=0
 path=M/f'master{index}.ttf';fb.save(path)
 d=SourceDescriptor();d.path=str(path);d.name=f'master{index}';d.familyName='Variable Probe';d.styleName='Regular';d.location=loc
 if index==0:d.copyInfo=d.copyLib=d.copyFeatures=True
 doc.addSource(d)
doc.write(F/'probe.designspace');font,_,_=build(doc);font.recalcTimestamp=False;font['head'].created=font['head'].modified=0;font.save(F/'variable.ttf')
records=[]
for name,limits in [('default',{}),('heavy',{'wght':700}),('condensed',{'wdth':80}),('display',{'opsz':60})]:
 partial=instantiateVariableFont(font,limits,inplace=False);partial.recalcTimestamp=False;partial.save(F/f'{name}-variable.ttf')
 defaults={a.axisTag:a.defaultValue for a in partial['fvar'].axes};fixed=instantiateVariableFont(partial,defaults,inplace=False);fixed.recalcTimestamp=False;fixed.save(F/f'{name}-static.ttf')
 records.append({'name':name,'pinned_axes':limits,'remaining_defaults':defaults,'variable_tables':sorted(partial.keys()),'static_tables':sorted(fixed.keys())})
(F/'LICENSE.txt').write_text('Original diagnostic font outlines. Copyright 2026 Wireframe contributors. SPDX-License-Identifier: GPL-3.0-only\n')
records={'generator':'FontTools varLib and instancer 4.67.0','cases':records,'sha256':{p.name:hashlib.sha256(p.read_bytes()).hexdigest()for p in sorted(F.glob('*.ttf'))}}
(F/'manifest.json').write_text(json.dumps(records,indent=2)+'\n')
