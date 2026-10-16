#!/usr/bin/env python3
"""Collect exact dependency notices for the supported native targets."""
import concurrent.futures
import json
from pathlib import Path
import re
import shutil
import subprocess
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
DEST = ROOT / 'licenses'
TARGETS = ('aarch64-apple-darwin', 'x86_64-apple-darwin', 'x86_64-pc-windows-msvc', 'x86_64-unknown-linux-gnu')

def cargo(*args):
    return subprocess.check_output(['cargo', *args], cwd=ROOT, text=True)

def is_notice(path):
    name = path.name.lower()
    return path.is_file() and (any(x in name for x in ('license', 'licence', 'copying', 'notice'))
        or name in ('ofl.txt', 'ufl.txt', 'hack-regular.txt')) and path.suffix not in ('.rs', '.json', '.toml', '.py')

def fetch(url):
    try:
        with urllib.request.urlopen(url, timeout=20) as response:
            return response.read()
    except (urllib.error.HTTPError, urllib.error.URLError):
        return None

def main():
    meta = json.loads(cargo('metadata', '--locked', '--format-version', '1'))
    used = set()
    for target in TARGETS:
        for line in cargo('tree', '--locked', '--edges', 'normal,build', '--prefix', 'none', '--format', '{p}', '--target', target).splitlines():
            match = re.match(r'([^ ]+) v([^ ]+)', line)
            if match: used.add(match.groups())
    packages = [p for p in meta['packages'] if p['source'] and (p['name'], p['version']) in used]
    DEST.mkdir(exist_ok=True)
    upstream = {}
    missing = []
    manifest = []
    for package in packages:
        source = Path(package['manifest_path']).parent
        key = f"{package['name']}-{package['version']}"
        folder = DEST / key
        folder.mkdir(exist_ok=True)
        paths = sorted(p for p in source.rglob('*') if is_notice(p))
        files = []
        for path in paths:
            relative = path.relative_to(source)
            output = folder / relative
            output.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, output)
            files.append(str(output.relative_to(DEST)))
        if package['name'] == 'krilla':
            # Its upstream NOTICE attributes content_draw_path to MPL code.
            # Retain the unchanged containing source alongside those notices.
            covered = folder / 'src' / 'content.rs'
            covered.parent.mkdir(exist_ok=True)
            shutil.copyfile(source / 'src' / 'content.rs', covered)
            files.append(str(covered.relative_to(DEST)))
        # Workspace crates sometimes exclude the repository-root license.
        # These packages contain asset notices but omit the library's own terms.
        need_root = not files or package['name'] in ('epaint_default_fonts', 'krilla')
        if need_root:
            vcs = source / '.cargo_vcs_info.json'
            sha = json.loads(vcs.read_text())['git']['sha1'] if vcs.exists() else None
            repo = package.get('repository') or ''
            match = re.search(r'github.com/([^/]+/[^/]+)', repo)
            if match and sha and package['name'] != 'dispatch':
                repo = match.group(1).removesuffix('.git')
                upstream.setdefault((repo, sha), []).append((folder, files))
            elif package['name'] == 'dispatch' and package['version'] == '0.2.0':
                # This published crate declares MIT but ships no license file.
                # Preserve its exact declaration and the SPDX reference text.
                declaration = folder / 'UPSTREAM-DECLARATION.toml'
                shutil.copyfile(source / 'Cargo.toml.orig', declaration)
                reference = folder / 'MIT-reference.txt'
                data = fetch('https://raw.githubusercontent.com/spdx/license-list-data/main/text/MIT.txt')
                if not data: raise SystemExit('Could not fetch MIT reference terms')
                reference.write_bytes(data)
                files.extend([str(declaration.relative_to(DEST)), str(reference.relative_to(DEST))])
                shutil.copytree(source / 'src', folder / 'src', dirs_exist_ok=True)
            elif not files:
                missing.append(key)
        manifest.append({'name':package['name'], 'version':package['version'], 'license':package['license'], 'repository':package['repository'], 'files':files})
        if package['license'] == 'MPL-2.0':
            # Supply the unchanged covered source alongside the binary notices.
            shutil.copytree(source / 'src', folder / 'src', dirs_exist_ok=True)
            shutil.copyfile(source / 'Cargo.toml.orig', folder / 'Cargo.toml')
    candidates = ('LICENSE', 'LICENSE-MIT', 'LICENSE-APACHE', 'LICENSE_MIT', 'LICENSE_APACHE', 'LICENSE.md', 'LICENSE.txt', 'LICENSE-MIT.txt', 'LICENSE-APACHE.txt', 'licenses/MIT.txt', 'licenses/Apache-2.0.txt', 'COPYING', 'NOTICE', 'NOTICE.md')
    jobs = [(repo, sha, name, f'https://raw.githubusercontent.com/{repo}/{sha}/{name}') for repo, sha in upstream for name in candidates]
    with concurrent.futures.ThreadPoolExecutor(max_workers=12) as pool:
        for (repo,sha,name,url), data in zip(jobs, pool.map(lambda job: fetch(job[3]), jobs)):
            if data and len(data) < 2_000_000:
                for folder, files in upstream[(repo,sha)]:
                    output = folder / ('upstream-' + name.replace('/', '-'))
                    output.write_bytes(data)
                    files.append(str(output.relative_to(DEST)))
    for folders in upstream.values():
        for folder, files in folders:
            if not any(Path(f).name.startswith('upstream-') and
                       any(term in Path(f).name.lower() for term in ('license', 'licence', 'copying', 'mit', 'apache'))
                       for f in files):
                missing.append(folder.name + ' (repository-root license)')
    missing.extend(f"{p['name']}-{p['version']}" for p in manifest if not p['files'])
    if missing:
        raise SystemExit('No notices found for: ' + ', '.join(sorted(set(missing))))
    (DEST / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    rows = ['# Third-party notices', '', 'Wireframe project code is GPL-3.0-only licensed. Dependencies retain the terms below. This inventory covers the locked normal and build dependency graph for macOS, Windows and Linux; some entries are build tools or platform-specific.', '', 'Full license texts and applicable copyright notices are in `licenses/`. Unchanged source for option-ext and the file containing Krilla\'s MPL-attributed path conversion is included there. The dispatch crate ships an MIT declaration without a license text; its exact declaration, source and SPDX reference terms are included. Dual-license expressions are upstream choices; inclusion of both texts does not remove that choice.', '', '| Package | License | Notices |', '| --- | --- | --- |']
    for p in manifest:
        key = f"{p['name']}-{p['version']}"
        rows.append(f"| {key} | {p['license']} | [files](licenses/{key}/) |")
    rows.extend(['', '## Canvas fonts', '', 'Comic Neue regular, bold, italic and bold italic are redistributed unchanged under OFL-1.1. Copyright 2014 The Comic Neue Project Authors. See [font license](assets/fonts/comic-neue/OFL.txt) and [provenance](assets/fonts/comic-neue/PROVENANCE.json).'])
    rows.extend(['', 'Wireframe Draft regular, bold, italic and bold italic are modified derivatives of Comic Neue Bold and Bold Italic, redistributed under OFL-1.1 with the original copyright retained. See [license](assets/fonts/wireframe-draft/OFL.txt), [modifications](assets/fonts/wireframe-draft/MODIFICATIONS.txt) and [reproducible build](docs/PROJECT-FONTS.md).'])
    rows.extend(['', 'Unicode emoji property data is covered by [Unicode License V3](assets/unicode/LICENSE.txt); pinned sources and regeneration instructions are in [Unicode data](assets/unicode/README.md).'])
    rows.extend(['', '## Bundled icons', '', 'Font Awesome Free 6.7.2 by Fonticons, Inc. is bundled under CC BY 4.0. SVG path data and dimensions are unchanged; metadata is compacted and free styles are selected. See [license](assets/icons/font-awesome-free/LICENSE.txt), [upstream](https://fontawesome.com), and [provenance](assets/icons/font-awesome-free/PROVENANCE.json). SVG and PNG exports containing these icons include attribution metadata.'])
    (ROOT / 'THIRD_PARTY_NOTICES.md').write_text('\n'.join(rows) + '\n')
    print(f'Collected notices for {len(manifest)} locked dependencies.')

if __name__ == '__main__': main()
