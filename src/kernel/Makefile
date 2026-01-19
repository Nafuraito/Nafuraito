# Nafuraito Kernel Build Configuration
# Builds all kernel libraries and links them into kernel.bin

RUSTC = rustc --emit obj -C panic=abort --target x86_64-unknown-linux-gnu \
        -C code-model=kernel -C relocation-model=static \
        -C opt-level=2 -C overflow-checks=no -C debug-assertions=no \
        -C target-feature=-sse,-sse2,-sse3,-ssse3,-sse4.1,-sse4.2,-avx,-avx2 

ASM = nasm
LD = ld
CC = gcc -m64 -nostdlib -fno-stack-protector -ffreestanding -c -Wall -Werror -Wpedantic

BUILD_DIR = ../../build/kernel
ROOT_BUILD_DIR = ../../build

# Library directories
DRIVERS_DIR = drivers
GDT_DIR = gdt
IDT_DIR = idt
MEMORY_DIR = memory

# Libraries (static archives)
LIBVIDEO = $(BUILD_DIR)/drivers/video/libvideo.a
LIBPIC = $(BUILD_DIR)/drivers/pic/libpic.a
LIBPCI = $(BUILD_DIR)/drivers/pci/libpci.a
LIBGDT = $(BUILD_DIR)/gdt/libgdt.a
LIBIDT = $(BUILD_DIR)/idt/libidt.a
LIBMEMORY = $(BUILD_DIR)/memory/libmemory.a

ALL_LIBS = $(LIBVIDEO) $(LIBPIC) $(LIBPCI) $(LIBGDT) $(LIBIDT) $(LIBMEMORY)

.PHONY: all clean libs drivers gdt idt memory compile

all: libs compile

# Build all libraries
libs: drivers gdt idt memory

drivers:
	$(MAKE) -C $(DRIVERS_DIR)

gdt:
	$(MAKE) -C $(GDT_DIR)

idt:
	$(MAKE) -C $(IDT_DIR)

memory:
	$(MAKE) -C $(MEMORY_DIR)

# Compile kernel entry and main, then link
compile: libs
	@mkdir -p $(BUILD_DIR)
	$(ASM) entry.asm -f elf64 -o $(BUILD_DIR)/entry.o
	$(RUSTC) main.rs -o $(BUILD_DIR)/main.o
	$(LD) -m elf_x86_64 -T linker.ld \
		$(BUILD_DIR)/entry.o \
		$(BUILD_DIR)/main.o \
		$(BUILD_DIR)/drivers/video/vga.o \
		$(LIBGDT) \
		$(LIBIDT) \
		$(LIBPIC) \
		$(LIBPCI) \
		$(LIBMEMORY) \
		-o $(ROOT_BUILD_DIR)/kernel.bin

clean:
	rm -rf $(BUILD_DIR)
	$(MAKE) -C $(DRIVERS_DIR) clean
	$(MAKE) -C $(GDT_DIR) clean
	$(MAKE) -C $(IDT_DIR) clean
	$(MAKE) -C $(MEMORY_DIR) clean

