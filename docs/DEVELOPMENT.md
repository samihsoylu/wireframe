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

Tests cover document behavior, editing, history, persistence and rendering. Passing tests or cross-compilation does not establish native desktop acceptance. See the [release checklist](RELEASE-CHECKLIST.md) for current limits and [packaging](PACKAGING.md) for distribution commands.

## Project records

The [current checkpoint](../SESSION-SPEC.md) records outstanding work and decisions. Detailed subsystem documents describe implementation contracts; the README and installation guide are written for people using the app. Completed changes belong in Git history.
