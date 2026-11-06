# Security

Peekless exists to keep sensitive data from leaking, so security reports get priority.

## Reporting a vulnerability

Please **do not open a public issue** for a vulnerability. Use GitHub's private reporting instead:
**Security → Report a vulnerability** on this repository. You will get a reply within a few days.

Helpful reports include the version, the operating system, what you did, and what leaked or could leak.

## What counts

- Sensitive content that should have been covered but was left visible **after** you confirmed a redaction (for example a box that misses part of a number, or metadata that survives "Remove hidden info").
- Peekless sending anything over the network. It must never do this.
- Peekless reading clipboard entries that password managers mark as concealed.
- Original files being modified or overwritten.

A type of data that Peekless does not detect at all (names, street addresses, a new ID format) is a feature request, not a vulnerability. Please open a normal issue for it.

## Design guarantees

- No network access, no telemetry, no update checks.
- Text recognition uses the OS engine (Apple Vision, Windows.Media.Ocr). On macOS the clipboard image never touches the disk.
- Redactions are burned into the pixels as opaque bars by default.
- Activity history stores only file names and counts, and can be turned off.
