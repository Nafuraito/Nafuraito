RUSTC=rustc --emit obj -C panic=abort --target x86_64-unknown-linux-gnu -C code-model=kernel -C relocation-model=static
ASM=nasm
LD=ld
CC=gcc -m64 -nostdlib -I./ -c
ASMFLAGS=-f obj

BUILD_DIR=../../build

.PHONY: all always entry drivers kernel binary

all: binary

drivers:
	$(CC) drivers/video/vga.c -o $(BUILD_DIR)/kernel/driver_video_vga.o

entry: always
	$(ASM) ./entry.asm -f elf64 -o $(BUILD_DIR)/kernel/entry.o

kernel: always
	$(RUSTC) entry.rs -o $(BUILD_DIR)/kernel/entry_rs.o

KERNEL_OBJS=$(wildcard $(BUILD_DIR)/kernel/*.o)

binary: always entry kernel drivers
	$(LD) -m elf_x86_64 -T linker.ld $(BUILD_DIR)/kernel/entry.o $(BUILD_DIR)/kernel/entry_rs.o $(BUILD_DIR)/kernel/driver_video_vga.o -o $(BUILD_DIR)/kernel.bin

always:
	mkdir -p $(BUILD_DIR)/kernel
