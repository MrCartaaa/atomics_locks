# Rust Development Rules

focus on code quality, safety, security, and reliability.

This project enforces Rust best practices using nightly tooling, aggressive linting, undefined behavior detection, dependency auditing, and comprehensive CI.

## Key Features

- **Nightly Rust toolchain** with `rustfmt`, `clippy`, and `miri` (`rust-toolchain.toml`)
- **Strict Clippy rules** banning `unwrap()`, `expect()`, and requiring documentation on `unsafe` blocks
- **Miri** for detecting undefined behavior in tests
- **cargo-deny** for license compliance, vulnerability scanning, yanked crate blocking, and source restrictions
- **Opinionated rustfmt** configuration for consistent code style
- **Optimized GitHub Actions CI** that runs checks only on changed `.rs` files
- **Compile-time denials** for common anti-patterns (configured in `lib.rs`)
- **Git pre-push hook** that automatically runs formatting, tests, Miri, Clippy, audit, and deny checks before pushing commits
- **Git pre-commit hook** that checks for latest rust_template changes
- **Comprehensive .gitignore** to exclude build artifacts, temporary files, environment files, Docker outputs, and IDE/editor settings

## Tooling Details

### Rust Toolchain (`rust-toolchain.toml`)

Pins the project to the latest `nightly` channel with essential components:

```toml
channel = "nightly"
components = ["rustfmt", "clippy", "miri"]
profile = "minimal"
```

### rustfmt (`rustfmt.toml`)

Enforces consistent formatting:

- 4-space indentation
- 100 character line width
- Trailing commas where possible
- Unix newlines
- Reordered imports

### Clippy (`clippy.toml` + `lib.rs`)

Bans footguns via `disallowed-methods`:

- `Option::unwrap` / `expect`
- `Result::unwrap` / `expect`

Additional compile-time denials in `lib.rs`:

```rust
#![deny(clippy::disallowed_methods)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(unsafe_op_in_unsafe_fn)]
```

### Miri

Miri interprets Rust MIR to catch undefined behavior (UB) such as invalid memory access, uninitialized reads, and more.

The CI runs:

```bash
cargo +nightly miri test --all-targets --all-features
```

**Resources**:
- Official repository: https://github.com/rust-lang/miri
- Documentation & usage: https://github.com/rust-lang/miri/blob/master/README.md
- Undefined Behavior in Rust: https://doc.rust-lang.org/reference/behavior-considered-undefined.html

### Test Utilities (`tests/must.rs`)

Safe, Clippy-compliant unwrap helpers **exclusively for tests**.

#### Purpose
- Production code is strictly forbidden from using `unwrap()` or `expect()` (enforced via `clippy.toml` + `disallowed_methods`)
- Tests often need to assert that a value *must* be present when logic guarantees it
- This module provides ergonomic, panic-on-failure helpers that:
  - Are gated behind `#[cfg(test)]`
  - Use `#[track_caller]` for accurate panic location reporting
  - Avoid triggering `unwrap_used` / `expect_used` lints
  - Produce clear failure messages

#### How It’s Added
The `sync-rust-template` script copies `tests/common.rs` from the template → `tests/must.rs` in your project:
- Skips if `must.rs` already exists
- Overwrites with `--force`

```bash
sync-rust-template --force   # creates / overwrites tests/must.rs
```

### cargo-deny (`deny.toml`)

Enforces workspace-wide dependency policy:

- Allowed licenses: `MIT`, `Apache-2.0`, `BSD-3-Clause`
- Forbidden licenses: `GPL-2.0`, `GPL-3.0`, `AGPL-3.0`
- Blocks known vulnerabilities and yanked crates
- Restricts sources to `crates.io` and your GitHub repositories

More info: https://github.com/embarkstudios/cargo-deny

### GitHub Actions CI (`.github/workflows/rust_ci.yaml`)

Triggers on push/PR to `main` or `master`. Includes:

- `cargo +nightly fmt -- --check`
- `cargo +nightly clippy -- -D warnings -D clippy::undocumented_unsafe_blocks`
- `cargo +nightly miri test`
- `cargo audit`
- `cargo deny check`

Optimizes performance by detecting changed `.rs` files and skipping checks when no Rust code is modified.

## Pre-Push Hook

### Git Pre-Push Hook (`.git/hooks/pre-push`)

