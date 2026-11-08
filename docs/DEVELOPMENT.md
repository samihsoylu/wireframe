# Build Wireframe

Install Rust 1.95 or newer and a platform C/C++ toolchain. Then run:

```sh
cargo run --release --locked
```

On macOS, install Xcode Command Line Tools. On Windows, use the MSVC Rust toolchain with Visual Studio C++ Build Tools. On Linux, install a C toolchain, pkg-config and development packages for X11, Wayland and OpenGL. File dialogs use a running desktop portal and matching backend.

```sh
cargo fmt --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

## Fast feedback

```sh
python3 scripts/check.py fast
# With dependencies already cached:
python3 scripts/check.py fast --offline
```

For a focused edit, select an existing regression group:

```sh
python3 scripts/check.py fast --offline --case arrows
python3 scripts/check.py fast --offline --case picker
python3 scripts/check.py fast --offline --case text --case io
```

The choices are `arrows` (seven geometry guards), `picker`, `drag`, `text`
(draft/save/history/reopen), `io` (async failure/ownership), and `link` (No link
cancellation). App choices each select one existing scenario. Repeated choices
are deduplicated. Only the needed root test targets are built and listed: an
arrow-only check does not build the application test binary. Cargo still checks
freshness, and executable/input hashes and exact result accounting remain in force.
The console and JSON identify omitted fast tests. Use the full command without
`--case` before handing off a change; focused success does not replace the full
fast lane. CI continues to run all 12 checks.

This host-headless lane builds only the root library and application test binaries, then runs seven arrow guards and five full-app scenarios (picker input/focus, interrupted drag/history, draft/save/reopen, async failure/ownership, and whole-control No link cancellation). It preserves the existing tests. Build and test phases are sequential, with two build jobs and two test threads by default; use `--jobs 1 --test-threads 1` for serial debugging.

The command lists each selected executable and verifies every required name and result. Missing, duplicate, ignored, failed or incomplete required results fail the command; unselected tests are never reported as passed. There are no retries, automatic timeouts, tool installs or profile changes. Configure the same Rust/C/C++/SDK/CMake environment as an ordinary build; all `WIREFRAME_*` app overrides are removed from child processes. Cross-target `CARGO_BUILD_TARGET` is rejected because the lane executes host test binaries.

A concise console result separates build, discovery and test execution time. The machine summary defaults to `target/check-fast.json`; choose another path with `--json PATH`. Check both the command's exit status and the summary's `status`. A new run replaces an old green report with `running` before building. Fixture/configuration and executable hashes identify the measured selection; timings describe that invocation, with no full-cold or whole-suite speed claim or fixed pass-time threshold.

The runner checks each recorded executable hash immediately before and after its discovery and execution, then checks all selected executables and the recorded fixture/configuration inputs again before reporting success. Observed drift fails the command. These are point-in-time concurrency checks; they cannot establish an atomic snapshot or detect a change that is restored between checks. Avoid concurrent builds and source edits during the lane.

Other tests, integration/vendor suites, native desktop/UI/clipboard/dialog behavior, visual checks, Windows desktop runtime and release builds are outside this command. Passing this lane is not release or native acceptance. The broader commands above remain separate checks. Runner contract tests use synthetic Cargo/libtest output:

```sh
python3 -m unittest discover -s scripts -p 'test_check.py'
```

The [Linux fast-feedback workflow](../.github/workflows/fast-feedback.yml) runs these runner contract tests and the same 12 selected checks on Ubuntu 24.04 with Rust 1.99.0. It triggers for pull requests, pushes to `main` or `codex/**`, and manual runs. The workflow lists the Linux build packages and records the actual compiler/CMake versions; it uses two build jobs and two test threads without starting a desktop session. The Windows build-only workflow remains separate.

CI retains console diagnostics and `summary.json` for 14 days, including failures. The JSON distinguishes expected, selected-discovered, executed and passed counts; missing/incomplete results cannot pass. Early setup failure may leave no runner JSON, and cancellation/timeouts may prevent artifact upload; check the job result and Actions logs as well. Obsolete runs are cancelled through GitHub concurrency on disposable hosted runners, with a 35-minute fast step and 45-minute job limit. Passing CI covers only this selection and its synthetic runner checks, with the same native/full-suite/release omissions above; it makes no cold-build speed claim.

Tests cover document behavior, editing, history, persistence and rendering. Passing tests or cross-compilation does not establish native desktop acceptance. See the [release checklist](RELEASE-CHECKLIST.md) for current limits and [packaging](PACKAGING.md) for distribution commands.

## Project records

Operational session notes are maintained privately outside this repository. Detailed subsystem documents describe implementation contracts; the README and installation guide are written for people using the app. Completed changes belong in Git history.
