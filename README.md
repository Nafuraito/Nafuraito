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

## Features

- x86_64 architecture support
- Bootloader with stage1 and stage2 loading
- ext4 filesystem with journaling
- Memory management with paging (LA57 support documented)
- PCI device enumeration and support
- VGA video driver
- Interrupt handling (IDT)
- Global Descriptor Table (GDT)
- Programmable Interrupt Controller (PIC) support

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

### Running

To build and run the OS in QEMU:

```bash
make run
```

## Project Structure

```
bootloader/       - Stage 1 and Stage 2 bootloaders
kernel/           - Main kernel source code
  ├── src/
  │   ├── drivers/      - Device drivers (ext4, PCI, video, etc.)
  │   ├── gdt/          - Global Descriptor Table
  │   ├── idt/          - Interrupt Descriptor Table
  │   └── memory/       - Memory management and paging
build/            - Build output and compiled binaries
DOCUMENTATIONS/   - Additional documentation
```

## Documentation

- [Kernel Architecture](./DOCUMENTATIONS/README.md)
- [ext4 Filesystem with Journaling](./DOCUMENTATIONS/filessystem-ext4journaling/fs.md)
- [LA57 Paging](./DOCUMENTATIONS/la57-paging.md)
- [Memory Zones and NUMA](./DOCUMENTATIONS/memory-zones-numa.md)
- [API Server](./DOCUMENTATIONS/API/API_Server.md)
- [ext4 Journaling Quick Start](./kernel/src/drivers/ext4journaling/QUICKSTART.md)

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
