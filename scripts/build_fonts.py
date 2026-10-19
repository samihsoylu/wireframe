#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright 2026 Wireframe contributors
"""Reproducibly build the bundled OFL font family from its retained sources."""
from pathlib import Path
import argparse,copy,hashlib,json,tempfile
import pathops
from fontTools.ttLib import TTFont
from fontTools.pens.recordingPen import DecomposingRecordingPen
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.pens.cu2quPen import Cu2QuPen
from font_repairs import add_breve_letters, repair_cedilla_letters
from font_spacing import draw_operators, space_letters
P=Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--check',action='store_true',help='Verify committed outputs without changing them')
args=parser.parse_args()
temporary=tempfile.TemporaryDirectory() if args.check else None
committed=P/'assets/fonts/wireframe-draft'
E=Path(temporary.name) if temporary else committed
E.mkdir(parents=True,exist_ok=True)
family='Wireframe Draft';scale=1.02;records=[]
for style,source,stroke,bold,italic in [('Regular','Bold',10.,False,False),('Bold','Bold',34.,True,False),('Italic','BoldItalic',10.,False,True),('BoldItalic','BoldItalic',34.,True,True)]:
 src=P/f'assets/fonts/comic-neue/ComicNeue-{source}.ttf';font=TTFont(src,recalcTimestamp=False);glyphset=font.getGlyphSet();oldcmap=font.getBestCmap().copy();new={};caps=[];shifts=[];components=[]
 marks={k for k,v in font['GDEF'].table.GlyphClassDef.classDefs.items() if v==3}
 for name in font.getGlyphOrder():
  old=font['glyf'][name]
  if old.isComposite() and name not in marks:
   glyph=copy.deepcopy(old)
   for c in glyph.components:
    _,transform=c.getComponentInfo();assert transform[:4]==(1,0,0,1),(name,transform)
    c.x=round(c.x*scale)
   if hasattr(glyph,'program'):del glyph.program
   new[name]=glyph;components.append(dict(glyph=name,parts=[c.glyphName for c in glyph.components]));continue
  recording=DecomposingRecordingPen(glyphset);glyphset[name].draw(recording);path=pathops.Path();recording.replay(TransformPen(path.getPen(),(scale,0,0,1,0,0)))
  pen=TTGlyphPen(None)
  if list(path.verbs):
   def expanded(width):
    outline=pathops.Path();outline.addPath(path);outline.stroke(width,pathops.LineCap.ROUND_CAP,pathops.LineJoin.ROUND_JOIN,4.);outline.convertConicsToQuads(.25)
    return pathops.op(path,outline,pathops.PathOp.UNION)
   actual=stroke;shape=expanded(actual);expected=len(list(path.contours))
   while actual>0 and len(list(shape.contours))!=expected:
    actual-=1.;shape=expanded(actual)
   assert len(list(shape.contours))==expected,(name,actual)
   if actual!=stroke:caps.append(dict(glyph=name,requested=stroke,actual=actual))
   # GPOS anchors remain in their original vertical coordinates: moving the
   # mark's ink gives composed and positioned combining forms the same clearance.
   dx=dy=0
   if bold and name in marks and not name.startswith(('uni0327','uni0328')):
    if name=='uni030C.alt':dx=24
    else:dy=-24 if getattr(old,'yMax',0)<0 else 24
    shifts.append(dict(glyph=name,dx=dx,dy=dy))
   shape.draw(TransformPen(Cu2QuPen(pen,max_err=.5,reverse_direction=True),(1,0,0,1,dx,dy)))
  new[name]=pen.glyph()
 for name,glyph in new.items():font['glyf'][name]=glyph
 for name,glyph in new.items():
  glyph.recalcBounds(font['glyf']);advance,_=font['hmtx'][name];font['hmtx'][name]=(round(advance*scale),getattr(glyph,'xMin',0))
 def transform(obj,seen):
  if id(obj) in seen:return
  seen.add(id(obj))
  if isinstance(obj,(list,tuple)):
   for x in obj:transform(x,seen)
  elif hasattr(obj,'__dict__'):
   for key,value in vars(obj).items():
    if key in ('XCoordinate','XPlacement','XAdvance') and isinstance(value,(int,float)):setattr(obj,key,round(value*scale))
    else:transform(value,seen)
 if 'GPOS' in font:transform(font['GPOS'].table,set())
 repairs=repair_cedilla_letters(font)+add_breve_letters(font)
 for repair in repairs:
  new[repair['glyph']]=font['glyf'][repair['glyph']]
  oldcmap[repair['codepoint']]=repair['glyph']
  components.append(dict(glyph=repair['glyph'],parts=repair['components']))
 operators=draw_operators(font,style,P/'assets/fonts/comic-neue')
 spacing=space_letters(font,style)
 for change in operators['changes']:new[change['glyph']]=font['glyf'][change['glyph']]
 assert not font['GDEF'].table.AttachList and not font['GDEF'].table.LigCaretList
 for table in ['fpgm','prep','cvt ','gasp','DSIG']:
  if table in font:del font[table]
 names=font['name'];display='Bold Italic' if style=='BoldItalic' else style
 values={1:family,2:display,3:family.replace(' ','')+'-'+style+'-1.004',4:family+' '+display,5:'Version 1.004',6:family.replace(' ','')+'-'+style,16:family,17:display,18:family+' '+display}
 for item in list(names.names):
  if item.nameID in values:names.setName(values[item.nameID],item.nameID,item.platformID,item.platEncID,item.langID)
 font['OS/2'].usWeightClass=700 if bold else 400
 font['OS/2'].fsSelection=(font['OS/2'].fsSelection & ~0x61)|(1 if italic else 0)|(32 if bold else 0)|(64 if not bold and not italic else 0)
 font['head'].macStyle=(1 if bold else 0)|(2 if italic else 0)
 font['head'].fontRevision=1.004
 font['OS/2'].xAvgCharWidth=round(sum(v[0] for v in font['hmtx'].metrics.values())/len(new))
 ymax=max(getattr(g,'yMax',0) for g in new.values());ymin=min(getattr(g,'yMin',0) for g in new.values())
 font['OS/2'].usWinAscent=max(font['OS/2'].usWinAscent,ymax);font['OS/2'].usWinDescent=max(font['OS/2'].usWinDescent,-ymin)
 font['head'].modified=font['head'].created;target=E/f'WireframeDraft-{style}.ttf';font.save(target);check=TTFont(target);assert check.getBestCmap()==oldcmap
 records.append(dict(style=style,source=src.name,source_sha256=hashlib.sha256(src.read_bytes()).hexdigest(),sha256=hashlib.sha256(target.read_bytes()).hexdigest(),horizontal_scale=scale,stroke=stroke,stroke_caps=caps,mark_shifts=shifts,preserved_components=components,character_repairs=repairs,operators=operators,spacing=spacing,glyphs=len(new),cmap_entries=len(oldcmap),ink_y_bounds=[ymin,ymax],win_ascent=check['OS/2'].usWinAscent,win_descent=check['OS/2'].usWinDescent,typo_ascent=check['OS/2'].sTypoAscender,typo_descent=check['OS/2'].sTypoDescender))
