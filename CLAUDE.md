# WhisprCatch

Open-source push-to-talk dictation, positioned as the open-source Wispr Flow alternative. Hold a hotkey → speak → release → punctuated, capitalized text is typed at your cursor in whatever app has focus. Speech-to-text runs 100% on-device (NVIDIA Parakeet 0.6B via ONNX Runtime, ~25× realtime on CPU) — no cloud, no account. MIT licensed. Linux (.deb) ships today; the macOS (Apple Silicon) port is in progress.

## Layout

- `apps/cli` — the `whisper-catch` binary (Rust workspace root ties it together)
- `crates/` — `core` (audio + inference pipeline), `hotkey` (global key listener), `inject` (types text at the cursor), `models` (model download/selection), `tray` (tray app, settings, history)
- `site/` — landing page: a single self-contained `index.html` (no build step) plus `api/waitlist.js` (Vercel function storing macOS waitlist emails in Vercel Blob)
- `packaging/deb`, `packaging/macos` — .deb and .dmg packaging scripts
- `docs/` — design notes

## Workflow rules

1. **Every PR gets a tracking issue.** Create the GitHub issue first (what's broken / what we're doing), then reference it from the PR body with `Closes #N`.
2. **Frontend changes require screenshots in the PR.** Any change to `site/` (or anything user-visible) must include screenshots for review: desktop (1440px) and mobile (390px and 360px), before/after for visual changes, plus any interactive states touched (e.g. form errors). Host them on the `pr-assets` orphan branch under `pr-<N>/` and hot-link via `https://raw.githubusercontent.com/AviroopPaul/whisper-catch/pr-assets/pr-<N>/<file>.png`. Never merge or delete `pr-assets`.
3. **Keep this file current.** Any change to architecture, deployment, or workflow lands with a matching update to CLAUDE.md in the same PR. Keep it lean — pointers and rules, not essays.

## Deploy & release

- **Site**: auto-deploys to https://whisper-catch.vercel.app via the Vercel git integration. Merging to `main` is a production deploy; every PR gets a preview deployment — check it before merging.
- **App**: push a `v*` tag → the `Release` workflow (`.github/workflows/release-macos.yml`) builds the Linux .deb and macOS .dmg and attaches both to the GitHub Release. The site's download buttons point at `releases/latest`, so publishing the release is shipping.
