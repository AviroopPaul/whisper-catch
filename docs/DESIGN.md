# WhisprCatch Design Brief

Single source of truth for **site/index.html** (web landing) and the **egui desktop app**
(theme.rs, settings_app.rs, wizard.rs, overlay.rs, tray menus). Every value here is a
decision, not a suggestion. If an implementer needs a value that isn't here, derive it
from the nearest token.

The two surfaces now speak **two deliberate languages**:

- **Landing page** — unchanged: Inter, one teal accent, light-friendly marketing surface
  (Part A below).
- **Desktop app** — "**tactile engineer dark**", adopted from the EchoNode design handoff
  (archived verbatim at `docs/DESIGN-handoff.md`): precise, developer-native,
  keyboard-first. The app should feel like a hardware push-to-talk radio — status LEDs,
  signal meters, mono uppercase labels (Part B below).

---

# Part A — Landing page (site/index.html)

## A1. Typography

One hosted font via a single Google Fonts `<link>`: **Inter** (weights 400, 500, 600, 700).
Fallback stack: `Inter, system-ui, -apple-system, "Segoe UI", sans-serif`.
Mono (kbd, code, demo): `ui-monospace, "Cascadia Code", "JetBrains Mono", monospace` — not hosted.

| Token        | Size (px)              | Weight | Line height | Tracking  | Use |
|--------------|------------------------|--------|-------------|-----------|-----|
| `display`    | clamp(38, 6vw, 60)     | 700    | 1.08        | -0.03em   | Hero h1 only |
| `h2`         | clamp(26, 3.5vw, 34)   | 700    | 1.15        | -0.02em   | Section titles |
| `h3`         | 18                     | 600    | 1.3         | -0.01em   | Card titles, FAQ questions |
| `body-lg`    | 18–19                  | 400    | 1.6         | 0         | Hero sub, section intros |
| `body`       | 15.5–16                | 400    | 1.6         | 0         | Card copy, FAQ answers |
| `small`      | 13.5                   | 400/500| 1.5         | 0         | Captions, footer, CTA note |
| `mono`       | 14                     | 400/600| 1.7         | 0         | Demo, install block, `kbd` |
| `eyebrow`    | 13                     | 600    | 1           | +0.08em, uppercase | Kicker above h2 |

## A2. Color (web only)

One accent family: **teal**. Neutrals are cool slate.

| Token           | Hex        | Use |
|-----------------|-----------|-----|
| `bg-0`          | `#0b0e14` | Page background |
| `bg-1`          | `#11151f` | Raised areas: demo frame, install block, alt bands |
| `bg-2`          | `#151a26` | Cards |
| `border`        | `#232a3a` | 1px hairlines |
| `border-strong` | `#334155` | Hovered card/control borders |
| `text-1`        | `#e6e9f0` | Primary text |
| `text-2`        | `#8b93a7` | Secondary/muted |
| `accent`        | `#5eead4` | Links, icons, highlights, focus rings (teal-300) |
| `accent-hover`  | `#99f6e4` | Hover on accent text/links only |
| `accent-subtle` | `rgba(94,234,212,.10)` | Tinted chips, selected states, icon plates |
| `on-accent`     | `#042f2c` | Text on accent-filled buttons |
| `success`       | `#34d399` | |
| `error`         | `#f87171` | |
| `warning`       | `#fbbf24` | Use sparingly |

Rules: never accent-fill large areas; no gradients; no pure black/white.
Section list, spacing scale, and copy voice for the landing page are unchanged — see git
history of this file (pre-EchoNode revision) if a web section needs its full spec again:
sections `96px` vertical rhythm, cards `24px` padding, radius 6/8/12/16, content column
960px, copy short/confident/privacy-forward, concrete numbers over adjectives.

---

# Part B — Desktop app ("tactile engineer dark")

Direction: precise, developer-native, keyboard-first. A hardware push-to-talk radio:
physical button, status LED, clean signal meter. **Dark only — there is no light theme
and no theme picker.** Source: `docs/DESIGN-handoff.md` §2–3 (surface composition and
design system); adapted here to WhisprCatch's real feature set.

All tokens live in `apps/cli/src/theme.rs`. Screens never hand-pick colors.

## B1. Type

Embedded in the binary (`apps/cli/assets/fonts/`, OFL — license alongside):

- Sans: **Geist** (Regular + Medium + SemiBold) — UI text, labels, buttons, titles.
- Mono: **Geist Mono** (Regular + Medium) — timestamps, hotkey chips, section labels,
  numeric readouts, paths.

egui families: `Proportional` → Geist, `Monospace` → Geist Mono, plus named families
`GeistMedium` / `GeistSemiBold` / `GeistMonoMedium` (egui's `strong()` only recolors, so
weight = family switch via `theme::medium/semibold/mono_medium`). egui-phosphor
(Regular) is appended for icons — used sparingly, muted.

Text scale: Body/Button 14 · Small 11.5 · Mono 12 · section labels mono 11 uppercase ·
wizard titles 23 SemiBold. Hierarchy comes from weight + muted color, never from many
sizes on one screen. **Anything uppercase is mono** (`theme::mono_upper`,
`theme::section_label`).

## B2. Palette (dark-only)

Zinc neutrals + three signal colors. Signal colors mean state — never decoration.

