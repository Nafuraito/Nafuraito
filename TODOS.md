# TODOS - Nafuraito KERNEL Development

---

## PHASE 1: CORE KERNEL FOUNDATION

### Bootloader
- [x] Stage 1: MBR/GPT boot sector (512 bytes)
- [x] Stage 2: Load kernel from disk
- [x] A20 line enablement
- [x] Switch to Protected Mode (32-bit)
- [x] Switch to Long Mode (64-bit)
- [x] Set up initial page tables for kernel
- [x] Parse memory map (E820/UEFI)
- [x] Refactor bootloader to C
- [ ] Multiboot2/Limine protocol support
- [ ] UEFI boot support (alternative to BIOS)

### Global Descriptor Table (GDT)
- [x] Define kernel code segment (ring 0)
- [x] Define kernel data segment (ring 0)
- [x] Define user code segment (ring 3)
- [x] Define user data segment (ring 3)
- [x] Task State Segment (TSS) for each CPU
- [x] GDT loading routine (`lgdt`)
- [x] TSS loading routine (`ltr`)
- [x] Segment reload routine

### Interrupt Descriptor Table (IDT)
- [x] Define all 256 interrupt vectors
- [x] CPU exception handlers (0-31):
  - [x] #DE - Divide Error (0)
  - [x] #DB - Debug Exception (1)
  - [x] #NMI - Non-Maskable Interrupt (2)
  - [x] #BP - Breakpoint (3)
  - [x] #OF - Overflow (4)
  - [x] #BR - BOUND Range Exceeded (5)
  - [x] #UD - Invalid Opcode (6)
  - [x] #NM - Device Not Available (7)
  - [x] #DF - Double Fault (8)
  - [x] #TS - Invalid TSS (10)
  - [x] #NP - Segment Not Present (11)
  - [x] #SS - Stack-Segment Fault (12)
  - [x] #GP - General Protection Fault (13)
  - [x] #PF - Page Fault (14)
  - [x] #MF - x87 FPU Error (16)
  - [x] #AC - Alignment Check (17)
  - [x] #MC - Machine Check (18)
  - [x] #XM - SIMD Floating-Point Exception (19)
  - [x] #VE - Virtualization Exception (20)
  - [x] #CP - Control Protection Exception (21)
- [x] Hardware interrupt handlers (32-47 for legacy PIC, or APIC)
- [x] Software interrupt handler (syscall)
- [x] IDT loading routine (`lidt`)
- [x] IST (Interrupt Stack Table) support for critical exceptions

### Programmable Interrupt Controller (PIC/APIC)
- [x] Legacy 8259 PIC initialization
- [x] PIC remapping (IRQ 0-15 → INT 32-47)
- [x] PIC masking/unmasking
- [x] EOI (End of Interrupt) handling
- [x] APIC detection and initialization
- [x] Local APIC setup
- [x] I/O APIC setup
- [ ] MSI/MSI-X support for modern devices

---

## PHASE 2: MEMORY MANAGEMENT

### Physical Memory Manager (PMM)
- [ ] Parse memory map from bootloader
- [ ] Bitmap allocator implementation
- [ ] Free list allocator (alternative)
- [ ] Buddy allocator (alternative)
- [ ] Page frame allocation (`alloc_frame`)
- [ ] Page frame deallocation (`free_frame`)
- [ ] Memory statistics tracking
- [ ] DMA-capable memory zones
- [ ] NUMA awareness (multi-socket systems)

### Virtual Memory Manager (VMM)
- [x] 4-level paging (PML4 → PDPT → PD → PT)
- [ ] 5-level paging support (LA57) for large memory
- [ ] Page table creation/destruction
- [ ] Virtual address mapping (`map_page`)
- [ ] Virtual address unmapping (`unmap_page`)
- [ ] Permission bits (R/W/X, User/Supervisor)
- [ ] Page attribute table (PAT) support
- [ ] Copy-on-Write (CoW) implementation
- [ ] Demand paging
- [ ] Memory-mapped files
- [ ] Guard pages for stack overflow detection

