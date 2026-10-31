#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Check cross-host notice closure without resolving/building packages."""
import unittest
import json
from pathlib import Path
import tempfile
from unittest.mock import patch
import generate_notices
from generate_notices import include_root_build_dependencies


class BuildNoticeTests(unittest.TestCase):
    def test_windows_host_helpers_are_retained_on_a_non_windows_collector(self):
        def dep(name, kind=None, target=None):
            return {'pkg': name, 'dep_kinds': [{'kind': kind, 'target': target}]}
        nodes = [
            {'id':'app','deps':[dep('embed','build'),dep('normal'),dep('test','dev')]},
            {'id':'embed','deps':[dep('sdk',target='cfg(windows)'),dep('cc','build')]},
            {'id':'sdk','deps':[dep('cc','build'),dep('sdk-test','dev')]},
            {'id':'cc','deps':[]}, {'id':'normal','deps':[]},
            {'id':'test','deps':[]}, {'id':'sdk-test','deps':[]},
        ]
        meta = {'resolve': {'root':'app','nodes':nodes},
                'packages':[{'id':n['id'],'name':n['id'],'version':'1'} for n in nodes]}
        used = {('prior','2')}
        include_root_build_dependencies(meta,used)
        self.assertEqual(used,{('prior','2'),('embed','1'),('sdk','1'),('cc','1')})

    def test_no_build_dependency_preserves_existing_inventory(self):
        meta = {'resolve':{'root':'app','nodes':[{'id':'app','deps':[]}]},
                'packages':[{'id':'app','name':'app','version':'1'}]}
        used={('existing','1')};include_root_build_dependencies(meta,used)
        self.assertEqual(used,{('existing','1')})

    def test_declaration_only_conditional_build_tool_retains_authors_and_reference_terms(self):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);source=root/'crate';source.mkdir();(source/'src').mkdir()
            (source/'Cargo.toml.orig').write_text('license="MIT OR Apache-2.0 OR LGPL-2.1-or-later"\n')
            (source/'AUTHORS').write_text('Original authors\n');(source/'src/lib.rs').write_text('// original source\n')
            (source/'.cargo_vcs_info.json').write_text(json.dumps({'git':{'sha1':'a'*40}}))
            meta={'resolve':{'root':'app','nodes':[{'id':'app','deps':[]}]},'packages':[
                {'id':'r-efi','name':'r-efi','version':'6.0.0','source':'registry',
                 'manifest_path':str(source/'Cargo.toml'),'license':'MIT OR Apache-2.0 OR LGPL-2.1-or-later',
                 'repository':'https://github.com/r-efi/r-efi'}]}
            def cargo(*args):return json.dumps(meta) if args[0]=='metadata' else 'r-efi v6.0.0\n'
            with patch.object(generate_notices,'ROOT',root),patch.object(generate_notices,'DEST',root/'licenses'), \
                 patch.object(generate_notices,'cargo',side_effect=cargo), \
                 patch.object(generate_notices,'fetch',return_value=b'reference terms\n') as fetch:
                generate_notices.main()
            folder=root/'licenses/r-efi-6.0.0'
            self.assertEqual((folder/'AUTHORS').read_text(),'Original authors\n')
            self.assertEqual((folder/'UPSTREAM-DECLARATION.toml').read_bytes(),(source/'Cargo.toml.orig').read_bytes())
            manifest=json.loads((root/'licenses/manifest.json').read_text())
            self.assertEqual(len(manifest[0]['files']),5)
            self.assertEqual(fetch.call_count,3)
            self.assertEqual({Path(call.args[0]).name for call in fetch.call_args_list},
                             {'MIT.txt','Apache-2.0.txt','LGPL-2.1-or-later.txt'})


if __name__ == '__main__':unittest.main()
