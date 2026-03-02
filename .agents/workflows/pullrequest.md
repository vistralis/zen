---
description: Open a pull request for the current branch
---

# Pull Request Workflow

Open a PR for the current dev branch against `main`.

// turbo-all

## 1. Run alpha validation

Run the full `/alpha` workflow first to ensure the code is clean.

```bash
cd /localdisk/projects/zen/zen && cargo fmt --all -- --check
```

If this fails, run `cargo fmt --all` to fix.

```bash
cd /localdisk/projects/zen/zen && cargo clippy -- -D warnings
```

```bash
cd /localdisk/projects/zen/zen && cargo test
```

```bash
cd /localdisk/projects/zen/zen && cargo build --release
```

```bash
/localdisk/projects/zen/zen/target/release/zen --version
```

## 2. Stage and commit any outstanding changes

```bash
cd /localdisk/projects/zen/zen && git status
```

If there are uncommitted changes, stage and commit them:

```bash
cd /localdisk/projects/zen/zen && git add -A && git commit -m "chore: pre-PR cleanup"
```

## 3. Push branch to origin

```bash
cd /localdisk/projects/zen/zen && BRANCH=$(git branch --show-current) && git push -u private "$BRANCH"
```

## 4. Open PR

Use the GitHub MCP tool to open a PR:
- **Owner**: `vistralis`
- **Repo**: `zen`
- **Head**: current branch name
- **Base**: `main`
- **Title**: descriptive title summarizing the branch's purpose
- **Body**: structured markdown with Summary, Changes, and Testing sections

## 5. Verify CI

Check that CI started on the PR:
- Lint (fmt + clippy + audit)
- Test (Ubuntu x86_64, Ubuntu 22.04, Ubuntu ARM)
- Release build
