# Nafuraito Kernel Build Configuration
# Builds all kernel libraries and links them into kernel.bin

RUSTC = rustc --emit obj -C panic=abort --target x86_64-unknown-linux-gnu \
        -C code-model=kernel -C relocation-model=static \
        -C opt-level=2 -C overflow-checks=no -C debug-assertions=no 

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

# Libraries
LIBVIDEO = $(BUILD_DIR)/drivers/video/libvideo.a
LIBPIC = $(BUILD_DIR)/drivers/pic/libpic.a
LIBGDT = $(BUILD_DIR)/gdt/libgdt.a
LIBIDT = $(BUILD_DIR)/idt/libidt.a
LIBMEMORY = $(BUILD_DIR)/memory/libmemory.a

ALL_LIBS = $(LIBVIDEO) $(LIBGDT) $(LIBIDT)

.PHONY: all clean libs drivers gdt idt memory

all: $(BUILD_DIR) libs kernel.bin

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

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

# Compile kernel entry assembly and main Rust file
kernel.bin: libs compile
	$(LD) -m elf_x86_64 -T linker.ld \
		$(BUILD_DIR)/entry.o \
		$(BUILD_DIR)/main.o \
		$(BUILD_DIR)/drivers/video/vga.o \
		$(BUILD_DIR)/gdt/gdt.o \
		$(BUILD_DIR)/idt/idt.o \
		-o $(ROOT_BUILD_DIR)/kernel.bin

compile: $(BUILD_DIR)
	$(ASM) entry.asm -f elf64 -o $(BUILD_DIR)/entry.o
	$(RUSTC) main.rs -o $(BUILD_DIR)/main.o

clean:
	rm -rf $(BUILD_DIR)
	$(MAKE) -C $(DRIVERS_DIR) clean
	$(MAKE) -C $(GDT_DIR) clean
	$(MAKE) -C $(IDT_DIR) clean
	$(MAKE) -C $(MEMORY_DIR) clean

