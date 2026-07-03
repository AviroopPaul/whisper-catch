# whisper-catch — Scope & Architecture

Local, on-device dictation (WhisperFlow-style): hold hotkey → speak → release → punctuated text appears at cursor. Linux + macOS. Floor device: MacBook Air M1 8GB. English-first.

> **Status (shipped):** Linux (evdev + XTEST + ksni) and **macOS (CGEventTap + CGEvent + NSStatusItem menu bar)** both work. Two selectable models: **Moonshine base** int8 (~64 MB, ~0.4 GB RAM — the macOS default for 8 GB Airs) and **Parakeet 0.6B** int8 (~660 MB — the Linux default). macOS ships an ad-hoc-signed `.dmg` (`packaging/macos/build-dmg.sh`); Developer ID notarization is the remaining distribution step.

> Research date: 2026-07-03. Prior art studied: [Handy](https://github.com/cjpais/handy) (Tauri + whisper-rs + Parakeet, MIT — closest existing project, study before building), [VoiceInk](https://github.com/Beingpax/VoiceInk) (Swift, macOS-only reference), whisper-overlay / Voxtype / numen (Linux plumbing reference).

---

## 1. Decisions (TL;DR)

| Area | Decision | Why |
|---|---|---|
| STT model | **Parakeet TDT 0.6B v2, int8 ONNX** | Beats Whisper large-v3 on English WER (6.05% vs ~7.4%), native punctuation + capitalization, ~640MB disk / ~1.2–2GB RAM, 8–15x realtime on M1 Air **CPU-only**, CC-BY-4.0 (commercial OK). Default in Handy/Spokenly/OpenWhispr. |
| Fallback model | whisper.cpp `small.en` (466MB) / `large-v3-turbo q5_0` (547MB, Metal) | MIT, single-file GGML, simplest stack. Costs ~1pt WER; needs VAD to stop silence hallucination. |
| Runtime | sherpa-onnx or [transcribe-rs](https://github.com/cjpais/transcribe-rs) (Rust, powers Handy) | In-process static link. No GPU dependency needed for MVP. |
| App shell | **Tauri v2**, tray-only (`ActivationPolicy::Accessory` on macOS, no window at startup; settings webview created on demand, destroyed on close) | Single codebase, first-party tray/autostart/updater/single-instance plugins, Handy proves the stack. Idle RAM ≈ bare Rust process. |
| Hotkey | **Linux: raw `evdev` listener** (works on X11 + every Wayland compositor, bare-modifier hold). **macOS: CGEventTap `.listenOnly`** (`flagsChanged` → bare modifiers incl. held-fn) | Only approaches that support press-AND-HOLD of a bare modifier globally. Tauri's global-shortcut plugin / `global-hotkey` crate CANNOT do this (no Wayland, no bare modifiers) — do not build on them. |
| Mic | cpal (CoreAudio / PipeWire-ALSA), capture native rate → resample to 16k mono via rubato, lock-free ring buffer, **warm stream + ~300ms pre-roll** | Pre-roll avoids clipping first syllable (CoreAudio stream start ~100ms). |
| Text injection | **Linux cascade**: wtype (wlroots/Hyprland) → in-process uinput virtual keyboard with layout-aware keymap (dotool model) → XTEST (X11) → clipboard. **macOS**: ≤200 chars typed via chunked `CGEventKeyboardSetUnicodeString` (≤20 UTF-16 units/event); longer via pasteboard + synthetic Cmd-V with save/restore | Voxtype ships exactly this Linux cascade. Wispr Flow uses paste on macOS. |
| Model delivery | **Download on first run** (resumable Range requests, SHA-256 verify, progress in tray). HF as primary CDN, GitHub Releases mirror. Store in `app_data_dir()/models` | Industry-unanimous (Handy, VoiceInk, superwhisper). Keeps installer 15–25MB. |
| Packaging | macOS: arm64-only signed+notarized `.dmg` + Homebrew cask. Linux: AppImage + `.deb` (postinst: input group + uinput udev rule) + `.rpm`. **No Flatpak** (sandbox blocks evdev/uinput) | Signing unavoidable: Sequoia removed Gatekeeper bypass; Homebrew 5.0 purging unsigned casks by Sep 2026. Budget $99/yr Apple Developer ID. |
| Updates | Tauri updater plugin (covers dmg + AppImage/deb/rpm), static `latest.json` on GitHub Releases | |

---

## 2. Architecture

```
┌─────────────────────────── tray icon (on/off, stats, settings, quit) ───┐
│                                                                          │
│  hotkey listener ──press──▶ recorder ──release──▶ STT engine ──▶ injector│
│  (evdev / CGEventTap)       (cpal ring buffer,     (Parakeet     (uinput/│
│                              16k mono, pre-roll)    int8 ONNX,    wtype/ │
│                                                     VAD-trimmed)  CGEvent)
└──────────────────────────────────────────────────────────────────────────┘
```

Flow: key-down → start consuming ring buffer (include pre-roll) → key-up → stop, VAD-trim, run inference → wait ~50ms after modifier release → inject text.

**Critical PTT bug class**: user still releasing the modifier when injection starts → injected keys combine with held modifier → triggers compositor shortcuts. Always wait for the release event + ~50ms, lift latched modifiers on the virtual device first.

### Crate layout

```
whisper-catch/
├─ crates/
│  ├─ core/            # AudioCapture, SttEngine trait, TextInjector trait, VAD, stats
│  ├─ engine-parakeet/ # sherpa-onnx / transcribe-rs impl
│  ├─ engine-whisper/  # whisper-rs impl (features: metal, vulkan) — fallback
│  ├─ hotkey/          # evdev listener (linux), CGEventTap (macos)
│  ├─ inject/          # uinput/wtype/XTEST (linux), CGEvent/pasteboard (macos)
│  └─ models/          # download manager: resume, sha256, manifest, mirrors
├─ src-tauri/          # tray, settings, autostart, updater wiring
├─ ui/                 # tiny settings page (vanilla TS/Solid — no heavy framework)
└─ .github/workflows/  # matrix build, sign+notarize, latest.json, cask bump
```

---

## 3. Platform specifics

### Permissions (be honest with users)

| Platform | Needed | Prompt/setup |
|---|---|---|
| macOS | Microphone (TCC, `NSMicrophoneUsageDescription` mandatory in Info.plist), Accessibility (injection + can cover hotkey via NSEvent monitor), Input Monitoring (only if raw CGEventTap) | Runtime prompts. **Stable Developer ID signature required** — re-signed/ad-hoc builds silently lose event tap + reset TCC grants. |
| Linux | `input` group membership + uinput udev rule (`KERNEL=="uinput", GROUP="input", MODE="0660", OPTIONS+="static_node=uinput"`) | `.deb` postinst installs both; AppImage shows one-time instructions. Grants keyboard-read to app — document plainly. |

### macOS gotchas
- CGEventTap auto-disables on timeout (`kCGEventTapDisabledByTimeout`) — listen and re-enable.
- Secure Input (password fields): detect via `IsSecureEventInputEnabled()`, offer "copied to clipboard" fallback.
- fn-key PTT: user must set "Press fn key to: Do Nothing"; many external keyboards handle fn in firmware.
- Sequoia broke Option-only Carbon hotkeys (error -9868) — irrelevant on the CGEventTap path but blocks Carbon fallback for Option-only.
- Metal on whisper.cpp: build with `GGML_METAL_EMBED_LIBRARY` (avoids missing-metallib bug). Avoid CoreML path (whisper-rs flags broken; minutes-long first-run ANE compile).

### Linux gotchas
- evdev is listen-only (never `EVIOCGRAB`) — can't consume the key, so compositor bindings on the same key also fire. Default to low-conflict key (Right-Ctrl); make configurable.
- Enumerate all `EV_KEY` devices + watch hotplug (udev/inotify) — laptop + external keyboards = multiple nodes.
- GNOME tray needs AppIndicator extension (Ubuntu ships it, Fedora doesn't) — app must be fully usable with no tray visible; detect missing StatusNotifierWatcher, show one-time hint.
- GlobalShortcuts portal (KDE ≥5.27, GNOME ≥48, Hyprland) = polish-layer fallback for combo hotkeys; can't bind bare modifiers.

---

## 4. Model tiers (user-selectable, downloaded on demand)

| Tier | Model | Disk | RAM | Notes |
|---|---|---|---|---|
| Accurate | parakeet-tdt-0.6b-v2 int8 | ~660MB | ~1.2–2GB | Best English WER in class, punct/caps native. **Shipped** (Linux default). |
| Small **(shipped)** | moonshine-base int8 (ONNX, transformers.js export) | ~64MB | ~0.4GB | English, punct/caps; runs on an 8GB M1 Air. Same ORT stack as Parakeet — no whisper.cpp/cmake. **macOS default.** |
| Whisper alt (future) | small.en / large-v3-turbo q5_0 GGML | 466MB / 547MB | ~850MB–2GB | Would need the `whisper-cpp` feature (Metal build). Not shipped. |

Upgrade paths (post-MVP): `parakeet-unified-en-0.6b` for true streaming (~160ms, blocked on sherpa-onnx export support, issue #3573); FluidAudio (CoreML/ANE) backend on Apple Silicon for battery/thermals; Kyutai stt-1b for live-typing UX.

Number quirk: Parakeet writes numbers as words ("twenty five") — post-MVP text normalization pass.

---

## 5. MVP cut

**In:**
1. Hold-hotkey → record → release → punctuated English text at cursor (the core loop)
2. evdev + CGEventTap hotkey (configurable key, default Right-Ctrl / fn)
3. Parakeet int8 via sherpa-onnx/transcribe-rs, Silero VAD trim
4. First-run model download with progress + checksum
5. Tray icon: on/off toggle, basic stats (words dictated, session), settings, quit
6. Injection cascade per platform
7. Packaging: signed .dmg, AppImage + .deb

**Out (later):** streaming/live-typing, auto-update, multi-language, custom vocab, history window, Flatpak, number normalization, CoreML/ANE backend.

**Risk register:**
- 8GB M1 Air + browser + 2GB inference RAM = tight; test early on real hardware. Canary-180m as escape hatch.
- Wayland injection matrix is the flakiest subsystem — cascade + clipboard fallback is mandatory, not optional.
- Apple Developer ID ($99/yr) required before any real macOS distribution.
- Latency target (≤1–2s finalize for 30s utterance): chunk-transcribe at pause boundaries during capture so release-to-text stays under ~1s.
