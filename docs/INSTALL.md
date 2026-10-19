# Install Wireframe

Version 0.1.0 is being prepared. These instructions describe the planned release files; no public installer is available yet.

## Mac

1. Download the `.dmg` for your Mac from [official Releases](https://github.com/samihsoylu/wireframe/releases).
2. Open the disk image and drag **Wireframe** onto **Applications**, following the arrow.
3. Eject the disk image and open Wireframe from Applications.

Wireframe will be distributed directly, without an Apple Developer ID signature or notarization. macOS may prevent the first launch. If you trust the official download, try opening the app, then open **System Settings → Privacy & Security → Open Anyway** and confirm. Availability depends on macOS and device-management settings. See [Apple’s first-launch guidance](https://support.apple.com/en-us/102445).

Do not run the app from the disk image. Keep your projects wherever you normally store documents; replacing the app should not require replacing your project files.

## Windows

Download the `.exe` matching the architecture listed in the release. Save it in a permanent folder and run it. The initial download is a portable application, not a setup wizard. The companion ZIP contains the same executable, source and licenses. Windows native testing is on hold; a cross-compiled file is not a verified Windows release.

## Linux

Download the `.tar.gz` for your processor and extract it. Open the application folder inside it and run `wireframe`. Some file managers require allowing executable files first.

The currently exercised environment is Debian 12 on ARM64 with X11. Runtime requirements include an OpenGL driver, XKB support and a running desktop portal with its desktop-specific backend for file dialogs. On Debian/Ubuntu, X11 also needs `libxkbcommon-x11-0`. Other distributions, Wayland and x86-64 need their own validation before being listed as supported.

## Updating

Until built-in updating is available, download a newer release, quit Wireframe and replace the application. Keep your `.wireframe` documents. Each release will provide a bullet-point list of changes and any migration instructions.

## Source and licenses

The Mac disk image includes a **Source and licenses** folder. Windows and Linux packages include corresponding source, the GPL-3.0-only license and third-party notices. These files are for reference and do not need to be installed beside the Mac application.
