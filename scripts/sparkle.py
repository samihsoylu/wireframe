#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Optional macOS updater payload; public configuration only, no signing secrets."""
import base64
import hashlib
import json
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import tempfile
from urllib.parse import urlsplit

VERSION = '2.10.0'
SHA256 = 'c2bf58aa8387266ac179357b1415d6f2635f044da8be41042af32425dae6da0c'
URL = f'https://github.com/sparkle-project/Sparkle/releases/download/{VERSION}/Sparkle-{VERSION}.tar.xz'
BUNDLE_ID = 'nl.samihsoylu.wireframe'


def configuration(path):
    data = json.loads(Path(path).read_text())
    if not isinstance(data, dict) or set(data) != {'feed_url', 'public_key', 'build'}:
        raise ValueError('Updater configuration requires only feed_url, public_key and build')
    if not all(isinstance(value, str) for value in data.values()):
        raise ValueError('Updater configuration values must be strings')
    url = urlsplit(data['feed_url'])
    _ = url.port  # Reject malformed/out-of-range ports before staging a build.
    if (url.scheme != 'https' or not url.hostname or url.username is not None
            or url.password is not None or '#' in data['feed_url'] or any(c.isspace() for c in data['feed_url'])):
        raise ValueError('Updater feed must use HTTPS without credentials, whitespace or a fragment')
    try:
        key = base64.b64decode(data['public_key'], validate=True)
    except ValueError as error:
        raise ValueError('Updater public key must be base64') from error
    if len(key) != 32 or base64.b64encode(key).decode() != data['public_key']:
        raise ValueError('Updater public key must encode exactly 32 bytes')
    if not re.fullmatch(r'[1-9][0-9]{0,17}', data['build']):
        raise ValueError('Updater build must be a positive, increasing integer')
    return {
        'CFBundleIdentifier': BUNDLE_ID,
        'CFBundleVersion': data['build'],
        'WireframeEnableUpdater': True,
        'SUFeedURL': data['feed_url'],
        'SUPublicEDKey': data['public_key'],
        # Checks are opt-in, installation always requires the user's action.
        'SUEnableAutomaticChecks': False,
        'SUAllowsAutomaticUpdates': False,
        'SUAutomaticallyUpdate': False,
        'SUSendProfileInfo': False,
        'SUVerifyUpdateBeforeExtraction': True,
        'SURequireSignedFeed': True,
        'SUSignedFeedFailureExpirationInterval': 0,
    }


def verified_archive(cache):
    cache.mkdir(parents=True, exist_ok=True)
    archive = cache / f'Sparkle-{VERSION}.tar.xz'
    if not archive.exists():
        with tempfile.TemporaryDirectory(prefix='.download-', dir=cache) as temporary:
            candidate = Path(temporary) / archive.name
            subprocess.run(['curl', '--fail', '--location', '--proto', '=https',
                            '--proto-redir', '=https', '--retry', '2', URL,
                            '--output', str(candidate)], check=True)
            verify_digest(candidate)
            candidate.replace(archive)
    verify_digest(archive)
    return archive


def verify_digest(path):
    if hashlib.sha256(path.read_bytes()).hexdigest() != SHA256:
        raise ValueError('Sparkle archive checksum does not match the pinned distribution')


def embed(bundle, cache):
    archive = verified_archive(cache)
    # Always extract verified bytes afresh; do not trust a mutable extracted cache.
    with tempfile.TemporaryDirectory(prefix='.framework-', dir=cache) as temporary:
        extracted = Path(temporary)
        subprocess.run(['tar', '-xf', str(archive), '-C', str(extracted)], check=True)
        framework = extracted / 'Sparkle.framework'
        info = plistlib.loads((framework / 'Resources/Info.plist').read_bytes())
        if info.get('CFBundleShortVersionString') != VERSION:
            raise ValueError('Sparkle framework version differs from the pinned version')
        destination = bundle / 'Contents/Frameworks/Sparkle.framework'
        destination.parent.mkdir(parents=True, exist_ok=True)
        if destination.exists() or destination.is_symlink():
            raise ValueError('Stage the updater in a fresh bundle')
        shutil.copytree(framework, destination, symlinks=True)
        notices = bundle / 'Contents/Resources'
        notices.mkdir(parents=True, exist_ok=True)
        shutil.copy2(extracted / 'LICENSE', notices / 'Sparkle-LICENSE.txt')
