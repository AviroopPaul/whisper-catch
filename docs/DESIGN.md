# WhisprCatch Design Brief

Single source of truth for **site/index.html** (web landing) and the **egui desktop app**
(settings, wizard — NOT overlay.rs, which is frozen). Every value here is a decision, not a
suggestion. If an implementer needs a value that isn't here, derive it from the nearest token.

Competitive study: Wispr Flow ("Don't type, just speak" — before/after demo up top),
superwhisper ("Just speak. Write faster." — keyboard visualization instead of video, offline
messaging as a feature card), Handy ("speak into any text field" — 4 blunt benefit callouts:
Free / Open Source / Private / Simple), Linear (tight tracking, generous section whitespace,
alternating text/visual), Raycast (keyboard rendered as UI, `kbd` keys as a brand element,
dark-first, one restrained accent). What we take: **the hotkey itself is the hero visual**,
privacy stated in plain declaratives ("Your voice stays on your computer"), one accent only,
copy in short imperative sentences.

---

## 1. Typography

### Web (landing page)
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
| `eyebrow`    | 13                     | 600    | 1           | +0.08em, uppercase | Kicker above h2 ("PRIVACY", "HOW IT WORKS") |

### App (egui — system/bundled fonts only, no custom font loading)
Extends the existing `theme::apply`. Keep egui's default family; sizes are the scale:

| egui TextStyle / usage      | Size | Notes |
|-----------------------------|------|-------|
| Display (wizard step title) | 26, via `RichText::size(26.0).strong()` | Wizard only |
| `Heading`                   | 22   | Window header ("WhisprCatch") |
| Section title               | 17, `RichText::size(17.0).strong()` | Card headers ("Dictation", "Application") |
| `Body` / `Button`           | 15   | |
| `Small`                     | 12.5 | Timestamps, meta, status lines |
| `Monospace`                 | 13   | Model path, hotkey names |

Hierarchy in the app comes from **weight + the weak() modifier + accent color**, never from
more than 3 sizes on one screen.

---

## 2. Color tokens

One accent family: **teal**. The blue (`#38bdf8`) and the teal→blue gradient on the current
landing page are **removed** — gradients read as neon; the brand is one flat teal.
Neutrals are cool slate so light and dark feel like the same product.

### Dark (default on web; app follows system)

| Token           | Hex        | Use |
|-----------------|-----------|-----|
| `bg-0`          | `#0b0e14` | Page / window background |
| `bg-1`          | `#11151f` | Raised areas: demo frame, install block, alt section bands |
| `bg-2`          | `#151a26` | Cards |
| `border`        | `#232a3a` | 1px hairlines everywhere |
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

### Light

| Token           | Hex        | Use |
|-----------------|-----------|-----|
| `bg-0`          | `#f8fafc` | Page / window background |
| `bg-1`          | `#ffffff` | Raised areas |
| `bg-2`          | `#ffffff` | Cards (differentiate with border + shadow `e-1`) |
| `border`        | `#e2e8f0` | |
| `border-strong` | `#cbd5e1` | |
| `text-1`        | `#0f172a` | |
| `text-2`        | `#64748b` | |
| `accent`        | `#0d9488` | teal-600 — AA on white for text ≥14px |
| `accent-hover`  | `#0f766e` | |
| `accent-subtle` | `rgba(13,148,136,.08)` | |
| `on-accent`     | `#ffffff` | |
| `success`       | `#059669` | |
| `error`         | `#dc2626` | |
| `warning`       | `#d97706` | |

### egui mapping (Color32)
- `ACCENT_DARK = (94,234,212)`, `ACCENT_LIGHT = (13,148,136)` — already in theme.rs, keep.
- Dark: `panel_fill (11,14,20)`, `faint_bg_color / card (17,21,31)`, card stroke `(35,42,58)`,
  text `(230,233,240)`, weak text `(139,147,167)`.
- Light: `panel_fill (248,250,252)`, card fill `(255,255,255)`, card stroke `(226,232,240)`,
  text `(15,23,42)`, weak text `(100,116,139)`.
- `selection.bg_fill` = accent at 25% alpha; `hyperlink_color` = accent;
  `error_fg_color` per semantic table above.
- Set these in `theme::apply` via `style.visuals` so `card()` and all widgets inherit —
  do not hand-color per screen.

Rules: never accent-fill large areas (accent fills are buttons and the progress bar only);
never place `text-2` on `accent-subtle`; no gradients anywhere; no pure black/white.

---

## 3. Spacing, radius, elevation

**Spacing** — 4px base unit. Allowed steps: `4, 8, 12, 16, 24, 32, 48, 64, 96`.
- Web: section vertical padding `96` (desktop) / `64` (<720px); grid gap `16`;
  card padding `24`; content max-width `960px` with `24px` gutters (keep current `.wrap`).
