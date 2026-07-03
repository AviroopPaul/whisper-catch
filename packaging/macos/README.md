# macOS packaging

Builds `WhisprCatch.app` and a `.dmg` for Apple Silicon.

```sh
packaging/macos/build-dmg.sh
# → dist/WhisprCatch.app
# → dist/WhisprCatch-<version>-arm64.dmg
```

The ONNX Runtime is **statically linked** into the binary, so the bundle is just
the single executable + icon + `Info.plist`. No dylibs to ship.

## Permissions

WhisprCatch needs three macOS privacy grants, all requested on first run:

| Permission | Why | Pane |
|---|---|---|
| **Accessibility** | type transcribed text into the focused app | Privacy › Accessibility |
| **Input Monitoring** | see the push-to-talk key globally | Privacy › Input Monitoring |
| **Microphone** | capture speech while the key is held | Privacy › Microphone |

The first-run wizard opens these panes and shows the system Accessibility prompt.

## Signing & notarization

By default the script **ad-hoc signs** (`codesign -s -`). That runs locally, but:

- Gatekeeper warns on first open → users must right-click → **Open** once.
- Each rebuild changes the signature, so macOS **resets the granted permissions**
  (you must re-grant after every dev rebuild).

For distribution, use an Apple Developer ID ($99/yr):

```sh
export SIGN_ID="Developer ID Application: Your Name (TEAMID)"
packaging/macos/build-dmg.sh

# then notarize + staple the dmg:
xcrun notarytool submit dist/WhisprCatch-*-arm64.dmg \
  --apple-id "you@example.com" --team-id TEAMID --password "app-specific-pw" --wait
xcrun stapler staple dist/WhisprCatch-*-arm64.dmg
```

A stable Developer ID signature is also what keeps the event tap and TCC grants
from resetting between updates.