This project includes a pre-push hook to enforce Rust quality checks **before any `git push`**. It runs automatically when you push commits and ensures your code passes formatting, linting, testing, Miri, and dependency checks.

* **Automatic execution**: `.git/hooks/pre-push`
* **Bypass options**:

  * `SKIP_RUST_GATE=1` — skips all Rust checks globally
  * `FAST_PUSH=1` — skips Miri tests but still runs other checks
* **Requirements**: Only runs if all of these files exist in your Rust project:

  * `clippy.toml`
  * `deny.toml`
  * `rustfmt.toml`
  * `rust-toolchain.toml`

### Behavior

1. Checks formatting:

```bash
cargo fmt -- --check
```

2. Runs all tests:

```bash
cargo test
```

3. Runs Miri unless `FAST_PUSH` is set:

```bash
cargo miri test
```

4. Enforces strict linting:

```bash
cargo clippy -- -D warnings
```

5. Audits dependencies:

```bash
cargo audit
```

6. Verifies workspace policies:

```bash
cargo deny check
```

If any check fails, the push is aborted.

---

## Git Ignore (`.gitignore`)

This project uses a comprehensive `.gitignore` to prevent committing unnecessary or sensitive files:

```gitignore
# Rust / Cargo
/target/
**/*.rs.bk
**/*.rs.orig
**/*.rs.tmp
**/debug/
**/release/
*.rs.meta
rust-project.json

# Environment files
.env
.env.*
.env.local
.env.production
.env.development

# Docker
docker-compose.override.yml
Dockerfile.*
.dockerignore
*.dockerfile
*.log
*.pid
docker-volume-*
docker-container-*

# IDEs / Editors
.vscode/
.idea/
*.swp
*.swo
*.bak
*.tmp
```

## Getting Started

```bash
# Create a repo from this template on GitHub, then:
git clone https://github.com/yourusername/your-project.git
cd your-project

# Toolchain auto-selected via rust-toolchain.toml
cargo build
cargo test
```

## Adding Dependencies

All new dependencies must pass `cargo deny check`. Update `Cargo.toml` and run:

```bash
cargo deny check
```

Also update the Git source allowlist in `deny.toml` if using private Git dependencies.

## Local Checks

Run these commands to verify code quality locally:

```bash
cargo +nightly fmt -- --check                    # Formatting
cargo +nightly clippy -- -D warnings              # Strict linting
cargo +nightly miri test                          # Undefined behavior
cargo audit                                       # Vulnerabilities
cargo deny check                                  # Licenses / sources / bans
```

## Syncing Template Configs & Documentation

This project includes a small helper script called `sync-rust-template` that lets you easily bring in the latest configuration files, documentation standards, and CI workflow from your rust_template directory into any Rust project.

What the script does

When run from the root of a Rust project (must contain `Cargo.toml`), it:

- Copies these config files (fails if they exist unless `--force` is used):
  - `clippy.toml` (strict linting rules)
  - `deny.toml` (license/vulnerability/source checks via cargo-deny)
  - `rust-toolchain.toml` (nightly Rust + rustfmt/clippy/miri)
  - `rustfmt.toml` (opinionated code formatting)

- Copies the GitHub Actions workflow:
  - `.github/workflows/rust-integrity-guard.yaml`  
    (creates the `.github/workflows/` directory if missing; skips if the file exists unless `--force` is used)

- Handles `DEVELOPMENT.md`:
  - Copies the template's `README.md` to `DEVELOPMENT.md`  
  - Normal mode: skips if `DEVELOPMENT.md` already exists  
  - With `--force`: overwrites `DEVELOPMENT.md` if it exists

- Handles `README.md`:
  - If missing → creates minimal version: `For development rules, see [DEVELOPMENT.md](DEVELOPMENT.md)`
  - If exists → prepends the above link (only once, idempotent check)

- Appends header to `src/lib.rs` from template (skips if header already present via content check)

Safety features:
- Fails immediately if not in a Rust project (no `Cargo.toml`)
- Pre-checks: refuses to overwrite config files without `--force`
- Skips missing template files with clear warnings
- Idempotent: won't duplicate headers or README links
- `--force` also enables overwriting `DEVELOPMENT.md` and the workflow file

Prerequisites

