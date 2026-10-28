# Wireframe

Turn rough ideas into clear wireframes. Sketch a landing page, plan a mobile screen, or connect a few pages into a clickable walkthrough. Your projects stay on your computer, with no account required.

[Releases](https://github.com/samihsoylu/wireframe/releases) · [Getting started](docs/GETTING-STARTED.md) · [Report an issue](https://github.com/samihsoylu/wireframe/issues)

**An early beta is available.** [Download from official Releases](https://github.com/samihsoylu/wireframe/releases) for **Mac Apple Silicon (ARM64)** or **Windows x86-64**. The Windows build is unsigned and **UNTESTED on Windows**; runtime testing remains on hold. Further feature and platform checks remain open. Linux remains an intended platform, with no public beta package yet.

## Make room for your ideas

- Build screens from 86 controls: buttons, forms, navigation, tables, charts, device frames and more.
- Choose a rough sketch or a clean style. Add your own images, icons and project fonts.
- Double-click controls to edit their text. Move, resize, align and group them, with Undo and Redo as you work.
- Organize pages, reuse components, and add notes and review comments.
- Link pages and present a clickable walkthrough, with a large pointer and optional link hints.
- Share PNGs, SVGs or a multi-page PDF with clickable links.
- Save editable `.wireframe` files locally. Recovery copies help you resume after an interrupted session.

## Install

Choose the file matching your computer from [Releases](https://github.com/samihsoylu/wireframe/releases):

| Computer | Download | Install |
|---|---|---|
| Mac Apple Silicon / ARM64 | `.dmg` | Open it, drag **Wireframe → Applications**, eject the image and open Wireframe from Applications. |
| Windows x86-64 — UNTESTED | Portable `.exe` | Save it in a folder and run it. A companion ZIP includes source and licenses. |
| Linux | No public beta package yet | See the build instructions below. |

The Mac beta is ad-hoc signed, without Apple Developer ID signing or notarization; macOS may block its first launch. See [Apple’s first-launch guidance](https://support.apple.com/en-us/102445). Check each release’s notes for available builds, known limits and installation details. No Apple Developer account is needed to use the app.

In the beta, choose **File → Open example project** to explore four editable pages. Use **File → Save as…** to keep your own copy. [Demo walkthrough](https://github.com/samihsoylu/wireframe/blob/v0.1.0-beta.1/docs/DEMO.md).

## Updates

Updates are manual downloads from [Releases](https://github.com/samihsoylu/wireframe/releases). Built-in macOS updates using Sparkle are planned; Windows automatic updates can follow later. Wireframe does not currently check for or install updates automatically.

## Build and contribute

Built with Rust. See [development instructions](docs/DEVELOPMENT.md), [packaging](docs/PACKAGING.md), and the [release checklist](docs/RELEASE-CHECKLIST.md). Please include your operating system, app version and steps to reproduce when reporting a problem.

## License

[GNU General Public License v3.0 only](LICENSE) (`GPL-3.0-only`), from the first commit. Copyright 2026 Wireframe contributors. [Third-party notices](THIRD_PARTY_NOTICES.md).
