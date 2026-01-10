# Nafuraito Operating System - Main Build Configuration

BOOTLOADER_DIR = src/bootloader
KERNEL_DIR = src/kernel
BUILD_DIR = build

.PHONY: all bootloader kernel disk_image run clean

all: bootloader kernel disk_image

# Build bootloader (Stage 1 + Stage 2)
bootloader:
	$(MAKE) -C $(BOOTLOADER_DIR)

# Build kernel
kernel:
	$(MAKE) -C $(KERNEL_DIR)

# Create disk image with bootloader and kernel
disk_image: bootloader kernel
	@mkdir -p $(BUILD_DIR)
	@echo "Creating disk image..."
	dd if=/dev/zero of=$(BUILD_DIR)/disk.img bs=1024 count=1634 2>/dev/null
	dd if=$(BUILD_DIR)/stage1.bin of=$(BUILD_DIR)/disk.img bs=512 seek=0 conv=notrunc 2>/dev/null
	dd if=$(BUILD_DIR)/stage2.bin of=$(BUILD_DIR)/disk.img bs=512 seek=1 conv=notrunc 2>/dev/null
	dd if=$(BUILD_DIR)/kernel.bin of=$(BUILD_DIR)/disk.img bs=512 seek=64 conv=notrunc 2>/dev/null
	@echo "Disk image created: $(BUILD_DIR)/disk.img"

# Run in QEMU
run: all
# run with 1 gig of RAM  
	qemu-system-x86_64 -m 1024 -drive file=$(BUILD_DIR)/disk.img,format=raw -boot c -enable-kvm -smp 1 -net nic -net user

# Run with debug output
run-debug: all
	qemu-system-x86_64 -m 1024 -drive file=$(BUILD_DIR)/disk.img,format=raw -boot c \
		-serial stdio -d int,cpu_reset -no-reboot -no-shutdown

clean:
	rm -rf $(BUILD_DIR)
