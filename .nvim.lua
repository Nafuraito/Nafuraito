-- .nvim.lua — project-local NeoVim settings for Nafuraito OS
--
-- LazyVim loads this file automatically when `vim.o.exrc = true` is set.
-- To enable: add `vim.o.exrc = true` to your LazyVim config (lua/config/options.lua).
--
-- This file configures:
--   1. rust-analyzer for this non-Cargo (Makefile-based) kernel project
--   2. GitHub Copilot context hints for OS/bare-metal development

-- ---------------------------------------------------------------------------
-- 1. rust-analyzer — non-Cargo project support via rust-project.json
-- ---------------------------------------------------------------------------
-- LazyVim uses rustaceanvim for Rust LSP.  Point it at our rust-project.json
-- so rust-analyzer can index the kernel crates without a Cargo workspace.
vim.g.rustaceanvim = vim.tbl_deep_extend("force", vim.g.rustaceanvim or {}, {
  server = {
    settings = {
      ["rust-analyzer"] = {
        linkedProjects = {
          vim.fn.getcwd() .. "/rust-project.json",
        },
        -- Use the nightly toolchain defined in rust-toolchain.toml
        rustfmt = {
          extraArgs = { "+nightly" },
        },
        -- Disable proc-macro expansion (not relevant for a no_std kernel)
        procMacro = { enable = false },
        -- Cargo is not used; disable cargo-based build script features
        cargo = { buildScripts = { enable = false } },
      },
    },
  },
})

-- ---------------------------------------------------------------------------
-- 2. Editor settings for kernel / bare-metal Rust development
-- ---------------------------------------------------------------------------
vim.opt_local.tabstop = 4
vim.opt_local.shiftwidth = 4
vim.opt_local.expandtab = true
vim.opt_local.colorcolumn = "100"

-- ---------------------------------------------------------------------------
-- 3. GitHub Copilot
-- ---------------------------------------------------------------------------
-- No runtime changes needed here — just ensure the LazyVim Copilot extra is
-- enabled in your config (see README "Editor Setup" section).
-- Copilot will use the open buffers and rust-analyzer diagnostics for context.
