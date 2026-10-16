#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright 2026 Wireframe contributors
from pathlib import Path
import json,hashlib,shutil
import argparse
parser=argparse.ArgumentParser(description="Regenerate pinned Unicode emoji tables from a local Unicode 17.0 source directory")
parser.add_argument("source",type=Path);args=parser.parse_args()
P=Path(__file__).resolve().parents[1];U=args.source
expected={"emoji-data.txt":"2cb2bb9455cda83e8481541ecf5b6dfda66a3bb89efa3fa7c5297eccf607b72b","emoji-variation-sequences.txt":"bb3d09ef03f206012c7532dd52dc0a21c9efddba0135ea4cf0d9201b8b9bba7e"}
for name,digest in expected.items():
 if hashlib.sha256((U/name).read_bytes()).hexdigest()!=digest:raise SystemExit(f"Unexpected source hash: {name}")
ranges=[]
for line in(U/'emoji-data.txt').read_text().splitlines():
 line=line.split('#')[0].strip()
 if not line:continue
 code,prop=map(str.strip,line.split(';'))
 if prop=='Emoji_Presentation':
  bounds=code.split('..');ranges.append((int(bounds[0],16),int(bounds[-1],16)))
variation=sorted({int(line.split()[0],16)for line in(U/'emoji-variation-sequences.txt').read_text().splitlines()if line and not line.startswith('#') and '; emoji style;'in line})
assert len(ranges)>50 and len(variation)>300
assert ranges==sorted(ranges) and all(a[1]<b[0]for a,b in zip(ranges,ranges[1:]))
p=P/'src/text_engine/portable/emoji_data.rs'
s='// SPDX-License-Identifier: Unicode-3.0\n// Derived from Unicode 17.0 emoji properties; see assets/unicode/README.md.\n// Generated; do not edit by hand.\n'
s+='pub(super) const PRESENTATION: &[(u32,u32)] = &[\n'+''.join(f'    (0x{a:X},0x{b:X}),\n'for a,b in ranges)+'];\n'
s+='pub(super) const VARIATIONS: &[u32] = &[\n'+''.join(f'    0x{a:X},\n'for a in variation)+'];\n';p.write_text(s)
import subprocess
subprocess.run(["rustfmt",str(p)],check=True)
