"""Bounded source/generated-input attestations; never invoke native executables."""
import hashlib
import json
from pathlib import Path
import re
import tarfile
from ci_common import require, sha

PINNED = {
    'tesseract': ('5.5.4', 'a8dbf8b8c8ee427ff6ad1d99116502feaf59440af1adb893caf60a6576826cf4'),
    'leptonica': ('1.87.0', 'fa2b40c5caea96d1bb93a97486262aed8731b69ce25a84a6bf5d25323e33f631'),
    'tessdata_fast-eng': ('87416418657359cb625c412a48b6e1d6d41c29bd',
                         '7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2'),
}


def source_attestation(source):
    root = source / 'vendor/wireframe-ocr'
    provenance = json.loads((root / 'PROVENANCE.json').read_text())
    require({r['name'] for r in provenance} == set(PINNED), 'OCR component inventory differs')
    for row in provenance:
        path = root / row['path']
        require((row['version'], row['sha256']) == PINNED[row['name']]
                and not path.is_symlink() and path.is_file()
                and path.stat().st_size == row['bytes'] and sha(path) == row['sha256'],
                'OCR archive/model provenance differs: ' + row['name'])
    build = (root / 'build.rs').read_text()
    for marker in ['disable_tiff_runtime_probe(&tess)', '.build_target("libtesseract")',
                   '.define("DISABLE_CURL", "ON")', '.define("DISABLE_ARCHIVE", "ON")',
                   '.define("BUILD_TESTS", "OFF")', '.define("BUILD_TRAINING_TOOLS", "OFF")',
                   '.define("OPENMP_BUILD", "OFF")', '.define("ENABLE_NATIVE", "OFF")']:
        require(marker in build, 'reviewed native build control missing: ' + marker)
    patch = (root / 'native/no-tiff-runtime-probe.cmake').read_text().strip()
    require(patch and 'try_run' not in patch.lower(), 'unexpected executable probe replacement')
    library = (root / 'src/lib.rs').read_text()
    require('include_bytes!' in library and 'eng.traineddata' in library,
            'embedded-model source binding missing')
    native_cmake = []
    for component in ['tesseract-5.5.4', 'leptonica-1.87.0']:
        with tarfile.open(root / 'native' / (component + '.tar.gz')) as archive:
            for member in archive:
                if member.isfile() and (member.name.endswith('.cmake') or member.name.endswith('CMakeLists.txt')):
                    require(member.size < 1024**2, 'CMake source member bound')
                    text = archive.extractfile(member).read().decode()
                    if re.search(r'\btry_run\s*\(', text, re.I) or 'check_leptonica_tiff_support()' in text:
                        native_cmake.append({'path': member.name, 'sha256': hashlib.sha256(text.encode()).hexdigest(),
                                             'tryRunCount': len(re.findall(r'\btry_run\s*\(', text, re.I)),
                                             'tiffCallCount': text.count('check_leptonica_tiff_support()')})
    require(sum(r['tryRunCount'] for r in native_cmake) == 1
            and sum(r['tiffCallCount'] for r in native_cmake) == 1, 'upstream runtime-probe inventory changed')
    for name in ['native/tesseract-LICENSE', 'native/leptonica-LICENSE.txt', 'models/LICENSE']:
        require((root / name).stat().st_size > 0, 'OCR terms missing')
    require((source / 'assets/ocr/NOTICE.txt').stat().st_size > 0, 'packaged OCR notices missing')
    return {'provenance': provenance, 'buildScriptSHA256': sha(root / 'build.rs'),
            'tiffProbeReplacementSHA256': sha(root / 'native/no-tiff-runtime-probe.cmake'),
            'upstreamProbeInventory': native_cmake,
            'scope': 'Static source/archive/model binding; actual builds/inference/generated-input checks remain separate'}


def generated_attestation(target, source):
    paths = sorted(target.glob('*/build/wireframe-ocr-*/out/source/tesseract-5.5.4/CMakeLists.txt'))
    require(paths, 'no actual generated OCR build inputs')
    rows = []
    for path in paths:
        text = path.read_text()
        replacement = (source / 'vendor/wireframe-ocr/native/no-tiff-runtime-probe.cmake').read_text().strip()
        require('check_leptonica_tiff_support()' not in text and replacement in text,
                'reviewed TIFF runtime-probe replacement missing')
        rows.append({'generatedCMakeInput': str(path), 'sha256': sha(path), 'tiffCallAbsent': True})
    return rows


def abi_versions(text):
    limits = {'GLIBC': (2, 36), 'GLIBCXX': (3, 4, 30), 'CXXABI': (1, 3, 13)}
    result = {}
    require('GLIBC_PRIVATE' not in text, 'private glibc ABI')
    for name, limit in limits.items():
        found = [tuple(map(int, s.split('.'))) for s in re.findall(r'\b' + name + r'_([0-9]+(?:\.[0-9]+)+)', text)]
        require(found or name != 'GLIBC', 'missing glibc version evidence')
        if found:
            require(max(found) <= limit, 'newer than Bookworm runtime ABI: ' + name)
            result[name] = '.'.join(map(str, max(found)))
    return result
