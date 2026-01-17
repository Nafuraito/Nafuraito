# Nafuraito Kernel - IDT Library
# Builds libidt.a containing Interrupt Descriptor Table implementation

include ../common.mk

LIB_NAME = libidt.a
BUILD_DIR = $(KERNEL_BUILD_DIR)/idt

.PHONY: all clean

all: $(BUILD_DIR) $(BUILD_DIR)/$(LIB_NAME)

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/$(LIB_NAME): $(BUILD_DIR)/idt.o $(BUILD_DIR)/idt_rs.o
	$(call create_lib,$@,$^)
	@echo "Built $(LIB_NAME)"

# Compile assembly sources
$(BUILD_DIR)/idt.o: idt.asm
	$(ASM) $(ASMFLAGS) $< -o $@

# Compile Rust library
$(BUILD_DIR)/idt_rs.o: lib.rs
	$(RUSTC) $< -o $@

clean:
	rm -rf $(BUILD_DIR)
