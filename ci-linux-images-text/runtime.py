import os, shutil, stat
from pathlib import Path
from ci_common import require
GIB = 1024**3
FIXED = {'CARGO_BUILD_TARGET', 'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS',
         'RUSTC', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER'}
def normalized(inherited, work, offline):
    removed = sorted(k for k in inherited if k in FIXED or k.startswith('CARGO_PROFILE_')
                     or k == 'WIREFRAME_INPUT_TRACE')
    require(len(removed) <= 65 and all(len(k) <= 256 for k in removed), 'removed-key bound')
    env = {k: v for k, v in inherited.items() if k not in removed}
    # The accepted six-key normalizer is retained. Refuse additional compiler
    # configuration rather than silently inventing a second profile policy.
    require(not any(k.endswith('_RUSTFLAGS') or k.startswith('CARGO_BUILD_')
                    for k in env if k != 'CARGO_BUILD_JOBS'), 'additional compiler override')
    native = {k for k in env if k in {'CC','CXX','CFLAGS','CXXFLAGS','CPPFLAGS','LDFLAGS','CMAKE'} or k.startswith(('CMAKE_', 'CFLAGS_', 'CXXFLAGS_', 'CC_', 'CXX_'))}
    removed += sorted(native)
    require(len(removed) <= 128 and all(len(k) <= 256 for k in removed), 'native compiler environment proof bound')
    env = {k:v for k,v in env.items() if k not in native}
    env.update(CMAKE_BUILD_PARALLEL_LEVEL='1', CARGO_HOME=str(work / 'cargo'), CARGO_TARGET_DIR=str(work / 'payload/target'),
               CARGO_BUILD_JOBS='1', CARGO_INCREMENTAL='0', CARGO_NET_OFFLINE=str(offline).lower(),
               RUSTUP_TOOLCHAIN='1.99.1', RUSTUP_SKIP_UPDATE_CHECK='1',
               PYTHONDONTWRITEBYTECODE='1', CARGO_TERM_COLOR='never',
               TMPDIR=str(work / 'payload/tmp'),
               XDG_CONFIG_HOME=str(work / 'payload/home/config'),
               XDG_DATA_HOME=str(work / 'payload/home/data'),
               XDG_CACHE_HOME=str(work / 'payload/home/cache'))
    require('WIREFRAME_INPUT_TRACE' not in env and not any(k in env for k in FIXED)
            and not any(k.startswith('CARGO_PROFILE_') for k in env), 'normalization failed')
    return env, {'removedKeys': removed, 'removedCount': len(removed),
                 'traceAbsent': True, 'compilerEnvironmentNormalized': True,
                 'jobs': 1, 'incremental': 0, 'offline': offline}

def allocated(root):
    total, seen = 0, set()
    for directory, dirs, files in os.walk(root, followlinks=False):
        for name in ['.', *dirs, *files]:
            path = Path(directory) / name
            st = path.lstat()
            # This is called only after the supervisor proves kernel ECHILD.
            # Count symlink inode storage, never traverse its target. Source
            # byte/type checks separately prohibit source/package symlinks.
            key = (st.st_dev, st.st_ino)
            if key not in seen:
                seen.add(key)
                total += max(st.st_blocks * 512, ((st.st_size + 4095) // 4096) * 4096)
    return total