### Kernel Heap
- [ ] Simple bump allocator (early boot)
- [ ] Slab allocator for kernel objects
- [ ] General-purpose allocator (`kmalloc`/`kfree`)
- [ ] Memory pool allocator
- [ ] Debug allocator with:
  - [ ] Memory leak detection
  - [ ] Buffer overflow detection
  - [ ] Use-after-free detection
  - [ ] Double-free detection

---

## PHASE 3: PROCESS & THREAD MANAGEMENT

### Process Management
- [ ] Process Control Block (PCB) structure
- [ ] Process creation (`fork`/`spawn`)
- [ ] Process termination (`exit`)
- [ ] Process waiting (`wait`/`waitpid`)
- [ ] Process states (Running, Ready, Blocked, Zombie)
- [ ] Process ID (PID) allocation
- [ ] Parent-child relationships
- [ ] Process groups and sessions
- [ ] Address space management per process
- [ ] File descriptor table per process
- [ ] Signal handling per process
- [ ] Resource limits (rlimits)

### Thread Management
- [ ] Thread Control Block (TCB) structure
- [ ] Kernel threads
- [ ] User threads
- [ ] Thread creation (`pthread_create`)
- [ ] Thread termination (`pthread_exit`)
- [ ] Thread joining (`pthread_join`)
- [ ] Thread-local storage (TLS)
- [ ] Thread states
- [ ] Stack allocation per thread
- [ ] Floating-point state saving (FPU/SSE/AVX)

### Scheduler
- [ ] Round-robin scheduler
- [ ] Priority-based scheduler
- [ ] Multi-level feedback queue (MLFQ)
- [ ] Completely Fair Scheduler (CFS) style
- [ ] Real-time scheduling (FIFO, RR)
- [ ] Context switching (save/restore registers)
- [ ] Timer-based preemption
- [ ] CPU affinity
- [ ] Load balancing (SMP)
- [ ] Idle task

### Synchronization Primitives
- [ ] Spinlocks
- [ ] Mutexes
- [ ] Semaphores
- [ ] Read-write locks
- [ ] Condition variables
- [ ] Barriers
- [ ] Atomic operations
- [ ] Lock-free data structures
- [ ] Priority inheritance (prevent priority inversion)
- [ ] Deadlock detection

---

## PHASE 4: SYSTEM CALLS & USER SPACE

### System Call Interface
- [ ] Syscall entry point (SYSCALL/SYSENTER or INT 0x80)
- [ ] Syscall dispatcher
- [ ] User/kernel mode transition
- [ ] Argument validation
- [ ] Return value handling
- [ ] Error codes (errno)

### Essential System Calls
- [ ] Process: `fork`, `exec`, `exit`, `wait`, `getpid`, `getppid`
- [ ] Memory: `brk`, `sbrk`, `mmap`, `munmap`, `mprotect`
- [ ] Files: `open`, `close`, `read`, `write`, `lseek`, `stat`, `fstat`
- [ ] Directories: `mkdir`, `rmdir`, `readdir`, `getcwd`, `chdir`
- [ ] File descriptors: `dup`, `dup2`, `pipe`, `fcntl`, `ioctl`
- [ ] Signals: `kill`, `signal`, `sigaction`, `sigprocmask`
- [ ] Time: `time`, `gettimeofday`, `nanosleep`, `clock_gettime`
- [ ] Users: `getuid`, `setuid`, `getgid`, `setgid`
- [ ] IPC: `shmget`, `shmat`, `msgget`, `msgsnd`, `msgrcv`
- [ ] Network: `socket`, `bind`, `listen`, `accept`, `connect`, `send`, `recv`

### User Space Support
- [ ] ELF executable parser
- [ ] ELF loader (static executables)
- [ ] Dynamic linker/loader (shared libraries)
- [ ] User-mode stack setup
- [ ] argc/argv/envp passing
- [ ] Auxiliary vector (auxv)
- [ ] Program interpreter support

---

## PHASE 5: INTER-PROCESS COMMUNICATION (IPC)

### Signals
- [ ] Signal delivery mechanism
- [ ] Default signal handlers
- [ ] Custom signal handlers
- [ ] Signal masking
- [ ] Real-time signals
- [ ] Signal queuing

### Pipes
- [ ] Anonymous pipes
- [ ] Named pipes (FIFOs)
- [ ] Pipe buffer management

