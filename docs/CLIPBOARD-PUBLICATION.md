# Clipboard publication and Cut

Cut of canvas controls, source rows, and complete owning libraries now waits for a correlated native clipboard result. The backend publishes text and verifies its read-back. An unavailable backend or failed write/read-back returns an error; the in-process clipboard fallback cannot authorize deletion. Ordinary text-field clipboard handling keeps its existing framework path.

A pending Cut snapshots the document, editor session, named-project origin, page/version/library/source, canvas context/selection and navigator selection. Completion rechecks that state and active modal/gesture boundaries. A stale completion leaves the copied content in place. Failure preserves document, internal clipboard, paste offset and history. A newer Copy/Cut supersedes an earlier pending Cut, and duplicate/unrelated replies cannot apply it again.

Later keyboard commands wait for the completion event, preserving Cut→Undo and Cut→Save order. A confirmed resource Cut removes complete requested owners in one history step, preserving unrelated dependencies, existing consumers and assets. Same-origin Paste can restore absent identities; it does not create a Trash copy. Single-owner source selection keeps source-only Cut, and library/mixed/cross-owner selections use full-library scope. Global mixed Delete remains separate and unchanged.

The adapter appends completion to native input and requests a redraw. arboard publication checks write errors and exact read-back. Wayland store is asynchronous; verification permits up to eight reads with10ms between unsuccessful attempts for a new selection offer to arrive. This is bounded settling, not a guarantee that another application will leave the clipboard unchanged. Native platform behavior must be accepted separately.

## Page destinations

Control clipboard payloads include optional project and editor-session identity.
The session preserves page links across the first Save or renaming of the same open
document. Matching named-project identity supports another window of that project.
For foreign or unidentified older payloads, Paste clears typed and inline page
links, including component sources, nested local overrides and copied library
placements. Labels, formatting, URL and Back targets remain. Older qualified source
clipboards can still establish their origin from their source records. Existing
native publication-before-Cut behavior is unchanged. Source text that cannot be
rewritten safely rejects Paste through the existing atomic transaction.

## Verification

235 application tests,7 library lifecycle tests,7 library transfer tests,14 source clipboard tests and1 backend fallback test pass (264 total). Tests cover failure, delayed/stale/superseded/duplicate results, changed document/target/session/navigator selection, held gestures, exact history and ordered Cut→Undo/Save. An earlier queue change consumed held-drag pointer events; the existing regression caught it and the corrected path preserves those events.

All-target warnings-denied Clippy, formatting/diff checks and optimized macOS ARM64 build with strict ad-hoc signature pass. Sixteen native/storage/profile assertions verify real system clipboard success for mixed complete libraries, source keyboard Cut and canvas Cut; system Paste restores the exact library graph and identities. Undo returns every fixture to its exact prior state. Existing recovery records remain unchanged and the disposable candidate is closed. This is self-review. Failure evidence is deterministic injected completion/backend-fallback testing, not a forced OS clipboard failure in the native app.

## Linux packaged-app follow-up

The unchanged optimized ARM64 package passes native X11 Unicode text Copy/Paste, GTK-to-product text transfer, shared-image canvas Cut/Paste, exact saved Undo/Redo and baseline restoration on Debian 12/Mesa llvmpipe. Eleven document/asset/input-identity assertions pass. The clipboard publication success path is exercised by real Cut. Remote ASCII transfer also works; remote Unicode corrupts in GTK independently of the product. This is a remote transport limitation, not evidence that native Unicode copying fails. The precise negotiated transport branch remains unproven. External `linux-clipboard-current/report.md` retains captures and exact executable identity. No runtime change or redundant broad test rerun was required.

## Remaining acceptance

Linux source/library Cut, Wayland and Windows runtime clipboard checks, asynchronous Wayland offer timing and queued pointer/focus combinations remain open. The X11 text/canvas acceptance above is scoped; package/build evidence is recorded separately in the release checklist. The earlier4 MB transport mismatch is corrected under the capacity checkpoint below; native capacity on other platforms remains unverified. Ordinary Copy/text-field backend error presentation is unchanged. Singular resource Cut status wording still needs polish. Full toolbar fidelity, retained import matching/dependencies and broader product/platform acceptance remain open. No installation, release replacement or commit occurred.

## Image-heavy clipboard capacity

All current-envelope and legacy readers share the64 MiB serialized-JSON project bound. The version prefix is outside that bound. Publication uses a bounded writer and rejects before emitting a partial clipboard command; invalid or oversized Cut cannot remove content. This is an encoded transport limit, not a promise that64 MiB of raw images fits after JSON encoding. Existing per-asset, graph and destination validation remain enforced.

A valid1024×512 PNG reproduces the former4 MB rejection. Twenty-four affected application tests now pass, including canvas/page/library transfer with exact image bytes and Undo/Redo, failed large Cut publication, true oversized Cut preservation, oversized-reader document/Redo preservation and exact writer boundary/UTF-8 compatibility. A stale old-limit expectation and a new test's incorrect active-library assumption were corrected; both failed runs are retained externally.

Native macOS ARM64 canvas Cut/Paste passes with a1,573,919-byte PNG whose JSON byte array exceeds4 MB. Publication acknowledgement, rendered image, exact saved Undo/Redo and baseline restoration pass. All-target warnings-denied lint, format, debug build and strict signature pass. External `clipboard-capacity-current/report.md` pins the candidate and scope. The review process is closed; no installed/distributed app was replaced. Native large page/library transfer, maximum-boundary OS behavior, Linux/Windows capacity and performance at the limit remain unverified. Windows remains held.
