#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Render original artwork with Rust; assemble ICNS/ICO using iconutil/ImageMagick."""
from pathlib import Path
import os
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
ART = ROOT / 'assets/app-icon'


def main():
    with tempfile.TemporaryDirectory(prefix='wireframe-icons-') as directory:
        rendered = Path(directory) / 'png'
        env = os.environ.copy()
        env['SDKROOT'] = subprocess.check_output(['xcrun', '--sdk', 'macosx', '--show-sdk-path'], text=True).strip()
        subprocess.run(['cargo', 'run', '--example', 'render_app_icon', '--', str(rendered)], cwd=ROOT, check=True, env=env)
        for size in [256, 1024]:
            shutil.copyfile(rendered / f'{size}.png', ART / f'wireframe-{size}.png')
        iconset = Path(directory) / 'Wireframe.iconset'
        iconset.mkdir()
        for points in [16, 32, 128, 256, 512]:
            for scale in [1, 2]:
                suffix = '@2x' if scale == 2 else ''
                shutil.copyfile(rendered / f'{points * scale}.png', iconset / f'icon_{points}x{points}{suffix}.png')
        subprocess.run(['iconutil', '-c', 'icns', str(iconset), '-o', str(ART / 'Wireframe.icns')], check=True)
        subprocess.run(['magick', *(str(rendered / f'{size}.png') for size in [16, 32, 64, 128, 256]),
                        str(ART / 'Wireframe.ico')], check=True)


if __name__ == '__main__':
    main()
