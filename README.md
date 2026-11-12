# PinPage

PinPage turns any web page into an always-visible desktop widget: online pages can float above other windows or be pinned to the desktop, while local HTML files can be imported and run directly—no browser tab, no app packaging.

PinPage is made for content you want at a glance: timers, dashboards, notes, monitoring pages, and local HTML tools generated with an LLM.

English | [简体中文](README.zh-CN.md)

## Screenshots

![Floating pages over the desktop](docs/screenshots/desktop.jpg)
*Pages pinned above the desktop, always in view.*

![Workspace](docs/screenshots/workspace.png)
*The on-demand workspace: page cards, status, and quick actions.*

![Page settings](docs/screenshots/pin-settings.png)
*Source, window appearance, and size.*

## Highlights

- **Pages above windows, not tabs** — float pages over other apps or pin them to the desktop; set always-on-top and a per-page frame style (system frame, borderless, or rounded borderless).
- **Transparent backgrounds** — allow transparency per page in floating and desktop modes on Windows without reloading its running state. Built-in examples are adapted; other pages must provide transparent backgrounds. See [verification status](docs/development.md#当前状态与验收).
- **Any web page or local HTML** — open HTTP/HTTPS addresses or import a folder or single HTML file, including LLM-generated tools that need no deployment or packaging; relative CSS, images, ES modules, and JSON keep working.
- **Portable page packages** — export an imported page with its localStorage, sessionStorage, IndexedDB, and default window layout as a `.pinpage` package and restore it elsewhere. Built-in examples use the same format.
- **Precise window control** — logical size and physical position, independent position and size locks, modifier + left-drag for borderless windows, plain right-click for a window menu (per-page opt-out to let the page handle it), and DPI/monitor correction when restoring windows. The drag modifier defaults to Ctrl; choose Shift or Off in settings.
- **Immediate window settings** — appearance, size, position, and locks apply and save directly, with undo for the last adjustment. Names and sources are saved separately under Content.
- **One-click window presets** — package appearance, size, and relative position into your own presets and switch them in one step. Save the current window layout as a new preset or overwrite an existing one. Each page owns its preset list, with a per-page manager for editing, renaming, copying, and deleting templates without touching the current window; imported page packages can carry a default layout.
- **Quiet by design** — floating pages skip the taskbar; the workspace is created on demand and destroyed on close; the tray keeps windows recoverable.
- **Optional launch at startup** — enable it in Settings → General; off by default. Start in the tray when signing in to Windows, with page restoration controlled separately.
- **Security boundaries** — separate WebViews for the workspace and page content, a scoped custom protocol for local files, and no shell or window-management permissions for untrusted pages; read-only folder access is granted per page through a native picker.
- **Page native capabilities** — content pages can read host info and a shared once-per-second system sample by default; read-only folder access is bounded to the granted root and never exported with packages. Page scripts stay same-origin while HTTPS outbound is allowed. See [architecture notes](docs/architecture.md).
- **Bilingual UI** — Simplified Chinese and English, or follow the system UI language on Windows; switching does not reload open pages.
- **GitHub Release updates** — check, download, verify, and install signed updates from this repository; installation requires confirmation. End-to-end Windows upgrade acceptance is pending; see [versioning and updates](docs/releases.md).
- **Six tools built in** — focus timer, quick notes, daily checklist, digital clock, folder browser, and system monitor, each added as an independent imported page. The clock offers minimal, flip-style, and analog faces, color themes, and responsive transparent layouts; the folder browser needs a per-page grant, and the monitor reads the shared system sample.
- **Online quote validation example** — a separately importable [stock watchlist](examples/stocks/README.md) offers three responsive layouts, actual intraday minute charts, watchlist management, and transparent panels. It is not registered in the built-in gallery and depends on external website endpoints.
- **Measured footprint** — first-round Release sampling measured about 205 / 291 / 377 MiB of private commit memory for 1 / 3 / 5 idle timers across the whole process group. Details are in the [architecture notes](docs/architecture.md).

## Quick start

1. Download the Windows installer from [GitHub Releases](../../releases/latest) and install it.
2. Click "Add page" and enter a web address, import a local page, or add a sample.
3. Choose Floating window or Pinned to desktop under "Window options", then create the page.
4. Reopen the workspace from the tray. Right-click a page to open its window menu: switch presets at the top, or choose "Page settings…" to open the manager with that page's settings dialog. Hold Ctrl (default) and left-drag to move a borderless window; change or disable the drag modifier in Settings → Controls.

Full instructions live in the [user guide](docs/user-guide.md) (Chinese). To build from source, see [Development and build](docs/development.md).

The current Windows x64 release is [v0.2.0](https://github.com/LLMxPM/pinpage/releases/tag/v0.2.0), the first version with in-app updates. Users on v0.1.0 need to install this version manually once.

Windows is the fully supported platform. macOS currently covers standard floating windows only—desktop mode, rounded corners, transparent backgrounds, window locks, and gestures are unavailable, and the macOS build has not yet been verified on real hardware. See [Platform support](docs/platforms.md).

## Documentation

- [User guide](docs/user-guide.md) (Chinese) — install, pages, window control, tray, data migration, FAQ.
- [Architecture and data](docs/architecture.md) (Chinese) — window lifecycle, security boundaries, local copies, page packages, configuration, performance.
- [Agent CLI and skill](docs/agent-cli.md) (Chinese) — a dedicated "Agents and CLI" panel in the workspace toolbar; implemented in current source: app-bundled CLI, page updates, desktop screenshots, diagnostics and portable skill management; not yet released. See the [implementation plan](docs/agent-cli-plan.md) and [acceptance status](docs/development.md#当前状态与验收).
- [Development and build](docs/development.md) (Chinese) — environment, checks, packaging, verification instances, and current acceptance status.
- [Platform support](docs/platforms.md) (Chinese) — Windows capabilities, macOS floating-window fallback, and acceptance status.
- [Versioning and updates](docs/releases.md) (Chinese) — version scripts, signed builds, GitHub Release publishing, and update behavior.

## License

PinPage is released under the [MIT License](LICENSE).
