# Git Push (MSYS2 Bash)

Add, commit, push — **MSYS2 bash only**. No PowerShell, no cmd.

Prefer an **external MSYS2 UCRT64** window (Start menu) for push. The Cursor integrated terminal can hit `index.lock` / Win32 error 5.

```bash
export PATH="$HOME/.cargo/bin:/c/msys64/ucrt64/bin:/c/msys64/usr/bin:$PATH"
export RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-stable-x86_64-pc-windows-gnu}"
cd /s/rust/GSV || cd "S:/rust/GSV"
unset CARGO_TARGET_DIR
/c/msys64/usr/bin/rm -f .git/index.lock
git status -sb
# stage sprint files only — never git add -A
git commit -m "type(scope): subject" \
  -m "Summary:" \
  -m "- Зміни: (modules/files)" \
  -m "- Перевірки: cargo fmt; cargo clippy --all-targets; cargo test"
git push origin main
```

Push-only after a commit: `cargo xtask push`

Never stage: `.env*` · `*.pem`/`*.key` · `data/*` (except `.gitkeep`) · `comitmsg/*.txt` · `.opencode/node_modules`
