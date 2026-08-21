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

Owner-set. Default assumption here: `D:/Obsidian Vault/` (MSYS2:
`/d/Obsidian\ Vault/`). Confirm with the owner before writing if the vault
has not been located yet.

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

Drain summaries may land as vault notes (one per band), linked from an
index note. Never stage vault files in the product repo.
