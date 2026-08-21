---
name: obsidian-vault
description: >-
  Conventions for working in the owner's Obsidian vault — flat structure,
  Title Case index notes, [[wikilinks]], no folders. Use when creating or
  editing Obsidian notes during a VDT drain session (Cursor, OpenCode, or
  Grok).
metadata:
  audience: gsv-vdt-kit
  clients: cursor-opencode-grok
source: mattpocock/skills
---

# Obsidian Vault

Adapted from [mattpocock/skills](https://github.com/mattpocock/skills)
`obsidian-vault` (installed via skills.sh; path localized for this machine).

## Vault location

`S:\rust\GSV\vault\` (MSYS2: `/s/rust/GSV/vault`) — inside the kit repo,
gitignored (`/vault/`), never staged. Open the folder as an Obsidian vault
once; Obsidian creates `.obsidian/` locally.

Mostly flat at root level.

## Naming conventions

- **Index notes**: aggregate related topics (e.g., `Drain Index.md`,
  `Skills Index.md`, `Rust Index.md`)
- **Title case** for all note names
- No folders for organization - use links and index notes instead

## Linking

- Use Obsidian `[[wikilinks]]` syntax: `[[Note Title]]`
- Notes link to dependencies/related notes at the bottom
- Index notes are just lists of `[[wikilinks]]`

## GSV tie-in

Drain summaries land as vault notes (one per band), linked from
`Drain Index.md`. Automation writes them — `cargo xtask vault-note --band N
--title "Federated Done" --summary "…"` creates `Band N <Title>.md` and
appends the `[[wikilink]]` row to the index. Never stage vault files in the
product repo.