### Shared Memory
- [ ] POSIX shared memory (`shm_open`)
- [ ] System V shared memory (`shmget`)
- [ ] Memory-mapped shared regions

### Message Queues
- [ ] POSIX message queues
- [ ] System V message queues

### Sockets (Local)
- [ ] Unix domain sockets
- [ ] Socket pairs

---

## PHASE 6: VIRTUAL FILE SYSTEM (VFS)

### VFS Layer
- [ ] Superblock structure
- [ ] Inode structure
- [ ] Dentry (directory entry) structure
- [ ] File structure
- [ ] File operations interface
- [ ] Inode operations interface
- [ ] Superblock operations interface
- [ ] Mount point management
- [ ] Path resolution
- [ ] Symbolic link resolution
- [ ] File descriptor management
- [ ] Open file table

### File System Features
- [ ] File permissions (rwx)
- [ ] File ownership (uid/gid)
- [ ] File timestamps (atime, mtime, ctime)
- [ ] Hard links
- [ ] Symbolic links
- [ ] Special files (devices, sockets, pipes)
- [ ] File locking (flock, fcntl)

---

## PHASE 7: TIMEKEEPING & CLOCKS

### Hardware Timers
- [ ] Programmable Interval Timer (PIT 8254)
- [ ] CMOS Real-Time Clock (RTC)
- [ ] APIC Timer
- [ ] High Precision Event Timer (HPET)
- [ ] Time Stamp Counter (TSC)
- [ ] ACPI PM Timer

### Timekeeping
- [ ] System uptime tracking
- [ ] Wall-clock time (UTC)
- [ ] Local time support
- [ ] Timezone handling
- [ ] NTP synchronization support
- [ ] Monotonic clock
- [ ] High-resolution timers
- [ ] Timer wheel implementation
- [ ] Delayed work queues

---

## PHASE 8: SYMMETRIC MULTIPROCESSING (SMP)

### Multi-Core Support
- [ ] AP (Application Processor) detection
- [ ] AP startup sequence (INIT-SIPI-SIPI)
- [ ] Per-CPU data structures
- [ ] Per-CPU variables
- [ ] Per-CPU IDT/GDT
- [ ] CPU topology detection
- [ ] Inter-Processor Interrupts (IPIs)
- [ ] TLB shootdown
- [ ] CPU hotplug support

### SMP Synchronization
- [ ] RCU (Read-Copy-Update)
- [ ] Sequence locks
- [ ] Per-CPU spinlocks
- [ ] Big kernel lock removal

---

## DRIVERS: INPUT DEVICES

### PS/2 Controller (8042)
- [ ] Controller initialization
- [ ] Command sending
- [ ] Data reading
- [ ] IRQ handling (IRQ 1, IRQ 12)

### PS/2 Keyboard
- [ ] Scancode set detection (1, 2, 3)
- [ ] Scancode to keycode translation
- [ ] Key press/release detection
- [ ] Modifier keys (Shift, Ctrl, Alt, Meta)
- [ ] Caps Lock, Num Lock, Scroll Lock
- [ ] Keyboard LED control
- [ ] Key repeat handling
- [ ] Keyboard layout support (US, DE, etc.)
- [ ] Dead keys and compose sequences

### PS/2 Mouse
- [ ] Mouse initialization
- [ ] Movement packet parsing
- [ ] Button state tracking
- [ ] Scroll wheel support
- [ ] Mouse acceleration

### USB HID (Human Interface Device)
- [ ] USB keyboard driver
- [ ] USB mouse driver
- [ ] USB gamepad/joystick driver
- [ ] HID report descriptor parsing

---

## DRIVERS: DISPLAY & GRAPHICS

### Text Mode (Legacy)
- [ ] VGA text mode (80x25)
- [ ] Cursor control
- [ ] Scrolling
- [ ] Color attributes
- [ ] Hardware cursor

### VGA Graphics
- [ ] VGA mode setting
- [ ] VGA register programming
- [ ] Planar memory access
- [ ] Standard VGA modes (320x200, 640x480)

### VESA/VBE
- [x] VBE mode enumeration
- [x] VBE mode setting
- [x] Linear framebuffer access
- [ ] EDID parsing (monitor detection)

### GOP (UEFI Graphics)
- [ ] GOP protocol usage
- [ ] Framebuffer setup