1. Set the environment variable pointing to your template directory:

   ```bash
   export RUST_TEMPLATE_DIR="/path/to/your/rust_template"
   ```

   Make it permanent (add to `~/.bashrc`, `~/.zshrc`, or `~/.profile`):

   ```bash
   echo 'export RUST_TEMPLATE_DIR="/path/to/your/rust_template"' >> ~/.zshrc
   source ~/.zshrc
   ```

2. Verify the template directory exists:

   ```bash
   ls "$RUST_TEMPLATE_DIR" # Should show clippy.toml, deny.toml, .github/workflows/rust-integrity-guard.yaml, etc.
   ```

Installation & Setup

1. Save the script to a directory in your `$PATH`:

   ```bash
   mkdir -p ~/.local/bin
   # Save script as ~/.local/bin/sync-rust-template
   ```

2. Make executable (chmod privileges):

   ```bash
   chmod +x ~/.local/bin/sync-rust-template
   ```

   Verify:

   ```bash
   ls -l ~/.local/bin/sync-rust-template # Should show -rwxr-xr-x
   ```

3. Add script directory to PATH (if not already):

   ```bash
   echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
   source ~/.zshrc
   ```

Usage

From any Rust project root:

```bash
# Safe mode (fails if config files exist; skips DEVELOPMENT.md & workflow if present)
sync-rust-template

# Force mode: overwrites config files, DEVELOPMENT.md, and workflow file if they exist
sync-rust-template --force
# or
sync-rust-template -f

# Show help
sync-rust-template --help
```

Example workflow:

```bash
cd ~/projects/my-rust-app
export RUST_TEMPLATE_DIR="$HOME/templates/rust_template"
sync-rust-template --force

# Check results
ls -l clippy.toml deny.toml rust-toolchain.toml rustfmt.toml DEVELOPMENT.md
ls -l .github/workflows/rust-integrity-guard.yaml
head -n 20 src/lib.rs # Should show template header
head -n 5 README.md   # Should show DEVELOPMENT.md link

# Commit
git add clippy.toml deny.toml rust-toolchain.toml rustfmt.toml \
       DEVELOPMENT.md README.md src/lib.rs \
       .github/workflows/rust-integrity-guard.yaml
git commit -m "chore: sync rust_template configs, workflow, and development docs"
```

Expected Output (first run with --force)

```
Syncing from template: /path/to/your/rust_template
Target directory:     /home/user/projects/my-rust-app
(FORCE mode: will overwrite existing config files + DEVELOPMENT.md)

'/path/to/your/rust_template/clippy.toml' -> './clippy.toml'
'/path/to/your/rust_template/deny.toml' -> './deny.toml'
'/path/to/your/rust_template/rust-toolchain.toml' -> './rust-toolchain.toml'
'/path/to/your/rust_template/rustfmt.toml' -> './rustfmt.toml'
'/path/to/your/rust_template/.github/workflows/rust-integrity-guard.yaml' -> './.github/workflows/rust-integrity-guard.yaml'
Created workflow file: .github/workflows/rust-integrity-guard.yaml
'/path/to/your/rust_template/README.md' -> './DEVELOPMENT.md'
Overwriting DEVELOPMENT.md (with --force)
Created/Updated DEVELOPMENT.md from template README.md
Created minimal README.md pointing to DEVELOPMENT.md
Appended header to src/lib.rs

Done:
  • 7 new file(s) created/copied
  • 1 file(s) overwritten (with --force)
  • 1 file(s) updated (header or README pointer)
```

Troubleshooting

- `RUST_TEMPLATE_DIR is not set` → run the export command
- `Template directory not found` → check path with `ls "$RUST_TEMPLATE_DIR"`
- `Permission denied` → `chmod +x sync-rust-template`
- `command not found` → add script dir to `$PATH` and `source ~/.zshrc`
- `Not a Rust project` → run from directory containing `Cargo.toml`
- `Files already exist` → use `--force` flag

After Syncing

Run these to verify everything works:

```bash
cargo +nightly fmt -- --check
cargo +nightly clippy -- -D warnings
cargo deny check
cargo +nightly miri test
```

Pro tip: Add an alias to your shell for convenience:

```bash
echo 'alias rust-sync="sync-rust-template --force"' >> ~/.zshrc
# Then just: rust-sync
```

Enjoy consistent, production-grade Rust tooling and integrity checks across all your projects! 🚀
