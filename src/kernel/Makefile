RUSTC=rustc --emit obj -C panic=abort --target i686-unknown-linux-gnu -C code-model=kernel -C relocation-model=static
ASM=nasm
LD=ld
CC=gcc -m32 -nostdlib -I./ -c
ASMFLAGS=-f obj

BUILD_DIR=../../build

.PHONY: all always entry drivers kernel binary

all: binary

drivers:
	$(CC) drivers/video/video.c -o $(BUILD_DIR)/kernel/driver_video.o

entry: always
	$(ASM) ./entry.asm -f elf32 -o $(BUILD_DIR)/kernel/entry.o

kernel: always
	$(RUSTC) entry.rs -o $(BUILD_DIR)/kernel/entry_rs.o

KERNEL_OBJS=$(wildcard $(BUILD_DIR)/kernel/*.o)

binary: always entry kernel drivers
	$(LD) -m elf_i386 $(KERNEL_OBJS) -o $(BUILD_DIR)/kernel.bin

always:
	mkdir -p $(BUILD_DIR)/kernel
