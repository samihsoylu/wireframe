#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright 2026 Wireframe contributors
from pathlib import Path
import json,struct,zlib,hashlib
from fontTools.ttLib import TTFont,newTable
from fontTools.ttLib.tables.sbixStrike import Strike
from fontTools.ttLib.tables.sbixGlyph import Glyph
from fontTools.colorLib.builder import buildCOLR,buildCPAL
R=Path(__file__).resolve().parents[1];F=R/'tests/fixtures/text';F.mkdir(parents=True,exist_ok=True)
base=R/'assets/fonts/wireframe-draft/WireframeDraft-Regular.ttf'
def png(w,h,valid=True,overlong=False,interlaced=False,fragmented=False):
 def chunk(t,b):return struct.pack('>I',len(b))+t+b+struct.pack('>I',zlib.crc32(t+b)&0xffffffff)
 header=struct.pack('>IIBBBBB',w,h,8,6,0,0,int(interlaced))
 raw=(b'\0'+bytes([230,40,20,255])*w)*h if valid else b''
 if overlong:raw=b'\0'*(64*1024*1024)
 if interlaced:
  raw=b''.join(b'\0'+bytes([230,40,20,255])*len(range(x,w,dx)) for x,y,dx,dy in [(0,0,8,8),(4,0,8,8),(0,4,4,8),(2,0,4,4),(0,2,2,4),(1,0,2,2),(0,1,1,2)] for row in range(y,h,dy) if x<w)
 body=zlib.compress(raw)
 return b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',header)+(b''.join(chunk(b'IDAT',body[i:i+1])for i in range(len(body))) if fragmented else chunk(b'IDAT',body))+chunk(b'IEND',b'')
records=[]
for name,ppem,w,h,valid in [('valid-sbix',32,32,32,True),('valid-fragmented',32,32,32,True),('valid-interlaced',32,31,27,True),('overlong-png',32,32,32,True),('decoded-oversize',32,65535,65535,False),('scaled-oversize',1,64,64,True),('overflow-header',32,0xffffffff,0xffffffff,False)]:
 font=TTFont(base,recalcTimestamp=False);glyph=font.getBestCmap()[ord('A')];table=newTable('sbix');strike=Strike(ppem=ppem,resolution=72);strike.glyphs[glyph]=Glyph(glyphName=glyph,originOffsetX=0,originOffsetY=0,graphicType='png ',imageData=png(w,h,valid,overlong=name=="overlong-png",interlaced=name=="valid-interlaced",fragmented=name=="valid-fragmented"));table.strikes[ppem]=strike;font['sbix']=table;path=F/(name+'.ttf');font.save(path);records.append({'name':name,'kind':'sbix','ppem':ppem,'png_dimensions':[w,h],'bytes':path.stat().st_size,'sha256':hashlib.sha256(path.read_bytes()).hexdigest()})
for name,layers,upem in [('valid-colr',2,1000),('colr-oversize',2,1),('colr-layers',257,1000)]:
 font=TTFont(base,recalcTimestamp=False);glyph=font.getBestCmap()[ord('A')];second=font.getBestCmap()[ord('B')];font['COLR']=buildCOLR({glyph:[(glyph if i%2==0 else second,i%2)for i in range(layers)]},version=0,glyphMap=font.getReverseGlyphMap());font['CPAL']=buildCPAL([[(1.,0.,0.,1.),(0.,0.,1.,1.)]]);font['head'].unitsPerEm=upem;path=F/(name+'.ttf');font.save(path);records.append({'name':name,'kind':'COLRv0','layers':layers,'units_per_em':upem,'bytes':path.stat().st_size,'sha256':hashlib.sha256(path.read_bytes()).hexdigest()})
from fontTools.ttLib.tables._c_m_a_p import CmapSubtable
f=TTFont(F/'valid-colr.ttf',recalcTimestamp=False);del f['COLR'];del f['CPAL']
t=CmapSubtable.newSubtable(12);t.platformID=3;t.platEncID=10;t.language=0;t.cmap=f.getBestCmap().copy();t.cmap[0x10ffff]=t.cmap[ord('A')];f['cmap'].tables.append(t)
f.save(F/'private-primary.ttf')
