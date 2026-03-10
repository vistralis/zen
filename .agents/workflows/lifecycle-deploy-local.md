---
description: Install locally in development mode — build release binary and deploy to ~/.local/bin
---

# /lifecycle-deploy-local — Local Development Install

Builds a release binary and installs it to `~/.local/bin/zen` — the canonical path used by `install.sh`. This avoids creating a second binary under `~/.cargo/bin/` that `cargo install` would produce.

// turbo-all

## Steps

1. Build release binary:
```bash
cargo build --release
```

2. Install to canonical path (remove first to avoid "Text file busy" when shell hook has zen loaded):
```bash
rm -f ~/.local/bin/zen && cp target/release/zen ~/.local/bin/zen
```

3. Verify:
```bash
zen --version
```

## Rules

- Always use `~/.local/bin/zen` as the install target — this matches `install.sh` and keeps a single binary in `PATH`.
- Never use `cargo install` for zen — it deploys to `~/.cargo/bin/` and creates conflicting paths.
- If `~/.local/bin` is not in `PATH`, warn the user.
