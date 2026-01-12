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

kernel.bin: entry.o entry_rs.o vga.o gdt.o idt.o
	$(LD) -m elf_x86_64 -T linker.ld \
		$(BUILD_DIR)/entry.o \
		$(BUILD_DIR)/entry_rs.o \
		$(BUILD_DIR)/vga.o \
		$(BUILD_DIR)/gdt.o \
		$(BUILD_DIR)/idt.o \
		-o ../../build/kernel.bin

entry.o: $(BUILD_DIR)
	$(ASM) entry.asm -f elf64 -o $(BUILD_DIR)/entry.o

entry_rs.o: $(BUILD_DIR)
	$(RUSTC) entry.rs -o $(BUILD_DIR)/entry_rs.o

vga.o: $(BUILD_DIR)
	$(CC) drivers/video/vga.c -o $(BUILD_DIR)/vga.o

gdt.o: $(BUILD_DIR)
	$(ASM) gdt/gdt.asm -f elf64 -o $(BUILD_DIR)/gdt.o

idt.o: $(BUILD_DIR)
	$(ASM) idt/idt.asm -f elf64 -o $(BUILD_DIR)/idt.o

clean:
	rm -rf $(BUILD_DIR)
