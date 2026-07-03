<div align="center">

<img src="assets/icon-512.png" width="96" alt="WhisprCatch icon">

# WhisprCatch

**Hold a key. Speak. Punctuated text appears wherever your cursor is.**

Local push-to-talk dictation for Linux — no cloud, no account, no audio leaving your machine.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Latest release](https://img.shields.io/github/v/release/AviroopPaul/whisper-catch)](https://github.com/AviroopPaul/whisper-catch/releases/latest)
[![Platform: Linux](https://img.shields.io/badge/platform-Linux-lightgrey.svg)](#install)

[**whisper-catch.vercel.app**](https://whisper-catch.vercel.app)

</div>

---

## Screenshots

While you dictate, a small pill floats near your cursor:

| Listening | Transcribing |
| :---: | :---: |
| ![Recording overlay](docs/screenshots/shot-overlay.png) | ![Transcribing overlay](docs/screenshots/shot-overlay-transcribing.png) |

Every utterance is logged locally (optional) and browsable in the settings window:

![History tab](docs/screenshots/shot-settings-history.png)

![Settings tab](docs/screenshots/shot-settings-settings.png)

## Why

- **On-device and private.** All inference runs locally via ONNX Runtime. Audio is never written to disk and never leaves the machine.
- **Real punctuation and capitalization.** The model emits properly punctuated text — no "period" or "comma" voice commands.
- **Fast.** ~25x realtime on CPU; text lands almost as soon as you release the key, and live typing streams words while you're still speaking.

## Install

**Ubuntu / Debian:**

1. Download the `.deb` from the [latest release](https://github.com/AviroopPaul/whisper-catch/releases/latest).
2. Double-click it (or right-click → *Open with App Center*) and install.
3. Launch **WhisprCatch** from your app menu.

A first-run wizard handles the rest: keyboard permission (a one-time polkit prompt) and the model download (~660 MB, resumable, SHA-256 verified).

**Terminal alternative:**

```sh
sudo apt install ./whisper-catch_amd64.deb
whisper-catch ptt
```

## Usage

Hold the hotkey, speak, release. The transcription is typed into whatever window has focus.

| Key (`key` in config) | Physical key |
| --- | --- |
| `ralt` *(default)* | Right Alt |
| `lalt` | Left Alt |
| `rctrl` | Right Ctrl |
| `lctrl` | Left Ctrl |
| `super` | Super / Windows |
| `f13` | F13 |
| `scrolllock` | Scroll Lock |

**Live typing** — with `streaming = true` (the default), words are typed as they stabilize while you're still holding the key; the remainder lands on release. Turn it off to get the full utterance in one shot.

**Tray** — a mic icon shows recording state, a Listening on/off toggle, session stats, and shortcuts to settings. GNOME needs the AppIndicator extension (Ubuntu ships it). Run with `--no-tray` to skip it.

**Settings & history** — `whisper-catch settings`, or click the app icon while the daemon is running. Browse past transcriptions, copy them, tweak options.

Other commands:

```sh
whisper-catch ptt --print-only   # transcripts to stdout instead of typing
whisper-catch record --seconds 5 # mic smoke test
whisper-catch transcribe file.wav
whisper-catch download-model     # pre-fetch the model
whisper-catch autostart --enable # start on login
```

## How it works

- **Model:** NVIDIA Parakeet TDT 0.6B v2, int8-quantized ONNX, run via ONNX Runtime ([transcribe-rs](https://crates.io/crates/transcribe-rs)). ~25x realtime on a modern CPU — no GPU needed.
- **Hotkey:** raw evdev press/release listener, so bare modifier keys work and it functions on X11 and every Wayland compositor.
- **Mic:** kept warm with a 300 ms pre-roll ring buffer, so the first syllable isn't clipped; released after idle.
- **Injection:** XTEST (via enigo) types the text at the display-server level into the focused window.

Workspace layout: `crates/core` (capture, resample, engine), `crates/hotkey`, `crates/inject`, `crates/models` (resumable downloader), `crates/tray` (ksni), `apps/cli` (the binary). See [`SCOPE.md`](SCOPE.md) for the full design doc.

## Configuration

`~/.config/whisper-catch/config.toml` (written with defaults on first run):

| Key | Default | Description |
| --- | --- | --- |
| `key` | `"ralt"` | Push-to-talk key — see table above |
| `streaming` | `true` | Type words live while speaking instead of all at once on release |
| `overlay` | `true` | Show the floating recording indicator while dictating |
| `history` | `true` | Keep a local log of transcriptions (`history.jsonl`) |
| `theme` | `"system"` | UI theme: `system`, `light`, `dark` |
| `model_dir` | *(unset)* | Override the model directory; defaults to `~/.local/share/whisper-catch/models/…` |

## Building from source

```sh
sudo apt install cmake clang libasound2-dev
cargo build --release -p whisper-catch
```

To produce a `.deb` (ships the desktop entry, icons, a udev rule for `/dev/uinput`, and a postinst that adds you to the `input` group):

```sh
cargo install cargo-deb
cd apps/cli && cargo deb   # → target/debian/whisper-catch_*.deb
```

## Roadmap

- **Wayland text injection cascade** — wlroots virtual-keyboard → uinput fallback (hotkey and capture already work on Wayland).
- **macOS** — CGEvent tap hotkey + CGEventPost injection.
- **Streaming-native model** — true incremental decoding instead of rolling re-transcription.

## License

[MIT](LICENSE) © Aviroop Paul
