# EchoNode — Design & Engineering Handoff

Push-to-talk dictation for Linux. One global hotkey records audio, a local
Whisper model transcribes it, and the text is pasted at the cursor. This
document is the source of truth for a coding agent building the app.

---

## 1. Product overview

**What it does.** User presses a global hotkey → a floating pill appears at the
bottom of the screen showing `Listening` → user speaks → releases hotkey → pill
switches to `Transcribing` → text is pasted at the cursor and stored in
history.

**Platform.** Linux desktop (X11 + Wayland). Ships as a background daemon plus
a GTK/Qt/Tauri app window plus an AppIndicator/StatusNotifierItem tray icon.

**Users.** Developers and power users who prefer keyboard-driven workflows.

**Non-goals.** Not a meeting recorder. Not a voice assistant. No cloud
transcription by default. No mobile.

---

## 2. Three surfaces

### 2.1 Main app window
Two tabs, top-center segmented control.

**Tab A — History**
- Left sidebar (288px): search input + chronological list of transcriptions.
- List row: timestamp (mono, uppercase, muted), 2-line clamped preview,
  optional amber dot flag.
- Right pane: selected transcript with metadata (duration, word count), copy
  and delete actions, and an ASCII-style waveform strip.
- Empty state: "No transcripts yet. Press Super+Space to dictate."

**Tab B — Configuration**
Sections, each labeled with a small mono uppercase heading:
1. **Engine Parameters** — Neural Model (picker), Language (auto/manual),
   Compute (CPU/CUDA/ROCm device).
2. **Hotkey** — Push-to-talk combo, displayed as a mono chip.
3. **Input Hardware** — Device picker + live input meter (dB, 10-bar).
4. **Output Behavior** — Toggles: paste at cursor, copy to clipboard,
   start on login, launch minimized to tray.

### 2.2 Transcription pill
Floating overlay, always-on-top, docked bottom-center of the active display,
~24px above the bottom edge. Rounded-full, dark translucent, subtle ring.

Two states:
- **Listening** — red pulsing LED + 4-bar live waveform + "Listening…" label +
  elapsed timer (mono, right-aligned, separated by a hairline).
- **Transcribing** — amber spinner + "Transcribing…" label + 3-dot progress.

Dismisses itself on completion (~200ms fade). Click-through everywhere except
a small "cancel" hit area over the LED.

### 2.3 Top-bar menu (system tray dropdown)
Anchored to the tray icon. 256px wide, rounded, dark, ring-1 white/10.

- **Header block** — status label ("Connected" green / "Idle" muted /
  "Recording" red), current model, hotkey chip, thin status bar.
- **Actions** — `Open History` (Super+H), `Preferences…`.
- **Divider**, then `Quit EchoNode` (destructive on hover).

---

## 3. Design system

Direction: **Tactile engineer dark** — precise, developer-native,
keyboard-first. Feels like a hardware push-to-talk radio: physical button,
status LED, clean signal meter.

### 3.1 Type
- Sans: **Geist** (400 / 500 / 600 / 700).
- Mono: **Geist Mono** (400 / 500 / 600) — timestamps, hotkeys, labels,
  numeric readouts.

### 3.2 Palette (oklch tokens, dark-only)
```
--background       oklch(0.145 0.005 285)   /* zinc-950 */
--surface          oklch(0.185 0.005 285)   /* zinc-900 */
--surface-2        oklch(0.22  0.005 285)   /* zinc-800 */
--foreground       oklch(0.92  0.005 285)   /* zinc-100 */
--muted-foreground oklch(0.6   0.01  285)   /* zinc-500 */
--border           oklch(1 0 0 / 8%)
--ring             oklch(1 0 0 / 20%)

--signal-red       #ef4444   /* recording */
--signal-amber     #f59e0b   /* processing / hotkey chip */
--signal-green     #10b981   /* connected / active */
```

No light theme in v1. Ship dark-only.

### 3.3 Radius, shadow, motion
- Radius: 4 / 6 / 10 / 14 (`--radius: 0.625rem` base).
- Shadow: window drop `0 32px 64px -16px rgba(0,0,0,0.5)`, pill `shadow-2xl`.
- Motion: `pulse-slow` (2s ease-in-out infinite, 1 → 0.4 → 1) for signal LEDs.
  Spinner is standard 1s linear. Everything else static or 150–200ms color
  transitions.

