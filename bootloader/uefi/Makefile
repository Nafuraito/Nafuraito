# Build an EFI application using gnu-efi.
# Install: pacman -S gnu-efi  |  apt install gnu-efi  |  dnf install gnu-efi-devel

ARCH        = x86_64
EFI_INCLUDE = /usr/include/efi
EFI_LIBS    = /usr/lib
GNUEFI_LIBS = $(EFI_LIBS)/crt0-efi-$(ARCH).o

CC      = gcc
LD      = ld
OBJCOPY = objcopy

CFLAGS  = -I$(EFI_INCLUDE) -I$(EFI_INCLUDE)/$(ARCH) -I$(EFI_INCLUDE)/protocol \
          -I../include \
          -fno-stack-protector -fpic -fshort-wchar -mno-red-zone \
          -Wall -DEFI_FUNCTION_WRAPPER

LDFLAGS = -nostdlib -znocombreloc -T $(EFI_LIBS)/elf_$(ARCH)_efi.lds \
          -shared -Bsymbolic $(GNUEFI_LIBS)

BUILD_DIR = ../../build/uefi

# Guard: skip the entire build when gnu-efi headers are absent rather than
# failing with a cryptic error.  The disk image target handles the missing
# BOOTX64.EFI gracefully (creates the ESP without it and prints a notice).
HAS_GNUEFI := $(shell test -d $(EFI_INCLUDE) && echo yes || echo no)

.PHONY: all clean

all:
ifeq ($(HAS_GNUEFI),yes)
	@$(MAKE) --no-print-directory $(BUILD_DIR)/BOOTX64.EFI
else
	@echo "  [UEFI] gnu-efi not found at $(EFI_INCLUDE) — skipping UEFI build."
	@echo "         Install gnu-efi to build BOOTX64.EFI."
endif

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

# Step 1: compile to ELF object
$(BUILD_DIR)/main.o: src/main.c | $(BUILD_DIR)
	$(CC) $(CFLAGS) -c $< -o $@

# Step 2: link into an EFI ELF shared object
$(BUILD_DIR)/bootx64.so: $(BUILD_DIR)/main.o
	$(LD) $(LDFLAGS) $< -o $@ \
	    -L$(EFI_LIBS) -lefi -lgnuefi

# Step 3: convert ELF shared object → PE32+ EFI binary.
# Modern binutils dropped the "efi-app-x86_64" pseudo-target.
# Explicitly declare the input format (-I) so objcopy doesn't mis-detect it,
# then emit a PE32+ EFI Application (-O pei-x86-64, --subsystem 10).
$(BUILD_DIR)/BOOTX64.EFI: $(BUILD_DIR)/bootx64.so
	$(OBJCOPY) -I elf64-x86-64 -O pei-x86-64 --subsystem=10 \
	           -j .text -j .sdata -j .data -j .dynamic \
	           -j .dynsym -j .rel -j .rela -j .reloc \
	           $< $@

clean:
	rm -rf $(BUILD_DIR)
