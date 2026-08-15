# AGENTS.md

This file provides guidance to AI coding assistants working with code in this repository.

## Development Commands

**Prerequisites:**

- [Rust](https://rustup.rs/) (latest stable)
- [Bun](https://bun.sh/) package manager

**Core Development:**

```bash
# Install dependencies
bun install

# Run in development mode
bun run tauri dev
# If cmake error on macOS:
CMAKE_POLICY_VERSION_MINIMUM=3.5 bun run tauri dev

# Build for production
bun run tauri build

# Frontend only development
bun run dev        # Start Vite dev server
bun run build      # Build frontend (TypeScript + Vite)
bun run preview    # Preview built frontend
```

**Disk space — do not trigger builds casually.** A debug tree costs ~2.5 GB and a
release tree ~4.5 GB, and `tauri build` creates the release tree _in addition to_
the debug one. This has filled the maintainer's system drive before. Rules:

- Never run `bun run tauri build` to "check whether it compiles" — use
  `cargo check` (in `src-tauri/`), which writes far less.
- After a release build, copy the installer out of `bundle/` and run
  `cargo clean --release`.
- The build tree belongs on a roomy volume via `CARGO_TARGET_DIR`; see
  [BUILD.md](BUILD.md#disk-space). Do not hardcode that path into
  `.cargo/config.toml` — that file is also read by the Linux CI.
- **Set it explicitly in every build command you run.** A tool session inherits
  the environment of the process that started it, so a persisted user variable
  is invisible to you and cargo silently falls back to `src-tauri\target` on the
  system drive:

  ```powershell
  $env:CARGO_TARGET_DIR = "D:\rust-target"; cargo check
  ```

**Linting and Formatting (run before committing):**

```bash
bun run lint              # ESLint for frontend
bun run lint:fix          # ESLint with auto-fix
bun run format            # Prettier + cargo fmt
bun run format:check      # Check formatting without changes
bun run format:frontend   # Prettier only
bun run format:backend    # cargo fmt only
```

**Model Setup (Required for Development):**

```bash
mkdir -p src-tauri/resources/models
curl -o src-tauri/resources/models/silero_vad_v4.onnx https://blob.handy.computer/silero_vad_v4.onnx
```

For detailed platform-specific build setup, see [BUILD.md](BUILD.md).

## Architecture Overview

Murmel is a cross-platform desktop speech-to-text application built with Tauri 2.x (Rust backend + React/TypeScript frontend).

### Backend Structure (src-tauri/src/)

- `lib.rs` - Main entry point, Tauri setup, manager initialization
- `managers/` - Core business logic:
  - `audio.rs` - Audio recording and device management
  - `model.rs` - Model downloading and management
  - `transcription.rs` - Speech-to-text processing pipeline
  - `history.rs` - Transcription history storage
- `audio_toolkit/` - Low-level audio processing:
  - `audio/` - Device enumeration, recording, resampling
  - `vad/` - Voice Activity Detection (Silero VAD)
- `commands/` - Tauri command handlers for frontend communication
- `cli.rs` - CLI argument definitions (clap derive)
- `shortcut.rs` - Global keyboard shortcut handling
- `settings.rs` - Application settings management
- `overlay.rs` - Recording overlay window (platform-specific)
- `signal_handle.rs` - `send_transcription_input()` reusable function
- `utils.rs` - Platform detection helpers

### Frontend Structure (src/)

- `App.tsx` - Main component with onboarding flow
- `components/` - React UI components:
  - `settings/` - Settings UI
  - `model-selector/` - Model management interface
  - `onboarding/` - First-run experience
  - `overlay/` - Recording overlay UI
  - `update-checker/` - App update notifications
  - `shared/`, `ui/`, `icons/`, `footer/` - Shared components
- `hooks/useSettings.ts` - Settings state management hook
- `stores/settingsStore.ts` - Zustand store for settings
- `bindings.ts` - Auto-generated Tauri type bindings (via tauri-specta)
- `overlay/` - Recording overlay window entry point
- `lib/types.ts` - Shared TypeScript type definitions

### Key Architecture Patterns

**Manager Pattern:** Core functionality organized into managers (Audio, Model, Transcription) initialized at startup and managed via Tauri state.

**Command-Event Architecture:** Frontend → Backend via Tauri commands; Backend → Frontend via events.

**Pipeline Processing:** Audio → VAD → Whisper/Parakeet → Text output → Clipboard/Paste

**State Flow:** Zustand → Tauri Command → Rust State → Persistence (tauri-plugin-store)

### Technology Stack

**Core Libraries:**

- `transcribe-cpp` - Local Whisper-family inference (GGML/GGUF) with GPU acceleration
- `transcribe-rs` - ONNX speech recognition (Parakeet, Moonshine, SenseVoice, etc.)
- `cpal` - Cross-platform audio I/O
- `vad-rs` - Voice Activity Detection
- `rdev` - Global keyboard shortcuts
- `rubato` - Audio resampling
- `rodio` - Audio playback for feedback sounds

### Application Flow

1. **Initialization:** App starts minimized to tray, loads settings, initializes managers
2. **Model Setup:** First-run downloads preferred Whisper model (Small/Medium/Turbo/Large)
3. **Recording:** Global shortcut triggers audio recording with VAD filtering
4. **Processing:** Audio sent to Whisper model for transcription
5. **Output:** Text pasted to active application via system clipboard

### Settings System

Settings are stored using Tauri's store plugin with reactive updates:

- Keyboard shortcuts (configurable, supports push-to-talk)
- Audio devices (microphone/output selection)
- Model preferences (Small/Medium/Turbo/Large Whisper variants)
- Audio feedback and translation options

### Debug builds keep their own state

`tauri dev` writes its history database and settings store to a `dev/`
subdirectory of the app data dir (`portable::app_state_dir` / `store_path`).
**Expect a dev run to start with an empty history** — that is the separation
working, not a bug. Models and recordings stay shared, so nothing has to be
re-downloaded.

Settings are not lost, though: the first dev run copies the installed app's
`settings_store.json` into `dev/` (`portable::seed_dev_settings`), so shortcuts
and model choice are there to test with. After that the two diverge. API keys
are unaffected either way — they live in the OS credential store, not the
settings file (`secrets.rs`).

The separation exists because database migrations define no `down` step: once a
dev build migrates `history.db` forward, every older binary is locked out of it.
That happened — an installed release stopped starting at all, because the
failure panicked during setup before a tray icon or window existed. The panic is
gone too (`HistoryManager` now carries the reason and reports it in the UI), but
a dev run should not be able to touch installed data in the first place.

### Single Instance Architecture

The app enforces single instance behavior — launching when already running brings the settings window to front rather than creating a new process. Remote control flags (`--toggle-transcription`, etc.) work by launching a second instance that sends args to the running instance via `tauri_plugin_single_instance`, then exits.

## Internationalization (i18n)

All user-facing strings must use i18next translations. ESLint enforces this (no hardcoded strings in JSX).

**Murmel ships German and English only.** The fork arrived with 24 languages;
they were removed in favour of the two the maintainer can actually proofread
(Murmel_Northstar.md §4.2). `check:translations` runs in CI and compares every
locale against the English reference, so **both** files must be updated for
every new string — an English-only key fails the build.

**Adding new text:**

1. Add key to `src/i18n/locales/en/translation.json` (the reference)
2. Add the same key to `src/i18n/locales/de/translation.json`
3. Use in component: `const { t } = useTranslation(); t('key.path')`

**File structure:**

```
src/i18n/
├── index.ts           # i18n setup, discovers locales via glob
├── languages.ts       # Language metadata
└── locales/
    ├── en/translation.json  # English (reference for the consistency check)
    └── de/translation.json  # German
```

`src-tauri/build.rs` generates the tray menu translations from these same files,
so adding or removing a locale directory needs no further wiring.

## Code Style

**Rust:**

- Run `cargo fmt` and `cargo clippy` before committing
- Handle errors explicitly (avoid unwrap in production)
- Use descriptive names, add doc comments for public APIs

**TypeScript/React:**

- Strict TypeScript, avoid `any` types
- Functional components with hooks
- Tailwind CSS for styling
- Path aliases: `@/` → `./src/`

## CLI Parameters

Murmel supports command-line parameters on all platforms for integration with scripts, window managers, and autostart configurations.

**Implementation:** `cli.rs` (definitions), `main.rs` (parsing), `lib.rs` (applying), `signal_handle.rs` (shared logic)

| Flag                     | Description                                                |
| ------------------------ | ---------------------------------------------------------- |
| `--toggle-transcription` | Toggle recording on/off on a running instance              |
| `--toggle-post-process`  | Toggle recording with post-processing on/off               |
| `--cancel`               | Cancel the current operation on a running instance         |
| `--start-hidden`         | Launch without showing the main window (tray icon visible) |
| `--no-tray`              | Launch without system tray (closing window quits the app)  |
| `--debug`                | Enable debug mode with verbose (Trace) logging             |

**Key design decisions:**

- CLI flags are runtime-only overrides — they do NOT modify persisted settings
- Remote control flags work via `tauri_plugin_single_instance`: second instance sends args, then exits
- `send_transcription_input()` in `signal_handle.rs` is shared between signal handlers and CLI

## Debug Mode

Access debug features: `Cmd+Shift+D` (macOS) or `Ctrl+Shift+D` (Windows/Linux)

## Platform Notes

- **macOS**: Metal acceleration, accessibility permissions required for keyboard shortcuts
- **Windows**: Vulkan acceleration, code signing
- **Linux**: OpenBLAS + Vulkan, limited Wayland support, overlay uses GTK layer shell (disable with `MURMEL_NO_GTK_LAYER_SHELL=1`)

## Troubleshooting

See the [Troubleshooting](README.md#troubleshooting) section in README.md.

## Fork-Kontext

Murmel is a personal fork of [Handy](https://github.com/cjpais/Handy) (MIT, © CJ Pais), maintained by a single owner. That shapes how to work here:

- **The Northstar is binding.** [`Murmel_Northstar.md`](Murmel_Northstar.md) defines what Murmel is and — just as importantly — what it deliberately will not become. Check a feature against it before building.
- **No community governance.** There is no feature freeze, no RFC process, and no Discussions requirement. Upstream's contributor ceremony does not apply.
- **Upstream is a source, not an authority.** Cherry-picking fixes from `upstream/master` is welcome; matching upstream's roadmap is not a goal.
- **`gh` targets the upstream repo by default.** GitHub knows this repo as a
  fork, so `gh run`, `gh workflow run` and friends resolve to `cjpais/Handy`
  unless told otherwise — a dispatch meant for Murmel would fire in someone
  else's project. Fix it once per clone:

  ```bash
  gh repo set-default Suppenterrine/murmel
  ```

  That writes `remote.origin.gh-resolved` into `.git/config`, which is **not**
  versioned — every fresh clone needs it again. Until then, pass
  `--repo Suppenterrine/murmel` explicitly.

**Never rename these** — they are external and unrelated to the Handy→Murmel rebrand:

- `handy-keys` / `handy_keys` / `HandyKeys` — the crates.io keyboard-hotkey package and its bindings
- `to_handy_string()` — a method on that crate's `Hotkey` type. Renaming it breaks the
  build; the macOS caller in `secure_input.rs` does not even compile on Windows, so
  a Windows-only check will not catch it.
- `blob.handy.computer` — the CDN the model catalog downloads from
- `handy-computer/*` — the Hugging Face organisation hosting the GGUF models
- `github.com/cjpais/{vad-rs,rodio,tao,hf-hub}` — real upstream git dependencies

**Commits:** Use conventional commit prefixes (`feat:`, `fix:`, `docs:`, `refactor:`, `chore:`). Focus the message on _why_, not _what_.