- App: window inner margin `16`; card inner margin `16`; between cards `12`;
  label→control gap `24` (grid col spacing); grid row spacing `10`.

**Radius**
| Token  | px  | Use |
|--------|-----|-----|
| `r-sm` | 6   | `kbd` keys, chips, small buttons |
| `r-md` | 8   | Buttons, inputs, egui widgets (matches theme.rs `CornerRadius::same(8)`) |
| `r-lg` | 12  | Cards, code blocks |
| `r-xl` | 16  | Hero demo frame only |
| `pill` | 999 | Recording pill, tags |

**Elevation** — borders-first, shadows-second.
- Dark theme (web + app): elevation = background step (`bg-0` → `bg-1` → `bg-2`) + 1px `border`. No drop shadows.
- Light theme (web): `e-1` = `0 1px 2px rgba(2,6,23,.05)` on cards;
  `e-2` = `0 4px 16px rgba(2,6,23,.08)` on hover / the demo frame. Nothing deeper.
- Motion: `transition: 0.15s ease` for color/border, `0.12s` transform; hover lift max
  `translateY(-1px)`. No parallax, no scroll-jacking.

---

## 4. Landing page — section list & copy direction

Single file `site/index.html`, self-contained CSS, dark theme only (matches current; the dark
palette above is canonical). Content column 960px. Nav: logo, Features, How it works, Privacy,
FAQ, GitHub → primary button "Download".

1. **Hero** — centered. Eyebrow-free. `display` headline, `body-lg` sub (max-width 560px),
   then the kbd ritual line (`hold ⟨Right Alt⟩ → speak → release` — the `kbd` element is a
   brand asset, style it lovingly: `bg-2`, 1px border, 3px bottom border, `r-sm`), then CTAs:
   primary "Download .deb" (flat accent fill, `on-accent` text — **no gradient**), ghost
   "View on GitHub". CTA note in `small`: "Free & open source · Linux x86-64 · No account, no cloud".
2. **Live demo visual** — directly under hero CTAs (Wispr Flow pattern: show the product in
   the first screenful). Keep the terminal-frame motif but animate it with ~15 lines of JS:
   a simulated session — "● recording…" pill state → transcribed sentence types itself out
   character-by-character in `text-1`, timing line ("8.2s audio → 0.31s inference") in accent.
   Loops with a long pause. `prefers-reduced-motion`: show the final frame statically.
