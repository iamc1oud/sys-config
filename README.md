# sysconfig

Manages configuration files scattered throughout your filesystem while keeping them synchronized with a Git repository. Track files anywhere, store them in Git, sync changes bidirectionally, and replicate across machines.

## Features

- **Track any file or directory** — no need to reorganize your filesystem
- **Bi-directional sync** — sync local changes to Git, apply Git changes back to disk
- **Conflict detection** — never silently overwrite local changes
- **Three-way hashing** — detects edits vs. moves with per-machine state
- **Portable manifests** — same mapping works across machines with different `$HOME`
- **Atomic writes** — temp files + rename prevent half-written configs
- **No dependencies** — single binary, Git CLI only

## Install

### From release (macOS/Linux)

```sh
curl -fsSL https://raw.githubusercontent.com/ajay-singh-questions/sys-config/main/install.sh | bash -s v0.1.0
```

Or manually specify version:
```sh
bash <(curl -fsSL https://raw.githubusercontent.com/ajay-singh-questions/sys-config/main/install.sh) v0.1.0
```

Binary installs to `~/.local/bin/sysconfig`. Add to PATH if needed:
```sh
export PATH="$HOME/.local/bin:$PATH"
```

### From source

Requires Rust 1.85+:
```sh
cargo install --path .
# or
cargo build --release
./target/release/sys-config --version
```

## Quick Start

### Machine A: Track and sync

```sh
# Initialize a new repository
sysconfig init ~/dotfiles

# Track a file
sysconfig track ~/.config/helix/config.toml

# Track a directory
sysconfig track ~/.config/nvim

# See what's tracked
sysconfig list

# Check status (what changed locally vs. Git)
sysconfig status

# View differences
sysconfig diff

# Copy local changes into Git working tree
sysconfig sync

# Commit and push manually
git -C ~/dotfiles add -A
git -C ~/dotfiles commit -m "my configs"
git -C ~/dotfiles push origin main
```

### Machine B: Clone and apply

```sh
# Clone the repository
git clone <your-remote> ~/dotfiles

# Restore configs to their original locations
cd ~/dotfiles
sysconfig apply

# Everything is now in place
cat ~/.config/helix/config.toml
```

## Commands

| Command | Purpose |
|---------|---------|
| `sysconfig init [PATH]` | Create a new repo (default: current directory) |
| `sysconfig track <PATHS>` | Start tracking files/directories |
| `sysconfig untrack <PATHS>` | Stop tracking, keep the stored copy |
| `sysconfig list` | Show all tracked paths and repo locations |
| `sysconfig status` | Display local changes, added, deleted, conflicts |
| `sysconfig diff` | Show unified diff of all differences |
| `sysconfig sync` | Copy local changes into Git working tree |
| `sysconfig apply` | Restore Git copies to original filesystem locations |

## Advanced Usage

### Specify repo location

```sh
# Via flag
sysconfig --repo ~/dotfiles status

# Via environment variable
export SMARTREPO_DIR=~/dotfiles
sysconfig status

# Via pointer (auto-created by `init`, fallback for any directory)
sysconfig apply  # works from anywhere after init
```

### Dry run (preview without changing)

```sh
sysconfig sync --dry-run
sysconfig apply --dry-run
```

### Non-interactive mode (for scripts)

```sh
# Confirm without prompting
sysconfig sync --yes

# Resolve conflicts automatically
sysconfig apply --yes --on-conflict use-repo
sysconfig apply --yes --on-conflict keep-local
sysconfig apply --yes --on-conflict abort
```

### Sensitive files

Warns before tracking SSH keys, credentials, etc. Skip with `--yes`:

```sh
sysconfig track ~/.ssh/config --yes
```

## How It Works

Each tracked file/directory has:
- **Target** — original location (e.g. `~/.config/helix/config.toml`)
- **Repo path** — where it's stored in Git (e.g. `helix/config.toml`)

The manifest (`sysconfig.toml`) maps these, stored in Git. Per-machine base hashes (`.sysconfig/state.toml`) are git-ignored and detect edits:

```
If:                         Action:
T==R                       → Unchanged
T!=B, R==B                 → Update (local changed)
T==B, R!=B                 → Update (repo changed)
T!=R, T!=B, R!=B           → Conflict (both changed)
T absent, R!=B             → Delete (local removed)
```

This three-way check prevents losing local work when syncing.

## Repository Structure

```
~/dotfiles/
├── sysconfig.toml              # Manifest: target <-> repo mapping (committed)
├── .sysconfig/state.toml       # Per-machine base hashes (git-ignored)
├── .git/                        # Git metadata
├── .gitignore                   # Ignores .sysconfig/
├── helix/config.toml           # Tracked copy (committed)
├── nvim/                        # Tracked copy (committed)
│   ├── init.lua
│   └── lua/plugins/...
└── gitconfig                    # Tracked copy (committed)
```

## Limitations (v0.1)

- No symlink mode yet (copies only)
- No permission-only change detection
- `commit`/`push` are manual (`git -C <repo> ...`)
- No automatic background sync
- macOS and Linux only

## v0.2+ Roadmap

- Symlink mode for immediate syncing
- Automatic `commit`/`push` 
- File watching
- Permission preservation
- `doctor` command for health checks
- Windows support

## Example Workflow

```sh
# 1. Set up on Machine A
sysconfig init ~/dotfiles
sysconfig track ~/.config/helix ~/.config/nvim ~/.gitconfig
sysconfig track ~/.zshrc
sysconfig sync --yes
git -C ~/dotfiles add -A && git -C ~/dotfiles commit -m "initial configs" && git -C ~/dotfiles push

# 2. On Machine B
git clone ssh://git@github.com/user/dotfiles.git ~/dotfiles
sysconfig apply --yes
# All configs now at ~/.config/helix, ~/.config/nvim, ~/.gitconfig, ~/.zshrc

# 3. Back on Machine A: edit and sync
echo "new setting" >> ~/.config/helix/config.toml
sysconfig sync --yes
git -C ~/dotfiles commit -am "helix: new setting" && git -C ~/dotfiles push

# 4. On Machine B: pull and apply
git -C ~/dotfiles pull
sysconfig apply --yes
# New setting now in place locally
```

## Testing

```sh
cargo test            # Unit tests
cargo test --test e2e # End-to-end: tracks, syncs, clones, applies
```

## Building & Releases

### Build locally

```sh
cargo build --release
./target/release/sys-config --version
```

### Create a release

```sh
# Build and package for current platform
./release.sh 0.1.0

# Upload to GitHub Releases (requires `gh` CLI)
gh release create v0.1.0 releases/* --generate-notes
```

Release artifacts are created in `./releases/`:
- `sys-config-{VERSION}-{OS}-{ARCH}` — executable binary
- `.sha256` — checksum for verification

Supported platforms:
- macOS (arm64)
- Linux (x86_64, arm64) — build on target platform
- Windows — build from WSL or native MSVC

## License

MIT