### Framebuffer Driver
- [x] Generic framebuffer interface
- [x] Pixel plotting
- [x] Line drawing
- [x] Rectangle drawing
- [ ] Bitmap blitting
- [ ] Hardware scrolling
- [ ] Double buffering
- [ ] VSync support

### GPU Drivers (Advanced)
- [ ] Intel integrated graphics
- [ ] AMD GPU (basic)
- [ ] NVIDIA GPU (basic, nouveau-style)
- [ ] Virtio-GPU (for VMs)
- [ ] Bochs/QEMU BGA display

### Font Rendering
- [x] Built-in bitmap fonts
- [ ] PSF font loading
- [ ] TrueType/OpenType support (FreeType)
- [ ] Anti-aliasing
- [ ] Subpixel rendering

---

## DRIVERS: STORAGE

### ATA/IDE
- [x] PIO mode read/write
- [ ] DMA mode read/write
- [ ] Drive detection
- [ ] IDENTIFY command
- [x] LBA28 addressing
- [ ] LBA48 addressing
- [ ] ATAPI support (CD/DVD)

### AHCI (SATA)
- [ ] AHCI controller detection
- [ ] HBA initialization
- [ ] Port initialization
- [ ] Command list setup
- [ ] FIS (Frame Information Structure)
- [ ] Read/write commands
- [ ] NCQ (Native Command Queuing)
- [ ] Hot-plug support
- [ ] ATAPI over AHCI

### NVMe
- [ ] Controller initialization
- [ ] Admin queue setup
- [ ] I/O queue setup
- [ ] Namespace management
- [ ] Read/write commands
- [ ] Interrupt handling (MSI-X)

### Floppy Disk Controller
- [ ] FDC initialization
- [ ] Read/write sectors
- [ ] Format support
- [ ] DMA setup

### Virtio Block
- [ ] Virtio-blk driver (for VMs)
- [ ] Virtqueue management

### SCSI
- [ ] SCSI command set
- [ ] SCSI over USB (USB Mass Storage)

### RAID (Software)
- [ ] RAID 0 (striping)
- [ ] RAID 1 (mirroring)
- [ ] RAID 5 (distributed parity)
- [ ] RAID 6 (double parity)

---

## DRIVERS: FILE SYSTEMS

### FAT Family
- [ ] FAT12 (floppy disks)
- [ ] FAT16
- [ ] FAT32
- [ ] Long filename support (VFAT)
- [ ] exFAT

### ext Family
- [ ] ext2
- [ ] ext3 (journaling)
- [x] ext4 (extents, journaling)

### Other File Systems
- [ ] ISO 9660 (CD-ROM)
- [ ] UDF (DVD/Blu-ray)
- [ ] NTFS (read-only initially)
- [ ] Btrfs
- [ ] XFS
- [ ] ZFS
- [ ] F2FS (flash-friendly)
- [ ] SquashFS (read-only compressed)
- [ ] FUSE (Filesystem in Userspace)

### Special File Systems
- [ ] tmpfs (RAM-based)
- [ ] ramfs
- [ ] procfs (/proc)
- [ ] sysfs (/sys)
- [ ] devfs/devtmpfs (/dev)
- [ ] debugfs

### Initrd/Initramfs
- [ ] CPIO archive parsing
- [ ] Initramfs extraction
- [ ] Root filesystem pivot

---

## DRIVERS: NETWORK

### Network Stack Architecture
- [ ] Socket layer
- [ ] Protocol layer
- [ ] Network device layer
- [ ] Packet buffer management (sk_buff equivalent)

### Ethernet (Layer 2)
- [ ] Ethernet frame handling
- [ ] MAC address management
- [ ] MTU handling
- [ ] Promiscuous mode

### ARP (Address Resolution Protocol)
- [ ] ARP request/reply
- [ ] ARP cache
- [ ] ARP table management

### IPv4
- [ ] IP packet parsing
- [ ] IP header checksum
- [ ] IP fragmentation
- [ ] IP reassembly
- [ ] IP routing table
- [ ] Subnet mask handling

### IPv6
- [ ] IPv6 packet parsing
- [ ] ICMPv6
- [ ] Neighbor Discovery Protocol
- [ ] Stateless Address Autoconfiguration (SLAAC)

