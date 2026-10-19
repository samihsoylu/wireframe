#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright 2026 Wireframe contributors
"""Rebuild the bundled icon catalog from immutable official upstream files."""
from pathlib import Path
from urllib.request import urlopen
import hashlib
import json
import re

ROOT = Path(__file__).resolve().parents[1]
COMMIT = 'af620534bfc3c2d4cbefcfeec29603bbe7809e64'
BASE = f'https://raw.githubusercontent.com/FortAwesome/Font-Awesome/{COMMIT}/'
FILES = {
    'metadata/icons.json': 'a3a705d0e03c4fbdf1a61aece3d8fd462024b33794187ef7ee2a0764439170eb',
    'metadata/categories.yml': 'a6b6900adcad4303d259a334f2a76c5b93f4a5bf70de8e9b39fae1a4fb708d5f',
    'LICENSE.txt': '9b914eae88817d63b576eab5aafde7068c7a1abae125d7cdfb034f1da43a9afc',
}
def categories_from_yaml(data, catalog):
    """Read the pinned upstream's restricted category schema; reject drift."""
    categories = []
    current = None
    for line in data.decode().splitlines():
        if not line.strip():
            continue
        if re.fullmatch(r'[a-z0-9-]+:', line):
            current = dict(id=line[:-1], label='', icons=[])
            categories.append(current)
        elif current is not None and line == '  icons:':
            pass
        elif current is not None and re.fullmatch(r"    - (?:[a-z0-9-]+|'[a-z0-9-]+')", line):
            current['icons'].append(line[6:].strip("'"))
        elif current is not None and line.startswith('  label: '):
            current['label'] = line[9:]
        else:
            raise ValueError(f'Unsupported category metadata: {line!r}')
    known = {icon['id'].rsplit('-', 1)[0] for icon in catalog}
    for category in categories:
        assert category['label'] and category['icons']
        category['icons'] = sorted(set(category['icons']) & known)
    return sorted(categories, key=lambda category: category['label'].lower())

def main():
    raw = {}
    for path, expected in FILES.items():
        data = urlopen(BASE + path, timeout=60).read()
        if hashlib.sha256(data).hexdigest() != expected:
            raise ValueError(f'Upstream checksum mismatch: {path}')
        raw[path] = data
    catalog = []
    for name, item in json.loads(raw['metadata/icons.json']).items():
        for style in item['free']:
            svg = item['svg'][style]
            catalog.append(dict(id=f'{name}-{style}', label=item['label'], style=style,
                keywords=sorted(set(item['search']['terms'] + item.get('aliases', {}).get('names', []))),
                width=svg['width'], height=svg['height'],
                paths=svg['path'] if isinstance(svg['path'], list) else [svg['path']]))
    catalog.sort(key=lambda icon: icon['id'])
    assert len(catalog) == 2060
    dest = ROOT/'assets/icons/font-awesome-free'
    dest.mkdir(parents=True, exist_ok=True)
    data = (json.dumps(catalog, ensure_ascii=False, separators=(',', ':'))+'\n').encode()
    (dest/'catalog.json').write_bytes(data)
    categories = (json.dumps(categories_from_yaml(raw['metadata/categories.yml'], catalog), ensure_ascii=False, separators=(',', ':'))+'\n').encode()
    (dest/'categories.json').write_bytes(categories)
    (dest/'LICENSE.txt').write_bytes(raw['LICENSE.txt'])
    (dest/'PROVENANCE.json').write_text(json.dumps(dict(
        project='Font Awesome Free', version='6.7.2', commit=COMMIT,
        upstream='https://github.com/FortAwesome/Font-Awesome',
        sources=[dict(url=BASE+p, sha256=h) for p,h in FILES.items()],
        license='CC-BY-4.0', attribution='Font Awesome Free by Fonticons, Inc. https://fontawesome.com',
        changes='Metadata compacted and sorted; free variants selected. SVG path strings and dimensions unchanged.',
        catalog_sha256=hashlib.sha256(data).hexdigest(), categories_sha256=hashlib.sha256(categories).hexdigest(), icons=len(catalog)), indent=2)+'\n')
    print(f'Imported {len(catalog)} icons from pinned public upstream; {len(data)} bytes')
if __name__ == '__main__':
    main()
