# Nafuraito Operating System - Main Build Configuration

BOOTLOADER_DIR = bootloader
KERNEL_DIR = kernel
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
	@echo "Creating bootable disk image with ext4 filesystem..."
	# Create empty disk image (128MB)
	dd if=/dev/zero of=$(BUILD_DIR)/disk.img bs=1M count=128 2>/dev/null

	# Create partition table FIRST (before writing bootloader, as parted mklabel overwrites MBR)
	parted -s $(BUILD_DIR)/disk.img mklabel msdos
	parted -s $(BUILD_DIR)/disk.img mkpart primary ext4 1MiB 100%
	parted -s $(BUILD_DIR)/disk.img set 1 boot on

	# Save the partition table (bytes 446-511)
	dd if=$(BUILD_DIR)/disk.img of=$(BUILD_DIR)/partition_table.tmp bs=1 skip=446 count=66 2>/dev/null
	# Write stage1 bootloader to first sector (overwrites everything including boot sig)
	dd if=$(BUILD_DIR)/stage1.bin of=$(BUILD_DIR)/disk.img bs=512 count=1 conv=notrunc 2>/dev/null
	# Restore the partition table (bytes 446-509, keeping our boot signature at 510-511)
	dd if=$(BUILD_DIR)/partition_table.tmp of=$(BUILD_DIR)/disk.img bs=1 seek=446 count=64 conv=notrunc 2>/dev/null
	rm -f $(BUILD_DIR)/partition_table.tmp
	# Write stage2 bootloader starting at second sector (sector 1, offset 512 bytes)
	dd if=$(BUILD_DIR)/stage2.bin of=$(BUILD_DIR)/disk.img bs=512 seek=1 conv=notrunc 2>/dev/null

	# Setup loop device for partition
	$(eval LOOP_DEV := $(shell sudo losetup -f))
	sudo losetup -P $(LOOP_DEV) $(BUILD_DIR)/disk.img

	# Create ext4 filesystem with journaling
	sudo mkfs.ext4 -j $(LOOP_DEV)p1
	
	# Mount and copy kernel
	sudo mkdir -p /mnt/nafuraito_build
	sudo mount $(LOOP_DEV)p1 /mnt/nafuraito_build
	sudo mkdir -p /mnt/nafuraito_build/nafuraito
	sudo cp $(BUILD_DIR)/kernel.bin /mnt/nafuraito_build/nafuraito/
	sudo umount /mnt/nafuraito_build
	sudo losetup -d $(LOOP_DEV)
	@echo "Bootable disk image with ext4 created: $(BUILD_DIR)/disk.img"

# Run in QEMU
run: all
# run with 1 gig of RAM  
	qemu-system-x86_64 -m 1024M -drive file=$(BUILD_DIR)/disk.img,format=raw -boot c -enable-kvm -smp 1 -net nic -net user

# Run with debug output
run-debug: all
	qemu-system-x86_64 -m 1024M -drive file=$(BUILD_DIR)/disk.img,format=raw -boot c -enable-kvm -smp 1 -net nic -net user -serial stdio -display none

clean:
	rm -rf $(BUILD_DIR)
