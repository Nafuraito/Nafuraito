# Nafuraito

A semi-professional x86_64 operating system designed with a focus on mainstream hardware support, ease of use, security, and performance.

> **Status**: Early Development (Alpha) - This is an active hobby project with serious professional aspirations.

## About

Nafuraito is a modern operating system being developed from the ground up. While currently in early development stages, the project follows professional development practices and aims for future release as a stable, production-ready system.

### Goals

- **Hardware Support**: Great compatibility with mainstream systems and hardware
- **Usability**: Easy to install, configure, and use
- **Security**: Security-first design principles throughout the system
- **Performance**: Optimized for speed and efficiency

## Building

### Requirements

- `rustc` - Rust compiler
- `NASM` - Netwide Assembler
- `ld` - GNU Linker
- `gcc` - GNU C Compiler
- `mtools` - Tools for manipulating MS-DOS files
- `make` - Build automation

### Compilation

To compile the project:

```bash
make
```

The final disk-image fill be located at `build/disk.img`

### Running

To build and run the OS in QEMU:

```bash
make run
```

## Editor Setup

### NeoVim with LazyVim and GitHub Copilot

This repository ships with everything needed to get a great editing experience
inside [NeoVim](https://neovim.io) with [LazyVim](https://lazyvim.org).

#### Prerequisites

| Tool | Purpose |
|------|---------|
| NeoVim ≥ 0.10 | Editor |
| [LazyVim](https://lazyvim.org) | NeoVim distribution |
| `rustup` with the **nightly** toolchain | Rust toolchain (see `rust-toolchain.toml`) |
| `rust-analyzer` | Rust language server — install via `rustup component add rust-analyzer` |
| A GitHub Copilot subscription | AI completions |

#### 1 — Enable the LazyVim Copilot extra

Add the Copilot extra to your LazyVim configuration
(`~/.config/nvim/lua/config/lazy.lua`):

```lua
require("lazy").setup({
  spec = {
    { "LazyVim/LazyVim", import = "lazyvim.plugins" },
    { import = "lazyvim.plugins.extras.coding.copilot" }, -- GitHub Copilot
    { import = "plugins" },
  },
})
```

Then run `:Lazy sync` inside NeoVim, followed by `:Copilot auth` to sign in
with your GitHub account.

#### 2 — Enable project-local configuration (exrc)

The repository contains a `.nvim.lua` file that configures `rust-analyzer` for
this non-Cargo kernel project. To allow NeoVim to load it automatically, add
the following to `~/.config/nvim/lua/config/options.lua`:

```lua
vim.o.exrc = true   -- load .nvim.lua / .nvimrc from the project root
```

The first time you open the project NeoVim will ask whether to trust
`.nvim.lua`; answer **yes** (or add it to your trust list with
`:trust .nvim.lua`).

#### 3 — Install the nightly toolchain and rust-analyzer

```bash
rustup toolchain install nightly
rustup component add rust-analyzer --toolchain nightly
```

#### 4 — Open the project

```bash
nvim .
```

`rust-analyzer` will start automatically, index all kernel crates listed in
`rust-project.json`, and GitHub Copilot will provide context-aware suggestions
for `no_std` / bare-metal Rust throughout the codebase.

#### Verifying the setup

| Check | Command |
|-------|---------|
| Copilot is running | `:Copilot status` |
| LSP is attached | `:LspInfo` (should show `rust_analyzer`) |
| Completions work | Type a few characters in a `.rs` file — both LSP and Copilot suggestions should appear |

## License

This project is licensed under a custom license. See [LICENSE.md](./LICENSE.md) for details.

**In summary**: You may view and study the source code, and modify it for personal, private, internal use only. Redistribution, commercial use, and sharing are prohibited without explicit written permission from the copyright holder.

## Contributing

Contributions are welcome and appreciated, but **only with the explicit prior permission of the copyright holder**. By submitting a contribution, you agree to grant the copyright holder a perpetual, irrevocable, worldwide, royalty-free, unrestricted right to use, modify, and distribute your contribution.

Please review the [LICENSE.md](./LICENSE.md) file thoroughly before contributing.

## Author

**Hannes Popp** - Copyright © 2026

---

For more information, questions, or to discuss contributing to the project, please reach out directly.
