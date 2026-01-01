ASMFLAGS=-f elf32
ASM=nasm
LD=ld
BUILD_DIR=../../build

all64bit: stage1 stage2_64

stage1: always
	$(ASM) ./stage1/boot.asm -f bin -o $(BUILD_DIR)/stage1.bin

stage2_64: always
	$(ASM) -f elf64 stage2/main.asm -o $(BUILD_DIR)/stage2/main.o
	$(ASM) -f elf64 stage2/kernel_loader_64_bit.asm -o $(BUILD_DIR)/stage2/kernel_loader.o
	$(LD) -m elf_x86_64 -T linker.ld --oformat binary $(BUILD_DIR)/stage2/main.o $(BUILD_DIR)/stage2/kernel_loader.o -o $(BUILD_DIR)/stage2.bin

always:
	mkdir -p $(BUILD_DIR)
	mkdir -p $(BUILD_DIR)/stage2
