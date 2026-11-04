# Distributable packages

The [GameData packages workflow](../.github/workflows/packages.yml) creates
Windows x64, Linux x64 and macOS Apple Silicon ([below](#macos)) ZIPs with optimized client and dedicated-server
executables at the archive root. Extract the entire platform ZIP directly into
the installed game's `GameData` directory, beside `base/`, and launch the client.
Keep the dedicated server beside it for Create game and local devmap. Updating
the executables does not replace the `SJK/` user files.

Each installation ZIP contains exactly four files: `sjk`, `sjk-server` (both with
`.exe` on Windows; on macOS the client is the `SJK.app` bundle, [below](#macos)), `README-SJK.txt` and `LICENSES-SJK.txt`; the suffix keeps them
from replacing another program's `README.txt` or `LICENSES.txt` in the same
GameData folder. The licence file consolidates the complete project, font and
dependency notices with their source labels and attribution inventory; no
original notice text is discarded. A source with a `NOTICE` file has it as the
first section.
The build manifest is a separate `SJK-<revision>-<platform>-build.json` artifact
with the exact revision, target, compiler and binary hashes. The installation ZIP
contains no retail assets, personal configuration, generated
test files, source tree or debug symbols. Each artifact also provides SHA-256
checksums and a matching source snapshot ZIP; the source snapshot is for
contributors and is not needed in GameData.

## Build and verify

Once the workflow is on the default branch, run it manually with `source_ref`
set to the exact commit or branch to package. Pull requests changing the workflow
or packaging script also exercise it against the PR's base revision, avoiding
unmerged game changes. Download the resulting `SJK-windows-x64`,
`SJK-linux-x64` and `SJK-macos-arm64` Actions artifacts. These are build artifacts with a 30-day
retention period, not automatically published GitHub Releases.

The Linux build uses Ubuntu 22.04, requiring glibc 2.35 or newer, ALSA and the
window-system/graphics-driver libraries. At startup the Linux client raises its
main-thread stack allowance to 64 MiB within the inherited hard limit, so terminal,
desktop and in-game update launches do not depend on the usual 8 MiB soft limit.
The Windows build targets MSVC with a
static C runtime, for Windows 10/11 x64. Drivers remain a system requirement.
The Windows release client is a windowed program (`windows_subsystem` in
[main.rs](../crates/sjk-viewer/src/main.rs)), so no console opens beside the game;
debug builds keep theirs. Its log is standard error, which a redirected start
(`sjk.exe > log.txt 2>&1`) still writes to a file. The console programs it starts
itself (the local server, PowerShell for the clipboard, `cmd` for links, `ipconfig`)
go through `platform::hidden_command`, which gives them no window of their own.
Source checkout and source archives preserve the repository's line endings on
both platforms, including embedded WGSL used by exact-text shader patches.

After building, [package_client.py](../scripts/package_client.py) extracts each
ZIP into an isolated directory with spaces, starts the dedicated server on an
ephemeral loopback port and closes it through stdin EOF,
and launches the client from a different working directory with synthetic asset
markers. The client must find those assets and save `SJK/config.cfg`, then exit
on the intentionally invalid content before creating a window. No real servers,
retail files or personal profiles are used. This is an installation/startup check,
not a Windows GPU or gameplay certification.

The script can also package an existing clean build locally:

```sh
cargo build --locked --release --target x86_64-unknown-linux-gnu -p sjk-viewer -p sjk-dedicated
python3 scripts/package_client.py --source . --target x86_64-unknown-linux-gnu --platform linux-x64 --output target/packages --smoke-check
```

Build on the intended distribution baseline: packaging a binary built on a newer
Linux distribution does not lower its glibc requirement. The workflow is the
reference environment for the distributed Linux ZIP.

## macOS

The `macos-arm64` package is built on GitHub's `macos-latest` runner for
`aarch64-apple-darwin` with `MACOSX_DEPLOYMENT_TARGET=11.0`, and passes the same smoke
check there. The client is an app bundle, `SJK.app`, so Finder starts it without
Terminal and the Dock shows SJK's name and icon: `Contents/MacOS/sjk-client`, an
`Info.plist` naming it and the release's version, and `Contents/Resources/SJK.icns`,
which [package_client.py](../scripts/package_client.py) writes from
`assets/branding/sjk-logo-512.png` and `sjk-logo.png`. `sjk-server` stays a bare
program beside the bundle. The client counts the folder holding `SJK.app` as its own
(`platform::install_folder`), for the game data, the `SJK/` client folder, the server
and updates. The program inside is named `sjk-client`, not `sjk`: macOS disks ignore
case by default, and before the bundle a bare `sjk` took the name of the `SJK/` folder
beside it. The client draws through wgpu's Metal backend (the
`metal` feature of `sjk-viewer`). Intel Macs have no build.

The bundle is only ad-hoc signed (`codesign --sign -` on the runner, so the signature
covers its `Info.plist` and icon), not with an Apple Developer ID nor notarized, so
Gatekeeper blocks a downloaded copy: the bundled README tells the player to clear the
quarantine flag (`xattr -dr com.apple.quarantine SJK.app sjk-server`). Allowing it in
System Settings > Privacy & Security instead keeps the flag, and macOS then runs a
quarantined app from a random read-only copy (App Translocation), away from GameData.
Signing and notarizing need a paid Apple Developer account and its certificate in the
workflow's secrets.

Game data: besides the drop-in folder, the client looks in Steam's macOS library for
`Jedi Academy/GameData` and for a `base/` folder inside any `.app` bundle's
`Contents` there (the Mac edition's layout, not checked on a real installation).
The auto-updater fetches `SJK-<version>-macos-arm64.zip`. What the Mac build lacks,
as on Linux: chat MP4 clips (Media Foundation), the video-memory reading for the
small-card settings. Copy and paste go through `pbcopy`/`pbpaste`.

## SJK releases

SJK's [release workflow](../.github/workflows/release.yml) runs the same build and
`package_client.py` smoke check on each `sjk-v<version>` tag and publishes a GitHub
Release. It passes `--name SJK --version <version> --repository <repo URL>`, so the
archives are named `SJK-<version>-<platform>.zip` (with
`SJK-<version>-source.zip`, `SJK-<version>-<platform>-build.json` and
`SJK-<version>-SHA256SUMS-<platform>.txt`) and the bundled instructions point at
SJK's source. The programs packaged are the `[[bin]]` names the crates declare
(`sjk` and `sjk-server`; `--client-bin`/`--server-bin` override them).
