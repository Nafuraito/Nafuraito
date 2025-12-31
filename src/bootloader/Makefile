ASMFLAGS=-f elf32
ASM=nasm
LD=ld
BUILD_DIR=../../build

all: stage1 stage2

stage1: always
	$(ASM) ./stage1/boot.asm -f bin -o $(BUILD_DIR)/stage1.bin

stage2: always
	$(ASM) -f elf32 stage2/main.asm -o $(BUILD_DIR)/stage2/main.o
	$(ASM) -f elf32 stage2/kernel_loader.asm -o $(BUILD_DIR)/stage2/kernel_loader.o
	$(LD) -m elf_i386 -T linker.ld --oformat binary $(BUILD_DIR)/stage2/main.o $(BUILD_DIR)/stage2/kernel_loader.o -o $(BUILD_DIR)/stage2.bin

always:
	mkdir -p $(BUILD_DIR)
	mkdir -p $(BUILD_DIR)/stage2
