# Nafuraito Kernel - PCI Driver Library
# Builds libpci.a containing PCI bus driver with MSI/MSI-X support

include ../../common.mk

LIB_NAME = libpci.a
BUILD_DIR = $(KERNEL_BUILD_DIR)/drivers/pci

.PHONY: all clean

all: $(BUILD_DIR) $(BUILD_DIR)/$(LIB_NAME)

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/$(LIB_NAME): $(BUILD_DIR)/pci_rs.o
	$(call create_lib,$@,$^)
	@echo "Built $(LIB_NAME)"

# Compile Rust library (lib.rs includes msi.rs as a module)
$(BUILD_DIR)/pci_rs.o: lib.rs msi.rs
	$(RUSTC) lib.rs -o $@

clean:
	rm -rf $(BUILD_DIR)
