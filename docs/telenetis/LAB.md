# Test lab — verdict: harness only (T20.1)

No VM lab. No WSL lab. The WSL direction was invented by the agent and
cancelled by the owner — this file stays only so the question does not
come back.

The test rig is the Rust harness, nothing else:

- `src/emu.rs` — phone double (lifecycle, storage, signer).
- `tests/phone_emulator.rs` — scenarios against an in-process server.
- `tests/phone_tour.rs` — narrated walk (`--nocapture`, `TOUR[…]` lines).
- Gate: `cargo fmt --all` → `cargo clippy --all-targets` (0) → `cargo test`.

Anything the harness cannot prove (real WebView taps, Download Manager,
Adreno/Mali WebGPU numbers, phone-to-phone bytes) goes to the two-phone
session in [`TWO_PHONES.md`](TWO_PHONES.md) — not to a virtual machine.