### ICMP
- [ ] Echo request/reply (ping)
- [ ] Destination unreachable
- [ ] Time exceeded

### UDP
- [ ] UDP socket implementation
- [ ] Port management
- [ ] Checksum calculation

### TCP
- [ ] TCP state machine
- [ ] Three-way handshake
- [ ] Connection teardown
- [ ] Sequence numbers
- [ ] Acknowledgments
- [ ] Window management
- [ ] Retransmission
- [ ] Congestion control
- [ ] Nagle's algorithm
- [ ] Keep-alive

### DHCP
- [ ] DHCP client
- [ ] DHCP server (optional)

### DNS
- [ ] DNS resolver
- [ ] DNS caching

### Higher-Level Protocols (Optional)
- [ ] HTTP client
- [ ] HTTPS/TLS support
- [ ] FTP client
- [ ] SSH client/server
- [ ] NFS client
- [ ] SMB/CIFS client

### Network Interface Drivers
- [ ] Intel e1000/e1000e
- [ ] Intel i217/i218/i219
- [ ] Intel i350/i354/i210/i211
- [ ] Realtek RTL8139
- [ ] Realtek RTL8169/RTL8168
- [ ] AMD PCnet
- [ ] Virtio-net (for VMs)
- [ ] Loopback interface

### Wireless (802.11)
- [ ] WiFi stack (cfg80211/mac80211 equivalent)
- [ ] WPA/WPA2 support
- [ ] Intel WiFi drivers
- [ ] Realtek WiFi drivers
- [ ] Atheros WiFi drivers

---

## DRIVERS: USB

### USB Host Controllers
- [ ] UHCI (USB 1.x)
- [ ] OHCI (USB 1.x)
- [ ] EHCI (USB 2.0)
- [ ] xHCI (USB 3.x)

### USB Core
- [ ] Device enumeration
- [ ] Configuration parsing
- [ ] Interface/endpoint management
- [ ] Control transfers
- [ ] Bulk transfers
- [ ] Interrupt transfers
- [ ] Isochronous transfers
- [ ] Hub driver
- [ ] Hot-plug support
- [ ] Power management

### USB Device Classes
- [ ] USB HID (keyboards, mice)
- [ ] USB Mass Storage (flash drives, HDDs)
- [ ] USB Audio
- [ ] USB Video (webcams)
- [ ] USB CDC (serial, Ethernet)
- [ ] USB Printer

---

## DRIVERS: AUDIO

### Audio Architecture
- [ ] Audio mixer
- [ ] Sample rate conversion
- [ ] Audio buffer management
- [ ] ALSA-like API

### Sound Cards
- [ ] PC Speaker (beep)
- [ ] Sound Blaster 16 (legacy)
- [ ] Intel HD Audio (HDA)
- [ ] AC'97
- [ ] Virtio-snd (for VMs)

---

## DRIVERS: BUSES & SYSTEM

### PCI/PCIe
- [ ] PCI configuration space access
- [ ] PCI device enumeration
- [ ] PCI BAR (Base Address Register) handling
- [ ] PCI interrupt routing
- [ ] PCIe extended configuration
- [ ] MSI/MSI-X setup

### ACPI
- [ ] RSDP (Root System Description Pointer)
- [ ] RSDT/XSDT parsing
- [ ] MADT (Multiple APIC Description Table)
- [ ] FADT (Fixed ACPI Description Table)
- [ ] DSDT/SSDT (AML tables)
- [ ] AML interpreter (basic)
- [ ] Power state management (S0-S5)
- [ ] CPU power states (C-states)
- [ ] CPU performance states (P-states)
- [ ] Thermal management
- [ ] Battery information
- [ ] AC adapter status
- [ ] Power button handling

### DMA
- [ ] ISA DMA (8237)
- [ ] Bus-mastering DMA
- [ ] IOMMU support (Intel VT-d, AMD-Vi)

### Serial Port (COM/UART)
- [ ] 8250/16550 UART driver
- [ ] Baud rate configuration
- [ ] Flow control
- [ ] Serial console support

### Parallel Port
- [ ] Basic LPT support

### I2C/SMBus
- [ ] I2C controller driver
- [ ] SMBus protocol support