### 3.4 Iconography
Heroicons outline, `stroke-width: 2`, `size-4`, `text-zinc-500` idle.

---

## 4. Data model

```ts
type Transcription = {
  id: string;               // uuid
  createdAt: string;        // ISO 8601
  durationMs: number;
  text: string;
  wordCount: number;
  model: string;            // e.g. "whisper-v3-turbo"
  language: string;         // ISO 639-1, or "auto"
  audioPath?: string;       // local file, optional
  flagged?: boolean;        // user star
};

type Settings = {
  model: string;
  language: string;         // "auto" | ISO 639-1
  compute: "cpu" | "cuda" | "rocm";
  inputDeviceId: string;
  hotkey: string;           // e.g. "Super+Space"
  pasteAtCursor: boolean;
  copyToClipboard: boolean;
  startOnLogin: boolean;
  launchMinimized: boolean;
};
```

Persist to `~/.config/echonode/`:
- `settings.json`
- `history.sqlite` (with FTS5 index on `text` for search)
- `audio/` (optional, opt-in retention with size cap)

---

## 5. System integration

### 5.1 Global hotkey
- Wayland: use `xdg-desktop-portal` GlobalShortcuts interface.
- X11: `libxkbcommon` + `xcb` grab. Fall back to Wayland portal if unavailable.
- Default combo: `Super+Space`. Rebindable in Preferences.

### 5.2 Audio capture
- PipeWire preferred, PulseAudio fallback.
- 16 kHz mono PCM, capture only while hotkey is held (push-to-talk).

### 5.3 Transcription
- Local `whisper.cpp` or `faster-whisper` (CTranslate2).
- Models downloaded on demand into `~/.cache/echonode/models/`.
- Compute autodetect: CUDA → ROCm → CPU (int8).

### 5.4 Text injection
- Wayland: `wtype` (no root) or `ydotool` (needs uinput).
- X11: `xdotool type`.
- Fallback: copy to clipboard + notify.

### 5.5 Tray icon
- `libappindicator` / `StatusNotifierItem`. GNOME requires the
  AppIndicator extension.
- Icon states: idle (outline), listening (red filled), processing (amber).

### 5.6 Pill overlay
- Wayland: `layer-shell` protocol, `overlay` layer, anchored bottom-center.
- X11: override-redirect window, always-on-top, click-through via
  XShape/`_NET_WM_STATE_ABOVE`.

---

## 6. Suggested tech stack

- **Shell/UI**: Tauri v2 (Rust core + React frontend). Alternatives: GTK4 +
  gtk-rs, or Qt6 + PySide6.
- **Frontend**: React 19 + Tailwind v4 (tokens above) + shadcn primitives.
- **Storage**: SQLite via `rusqlite` (Tauri) or `better-sqlite3`.
- **Audio**: `cpal` (Rust) or `sounddevice` (Python).
- **Inference**: `whisper.cpp` bindings.
- **Packaging**: `.deb`, `.rpm`, AppImage, Flatpak.

---

## 7. Milestones

1. **M1 — Skeleton**: Tauri app shell, main window with History + Settings
   tabs (static mocks), design tokens wired.
2. **M2 — Capture loop**: Global hotkey → audio capture → whisper.cpp →
   paste at cursor. No UI polish.
3. **M3 — Pill overlay**: layer-shell overlay with Listening/Transcribing
   states, driven by capture loop.
4. **M4 — Tray + dropdown**: StatusNotifierItem with the header block and
   action list.
5. **M5 — History persistence**: SQLite + FTS5 search, delete/copy actions,
   waveform rendering.
6. **M6 — Settings**: Model download UI, hotkey rebind, input device picker,
   output toggles.
7. **M7 — Packaging**: `.deb` + AppImage + Flatpak, autostart via
   `~/.config/autostart/`.

---

## 8. Reference implementation

The design showcase at `/` renders all three surfaces in React + Tailwind v4
with the exact tokens and composition. Use it as visual truth. Files:
- `src/styles.css` — design tokens (oklch + signal colors + `animate-signal`).
- `src/routes/index.tsx` — TopBar, MenubarDropdown, MainWindow (HistoryPane /
  SettingsPane), ListeningPill, TranscribingPill.

Port the components verbatim into whatever shell (Tauri / GTK / Qt) you pick.
