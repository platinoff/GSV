# ECO-4a — read-only environment / git-state audit (2026-11-27)

**Scope:** evidence only. Nothing was deleted, no `cargo clean`, no toolchain
change, no remote created, no push outside GSV. All remaining ECO-4 bands stay
with the owner. Ticket `ECO-4a` (`t-1795821561513317700`) closed on the GSV
board with this report as evidence.

## 1. Disk

| Volume | Size | Used | Free | Use % |
|---|---|---|---|---|
| `S:` | 170 G | 127 G | **43 G** | 76 % |
| `L:` | 63 G | 56 G | **7.3 G** | 89 % |

`L:` pressure is dominated by the immutable 21 GB source GGUF + 20 GB
expert-major pack under `L:/gsv_mirror/models/gsv-versus-ai/` (needed for the
LOAD lane — **do not clean**). `S:` has comfortable headroom.

## 2. `target/` sizes (read-only `du -sh`)

| Repo | target |
|---|---|
| GSV | 17 G (includes `target/live` — **untouchable**, live `:9999` listener) |
| adoreble_wifi | 4.2 G |
| llama-rs | 2.6 G |
| poolAI | 2.4 G |
| rusty-engine | 1.5 G |
| LinFS | 1.5 G |
| omniroute | 979 M |
| telenetis | 317 M |
| GSV-WGPU | 135 M |
| rebook | 112 M |
| CoreBoard_Labs_shop | 80 M |
| mesh-node | 65 M |

No cache cleanup was performed: ECO-4 explicitly requires owner approval
before touching non-live `target/` dirs or re-gating.

## 3. Toolchains / caches

- Installed: `stable-x86_64-pc-windows-gnu` (active default) and
  `stable-x86_64-pc-windows-msvc`.
- `.rustup` = 1.8 G, `.cargo` = 1.7 G — much smaller than the 9.4 G / 11.92
  retire candidates recorded in the original ECO-4 note. **Band (4) is now
  largely moot**; no toolchain retired.
- Local adoreble_wifi gates used the installed gnu toolchain because this box
  has no MSVC linker; CI remains on the host-agnostic `stable` channel.

## 4. Git state (`products --state` + per-repo `status -sb`)

- Registered products: 16 rows; all clean except **GSV** with
  `M docs/gsv/tickets.jsonl` — the MCP board write for the ECO-4a ticket
  itself, included in the GSV docs commit (no product code change).
- `origin` present and tracking for GSV, rusty-engine, LinFS, adoreble_wifi,
  omniroute (`release/v3.8.50`), poolAI and the other registered plugins.
- **No remote** on `mesh-node` (`08cf81b`) and `GSV-WGPU` (`9c65f59`) — owner
  must create `github.com/platinoff/mesh-node` / confirm GSV-WGPU remote.
- No dangling stashes observed in the checked repos.
- Non-registered trees present: `freebeat.ai`, `reebeat.ai`, `Tokenomica`,
  `platinoff-profile` (plus `poolAI_concept.txt` mentioned in ECO-4) —
  keep/move is an **owner decision**, not touched.

## 5. ECO-4 bands still awaiting owner

1. Keep/move decision for the non-registered dirs above.
2. Any `target/` cleanup / full re-gate beyond the live GSV copy.
3. Creating the `mesh-node` (and confirming GSV-WGPU) GitHub remotes.
4. Toolchain retirement / `RUSTUP_HOME` migration — currently low value
   (only 1.8 G + 1.7 G).

**Result of this ticket:** the read-only evidence is complete; none of the
owner-gated bands were executed. No quality claim (G13 blocked).
