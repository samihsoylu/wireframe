# SPDX-License-Identifier: GPL-3.0-only
# Adapted from samihsoylu/side-tray installer tooling.
"""Write a portable Finder layout; no local paths or user preferences are copied."""
import pathlib, sys
from ds_store import DSStore
from mac_alias import Alias
root = pathlib.Path(sys.argv[1])
background = Alias.for_file(str(root / '.background/installer.tiff'))
background.volume.posix_path = '/Volumes/Wireframe Installer'
with DSStore.open(str(root / '.DS_Store'), 'w+') as store:
    store['.']['bwsp'] = {'ShowToolbar': False, 'ShowStatusBar': False,
        'ShowSidebar': False, 'ContainerShowSidebar': False, 'ShowTabView': False, 'ShowPathbar': False,
        'WindowBounds': '{{200, 160}, {540, 420}}'}
    store['.']['vSrn'] = ('long', 1)
    store['.']['icvl'] = ('type', b'icnv')
    store['.']['icvp'] = {'viewOptionsVersion': 1, 'backgroundType': 2,
        'backgroundImageAlias': background.to_bytes(),
        'backgroundColorRed': 1.0, 'backgroundColorGreen': 1.0, 'backgroundColorBlue': 1.0,
        'iconSize': 96.0, 'textSize': 14.0, 'labelOnBottom': True,
        'arrangeBy': 'none', 'gridSpacing': 100.0, 'gridOffsetX': 0.0,
        'gridOffsetY': 0.0, 'showIconPreview': True, 'showItemInfo': False}
    store['Wireframe.app']['Iloc'] = (140, 110)
    store['Applications']['Iloc'] = (400, 110)
    store['Install and Open.txt']['Iloc'] = (140, 260)
    store['Source and licenses']['Iloc'] = (400, 260)
