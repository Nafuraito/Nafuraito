BOOTLOADER_DIR=src/bootloader/
KERNEL_DIR=src/kernel/
USER_INTERFACE_DIR=src/UserInterface

.PHONY: always all

all: bootloader kernel disk_image

#
#	Create a disk Image with proper partitioning
#
disk_image: bootloader kernel always
	dd if=/dev/zero of=build/disk.img bs=1024 count=1634
	
	# Write stage1 bootloader to MBR (sector 0)
	dd if=build/stage1.bin of=build/disk.img bs=512 seek=0 conv=notrunc
	
	# Write stage2 bootloader at fixed location 0x7E00 (sector 31.75, so we use sector 32)
	# 0x7E00 = 32256 bytes = sector 63 (512 byte sectors)
	dd if=build/stage2.bin of=build/disk.img bs=512 seek=63 conv=notrunc
	
	# Create partition table starting after stage2 (sector 2048 to be safe)
	echo -e "o\nn\np\n1\n2048\n\nw" | fdisk build/disk.img
	
	# Set up loop device and create filesystem for kernel and other files
	sudo losetup -D
	LOOP_DEV=$$(sudo losetup --show -f build/disk.img); \
	echo "Loop device: $$LOOP_DEV"; \
	sudo partprobe $$LOOP_DEV; \
	if [ -b $${LOOP_DEV}p1 ]; then \
		sudo mkfs.ext4 $${LOOP_DEV}p1; \
		mkdir -p /tmp/disk_mount; \
		sudo mount $${LOOP_DEV}p1 /tmp/disk_mount; \
		sudo cp build/kernel.bin /tmp/disk_mount/kernel.bin; \
		sudo umount /tmp/disk_mount; \
		rmdir /tmp/disk_mount; \
		sudo losetup -d $$LOOP_DEV; \
	else \
		echo "Partition device not found, using offset mount"; \
		mkdir -p /tmp/disk_mount; \
		sudo losetup -d $$LOOP_DEV; \
		PART_LOOP=$$(sudo losetup --show -f -o 1048576 build/disk.img); \
		sudo mkfs.ext4 $$PART_LOOP; \
		sudo mount $$PART_LOOP /tmp/disk_mount; \
		sudo cp build/kernel.bin /tmp/disk_mount/kernel.bin; \
		sudo umount /tmp/disk_mount; \
		sudo losetup -d $$PART_LOOP; \
		rmdir /tmp/disk_mount; \
	fi

bootloader:
	$(MAKE) -C $(BOOTLOADER_DIR)

kernel:
	$(MAKE) -C $(KERNEL_DIR)

always:
	mkdir -p build
	mkdir -p build/stage2

run: all
# run with 4 gigs of RAM  
	qemu-system-x86_64 -m 2048 -drive file=build/disk.img,format=raw -boot c -enable-kvm -smp 1 -net nic -net user

run-debug: all
# Debug mode with serial output and CPU trace
	qemu-system-x86_64 -m 2048 -drive file=build/disk.img,format=raw -boot c \
		-serial stdio -d int,cpu_reset -no-reboot -no-shutdown

clean:
	rm -rf build
	sudo losetup -D
