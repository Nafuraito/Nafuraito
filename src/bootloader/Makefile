# Nafuraito Bootloader Build Configuration

ASM = nasm
CC = gcc
LD = ld

BUILD_DIR = ../../build

# C compiler flags for freestanding environment
CFLAGS = -ffreestanding -nostdlib -fno-builtin -fno-stack-protector \
         -mno-red-zone -mcmodel=kernel -mno-sse -mno-sse2 -fno-pic -fno-pie \
         -O2 -Wall -Wextra -Wno-unused-parameter -Werror -Wpedantic

.PHONY: all clean stage1 stage2

all: stage1 stage2

stage1: $(BUILD_DIR)
	$(ASM) stage1/boot.asm -f bin -o $(BUILD_DIR)/stage1.bin

stage2: $(BUILD_DIR)/stage2
	$(ASM) -f elf64 stage2/bridge.asm -o $(BUILD_DIR)/stage2/bridge.o
	$(CC) $(CFLAGS) -c stage2/kernel_loader.c -o $(BUILD_DIR)/stage2/kernel_loader.o
	$(CC) $(CFLAGS) -c stage2/ext4_boot.c -o $(BUILD_DIR)/stage2/ext4_boot.o
	$(LD) -m elf_x86_64 -T linker.ld --oformat binary \
		$(BUILD_DIR)/stage2/bridge.o \
		$(BUILD_DIR)/stage2/kernel_loader.o \
		$(BUILD_DIR)/stage2/ext4_boot.o \
		-o $(BUILD_DIR)/stage2.bin


$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/stage2:
	mkdir -p $(BUILD_DIR)/stage2

clean:
	rm -rf $(BUILD_DIR)/stage1.bin $(BUILD_DIR)/stage2.bin $(BUILD_DIR)/stage2
