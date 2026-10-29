#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Regenerate synthetic screenshot fixture with Pillow 11.3.0 and bundled OFL font."""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
root = Path(__file__).resolve().parents[3]
font_path = root / 'assets/fonts/comic-neue/ComicNeue-Regular.ttf'
font = ImageFont.truetype(str(font_path), 36)
small = ImageFont.truetype(str(font_path), 28)
image = Image.new('RGB', (1100, 620), 'white')
draw = ImageDraw.Draw(image)
draw.rectangle((0, 0, 1100, 85), fill='#f0f2f5')
draw.text((35, 22), 'Project overview', font=font, fill='#202020')
draw.text((35, 125), 'Welcome to Wireframe', font=font, fill='#202020')
draw.text((35, 205), 'Your ideas, clearly arranged.', font=small, fill='#202020')
draw.rectangle((35, 295, 490, 425), fill='#f7f8fa', outline='#c8c8c8', width=2)
draw.text((55, 320), 'Draft screens: 12', font=small, fill='#202020')
draw.text((55, 365), 'Ready for review', font=small, fill='#202020')
draw.rounded_rectangle((35, 490, 285, 555), radius=10, fill='#e5ecfc', outline='#7898d0', width=2)
draw.text((60, 505), 'Create project', font=small, fill='#202020')
image.save(Path(__file__).parent / 'fixtures/screenshot.png')

# Deliberately oversized IHDR with valid CRC; pixels must never be decoded.
import struct, zlib
def chunk(kind, data):
    return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
large = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', 32768, 32768, 8, 2, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(b'\0')) + chunk(b'IEND', b'')
(Path(__file__).parent / 'fixtures/oversized-header.png').write_bytes(large)