| Token       | Value                | Use |
|-------------|----------------------|-----|
| `BG`        | `#09090b` (zinc-950) | Window background |
| `SURFACE`   | `#18181b` (zinc-900) | Cards, selected list rows, pill plates |
| `SURFACE_2` | `#27272a` (zinc-800) | Buttons, inputs, raised controls |
| `SURFACE_3` | `#34343a`            | Hover/active fills, toggle troughs |
| `FG`        | `#e8e8eb`            | Primary text, primary-button fill |
| `TEXT_2`    | `#a1a1aa` (zinc-400) | Secondary text |
| `MUTED`     | `#71717a` (zinc-500) | Labels, timestamps, metadata |
| `BORDER`    | white 8%             | 1px hairlines everywhere |
| `RING`      | white 20%            | Focus/selected/hover rings |
| `RED`       | `#ef4444`            | Recording (LED, waveform, destructive) |
| `AMBER`     | `#f59e0b`            | Processing (spinner, dots) + hotkey chips |
| `GREEN`     | `#10b981`            | Active / ready / ok |

`theme::tint(color)` = the color at ~12% alpha, for chip fills behind signal text.
The primary button is `FG` fill with `BG` text — signal colors are never button fills.

## B3. Radius, elevation, motion

- Radius: **4** (chips) / **6** (buttons, inputs, list rows) / **10** (cards) /
  **14** (windows). Pill overlay is fully rounded.
- Elevation is borders-first: background step (`BG` → `SURFACE` → `SURFACE_2`) + 1px
  `BORDER`. No drop shadows inside windows.
- Motion: LED pulse 2s ease-in-out (opacity 1 → 0.4 → 1), spinner 1s linear,
  150–200ms color transitions. Nothing else animates.

## B4. Components (theme.rs)

- `led(ui, color, pulse)` — status LED with soft halo.
- `key_chip(ui, label)` — hotkey chip: amber mono uppercase on amber tint, radius 4.
- `section_label(ui, text)` — mono uppercase muted 11px heading.
- `mono_upper(text, size, color)` — mono uppercase micro-text (timestamps, readouts).
- `card(ui)` — SURFACE fill, hairline ring, radius 10, 16px inset.
- `toggle(ui, &mut bool)` — hardware-style switch, green when on.
- `primary_button(ui, text)` — the one high-emphasis action per screen.

## B5. Surfaces

### Main window (`settings_app.rs`)
Opens maximized. Header (52px): green LED + "WhisprCatch" left · **top-center segmented
control** (History | Settings) · mono stats readout right ("163 WORDS · 9 UTT · 1 MIN").

- **History**: left sidebar (288px) = search field + chronological list. Row = mono
  uppercase muted timestamp ("TODAY 23:21") + right-aligned mono duration + 2-line
  clamped preview; selected = SURFACE fill + RING ring. Footer = mono count + quiet
  "Clear all" with inline red confirm. Right pane = mono timestamp + metadata readout
  ("6.1S SPOKEN · 19 WORDS · 0.38S INFERENCE"), ghost Copy/Delete top-right (delete
  confirms inline, red), hairline, then the transcript at 15px in a ≤720px column.
  Empty state: muted mic glyph on a surface plate, "No transcripts yet", "Hold
  ⟨key chip⟩ and speak to dictate." with the *configured* hotkey.
- **Settings**: centered 560px column of sections, each = mono uppercase label + card:
  **ENGINE PARAMETERS** (model picker, mono RAM/download readout, green READY LED or
  green progress bar), **HOTKEY** (key picker + amber key chip preview), **OUTPUT
  BEHAVIOR** (green toggles: live typing, recording indicator, keep history, start on
  login), **ABOUT** (version, links, config path in mono). One primary Save button.

### Pill overlay (`overlay.rs`)
232×40, bottom-center, dark translucent (zinc-950 @ ~92%) with a subtle white ring,
fully rounded, click-through, never takes focus.

- **Listening**: red pulsing LED (2s) + 4-bar red waveform + "Listening…" + elapsed
  mono timer right-aligned behind a vertical hairline.
- **Transcribing**: amber arc spinner (1s) + "Transcribing…" + 3 amber dots pulsing
  sequentially behind the hairline.

### Tray / menu bar (`crates/tray`)
Native menus can't be themed; the language shows in structure + icon states.
Menu: status header (state — model, "Hold ⟨key⟩ to dictate", disabled rows) ·
Listening toggle · **Open History** / **Preferences…** (opens the Settings tab) ·
divider · **Quit WhisprCatch**. Icons: idle = outline/template mic, recording = red,
muted = crossed mic (Linux icon names; macOS uses a template glyph).

### Wizard (`wizard.rs`)
520×560 fixed. Green step dots (done fill / current ring), "STEP N OF 4" in mono
uppercase, painted stroke icon on a surface plate, SemiBold 23 title, green mono
privacy chip, green download progress bar with mono readout, amber spinner while
waiting on authorization, amber key chip on the done screen. One primary button,
pinned near the bottom.

## B6. Copy voice (app)

Same voice as the site: short, confident, privacy-forward, concrete numbers
("0.38S INFERENCE", not "blazingly fast"). Mono uppercase for machine facts, sentence
case for human sentences. Quirk allowed once per surface (wizard done-screen).
