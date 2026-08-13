# Build Instructions

This guide covers how to set up the development environment and build Murmel from source across different platforms.

## Prerequisites

### All Platforms

- [Rust](https://rustup.rs/) (latest stable)
- [Bun](https://bun.sh/) package manager
- [Tauri Prerequisites](https://tauri.app/start/prerequisites/)

### Platform-Specific Requirements

#### macOS

- Xcode Command Line Tools
- Install with: `xcode-select --install`

##### Intel Mac (x86_64)

Prebuilt ONNX Runtime binaries are not available for Intel Macs. Install ONNX Runtime via Homebrew and link dynamically:

```bash
brew install onnxruntime
ORT_LIB_LOCATION=$(brew --prefix onnxruntime)/lib ORT_PREFER_DYNAMIC_LINK=1 bun run tauri dev
```

The same environment variables apply for production builds:

```bash
ORT_LIB_LOCATION=$(brew --prefix onnxruntime)/lib ORT_PREFER_DYNAMIC_LINK=1 bun run tauri build
```

#### Windows

- Microsoft C++ Build Tools: Visual Studio 2019/2022 with C++ development
  tools, or Visual Studio Build Tools 2019/2022
- [CMake](https://cmake.org/download/) (must be on `PATH`):

  ```powershell
  winget install Kitware.CMake
  ```

- [Vulkan SDK](https://vulkan.lunarg.com/sdk/home) from LunarG — required to
  build the Vulkan GPU backend (`vulkan-shaders-gen` needs the SDK's headers
  and `glslc`):

  ```powershell
  winget install KhronosGroup.VulkanSDK --accept-package-agreements --accept-source-agreements
  ```

  Open a new terminal afterward so `VULKAN_SDK` is set.

  Murmel additionally pins the SDK path in [`.cargo/config.toml`](.cargo/config.toml)
  under `[env]`, so the build does not depend on which variables a given shell
  happens to have. Cargo does not override an already-set environment variable,
  so the global one (which the installer sets) still wins — **update the path in
  that file when you upgrade the SDK**, or the pinned value goes stale.

> [!NOTE]
> Windows' 260-character path limit used to break the native Vulkan build in
> most checkouts. Since `transcribe-cpp` 0.1.3 the build works around it
> automatically (it compiles through a short NTFS junction — no admin rights
> or setup needed), so a normal checkout just builds. If you still hit
> path-limit errors, see
> [Windows build fails with path-limit errors](#windows-build-fails-with-path-limit-errors-msb3491--ftk1011--msb6003)
> in Troubleshooting.

#### Linux

- Build essentials
- ALSA development libraries
- Install with:

  ```bash
  # Ubuntu/Debian
  sudo apt update
  sudo apt install build-essential clang libclang-dev libevdev-dev libasound2-dev pkg-config libssl-dev libvulkan-dev vulkan-tools glslc spirv-headers glslang-tools libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libgtk-layer-shell0 libgtk-layer-shell-dev patchelf cmake

  # Fedora/RHEL
  sudo dnf groupinstall "Development Tools"
  sudo dnf install alsa-lib-devel pkgconf openssl-devel vulkan-devel glslc \
    clang clang-devel libevdev-devel \
    spirv-headers-devel spirv-tools-devel glslang \
    gtk3-devel webkit2gtk4.1-devel libappindicator-gtk3-devel librsvg2-devel \
    gtk-layer-shell gtk-layer-shell-devel \
    cmake

  # Arch Linux
  sudo pacman -S base-devel clang libevdev shaderc spirv-headers glslang alsa-lib pkgconf openssl vulkan-devel \
    gtk3 webkit2gtk-4.1 libappindicator-gtk3 librsvg gtk-layer-shell \
    cmake
  ```

## Setup Instructions

### 1. Clone the Repository

```bash
git clone git@github.com:Suppenterrine/murmel.git
cd Murmel
```

### 2. Install Dependencies

```bash
bun install
```

### 3. Start Dev Server

```bash
bun tauri dev
```

### 4. Build for Production

```bash
bun run tauri build
```

This compiles a release binary and generates platform-specific bundles (deb, rpm, AppImage on Linux; dmg on macOS; msi on Windows).

### Disk space

Building Murmel is not cheap on disk. A full debug tree costs ~2.5 GB, a full
release tree ~4.5 GB — and `tauri build` creates the second one _next to_ the
first, so a machine that alternates between `tauri dev` and `tauri build` carries
~7 GB. The bulk is not Rust but the native `transcribe-cpp`/ggml build, which
compiles many CPU variants plus the Vulkan backend.

Two consequences worth planning for:

- **Keep the build tree off a cramped system drive.** Point `CARGO_TARGET_DIR` at
  a roomy volume (see the path-limit section below — a short path helps on
  Windows for a second reason):

  ```powershell
  [Environment]::SetEnvironmentVariable('CARGO_TARGET_DIR', 'D:\rust-target', 'User')
  ```

- **`cargo clean` does not distinguish profiles.** To reclaim only the release
  tree after pulling the installer out of `bundle/`, use
  `cargo clean --release`. Copy the installer somewhere else first — it lives
  inside the tree you are about to delete.

## Releasing

Installed copies of Murmel update themselves. The app checks on startup (the
setting `update_checks_enabled` defaults to on) and offers a manual check in the
footer; it downloads the new version, verifies its signature and installs it.
Nobody needs to fetch an installer by hand, and re-running an old installer only
reinstalls the old version.

Cutting a release means running the **Release** workflow
(`.github/workflows/release.yml`, `workflow_dispatch`). Two steps around it are
easy to miss, and both fail _silently_ — the app simply never sees an update:

1. **Bump the version first.** The workflow reads it out of
   `src-tauri/tauri.conf.json` and derives the git tag `v<version>` from it. If
   it still says the shipped version, the release collides with the existing
   tag. There is no script for this; keep the three files in step by hand:

   ```
   src-tauri/tauri.conf.json    ← the one the workflow reads
   src-tauri/Cargo.toml         ← also update Cargo.lock (cargo check)
   package.json
   ```

2. **Publish the draft.** The workflow deliberately creates a **draft** release
   so the artifacts can be checked before anyone gets them. The updater asks
   `releases/latest/download/latest.json`, and GitHub does not count drafts as
   "latest". Until the draft is published, every installation keeps reporting
   "up to date".

### The signing key is what makes updates work at all

`plugins.updater.pubkey` in `tauri.conf.json` is baked into every build. An
installed copy only accepts updates signed by the matching private key (the
GitHub secret `TAURI_SIGNING_PRIVATE_KEY`).

The consequence is easy to overlook: **changing the key orphans every copy built
before the change.** Those installations reject the new signature and stay where
they are, with no useful error. Murmel switched from the upstream key to its own
in `0d0c6e7`, _after_ the 0.10.0 release commit — so a 0.10.0 built from an
earlier commit cannot update itself and has to be replaced with a fresh
installer once.

To check which key an installed build carries:

```powershell
$pub = (Get-Content src-tauri\tauri.conf.json -Raw | ConvertFrom-Json).plugins.updater.pubkey
$exe = "$env:LOCALAPPDATA\Murmel\murmel.exe"
$text = [System.Text.Encoding]::ASCII.GetString([System.IO.File]::ReadAllBytes($exe))
$text.Contains($pub.Substring(0, 80))   # True = this build accepts current releases
```

## Linux Install (from source)

The raw binary (`src-tauri/target/release/murmel`) cannot run standalone — it needs Tauri resource files (tray icons, sounds, VAD model) to be co-located at the expected path.

**Install from the deb bundle** (works on any Linux distro):

```bash
cd /tmp
ar x /path/to/Murmel/src-tauri/target/release/bundle/deb/Murmel_*_amd64.deb data.tar.gz
tar xzf data.tar.gz
sudo cp usr/bin/murmel /usr/bin/
sudo cp -a usr/lib/. /usr/lib/
sudo cp -r usr/share/icons/hicolor/* /usr/share/icons/hicolor/
sudo cp usr/share/applications/Murmel.desktop /usr/share/applications/
```

The runtime libraries live in the app-private `/usr/lib/Murmel/` (on the binary's rpath), so no `ldconfig` step is needed.

After subsequent rebuilds, copy the binary and any refreshed runtime libraries:

```bash
sudo cp src-tauri/target/release/murmel /usr/bin/
sudo mkdir -p /usr/lib/Murmel
sudo cp -a src-tauri/transcribe-libs/. /usr/lib/Murmel/
```

Resources only need re-copying if they change upstream (new icons, sounds, models, etc.).

## Troubleshooting

### Windows: `Cannot restore timestamp ... generate.stamp: Zugriff verweigert`

Symptom — the native build dies partway through `transcribe-cpp-sys`, usually
naming a couple of CPU-variant projects:

```
CUSTOMBUILD : CMake error : Cannot restore timestamp
".../ggml/src/CMakeFiles/generate.stamp": Zugriff verweigert
  [ggml-cpu-sandybridge-feats.vcxproj]
  [ggml-cpu-sse42-feats.vcxproj]
```

followed a few lines later by the misleading

```
CMake Error at .../CMakePackageConfigHelpers.cmake:519 (configure_file):
  No such file or directory
-- Configuring incomplete, errors occurred!
```

**Ignore the second error — it is only a consequence of the first.**

**Cause: build parallelism.** `GGML_CPU_ALL_VARIANTS=ON` makes ggml build each
CPU variant as its own MSBuild project (`ggml-cpu-sse42-feats`,
`-sandybridge-feats`, …). Whenever CMake re-runs its generate step, several of
those projects touch the same `ggml/src/CMakeFiles/generate.stamp` at once, and
Windows refuses the concurrent access. Note which projects the error names —
there are always at least two, which is the tell.

**Already handled in this repo:** [`.cargo/config.toml`](.cargo/config.toml)
pins `[build] jobs = 4`. cmake-rs derives `cmake --build --parallel N` from
cargo's `NUM_JOBS`, so capping cargo caps CMake. That keeps the concurrency
below the collision threshold. Do not raise it on Windows without testing a
native rebuild.

If you hit the error anyway, the build tree is left half-configured and must be
cleared — re-running the same command will not repair it:

```bash
# 1. Drop the short NTFS junction (rmdir removes only the link, never the target)
cmd //c rmdir "%LOCALAPPDATA%\tcs\<hash>"

# 2. Drop the half-configured build dir
rm -rf src-tauri/target/debug/build/transcribe-cpp-sys-<hash>

# 3. Build again
bun run tauri dev
```

Find `<hash>` by matching the junction in `%LOCALAPPDATA%\tcs\` to the
`transcribe-cpp-sys-*` directory it points at.

**Also worth knowing: `cargo build` does not warm up `tauri dev`.** `tauri dev`
compiles with `--no-default-features`, which is a _different_ build — different
feature hash, different `OUT_DIR`, its own native build tree. Pre-building buys
nothing and only doubles the native compile.

### AppImage build fails on Arch / rolling-release distros

`linuxdeploy` bundles its own `strip` binary which is too old to process system libraries built with newer toolchains on rolling-release distros (Arch, CachyOS, Manjaro, EndeavourOS).

The error from Tauri:

```
Bundling Murmel_*_amd64.AppImage
failed to bundle project `failed to run linuxdeploy`
```

Tauri swallows the real linuxdeploy error. To see it, run linuxdeploy manually:

```bash
cd src-tauri/target/release/bundle/appimage
~/.cache/tauri/linuxdeploy-x86_64.AppImage --appimage-extract-and-run \
  --appdir Murmel.AppDir --plugin gtk --output appimage
```

**Workaround:** The binary, deb, and rpm bundles all build fine — only the AppImage step fails. To skip it:

```bash
bun run tauri build -- --bundles deb
```

Then install using the deb extraction method above.

### Windows build fails with path-limit errors (`MSB3491` / `FTK1011` / `MSB6003`)

On Windows the native build can fail partway through `transcribe-cpp-sys` with
any of these (all the same root cause):

```
error MSB3491: Could not write lines to file "...VCTargetsPath.tlog\VCTargetsPath.lastbuildstate".
Path: ... exceeds the OS max path limit. The fully qualified file name must be less than 260 characters.
```

```
FileTracker : error FTK1011: could not create the new file tracking log file:
...\vulkan-shaders-gen-build\...\cmTC_xxxxx.tlog\link.write.1.tlog.
The system cannot find the path specified.
```

```
error MSB6003: The specified task executable "CL.exe" could not be run.
System.IO.DirectoryNotFoundException: Could not find a part of the path ...
```

This is **not** a code or toolchain problem — it's Windows' legacy 260-character
path limit (`MAX_PATH`), overflowed by the Vulkan shader generator's nested
CMake build tree on top of Cargo's already-deep
`target\release\build\<crate>-<hash>\out\build\...` directory.

Since `transcribe-cpp` 0.1.3 this is mitigated automatically: the native build
compiles through a short NTFS junction under `%LOCALAPPDATA%\tcs` (created
without admin rights), so a normal checkout builds with no setup. Enabling
Windows long paths does **not** reliably help here — MSBuild's native
`FileTracker` (`tracker.exe`) ignores the long-paths flag — which is why the
junction, not the registry flag, is the fix.

If you still see the errors above, junction creation was likely blocked
(filesystem or corporate policy) — the failing build's log then contains a
`transcribe-cpp-sys: could not create short build junction ...` warning — or
your checkout is deep enough to overflow even the shortened layout. Work
around either case with a short Cargo target directory:

```powershell
# Per-shell:
$env:CARGO_TARGET_DIR = "C:\h"

# Or persist it for all future terminals (note: redirects ALL your
# Rust projects' build output, not just Murmel):
[Environment]::SetEnvironmentVariable('CARGO_TARGET_DIR', 'C:\h', 'User')
```

Artifacts then land in `C:\h\release\...` instead of the repo's
`src-tauri\target\`. Open a **new terminal** if you persisted the variable —
it is only picked up by freshly started processes. Then `bun run tauri dev`
and `bun run tauri build` work normally.

### Windows `tauri build` fails at bundling with `program not found`

If the build compiles all the way to `Built application at: ...\murmel.exe` and
then fails with:

```
Signing C:\...\murmel.exe with a custom signing command
failed to bundle project `program not found`
```

that's the code-signing step: `tauri.conf.json` configures a custom
`signCommand` (`trusted-signing-cli`, Azure Trusted Signing) that only exists
in the release CI environment. Local development doesn't need it:

```powershell
# Development (no bundling/signing at all):
bun run tauri dev

# Or compile a release binary without the installer/signing step:
bun run tauri build --no-bundle
```
