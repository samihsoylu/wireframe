#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Adapted from samihsoylu/side-tray. Never modifies the input app.
set -eu
cd "$(dirname "$0")/.."
if [ "$#" -ne 3 ]; then
 echo "Usage: sh scripts/package-dmg.sh APP OUTPUT.dmg CORRESPONDING-PACKAGE.zip" >&2
 exit 2
fi
dmg_source=$1
dmg_output=$2
dmg_archive=$3
if [ -e "$dmg_output" ]; then
 echo "Refusing to replace an existing installer: $dmg_output" >&2
 exit 1
fi
codesign --verify --deep --strict "$dmg_source"
# Check the companion archive's app identity before distributing its source.
python3 scripts/verify-dmg-input.py "$dmg_source" "$dmg_archive"
if [ ! -x .tooling/dmg/bin/python ]; then python3 -m venv .tooling/dmg; fi
.tooling/dmg/bin/python -m pip install --disable-pip-version-check -q -r scripts/dmg-requirements.txt
mkdir -p dist "$(dirname "$dmg_output")"
dmg_work=$(mktemp -d dist/dmg-work.XXXXXX)
dmg_stage="$dmg_work/files"
dmg_mount="$dmg_work/mounted"
dmg_attached=0
cleanup() {
 if [ "$dmg_attached" = 1 ]; then hdiutil detach -quiet "$dmg_mount" || return; fi
 rm -rf "$dmg_work"
}
trap cleanup EXIT
mkdir -p "$dmg_stage/.background" "$dmg_stage/Source and licenses" "$dmg_mount"
ditto "$dmg_source" "$dmg_stage/Wireframe.app"
ln -s /Applications "$dmg_stage/Applications"
cp docs/INSTALL.txt "$dmg_stage/Install and Open.txt"
cp "$dmg_archive" "$dmg_stage/Source and licenses/"
cp LICENSE THIRD_PARTY_NOTICES.md "$dmg_stage/Source and licenses/"
xcrun swift scripts/dmg-background.swift "$dmg_stage/.background/installer.tiff"
hdiutil create -quiet -fs HFS+ -volname "Wireframe Installer" -srcfolder "$dmg_stage" -format UDRW "$dmg_work/writable.dmg"
hdiutil attach -quiet -nobrowse -noautoopen -mountpoint "$dmg_mount" "$dmg_work/writable.dmg"
dmg_attached=1
.tooling/dmg/bin/python scripts/dmg-layout.py "$dmg_mount"
codesign --verify --deep --strict "$dmg_mount/Wireframe.app"
hdiutil detach -quiet "$dmg_mount"
dmg_attached=0
hdiutil convert -quiet "$dmg_work/writable.dmg" -format UDZO -o "$dmg_output"
hdiutil verify "$dmg_output"
python3 - "$dmg_output" <<'PY'
import hashlib, pathlib, sys
path = pathlib.Path(sys.argv[1])
digest = hashlib.sha256()
with path.open('rb') as stream:
    for chunk in iter(lambda: stream.read(1024 * 1024), b''):
        digest.update(chunk)
path.with_suffix(path.suffix + '.sha256').write_text(
    f'{digest.hexdigest()}  {path.name}\n')
PY
echo "Prepared $dmg_output"
