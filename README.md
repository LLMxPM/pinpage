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

- **Pages above windows, not tabs** — float pages over other apps or pin them to the desktop; set always-on-top and borderless rounded corners per page.
- **Any web page or local HTML** — open HTTP/HTTPS addresses or import a folder or single HTML file, including LLM-generated tools that need no deployment or packaging; relative CSS, images, ES modules, and JSON keep working.
- **Portable page packages** — export an imported page with its localStorage, sessionStorage, and IndexedDB as a `.pinpage` package and restore it elsewhere.
- **Precise window control** — logical size and physical position, independent position and size locks, modifier + left-drag for borderless windows, modifier + right-click for a window menu, and DPI/monitor correction when restoring windows. The shared modifier defaults to left Alt; choose Ctrl, Shift, or Off in settings.
- **Immediate window settings** — appearance, size, position, and locks apply and save directly, with undo for the last adjustment. Names and sources are saved separately under Content.
- **One-click window presets** — four built-ins (full-screen overlay without always-on-top, standard window, 320 × 480 top-right overlay, and 320 × 480 bottom-right desktop widget) switch appearance, size, and relative position together. Save adjustments as a new preset or update a custom one; a shared manager supports editing, renaming, copying, and deleting templates without changing existing pages.
- **Quiet by design** — floating pages skip the taskbar; the workspace is created on demand and destroyed on close; the tray keeps windows recoverable.
- **Security boundaries** — separate WebViews for the workspace and page content, a scoped custom protocol for local files, and no filesystem, shell, or window-management permissions for untrusted pages.
- **Bilingual UI** — Simplified Chinese and English, or follow the Windows UI language; switching does not reload open pages.
- **GitHub Release updates** — check, download, verify, and install signed updates from this repository; installation requires confirmation. End-to-end Windows upgrade acceptance is pending; see [versioning and updates](docs/releases.md).
- **Four offline tools built in** — focus timer, quick notes, daily checklist, and digital clock, each added as an independent imported page.
- **Measured footprint** — first-round Release sampling measured about 205 / 291 / 377 MiB of private commit memory for 1 / 3 / 5 idle timers across the whole process group. Details are in the [architecture notes](docs/architecture.md).

## Quick start

1. Download the Windows installer from [GitHub Releases](../../releases/latest) and install it.
2. Click "Add page" and enter a web address, import a local page, or add a sample.
3. Choose Floating window or Pinned to desktop under "Window options", then create the page.
4. Reopen the workspace from the tray. By default, hold left Alt and left-drag a borderless window to move it, or Alt+right-click a page to open its window menu; release the right button and then Alt so the menu appears. Change or disable this modifier in Settings → Controls.

Full instructions live in the [user guide](docs/user-guide.md) (Chinese). To build from source, see [Development and build](docs/development.md).

## Documentation

- [User guide](docs/user-guide.md) (Chinese) — install, pages, window control, tray, data migration, FAQ.
- [Architecture and data](docs/architecture.md) (Chinese) — window lifecycle, security boundaries, local copies, page packages, configuration, performance.
- [Development and build](docs/development.md) (Chinese) — environment, checks, packaging, verification instances.
- [Versioning and updates](docs/releases.md) (Chinese) — version scripts, signed builds, GitHub Release publishing, and update behavior.
- [Verification log](docs/verification.md) (Chinese) — implemented and verified items, performance data, pending acceptance scenarios.

## License

PinPage is released under the [MIT License](LICENSE).
