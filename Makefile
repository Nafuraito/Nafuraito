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
	
	# Write stage2 bootloader immediately after MBR (sector 1)
	# Stage1 loads this to 0x7E00 in memory
	dd if=build/stage2.bin of=build/disk.img bs=512 seek=1 conv=notrunc
	
	# Write kernel.bin at sector 64 (raw, no filesystem) for easy loading
	# This allows bootloader to load kernel without ext4 driver
	dd if=build/kernel.bin of=build/disk.img bs=512 seek=64 conv=notrunc

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
