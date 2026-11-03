# Beta 5 verification record

Status: **ready for the authorized Beta5 prerelease** after parent acceptance of source, exact Mac native and all three platform package/static checkpoints. Actual publication and public-download verification remain pending; no public release is claimed by this readiness record.

Runtime and corresponding source are frozen at `18e068b52b97e8db652546337f43eef579f637e6`, version `0.1.0-beta.5`; Mac bundle short version0.1.0/build5. Later documentation does not change those packaged bytes. The source change fixes Escape ownership and text delivery after keyboard focus changes; the data format and dependencies are unchanged from Beta4.

| Platform or check | Confirmed result | Limits |
| --- | --- | --- |
| Source correction | Accepted exact v3 source:20 review passes,27 shared shortcut/modal/input-order passes, one existing render check ignored; strict all-target lint and format pass. Exact failing-before regressions retained. | Prior622-pass/3-ignore broad suite is v2-scoped, not a fresh v3/Beta5 full-suite claim. |
| Apple silicon Mac behavior | Exact signed Beta5 app passed the finite keyboard review route: popup dismissal, focused first Escape retaining draft/presentation, second Escape returning editor, explicit keyboard Cancel and reachable Add a Comment, five complete saved graphs and fresh-process reopen/empty drawing history. Both actual app waits0; all546 original scoped files/native settings fingerprints restored exactly. Independent retained review65103378 and parent acceptance complete. | Semantic CUA input; fast/coalesced timing remains source-test scope. Earlier main settings observations remain Beta4-scoped. No physical-input, IME, screen-reader or fullP04/R10 acceptance. |
| Mac distribution | Exact ZIP/final DMG passed independent review8d227188:58-entry app/signature,1898 corresponding-source files,410 notices/747 fulltext references, full ZIP/metadata/reader checks; final read-only DMG contents/Applications link/arrow/serialized layout and normal detach0. | Ad-hoc/non-notarized. No downloaded/installed first launch, live Finder drag/install, Intel/minimum OS or production updater acceptance. |
| Windows x86-64 | [Original build-only run41736075988](https://github.com/samihsoylu/wireframe/actions/runs/41736075988) succeeded. Downloaded EXE/ZIP passed independent static review0c00c74e:PE/imports, five icon resources, exact embedded model, source/notices/readers/checksums. | **UNTESTED ON WINDOWS.** Unsigned; no application, model, Rust test or example execution. Explorer/DPI, OS support and runtime behavior unverified. |
| Linux ARM64 | [Original release-only run41736981523](https://github.com/samihsoylu/wireframe/actions/runs/41736981523) succeeded at controllerf0aa0180c627e9dd9f2e78366e7037b4b491823f. Downloaded artifact digests, build/source/controller/attempt receipts and local strict package checks pass:1900 source pins,1898 packaged source files,410 notices, exact ELF/TAR hashes. Independent qualified artifact review7052117f and parent acceptance complete. | One build/static group, zero test harnesses or native UI. Earlier713 full-suite/738 native and S3 results retain their own identities; none is a fresh Beta5 runtime pass. |
| Demo | Four-page demo and notices retain byte-identical accepted Beta4 public copies. | Existing named demo checks are reused. |

The private preservation fingerprints establish paths/types/contents/modes/uid/gid, not a new timestamp/xattr guarantee. Independent keyboard review assessed the unchanged restoration code and actual receipts without reading private backup contents. Mac/Windows package reviews are separate from product behavior acceptance.

Two local companion-checker schema assumptions failed before output creation; an extra Windows model-path probe failed after the core static check passed. Original failures are retained, corrected bindings preserve the actual source/type/mode/byte/hash checks, and no product defect, rebuild, re-sign or native retry was invented from them. The earlier Beta5 assembly-wrapper postcheck1 remains1; separate independently reviewed static evidence corrected its mistaken resource-change expectation.

## Intended download identities

Seven payloads plus combined SHA256SUMS are staged. All current prerelease candidate gates are accepted. Public metadata and actual downloads must still be checked after publication.

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| DEMO-LICENSES.md | 1299 | `2592fce052154ae874e153ad1b8058d6306b4d9f22474fc643d296ad3dfc1055` |
| Wireframe-0.1.0-beta.5-darwin-arm64.dmg | 60000150 | `525e6c20ad6367be3bcac78e5651cc76e1dfc57f34e0bac15ba0a8a20ab3d142` |
| Wireframe-0.1.0-beta.5-darwin-arm64.zip | 46348865 | `68428880263c97fd76363d4bc4949d5789448b04da6e7d198bf6aa8bd368afda` |
| Wireframe-0.1.0-beta.5-linux-aarch64.tar.gz | 41917343 | `d5a0bad40c782284066f86c23e4dfaaa876c24cce921ba2fea57e43890ce0974` |
| Wireframe-0.1.0-beta.5-windows-x86_64.exe | 39619584 | `48d9abf4bfd38d8a14b9735df43f334023947320ae0d71f3a02bf54a4410eb43` |
| Wireframe-0.1.0-beta.5-windows-x86_64.zip | 45238089 | `579ad3f45235f3281da449ca9a1390e39c6cc294fc4d7a9975200c64da9de7f4` |
| demo.wireframe | 283805 | `5a581f3473aaa83eb09c702dd7645d9de4603d9ef24c38436adfc097566abe24` |
| SHA256SUMS | 702 | `be66f6d20943e5bb61821c6d077306e18e062b5a960fc8df81551adddd6d4584` |

Updates remain manual. Apple Developer membership/notarization is declined. Production updater/Keychain/feed, protected installation/first-launch, main/stable merge and action-time deletion/reference/security handoffs retain their boundaries. Windows runtime remains held. Existing-format project import remains the sole explicit feature deferral.

The original V01–V05/Q3–Q8 goal remains open. Accepted appearance and namedQ3/Q4/Q6 scopes are retained; Q5/Q7/Q8 and22 follow-up rows remain unresolved in the current [remaining-checks index](verification/REMAINING-CHECKS.md). A beta does not close compound parent scope or transform requirement classifications into passes. No new beta-readiness exception or feature deferral is introduced.
