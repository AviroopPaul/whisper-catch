// macOS waitlist signup. Stores one private blob per email (keyed by hash,
// so repeat signups overwrite instead of duplicating).
const { put } = require("@vercel/blob");
const crypto = require("node:crypto");

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]{2,}$/;

module.exports = async function handler(req, res) {
  if (req.method !== "POST") {
    res.setHeader("Allow", "POST");
    return res.status(405).json({ ok: false, error: "method not allowed" });
  }

  const email = String((req.body && req.body.email) || "")
    .trim()
    .toLowerCase();

  if (!email || email.length > 254 || !EMAIL_RE.test(email)) {
    return res.status(400).json({ ok: false, error: "invalid email" });
  }

  const key = crypto.createHash("sha256").update(email).digest("hex").slice(0, 32);
  await put(
    `waitlist/macos/${key}.json`,
    JSON.stringify({ email, platform: "macos", ts: new Date().toISOString() }),
    { access: "private", addRandomSuffix: false, allowOverwrite: true }
  );

  return res.status(200).json({ ok: true });
};
