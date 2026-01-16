# Nafuraito Kernel Build Configuration

RUSTC = rustc --emit obj -C panic=abort --target x86_64-unknown-linux-gnu \
        -C code-model=kernel -C relocation-model=static \
        -C opt-level=2 -C overflow-checks=no -C debug-assertions=no 

ASM = nasm
LD = ld
CC = gcc -m64 -nostdlib -fno-stack-protector -ffreestanding -c -Wall -Werror -Wpedantic

BUILD_DIR = ../../build/kernel

.PHONY: all clean

all: $(BUILD_DIR) kernel.bin

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

# Note: pic.rs is included as a module in entry.rs, so it's compiled together
kernel.bin: compile
	$(LD) -m elf_x86_64 -T linker.ld \
		$(BUILD_DIR)/entry.o \
		$(BUILD_DIR)/entry_rs.o \
		$(BUILD_DIR)/vga.o \
		$(BUILD_DIR)/gdt.o \
		$(BUILD_DIR)/idt.o \
		-o ../../build/kernel.bin

compile: $(BUILD_DIR)
	$(ASM) entry.asm -f elf64 -o $(BUILD_DIR)/entry.o

	$(RUSTC) entry.rs -o $(BUILD_DIR)/entry_rs.o

	$(CC) drivers/video/vga.c -o $(BUILD_DIR)/vga.o

	$(ASM) gdt/gdt.asm -f elf64 -o $(BUILD_DIR)/gdt.o

	$(ASM) idt/idt.asm -f elf64 -o $(BUILD_DIR)/idt.o

clean:
	rm -rf $(BUILD_DIR)
