# Copilot Instructions

## Project Context
This is a custom operating system written primarily in Rust, developed from scratch. The goal is to build a minimal, educational OS with modern systems programming practices.

## Code Style & Standards

### Rust Conventions
- Follow standard Rust idioms and the Rust API guidelines
- Use `rustfmt` formatting (enforced in CI)
- Prefer explicit error handling with `Result<T, E>` over panics
- Use `unsafe` blocks sparingly and document safety invariants with `SAFETY:` comments
- No `unwrap()` or `expect()` in production code paths

### Documentation
- Every public function, struct, and module must have doc comments (`///`)
- Complex logic blocks should have inline comments explaining "why", not "what"
- Target: ~1 explanatory comment per 3-5 lines for non-trivial code
- Mark unfinished work with `TODO:` or `FIXME:` comments

### OS-Specific Patterns
- Hardware interaction should be abstracted behind safe interfaces
- Use memory-mapped I/O wrappers, not raw pointer derefs
- All interrupt handlers must be clearly marked and documented
- Boot/initialization code should be in separate modules from runtime code

## Architecture

### Module Organization
- `kernel/` - Core kernel functionality
- `drivers/` - Hardware drivers
- `mem/` - Memory management
- `fs/` - Filesystem implementation
- `proc/` - Process/task management

### Key Guidelines
- Keep platform-specific code isolated in `arch/` subdirectories
- Use traits for abstraction over hardware differences
- Minimize dependencies; prefer `core` and `alloc` over `std`

## What to Avoid
- Don't use standard library (`std`) - this is a `no_std` environment
- Avoid allocations in interrupt handlers or critical sections
- Don't introduce external dependencies without discussion
- No floating point operations in kernel space

## Testing
- Use QEMU for testing boot and runtime behavior

## Current Priorities
Check `TODOS.md` for active work items and priorities before suggesting new features.

## Questions & Clarifications
If code suggestions involve architectural decisions, memory safety trade-offs, or deviation from these guidelines, add a comment asking for review rather than making assumptions.
