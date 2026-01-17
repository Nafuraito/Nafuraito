# Nafuraito Kernel - GDT Library
# Builds libgdt.a containing Global Descriptor Table implementation

include ../common.mk

LIB_NAME = libgdt.a
BUILD_DIR = $(KERNEL_BUILD_DIR)/gdt

.PHONY: all clean

all: $(BUILD_DIR) $(BUILD_DIR)/$(LIB_NAME)

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/$(LIB_NAME): $(BUILD_DIR)/gdt.o $(BUILD_DIR)/gdt_rs.o
	$(call create_lib,$@,$^)
	@echo "Built $(LIB_NAME)"

# Compile assembly sources
$(BUILD_DIR)/gdt.o: gdt.asm
	$(ASM) $(ASMFLAGS) $< -o $@

# Compile Rust library
$(BUILD_DIR)/gdt_rs.o: lib.rs
	$(RUSTC) $< -o $@

clean:
	rm -rf $(BUILD_DIR)