### SPI
- [ ] SPI controller driver

---

## DRIVERS: HARDWARE MONITORING

### CPU Temperature
- [ ] Intel Core temp
- [ ] AMD K10 temp
- [ ] ACPI thermal zones

### Fan Control
- [ ] ACPI fan control
- [ ] PWM fan control

### Voltage/Power
- [ ] Motherboard voltage sensors
- [ ] Power consumption monitoring

---

## DRIVERS: SECURITY HARDWARE

### TPM (Trusted Platform Module)
- [ ] TPM 1.2 driver
- [ ] TPM 2.0 driver
- [ ] Secure boot support

### Random Number Generation
- [ ] RDRAND/RDSEED instructions
- [ ] Hardware RNG
- [ ] /dev/random, /dev/urandom

---

## USER INTERFACE

### Terminal/Console
- [ ] Virtual terminals (VT1-VT7)
- [ ] Terminal emulator (VT100/ANSI)
- [ ] PTY (pseudo-terminal) support
- [ ] Line discipline
- [ ] Terminal control (termios)
- [ ] Screen and tmux support

### Windowing System
- [ ] Window manager
- [ ] Compositor
- [ ] Window decorations
- [ ] Input event routing
- [ ] Clipboard support
- [ ] Drag and drop

### GUI Toolkit
- [ ] Widget library (buttons, text fields, etc.)
- [ ] Layout managers
- [ ] Event system
- [ ] Theme support

### Desktop Environment
- [ ] Desktop shell
- [ ] Application launcher
- [ ] System tray
- [ ] Notifications
- [ ] Settings application

---

## SYSTEM UTILITIES & SHELL

### Shell
- [ ] Command parsing
- [ ] Built-in commands (cd, echo, export)
- [ ] Environment variables
- [ ] Path resolution
- [ ] I/O redirection
- [ ] Pipes
- [ ] Job control
- [ ] History
- [ ] Tab completion
- [ ] Scripting support

### Core Utilities
- [ ] ls, cd, pwd, mkdir, rmdir
- [ ] cp, mv, rm, ln
- [ ] cat, head, tail, less, more
- [ ] grep, sed, awk
- [ ] find, locate
- [ ] chmod, chown, chgrp
- [ ] ps, top, kill
- [ ] df, du, free
- [ ] date, cal
- [ ] mount, umount
- [ ] tar, gzip, bzip2

### System Tools
- [ ] init/systemd-style service manager
- [ ] Login manager (getty)
- [ ] User management (useradd, passwd)
- [ ] Cron/task scheduler
- [ ] Package manager

---

## KERNEL DEBUGGING & DEVELOPMENT

### Debugging Features
- [ ] Kernel panic handler
- [ ] Stack trace (backtrace)
- [ ] Symbol table loading
- [ ] Kernel debugger (kdb)
- [ ] GDB stub for remote debugging
- [ ] Debug logging (printk levels)
- [ ] Serial console output
- [ ] Kernel log buffer (dmesg)
- [ ] JTAG debugging support

### Testing
- [ ] Unit testing framework
- [ ] Integration tests
- [ ] Stress tests
- [ ] Fuzzing infrastructure

### Profiling
- [ ] Performance counters
- [ ] Tracing (ftrace equivalent)
- [ ] Sampling profiler
- [ ] Lock contention analysis

---

## SECURITY

### Access Control
- [ ] Discretionary Access Control (DAC)
- [ ] Capabilities
- [ ] Mandatory Access Control (MAC/SELinux-style)
- [ ] Namespaces
- [ ] Seccomp

### Memory Protection
- [ ] ASLR (Address Space Layout Randomization)
- [ ] Stack canaries
- [ ] NX (No-Execute) bit
- [ ] SMEP/SMAP (Supervisor protection)
- [ ] Kernel page table isolation (KPTI)

### Cryptography
- [ ] AES
- [ ] SHA-256/SHA-512
- [ ] RSA
- [ ] Elliptic curve cryptography
- [ ] Random number generation
- [ ] Key management

### Secure Boot
- [ ] UEFI Secure Boot support
- [ ] Kernel signature verification
- [ ] Module signature verification

---

## VIRTUALIZATION

### Hardware Virtualization (Optional)
- [ ] Intel VT-x support
- [ ] AMD-V support
- [ ] KVM-style hypervisor

