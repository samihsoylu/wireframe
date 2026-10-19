# Wireframe application icon

Original Wireframe artwork, licensed GPL-3.0-only with the application. The page frame, image placeholder and selection handles describe wireframing without text or borrowed branding.

`wireframe.svg` is the editable source; `wireframe-small.svg` simplifies it for crisp 16/32 pixel use. The PNGs supply the desktop window icon; `Wireframe.icns` supplies the macOS application icon. `Wireframe.ico` is prepared for Windows executable-resource packaging, which still needs Windows integration and verification.

Regenerate on macOS with `python3 scripts/generate-app-icon.py`. This uses the project's pinned Rust SVG renderer, Apple's `iconutil`, and ImageMagick only to assemble the Windows ICO. Install Rust and ImageMagick first. Generated assets are committed so ordinary app builds do not need these extra tools.