3. **How it works** — eyebrow "HOW IT WORKS", h2 "Three moves. No windows." Three numbered
   cards in a row (stack <720px): **1 Hold** ("Hold Right Alt anywhere — any app, any text
   field."), **2 Speak** ("Say what you mean. A small pill shows it's listening."),
   **3 Release** ("Punctuated, capitalized text lands at your cursor in under half a second.").
   Number rendered large (`h2` size) in `accent-subtle` circle.
4. **Features** — h2 "Why WhisprCatch". Keep the current six cards & copy (they're good);
   restyle: replace `▸` pseudo-element with a 20px inline SVG stroke icon in `accent` on an
   `accent-subtle` rounded plate. Order: 100% on-device · Fast on CPU · Punctuation that just
   works · Types anywhere · Tray-native · Your data stays yours.
5. **Privacy** — its own full-width band on `bg-1` (superwhisper/Handy both earn trust here;
   we're local-ONLY, so say it harder). Eyebrow "PRIVACY", h2 "Your voice never leaves this
   laptop." Three short declaratives with check icons: "Zero network calls during dictation —
   the model runs on your CPU." / "History is a plain file on your disk. Read it, grep it,
   delete it." / "MIT-licensed and open source. Audit every line." Ghost link: "Read the source →".
6. **Install** — h2 "Install in a minute." Keep two paths: GUI paragraph (download → double-click
   → wizard) and the `code` block (apt / from source). Add a `small` note: "First launch
   downloads the ~660 MB speech model, once."
7. **FAQ** — h2 "Questions". `<details>` accordions, `h3` questions, `body` answers, hairline
   separators. Seed: Does audio ever leave my machine? (No — and here's how to verify) ·
   Wayland or X11? · Which languages? · GPU needed? (No) · How accurate is it? (Parakeet 0.6B
   vs Whisper large-v3 claim) · Can I change the hotkey? · How do I uninstall?
8. **Footer** — hairline top border. Left: "MIT © 2026 Aviroop Paul". Right: GitHub ·
   Issues · Changelog. One quiet quirk line, `small`, centered above: "Built for people who
   think faster than they type."

---

## 5. Desktop app (egui)

### Settings window (`settings_app.rs`)
- Opens **MAXIMIZED**: `ViewportBuilder::default().with_maximized(true)` (keep
  `min_inner_size [440,420]`).
- Because the window is now large: constrain content to a **centered column, max-width 760px**
  (`ui.set_max_width(760.0)` inside a centered layout) so cards don't stretch across 1920px.
- Header row: "WhisprCatch" heading in accent, left; **three stat chips** right (words ·
  utterances · minutes spoken) — each a small `accent-subtle` pill, value `strong`, label
  `small weak`. Below: History | Settings as selectable tabs; selected tab gets accent text +
  2px accent underline (paint with `ui.painter().hline`).
- **History tab**: toolbar (search field grows to fill, `Reload`, right-aligned destructive
  `Clear…` with inline confirm — keep current confirm pattern). Cards in a scroll area:
  row 1 = timestamp (`small`, accent) · "· 8.2s" (`small weak`) · Copy button right-aligned
  (small, ghost); row 2 = transcript at `Body` size, full width. 4px between cards is too
  tight at full screen — use `8`. Empty state: vertically centered glyph "🎙" at 40px +
  two `weak` lines (keep current copy).
- **Settings tab**: two cards ("Dictation", "Application") with section titles at 17/strong,
  2-col grids `[24,10]` spacing — structure already right. Save row: primary button styled
  as accent fill (`Button::new(...).fill(accent)` with `on-accent` text); status message
  beside it in `small weak`, prefixed ✓ in `success` when saved.

### Onboarding wizard (`wizard.rs`) — four steps: Welcome → Permission → Download → Done
- Window: fixed `520 × 560`, centered, not resizable. Content vertically centered with
  generous whitespace: 32 top, 24 between blocks.
- **Progress indicator**: four 8px dots, centered under the header; done = accent fill,
  current = accent ring (stroke, hollow), upcoming = `border` fill. Paint via
  `ui.painter().circle_*`. Plus `small weak` "Step 2 of 4" beneath.
- **Per-step icon**: one large glyph, 44–48px, in `accent`, centered on a 72px
  `accent-subtle` circle plate (painted). Use egui's bundled emoji/glyphs: Welcome 🎙 ·
  Permission ⌨ · Download ⬇ · Done ✔ (fall back to painted shapes if a glyph is missing).
- **Welcome (new step)**: title (26/strong) "Welcome to WhisprCatch", body "Two quick steps:
  allow keyboard access, then download the speech model. After that you never see this
  window again." Primary button "Set up →".
- **Permission**: title "Keyboard access". Keep current explanation copy + pkexec flow;
  spinner + "waiting for authorization…" while granting; errors in `error` color with the
  existing retry guidance. Button: "Grant keyboard access…" (accent fill).
- **Download**: title "Speech model". Keep copy ("~660 MB, one time… audio never leaves this
  machine"). Progress bar full-width, accent fill, `r-md`; beneath it `small weak`
  "412 / 660 MB — encoder.onnx". Keep resume-on-retry hint on failure.
- **Done**: title "You're all set." Body: "Hold ⟨Right Alt⟩, speak, release. Your words appear
  wherever your cursor is. WhisprCatch lives in your tray." Render the hotkey as a drawn kbd
  chip (bg-2 fill, border stroke, mono text). Primary button "Start dictating".
- No card frame around wizard content — the window IS the card. Whitespace does the work.

### Both windows
- All colors through `theme::` helpers; light + dark must both be checked before shipping.
- Never more than one accent-filled button visible per screen.

---

## 6. Copywriting voice

Short. Confident. Privacy-forward. Concrete numbers over adjectives ("0.31s inference", not
"blazingly fast"). Second person. No exclamation marks, no "revolutionary/seamless/supercharge",
no emoji in web copy (app empty-states may use one glyph). Quirk is allowed in exactly one
place per surface (footer line; wizard done-screen). Say "your machine / this laptop", not
"the edge" or "on-prem".

**Headline candidates** (pick one, hero `display`):
1. "Speak. Release. Text appears." *(current — keep as default)*
2. "Hold a key. Say it. It's typed."
3. "Your voice, typed. Nothing leaves your laptop."

**Subheadline candidates**:
1. "Push-to-talk dictation that runs entirely on your machine. No cloud, no subscription,
   no audio leaving your laptop." *(current — keep)*
2. "Local speech-to-text for Linux. Hold Right Alt, speak, and punctuated text lands at
   your cursor — in about a third of a second."
3. "On-device dictation for every app on your Linux desktop. Free, open source, and offline."

Reference one-liners implementers may reuse: "Zero network calls during dictation." ·
"It's a file on your disk, nothing more." · "No GPU required." · "Built for people who think
faster than they type."
