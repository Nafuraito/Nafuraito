# Nafuraito Bootloader Build Configuration

ASM = nasm
LD = ld

BUILD_DIR = ../../build

.PHONY: all clean

all: stage1 stage2

stage1: $(BUILD_DIR)
	$(ASM) stage1/boot.asm -f bin -o $(BUILD_DIR)/stage1.bin

stage2: $(BUILD_DIR)/stage2
	$(ASM) -f elf64 stage2/main.asm -o $(BUILD_DIR)/stage2/main.o
	$(ASM) -f elf64 stage2/kernel_loader_64_bit.asm -o $(BUILD_DIR)/stage2/kernel_loader.o
	$(LD) -m elf_x86_64 -T linker.ld --oformat binary \
		$(BUILD_DIR)/stage2/main.o \
		$(BUILD_DIR)/stage2/kernel_loader.o \
		-o $(BUILD_DIR)/stage2.bin

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/stage2:
	mkdir -p $(BUILD_DIR)/stage2

clean:
	rm -rf $(BUILD_DIR)/stage1.bin $(BUILD_DIR)/stage2.bin $(BUILD_DIR)/stage2
