# Murmel

**Ein persönlicher, lokal laufender Diktier-Assistent. Kein SaaS, kein Cloud-Zwang.**

Taste drücken, sprechen, Text landet am Cursor — in jeder Anwendung, ohne dass ein
Byte den Rechner verlässt. Murmel ist ein privater Fork von
[Handy](https://github.com/cjpais/Handy) und verfolgt eine eigene Vision:
siehe **[Murmel_Northstar.md](Murmel_Northstar.md)**.

> **Status:** Frisch geforkt. Das Rebranding steht, die Murmel-eigenen Features
> (Text-Nachbearbeitung per LLM, lokale Nutzungsstatistiken, schlankeres UI) sind
> in Arbeit. Es gibt noch keine veröffentlichten Releases — Murmel wird selbst gebaut.

## Warum ein eigener Fork?

- **Privat** — keine Telemetrie, kein Cloud-Sync, keine Netzwerk-Requests außer dem
  einmaligen Modell-Download
- **Schnell auf Windows** — der Alltagsrechner ist Windows; Ubuntu 24.04 ist
  gleichberechtigtes Ziel
- **Unaufdringlich** — da, wenn man ihn braucht, unsichtbar, wenn nicht
- **Wartbar** — solide Rust/Tauri-Basis statt Python-Skript ohne Tests

## Wie es funktioniert

1. **Hotkey drücken** (Toggle oder Push-to-Talk)
2. **Sprechen**
3. **Loslassen** — Murmel transkribiert lokal
4. **Fertig** — der Text wird in die aktive Anwendung eingefügt

Die gesamte Verarbeitung ist lokal:

- Stille wird per VAD (Silero) herausgefiltert
- Transkription wahlweise mit **Whisper** (GPU-beschleunigt) oder
  **Parakeet V3** (CPU-optimiert, automatische Spracherkennung)
- Läuft auf Windows, Linux und macOS

## Quick Start

Es gibt noch keine Binaries — Murmel wird aus dem Quellcode gebaut:

```bash
bun install
mkdir -p src-tauri/resources/models
curl -o src-tauri/resources/models/silero_vad_v4.onnx https://blob.handy.computer/silero_vad_v4.onnx
bun run tauri dev
```

Plattformspezifische Build-Voraussetzungen stehen in [BUILD.md](BUILD.md).

## Architecture

Murmel is built as a Tauri application combining:

- **Frontend**: React + TypeScript with Tailwind CSS for the settings UI
- **Backend**: Rust for system integration, audio processing, and ML inference
- **Core Libraries**:
  - `transcribe-cpp`: Local speech recognition with Whisper-family models (GGML/GGUF)
  - `transcribe-rs`: CPU-optimized speech recognition with Parakeet models
  - `cpal`: Cross-platform audio I/O
  - `vad-rs`: Voice Activity Detection
  - `rdev`: Global keyboard shortcuts and system events
  - `rubato`: Audio resampling

### Debug Mode

Murmel includes an advanced debug mode for development and troubleshooting. Access it by pressing:

- **macOS**: `Cmd+Shift+D`
- **Windows/Linux**: `Ctrl+Shift+D`

### CLI Parameters

Murmel supports command-line flags for controlling a running instance and customizing startup behavior. These work on all platforms (macOS, Windows, Linux).

**Remote control flags** (sent to an already-running instance via the single-instance plugin):

```bash
murmel --toggle-transcription    # Toggle recording on/off
murmel --toggle-post-process     # Toggle recording with post-processing on/off
murmel --cancel                  # Cancel the current operation
```

**Startup flags:**

```bash
murmel --start-hidden            # Start without showing the main window
murmel --no-tray                 # Start without the system tray icon
murmel --debug                   # Enable debug mode with verbose logging
murmel --help                    # Show all available flags
```

Flags can be combined for autostart scenarios:

```bash
murmel --start-hidden --no-tray
```

> **macOS tip:** When Murmel is installed as an app bundle, invoke the binary directly:
>
> ```bash
> /Applications/Murmel.app/Contents/MacOS/Murmel --toggle-transcription
> ```

## Known Issues & Current Limitations

This project is actively being developed and has some [known issues](https://github.com/Suppenterrine/murmel/issues). We believe in transparency about the current state:

### Major Issues (Help Wanted)

**Whisper Model Crashes:**

- Whisper models crash on certain system configurations (Windows and Linux)
- Does not affect all systems - issue is configuration-dependent
  - If you experience crashes and are a developer, please help to fix and provide debug logs!

**Wayland Support (Linux):**

- Limited support for Wayland display server
- Requires [`wtype`](https://github.com/atx/wtype) or [`dotool`](https://sr.ht/~geb/dotool/) for text input to work correctly (see [Linux Notes](#linux-notes) below for installation)

### Linux Notes

**Text Input Tools:**

For reliable text input on Linux, install the appropriate tool for your display server:

| Display Server | Recommended Tool | Install Command                                    |
| -------------- | ---------------- | -------------------------------------------------- |
| X11            | `xdotool`        | `sudo apt install xdotool`                         |
| Wayland        | `wtype`          | `sudo apt install wtype`                           |
| Both           | `dotool`         | `sudo apt install dotool` (requires `input` group) |

- **X11**: Install `xdotool` for both direct typing and clipboard paste shortcuts
- **Ubuntu 26.04**: Has Wayland display server by default. `wtype` does not work, you need to install `ydotool` and configure systemd as described [here](https://github.com/cjpais/Handy/pull/557#issuecomment-3781249267).
- **Wayland**: Install `wtype` (preferred) or `dotool` for text input to work correctly
- **dotool setup**: Requires adding your user to the `input` group: `sudo usermod -aG input $USER` (then log out and back in)

Without these tools, Murmel falls back to enigo which may have limited compatibility, especially on Wayland.

**Other Notes:**

- **Runtime library dependency (`libgtk-layer-shell.so.0`)**:
  - Murmel links `gtk-layer-shell` on Linux. If startup fails with `error while loading shared libraries: libgtk-layer-shell.so.0`, install the runtime package for your distro:

    | Distro        | Package to install    | Example command                        |
    | ------------- | --------------------- | -------------------------------------- |
    | Ubuntu/Debian | `libgtk-layer-shell0` | `sudo apt install libgtk-layer-shell0` |
    | Fedora/RHEL   | `gtk-layer-shell`     | `sudo dnf install gtk-layer-shell`     |
    | Arch Linux    | `gtk-layer-shell`     | `sudo pacman -S gtk-layer-shell`       |

  - For building from source on Ubuntu/Debian, you may also need `libgtk-layer-shell-dev`.

- The recording overlay is disabled by default on Linux (`Overlay Position: None`) because certain compositors treat it as the active window. When the overlay is visible it can steal focus, which prevents Murmel from pasting back into the application that triggered transcription. If you enable the overlay anyway, be aware that clipboard-based pasting might fail or end up in the wrong window.
- If you are having trouble with the app, running with the environment variable `WEBKIT_DISABLE_DMABUF_RENDERER=1` may help
- If Murmel fails to start reliably on Linux, see [Troubleshooting → Linux Startup Crashes or Instability](#linux-startup-crashes-or-instability).
- **Global keyboard shortcuts (Wayland):** On Wayland, system-level shortcuts must be configured through your desktop environment or window manager. Use the [CLI flags](#cli-parameters) as the command for your custom shortcut.

  **GNOME:**
  1. Open **Settings > Keyboard > Keyboard Shortcuts > Custom Shortcuts**
  2. Click the **+** button to add a new shortcut
  3. Set the **Name** to `Toggle Murmel Transcription`
  4. Set the **Command** to `murmel --toggle-transcription`
  5. Click **Set Shortcut** and press your desired key combination (e.g., `Super+O`)

  **KDE Plasma:**
  1. Open **System Settings > Shortcuts > Custom Shortcuts**
  2. Click **Edit > New > Global Shortcut > Command/URL**
  3. Name it `Toggle Murmel Transcription`
  4. In the **Trigger** tab, set your desired key combination
  5. In the **Action** tab, set the command to `murmel --toggle-transcription`

  **Sway / i3:**

  Add to your config file (`~/.config/sway/config` or `~/.config/i3/config`):

  ```ini
  bindsym $mod+o exec murmel --toggle-transcription
  ```

  **Hyprland:**

  Add to your config file (`~/.config/hypr/hyprland.conf`):

  ```ini
  bind = $mainMod, O, exec, murmel --toggle-transcription
  ```

- You can also trigger Murmel externally via Unix signals or the CLI flags, which lets Wayland window managers or other hotkey daemons keep ownership of keybindings:

  | Action                                    | Trigger                                                    |
  | ----------------------------------------- | ---------------------------------------------------------- |
  | Toggle transcription                      | `pkill -USR2 -n murmel` or `murmel --toggle-transcription` |
  | Toggle transcription with post-processing | `murmel --toggle-post-process`                             |

  Example Sway config:

  ```ini
  bindsym $mod+o exec pkill -USR2 -n murmel
  bindsym $mod+p exec murmel --toggle-post-process
  ```

  `pkill` here simply delivers the signal—it does not terminate the process.

  > **Behavior change:** older releases also accepted `SIGUSR1` for toggling transcription with post-processing. WebKitGTK — the webview engine embedded in Murmel on Linux — uses SIGUSR1 internally to coordinate JavaScript garbage collection, so listening for it caused phantom recordings and interrupted dictations every few minutes ([#1660](https://github.com/cjpais/Handy/issues/1660)). Murmel no longer listens for SIGUSR1 on Linux; the post-processing toggle is still available via `murmel --toggle-post-process`. **Remove any `pkill -USR1` bindings**: the signal is now delivered straight to WebKit's internal handler and can crash the app.

**Overlay & Pasting Issues (Linux):**

- The recording overlay window can interfere with pasting transcribed text into target applications on Linux (X11)
- **Solution:** Open **Settings > Advanced** and set **"Overlay Position"** to **"None"** to disable the overlay
- Enable **"Audio Feedback"** (also in Advanced) if you still want audible confirmation of recording state
- Users who upgrade from older versions or import settings from other platforms may need to manually apply this change

### Platform Support

- **macOS (both Intel and Apple Silicon)**
- **x64 Windows**
- **x64 Linux**

### System Requirements/Recommendations

The following are recommendations for running Murmel on your own machine. If you don't meet the system requirements, the performance of the application may be degraded. We are working on improving the performance across all kinds of computers and hardware.

**For Whisper Models:**

- **macOS**: M series Mac, Intel Mac
- **Windows**: Intel, AMD, or NVIDIA GPU
- **Linux**: Intel, AMD, or NVIDIA GPU
  - Ubuntu 22.04, 24.04

**For Parakeet V3 Model:**

- **CPU-only operation** - runs on a wide variety of hardware
- **Minimum**: Intel Skylake (6th gen) or equivalent AMD processors
- **Performance**: ~5x real-time speed on mid-range hardware (tested on i5)
- **Automatic language detection** - no manual language selection required

## Roadmap

Die Roadmap steht im [Northstar-Dokument](Murmel_Northstar.md#8-roadmap). Kurzfassung:

- **Jetzt:** Rebranding abgeschlossen, Windows-Pfad stabilisieren
- **Als Nächstes:** Text-Nachbearbeitung per lokalem LLM (formatieren, aufräumen),
  lokale Nutzungsstatistiken, schlankeres UI
- **Danach:** Ubuntu 24.04 inkl. sauberer Wayland-Anbindung über D-Bus
- **Langfristig:** Android-Tastatur als eigenständiger Client

## Updater-Signaturen (offener Punkt)

> **Achtung:** `plugins.updater.pubkey` in [`src-tauri/tauri.conf.json`](src-tauri/tauri.conf.json)
> ist noch der öffentliche Schlüssel des Upstream-Projekts. Der zugehörige private
> Schlüssel liegt nicht in diesem Fork, es können also keine signierten Updates
> ausgeliefert werden.
>
> Vor dem ersten eigenen Release ein eigenes Schlüsselpaar erzeugen und den
> `pubkey` ersetzen:
>
> ```bash
> bun run tauri signer generate -w ~/.tauri/murmel.key
> ```
>
> Den privaten Schlüssel niemals committen — er gehört als
> `TAURI_SIGNING_PRIVATE_KEY` in die GitHub-Secrets.

## Troubleshooting

### Manual Model Installation (For Proxy Users or Network Restrictions)

If you're behind a proxy, firewall, or in a restricted network environment where Murmel cannot download models automatically, you can manually download and install them. The URLs are publicly accessible from any browser.

#### Step 1: Find Your App Data Directory

1. Open Murmel settings
2. Navigate to the **About** section
3. Copy the "App Data Directory" path shown there, or use the shortcuts:
   - **macOS**: `Cmd+Shift+D` to open debug menu
   - **Windows/Linux**: `Ctrl+Shift+D` to open debug menu

The typical paths are:

- **macOS**: `~/Library/Application Support/com.pais.murmel/`
- **Windows**: `C:\Users\{username}\AppData\Roaming\com.pais.murmel\`
- **Linux**: `~/.config/com.pais.murmel/`

#### Step 2: Create Models Directory

Inside your app data directory, create a `models` folder if it doesn't already exist:

```bash
# macOS/Linux
mkdir -p ~/Library/Application\ Support/com.pais.murmel/models

# Windows (PowerShell)
New-Item -ItemType Directory -Force -Path "$env:APPDATA\com.pais.murmel\models"
```

#### Step 3: Download Model Files

Download the models you want from below

**Whisper Models (single .bin files):**

- Small (487 MB): `https://blob.handy.computer/ggml-small.bin`
- Medium (492 MB): `https://blob.handy.computer/whisper-medium-q4_1.bin`
- Turbo (1600 MB): `https://blob.handy.computer/ggml-large-v3-turbo.bin`
- Large (1100 MB): `https://blob.handy.computer/ggml-large-v3-q5_0.bin`

**Parakeet Unified EN 0.6B (single `.gguf` file, recommended):**

- Q8_0 (731 MB): `https://huggingface.co/handy-computer/parakeet-unified-en-0.6b-gguf/resolve/main/parakeet-unified-en-0.6b-Q8_0.gguf`

**Parakeet Models (compressed archives):**

- V2 (473 MB): `https://blob.handy.computer/parakeet-v2-int8.tar.gz`
- V3 (478 MB): `https://blob.handy.computer/parakeet-v3-int8.tar.gz`

#### Step 4: Install Models

**For Whisper Models (.bin files):**

Simply place the `.bin` file directly into the `models` directory:

```
{app_data_dir}/models/
├── ggml-small.bin
├── whisper-medium-q4_1.bin
├── ggml-large-v3-turbo.bin
└── ggml-large-v3-q5_0.bin
```

**For GGUF Models (.gguf files):**

Place the `.gguf` file directly into the `models` directory, exactly like the Whisper `.bin` files above. Murmel also picks up models already present in the shared Hugging Face cache (`~/.cache/huggingface/hub`), so a copy downloaded by another tool works without being moved.

**For Parakeet Models (.tar.gz archives):**

1. Extract the `.tar.gz` file
2. Place the **extracted directory** into the `models` folder
3. The directory must be named exactly as follows:
   - **Parakeet V2**: `parakeet-tdt-0.6b-v2-int8`
   - **Parakeet V3**: `parakeet-tdt-0.6b-v3-int8`

Final structure should look like:

```
{app_data_dir}/models/
├── parakeet-tdt-0.6b-v2-int8/     (directory with model files inside)
│   ├── (model files)
│   └── (config files)
└── parakeet-tdt-0.6b-v3-int8/     (directory with model files inside)
    ├── (model files)
    └── (config files)
```

**Important Notes:**

- For Parakeet models, the extracted directory name **must** match exactly as shown above
- Do not rename the `.bin` or `.gguf` files—use the exact filenames from the download URLs
- After placing the files, restart Murmel to detect the new models

#### Step 5: Verify Installation

1. Restart Murmel
2. Open Settings → Models
3. Your manually installed models should now appear as "Downloaded"
4. Select the model you want to use and test transcription

### Custom Whisper Models

Murmel can auto-discover custom Whisper GGML models placed in the `models` directory. This is useful for users who want to use fine-tuned or community models not included in the default model list.

**How to use:**

1. Obtain a Whisper model in GGML `.bin` format (e.g., from [Hugging Face](https://huggingface.co/models?search=whisper%20ggml))
2. Place the `.bin` file in your `models` directory (see paths above)
3. Restart Murmel to discover the new model
4. The model will appear in the "Custom Models" section of the Models settings page

**Important:**

- Community models are user-provided and may not receive troubleshooting assistance
- The model must be a valid Whisper GGML format (`.bin` file)
- Model name is derived from the filename (e.g., `my-custom-model.bin` → "My Custom Model")

### Linux Startup Crashes or Instability

If Murmel fails to start reliably on Linux — for example, it crashes shortly after launch, never shows its window, or reports a Wayland protocol error — try the steps below in order.

**1. Install (or reinstall) `gtk-layer-shell`**

Murmel uses `gtk-layer-shell` for its recording overlay and links against it at runtime. A missing or broken installation is the most common cause of startup failures and can manifest as a crash or a hang well before any window is shown. Make sure the runtime package is installed for your distro:

| Distro        | Package to install    | Example command                        |
| ------------- | --------------------- | -------------------------------------- |
| Ubuntu/Debian | `libgtk-layer-shell0` | `sudo apt install libgtk-layer-shell0` |
| Fedora/RHEL   | `gtk-layer-shell`     | `sudo dnf install gtk-layer-shell`     |
| Arch Linux    | `gtk-layer-shell`     | `sudo pacman -S gtk-layer-shell`       |

If it is already installed and you still see startup problems, try reinstalling it (e.g. `sudo pacman -S gtk-layer-shell` again) in case the library files were corrupted by a partial upgrade.

**2. Disable the GTK layer shell overlay (`MURMEL_NO_GTK_LAYER_SHELL`)**

If installing the library does not help, you can skip `gtk-layer-shell` initialization entirely as a workaround. On some compositors (notably KDE Plasma under Wayland) it has been reported to interact poorly with the recording overlay. With this variable set, the overlay falls back to a regular always-on-top window:

```bash
MURMEL_NO_GTK_LAYER_SHELL=1 murmel
```

**3. Disable WebKit DMA-BUF renderer (`WEBKIT_DISABLE_DMABUF_RENDERER`)**

On some GPU/driver combinations the WebKitGTK DMA-BUF renderer can cause the window to fail to render or to crash. Try:

```bash
WEBKIT_DISABLE_DMABUF_RENDERER=1 murmel
```

**Making a workaround permanent**

Once you've found a flag that helps, export it from your shell profile (`~/.bashrc`, `~/.zshenv`, …) or from the desktop autostart entry that launches Murmel. If you launch Murmel from a `.desktop` file, you can prefix the `Exec=` line, e.g.:

```ini
Exec=env MURMEL_NO_GTK_LAYER_SHELL=1 murmel
```

If a workaround helps you, please [open an issue](https://github.com/Suppenterrine/murmel/issues) describing your distro, desktop environment, and session type — that information helps us narrow down the underlying bug.

### Murmel Starts or Stops Recording on Its Own (Linux)

Murmel 0.9.4 and earlier listened for `SIGUSR1` as a remote-control trigger. WebKitGTK — the webview engine embedded in Murmel on Linux — uses that same signal internally to coordinate JavaScript garbage collection, so GC cycles were misread as hotkey presses: recordings started on their own, or real dictations were cut off mid-sentence (typically ~2 minutes in). See [#1660](https://github.com/cjpais/Handy/issues/1660).

Update to a newer release, and replace any `pkill -USR1 -n murmel` keybindings with `murmel --toggle-post-process`.

### Mitmachen

Murmel ist in erster Linie ein persönliches Projekt und folgt dem
[Northstar](Murmel_Northstar.md). Issues und PRs sind willkommen, werden aber
daran gemessen — nicht jedes sinnvolle Feature passt zu Murmel.

## Verwandte Projekte

- **[Handy](https://github.com/cjpais/Handy)** — das Upstream-Projekt, auf dem Murmel aufbaut
- **[handy.computer](https://handy.computer)** — Website des Upstream-Projekts;
  hostet auch die Modelle, die Murmel herunterlädt

## Lizenz

MIT License — siehe [LICENSE](LICENSE).

Murmel ist ein Fork von [Handy](https://github.com/cjpais/Handy),
Copyright © 2025 CJ Pais, ebenfalls MIT-lizenziert. Der ursprüngliche
Copyright-Vermerk bleibt in [LICENSE](LICENSE) erhalten.

**Markenrechte:** Upstream stellt den Code unter MIT, behält sich aber Name, Logo,
Icon und Markenassets von Handy vor. Murmel verwendet daher durchgehend eigenes
Branding — eigener Name, eigene Wortmarke, eigenes Icon-Set — und steht in keiner
Verbindung zu Handy oder CJ Pais und wird von diesen weder unterstützt noch
befürwortet.

## Danksagungen

- **[Handy](https://github.com/cjpais/Handy)** von CJ Pais — die Codebasis, die
  Murmel überhaupt erst möglich macht
- **Whisper** von OpenAI für das Spracherkennungsmodell
- **ggml und transcribe.cpp** für plattformübergreifende STT-Inferenz
- **Silero** für die leichtgewichtige VAD
- **Tauri**-Team für das Rust-basierte App-Framework
