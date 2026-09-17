# Test lab (T19): WSL2-Ubuntu instead of a full VM

Status: defined, NOT installed — needs owner decisions (disk + install).

## Audit (T19.1, 2026-09-17, read-only)

- Hyper-V role: **absent** (`vmms` service missing; only `vmcompute/Stopped`).
  Install needs elevation + reboot + downloads — kills live services, not mine to do.
- RAM total: **7.9 GB** (live llama 27B mmap + services live here — a VM would starve them).
- Docker: absent. WSL: version 2 present, **no distros installed**.
- Disk free: C 13.9 GB / S 13.2 GB at a 12 GB floor — a full VM disk
  (10–20 GB + Rust target caches) does not fit without surgery.

## Verdict

Full KVM-style VM now = **no** (no hypervisor role, no RAM headroom, no disk
headroom). The viable lab is **WSL2-Ubuntu**: per-user install, no reboot,
shares the box instead of partitioning it.

## Lab definition (T19.2) — runs only after owner says go

1. Free disk first: `cargo clean` in a scratch profile or move archives;
   need ≥6 GB headroom (distro ~1 GB + Rust 3 GB + build caches).
2. `wsl --install -d Ubuntu` (per-user, ~600 MB download).
3. Inside: `apt install build-essential`, rustup stable, `git clone` GSV +
   poolAI repos (paths differ from `S:/` — use `/home/<user>/rust`).
4. Ports: lab runs on **different** ports (e.g. GSV 19999, Telenetis 19800)
   so live `:9999`/`:9800` are never touched. No tunnels from the lab.
5. Scope: build + `cargo test` + emulator/tour for GSV/Telenetis. llama-rs
   full C++ rebuild and 27B model stay on the host (RAM/disk).
6. Debug from the lab against lab services only; findings land as tickets
   in the host board, fixes commit in host repos.

## What stays primary regardless

The headless emulator (`src/emu.rs`, `tests/phone_emulator.rs`,
`tests/phone_tour.rs`) — process-level, hermetic, zero disk/RAM drama.
The WSL lab adds a real Linux kernel + network stack when the emulator
is not enough to explain a failure.