(E/'OFL.txt').write_bytes((P/'assets/fonts/comic-neue/OFL.txt').read_bytes())
(E/'MODIFICATIONS.txt').write_text('Derivative of Comic Neue Regular, Bold, Italic and Bold Italic under the SIL Open Font License. Renamed Wireframe Draft. Horizontal outlines, advances and horizontal GPOS values scale by 1.02. Four distinct roles use rounded outline expansion of 10 or 34 design units; individual simple glyphs/marks are capped to retain contour counts. Non-mark composites reuse the same expanded base letters, preserving their weight. Detached bold accents move 24 units outward; cedilla/ogonek remain attached. Version 1.004 fits plus and multiplication from the licensed Regular/Italic outlines to lowercase x-height with a 0.65em advance and 32-unit bold expansion. Role-specific category advances refine letter/digit/punctuation spacing; ordinary and nonbreaking spaces share a 0.75 advance factor. Outlines and inherited positioning of other glyphs are unchanged. Revision 1.003 added U+011E/U+011F using existing G/g and breve components with case-sensitive accents and base positioning. The U+0162/U+0163 cedilla repair from revision 1.002 is retained. Pair positioning, mark attachment and contextual accent behavior are preserved. Windows clipping metrics enclose all ink; line spacing metrics remain unchanged. Hint programs removed. Exact modifications and input/output hashes are in derivation.json.\n')
(E/'derivation.json').write_text(json.dumps(records,indent=2)+'\n')
print(json.dumps([dict(style=r['style'],glyphs=r['glyphs'],cmap=r['cmap_entries'],caps=len(r['stroke_caps']),shifted_marks=len(r['mark_shifts']),preserved_composites=len(r['preserved_components']),ink_y_bounds=r['ink_y_bounds']) for r in records],indent=2))

if args.check:
 for path in E.iterdir():
  target=committed/path.name
  assert target.exists() and path.read_bytes()==target.read_bytes(),f"Font output differs: {path.name}"
 print('All bundled font outputs reproduce exactly.')
 temporary.cleanup()
