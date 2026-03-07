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

# Create a hybrid BIOS + UEFI bootable disk image.
#
# Partition layout (128 MiB total):
#   [  0 B –   1 MiB ]  Unpartitioned gap: MBR (stage1) + stage2 sectors
#   [  1 MiB –  65 MiB ]  Partition 1: FAT32 ESP — UEFI boot path
#                           EFI/BOOT/BOOTX64.EFI  (if gnu-efi is installed)
#                           nafuraito/kernel.bin
#   [ 65 MiB – 128 MiB ]  Partition 2: ext4 — BIOS/MBR boot path
#                           /nafuraito/kernel.bin
#
# LBA offsets (512-byte sectors):
#   Partition 1 (ESP)  starts at LBA   2048  (1 MiB)
#   Partition 2 (ext4) starts at LBA 133120  (65 MiB)  ← EXT4_PARTITION_LBA
#
# No loop devices, no mount, no modprobe required:
#   FAT32 population uses mtools  (mformat / mmd / mcopy)
#   ext4  population uses debugfs (part of e2fsprogs)
#
# The recipe is one shell script joined with \ so that $$ shell variables
# survive across lines and "set -e" aborts on the first error.
disk_image: bootloader kernel
	@mkdir -p $(BUILD_DIR)
	@set -e; \
	\
	echo "  [1/7] Creating blank 128 MiB disk image..."; \
	dd if=/dev/zero of=$(BUILD_DIR)/disk.img bs=1M count=128 2>/dev/null; \
	\
	echo "  [2/7] Writing partition table (ESP at 1-65 MiB, ext4 at 65-128 MiB)..."; \
	parted -s $(BUILD_DIR)/disk.img mklabel msdos; \
	parted -s $(BUILD_DIR)/disk.img mkpart primary fat32  1MiB  65MiB; \
	parted -s $(BUILD_DIR)/disk.img mkpart primary ext4  65MiB 100%; \
	\
	echo "  [3/7] Writing bootloaders (preserving partition table)..."; \
	dd if=$(BUILD_DIR)/disk.img of=$(BUILD_DIR)/partition_table.tmp \
	   bs=1 skip=446 count=66 2>/dev/null; \
	dd if=$(BUILD_DIR)/stage1.bin of=$(BUILD_DIR)/disk.img \
	   bs=512 count=1 conv=notrunc 2>/dev/null; \
	dd if=$(BUILD_DIR)/partition_table.tmp of=$(BUILD_DIR)/disk.img \
	   bs=1 seek=446 count=64 conv=notrunc 2>/dev/null; \
	rm -f $(BUILD_DIR)/partition_table.tmp; \
	dd if=$(BUILD_DIR)/stage2.bin of=$(BUILD_DIR)/disk.img \
	   bs=512 seek=1 conv=notrunc 2>/dev/null; \
	\
	echo "  [4/7] Creating FAT32 ESP image (64 MiB) with mtools..."; \
	dd if=/dev/zero of=$(BUILD_DIR)/esp.img bs=1M count=64 2>/dev/null; \
	mformat -i $(BUILD_DIR)/esp.img -F -v "NAFURAITO" ::; \
	mmd -i $(BUILD_DIR)/esp.img ::/EFI; \
	mmd -i $(BUILD_DIR)/esp.img ::/EFI/BOOT; \
	mmd -i $(BUILD_DIR)/esp.img ::/nafuraito; \
	mcopy -i $(BUILD_DIR)/esp.img $(BUILD_DIR)/kernel.bin ::/nafuraito/kernel.bin; \
	if [ -f $(BUILD_DIR)/uefi/BOOTX64.EFI ]; then \
		mcopy -i $(BUILD_DIR)/esp.img $(BUILD_DIR)/uefi/BOOTX64.EFI \
		      ::/EFI/BOOT/BOOTX64.EFI; \
		echo "  [4/7] BOOTX64.EFI embedded in ESP."; \
	else \
		echo "  [4/7] NOTE: $(BUILD_DIR)/uefi/BOOTX64.EFI not found."; \
		echo "             ESP created without UEFI bootloader."; \
		echo "             Install gnu-efi and rebuild to enable UEFI boot."; \
	fi; \
	\
	echo "  [5/7] Embedding ESP into disk image at 1 MiB offset..."; \
	dd if=$(BUILD_DIR)/esp.img of=$(BUILD_DIR)/disk.img \
	   bs=1M seek=1 conv=notrunc 2>/dev/null; \
	rm -f $(BUILD_DIR)/esp.img; \
	\
	echo "  [6/7] Creating ext4 partition image (63 MiB) for BIOS boot..."; \
	dd if=/dev/zero of=$(BUILD_DIR)/partition.img bs=1M count=63 2>/dev/null; \
	mkfs.ext4 -j $(BUILD_DIR)/partition.img; \
	debugfs -w $(BUILD_DIR)/partition.img -R "mkdir nafuraito"; \
	debugfs -w $(BUILD_DIR)/partition.img -R \
	     "write $(BUILD_DIR)/kernel.bin nafuraito/kernel.bin"; \
	\
	echo "  [7/7] Embedding ext4 partition into disk image at 65 MiB offset..."; \
	dd if=$(BUILD_DIR)/partition.img of=$(BUILD_DIR)/disk.img \
	   bs=1M seek=65 conv=notrunc 2>/dev/null; \
	rm -f $(BUILD_DIR)/partition.img; \
	\
	echo "Hybrid BIOS+UEFI disk image created: $(BUILD_DIR)/disk.img"; \
	\
	cp /usr/share/edk2/x64/OVMF_VARS.4m.fd $(BUILD_DIR)/OVMF_VARS.4m.fd; \
	echo "OVMF VARS copied to $(BUILD_DIR)/OVMF_VARS.4m.fd (writable UEFI NVRAM)"

# Run in QEMU with UEFI firmware (OVMF).
# Uses two pflash drives: CODE (read-only firmware) + VARS (read-write NVRAM).
# Serial output is captured to build/serial.log (silent in terminal).
# To fall back to legacy BIOS boot, replace the two -drive pflash lines with:
#   -boot c
run: all
	qemu-system-x86_64 \
		-m 1024M \
		-drive if=pflash,format=raw,readonly=on,file=/usr/share/edk2/x64/OVMF_CODE.4m.fd \
		-drive if=pflash,format=raw,file=$(BUILD_DIR)/OVMF_VARS.4m.fd \
		-drive file=$(BUILD_DIR)/disk.img,format=raw \
		-enable-kvm -smp 1 -net nic -net user \
		-serial file:$(BUILD_DIR)/serial.log

# Run with debug output (UEFI + serial console) — serial logged to terminal
run-debug: all
	qemu-system-x86_64 \
		-m 1024M \
		-drive if=pflash,format=raw,readonly=on,file=/usr/share/edk2/x64/OVMF_CODE.4m.fd \
		-drive if=pflash,format=raw,file=$(BUILD_DIR)/OVMF_VARS.4m.fd \
		-drive file=$(BUILD_DIR)/disk.img,format=raw \
		-enable-kvm -smp 1 -net nic -net user -serial stdio -display none

clean:
	rm -rf $(BUILD_DIR)
