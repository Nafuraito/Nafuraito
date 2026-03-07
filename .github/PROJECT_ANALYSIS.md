# Nafuraito — Project Analysis

Generated: 2026-03-03

Summary
- Project: Nafuraito — a custom x86_64 operating system (early alpha)
- Primary language: Rust (no_std kernel) with assembly and C for boot/low-level parts
- Build: `make` (requires `rustc`, `nasm`, `ld`, `gcc`, `mtools`, `make`)

Key locations
- Root README: README.md — build & run notes
- TODO list: TODOS.md — detailed roadmap and status
- Bootloader: bootloader/src/ (assembly + C bridge)
- Kernel entry and sources: kernel/src/
  - Entry: `kernel/src/main.rs` (kernel entry point, heap allocator, runtime shims)
  - Low-level: `kernel/src/entry.asm`, linker scripts at `kernel/src/linker.ld`
  - Subsystems organized under: `gdt/`, `idt/`, `memory/`, `drivers/`

Drivers and subsystems (survey)
- Drivers: ATA, PCI (MSI support), PIC, Video (VGA/VESA/framebuffer), ext4 journaling module
- Memory: physical allocator (`pmm.rs`), paging (`paging.rs`), demand paging, COW implementation
- Descriptor tables: GDT under `gdt/` (TSS support), IDT under `idt/` with handlers
- Filesystem: `drivers/ext4journaling/` containing an ext4 journaling implementation used at boot

Repository metrics (quick)
- Rust source files discovered: 24 (kernel/src/**/*.rs)
- Notable large file: `kernel/src/main.rs` (kernel runtime + allocator + shims)

Build & run (from README)
- Build: `make`
- Run in QEMU: `make run`
- Output artifact: `build/disk.img`

Notes & observations
- The kernel is `#![no_std]` and provides its own alloc/runtime shims and a bump allocator.
- Bootloader stages exist and hand control to the kernel; bootloader contains stage1/2 with assembler and C.
- The project aims for broad OS features (VFS, scheduler, SMP, drivers) but many TODOs remain in `TODOS.md`.
- Code style follows low-level OS idioms; unsafe/inline assembly are used appropriately for hardware control.

Suggested next automated steps (optional)
1. Run a source-file inventory script and store JSON metadata (`.github/project_summary.json`).
2. Extract crate-level features (no_std, crate_type) and list of public interfaces.
3. Add a CI workflow placeholder in `.github/workflows/` to run `make` or basic lint checks.

Files created by this scan
- `.github/PROJECT_ANALYSIS.md` (this file)
- `.github/project_summary.json` (machine-readable summary)

If you want, I can now generate the JSON summary and add a minimal GitHub Actions workflow.
