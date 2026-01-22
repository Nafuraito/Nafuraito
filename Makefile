# Nafuraito Operating System - Main Build Configuration

BOOTLOADER_DIR = src/bootloader
KERNEL_DIR = src/kernel
BUILD_DIR = build

# Disk image layout:
# - Sector 0: Stage 1 bootloader (MBR, 512 bytes)
# - Sectors 1-20: Stage 2 bootloader (~10KB)
# - Sectors 64+: EXT4 filesystem with journaling containing /nafuraito/kernel.bin

# EXT4 partition configuration
EXT4_START_SECTOR = 64
EXT4_SIZE_MB = 30
DISK_SIZE_MB = 32

.PHONY: all bootloader kernel disk_image run clean

all: bootloader kernel disk_image

# Build bootloader (Stage 1 + Stage 2)
bootloader:
	$(MAKE) -C $(BOOTLOADER_DIR)

# Build kernel
kernel:
	$(MAKE) -C $(KERNEL_DIR)

# Create disk image with bootloader and ext4 filesystem containing kernel
disk_image: bootloader kernel
	@mkdir -p $(BUILD_DIR)
	@echo "=========================================="
	@echo "Creating disk image with ext4 filesystem..."
	@echo "=========================================="
	
	# Step 1: Create empty disk image (32MB)
	dd if=/dev/zero of=$(BUILD_DIR)/disk.img bs=1M count=$(DISK_SIZE_MB) 2>/dev/null
	
	# Step 2: Write Stage 1 bootloader (MBR) at sector 0
	dd if=$(BUILD_DIR)/stage1.bin of=$(BUILD_DIR)/disk.img bs=512 seek=0 conv=notrunc 2>/dev/null
	
	# Step 3: Write Stage 2 bootloader starting at sector 1
	dd if=$(BUILD_DIR)/stage2.bin of=$(BUILD_DIR)/disk.img bs=512 seek=1 conv=notrunc 2>/dev/null
	
	# Step 4: Create ext4 filesystem image (separate file first)
	@echo "Creating ext4 filesystem with journaling..."
	dd if=/dev/zero of=$(BUILD_DIR)/ext4.img bs=1M count=$(EXT4_SIZE_MB) 2>/dev/null
	mkfs.ext4 -F -O has_journal -L "NAFURAITO" $(BUILD_DIR)/ext4.img >/dev/null 2>&1
	
	# Step 5: Mount ext4 filesystem and copy kernel
	@echo "Mounting ext4 filesystem and installing kernel..."
	@mkdir -p $(BUILD_DIR)/mnt
	sudo mount -o loop $(BUILD_DIR)/ext4.img $(BUILD_DIR)/mnt
	sudo mkdir -p $(BUILD_DIR)/mnt/nafuraito
	sudo cp $(BUILD_DIR)/kernel.bin $(BUILD_DIR)/mnt/nafuraito/kernel.bin
	sudo chmod 644 $(BUILD_DIR)/mnt/nafuraito/kernel.bin
	@echo "Kernel installed at /nafuraito/kernel.bin"
	@ls -la $(BUILD_DIR)/mnt/nafuraito/
	sudo umount $(BUILD_DIR)/mnt
	@rmdir $(BUILD_DIR)/mnt
	
	# Step 6: Write ext4 filesystem to disk image at the partition offset
	dd if=$(BUILD_DIR)/ext4.img of=$(BUILD_DIR)/disk.img bs=512 seek=$(EXT4_START_SECTOR) conv=notrunc 2>/dev/null
	
	# Step 7: Clean up temporary ext4 image
	rm -f $(BUILD_DIR)/ext4.img
	
	@echo "=========================================="
	@echo "Disk image created: $(BUILD_DIR)/disk.img"
	@echo "  - Stage 1: sector 0"
	@echo "  - Stage 2: sectors 1-20"
	@echo "  - EXT4 partition: sector $(EXT4_START_SECTOR)+"
	@echo "  - Kernel at: /nafuraito/kernel.bin"
	@echo "=========================================="

# Run in QEMU
run: all
	# Run with 512MB of RAM
	qemu-system-x86_64 -m 512M -drive file=$(BUILD_DIR)/disk.img,format=raw -boot c -enable-kvm -smp 1 -net nic -net user

# Run with debug output
run-debug: all
	qemu-system-x86_64 -m 1024 -drive file=$(BUILD_DIR)/disk.img,format=raw -boot c \
		-serial stdio -d int,cpu_reset -no-reboot -no-shutdown

# Run without KVM (for systems without hardware virtualization)
run-nokvm: all
	qemu-system-x86_64 -m 512M -drive file=$(BUILD_DIR)/disk.img,format=raw -boot c -smp 1 -net nic -net user

clean:
	rm -rf $(BUILD_DIR)