### Paravirtualization Drivers
- [ ] Virtio-blk
- [ ] Virtio-net
- [ ] Virtio-console
- [ ] Virtio-balloon
- [ ] Virtio-gpu
- [ ] VMware tools
- [ ] VirtualBox guest additions
- [ ] Hyper-V integration

---

## POWER MANAGEMENT

### System States
- [ ] Suspend to RAM (S3)
- [ ] Suspend to disk (S4/hibernate)
- [ ] Soft power off (S5)
- [ ] Hybrid sleep

### CPU Power
- [ ] CPU frequency scaling
- [ ] CPU idle states (C-states)
- [ ] Turbo boost control

### Device Power
- [ ] Runtime PM
- [ ] Wake-on-LAN
- [ ] USB selective suspend

---

## PORTABILITY & ARCHITECTURE SUPPORT

### x86-64 (Primary)
- [ ] Full x86-64 support
- [ ] x86-32 compatibility (optional)

### ARM64/AArch64 (Optional)
- [ ] ARM64 boot
- [ ] Device tree parsing
- [ ] GIC (interrupt controller)
- [ ] ARM-specific drivers

### RISC-V (Optional)
- [ ] RISC-V boot
- [ ] PLIC (interrupt controller)
- [ ] RISC-V specific features

---

## DOCUMENTATION

### User Documentation
- [ ] Installation guide
- [ ] User manual
- [ ] FAQ

### Developer Documentation
- [ ] Kernel architecture overview
- [ ] API documentation
- [ ] Driver development guide
- [ ] Contributing guide

### Man Pages
- [ ] System call man pages (section 2)
- [ ] Library function man pages (section 3)
- [ ] Command man pages (section 1)

---

## BUILD SYSTEM & TOOLING

### Build System
- [ ] Cross-compiler setup (GCC/Clang)
- [x] Makefile/build system
- [ ] Dependency tracking
- [ ] Incremental builds
- [ ] Build configuration (Kconfig-style)
- [ ] Out-of-tree module support

### Testing Infrastructure
- [ ] CI/CD pipeline
- [ ] Automated testing
- [ ] Code coverage
- [ ] Static analysis

### Distribution
- [ ] ISO image generation
- [ ] USB bootable image
- [ ] Installation program
- [ ] Package repository

---

## COMPLIANCE & STANDARDS

### POSIX Compliance
- [ ] POSIX.1 system calls
- [ ] POSIX.1b real-time extensions
- [ ] POSIX threads (pthreads)

### Hardware Standards
- [ ] ACPI compliance
- [ ] UEFI compliance
- [ ] PCI/PCIe compliance
- [ ] USB compliance

### File System Standards
- [ ] Filesystem Hierarchy Standard (FHS)

---

## Application Programming Library

### Foundation
- [ ] Structure
- [ ] create Modules and functions (w/o implementations)
- [ ] create main renderer
  - [ ] call render funcs of all elements to get rendered pixel for each element
  - [ ] overlay pixel based on the order (z-index) of elements and respecting positions
  - [ ] send the rendered pixels to OS

### Lib
- [ ] create GUI elements (not implemented)
- [ ] implement window element
- [ ] implement most important GUI Elements: text, titles, buttons, checkboxes, inputfields, boxes (divs)
- [ ] implement advanced styling and config possibilities
- [ ] implement way more GUI elements
- [ ] define and implement helper functions for user and lib

## MILESTONES

1.  **Boot**: ~~Bootloader loads kernel, prints "Hello World"~~ ✅
2.  **Interrupts**: GDT, IDT, PIC working, timer interrupt firing
3.  **Memory**: Physical and virtual memory management complete
4.  **Storage**: Can read/write to disk (ATA/AHCI)
5.  **File System**: VFS layer with ext2/FAT support
6.  **Processes**: Process creation, scheduling, context switching
7.  **User Space**: User mode, system calls, ELF loading
8.  **Shell**: Working command-line shell
9.  **Network**: TCP/IP stack with basic connectivity
10. **GUI**: Basic windowing system
11. **Application Programming Library**: Rust based lib for pogramming apps for Nafuraito
12. **Self-Hosting**: Can compile itself on itself
