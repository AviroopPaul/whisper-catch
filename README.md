# whisper-catch

Local push-to-talk dictation for Linux (macOS planned). Hold a key, speak, release — punctuated text is typed into whatever has focus. All inference on-device (NVIDIA Parakeet TDT 0.6B v2, int8 ONNX); no audio leaves the machine.

## Install (Ubuntu/Debian)

```sh
sudo apt install ./whisper-catch_0.1.0-1_amd64.deb
# postinst adds you to the `input` group — log out and back in once
```

Or build from source: `cargo build --release -p whisper-catch` (needs `cmake clang libasound2-dev`).

## Usage

```sh
whisper-catch ptt              # daemon: tray icon + hotkey (default: hold Right-Alt)
whisper-catch ptt --print-only # transcripts to stdout instead of typing
whisper-catch record --seconds 5
whisper-catch transcribe file.wav
whisper-catch download-model   # pre-fetch model (~660 MB, auto-fetched on first run)
whisper-catch autostart --enable
```

First run downloads the model (resumable, SHA-256 verified) to `~/.local/share/whisper-catch/models/`.

Config: `~/.config/whisper-catch/config.toml`

```toml
key = "ralt"   # rctrl, lctrl, ralt, lalt, super, f13, scrolllock
```

## Tray

Mic icon in top bar (GNOME needs the AppIndicator extension — Ubuntu ships it). Shows recording state, listening on/off toggle, session stats. `--no-tray` to run without.

## Architecture

Cargo workspace:

- `crates/core` — warm mic capture (cpal, 300ms pre-roll) + resample (rubato) + engine wrapper (transcribe-rs / ONNX Runtime)
- `crates/hotkey` — raw evdev press/release listener (works on X11 and every Wayland compositor; bare-modifier keys OK)
- `crates/inject` — text injection (XTEST via enigo; Wayland uinput cascade planned)
- `crates/models` — resumable, checksummed model downloader
- `crates/tray` — StatusNotifierItem via ksni
- `apps/cli` — the `whisper-catch` binary

See `SCOPE.md` for the full design doc and macOS plan.

## Packaging

```sh
cd apps/cli && cargo deb   # → target/debian/whisper-catch_*.deb
```

The .deb ships a udev rule for `/dev/uinput` (future Wayland injection) and a postinst that adds the installing user to the `input` group (evdev hotkey).
