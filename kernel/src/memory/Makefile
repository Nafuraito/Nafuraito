# Nafuraito Kernel - Memory Library
# Builds libmemory.a containing memory management implementation

include ../common.mk

LIB_NAME = libmemory.a
BUILD_DIR = $(KERNEL_BUILD_DIR)/memory

.PHONY: all clean

all: $(BUILD_DIR) $(BUILD_DIR)/$(LIB_NAME)

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/$(LIB_NAME): $(BUILD_DIR)/memory_rs.o
	$(call create_lib,$@,$^)
	@echo "Built $(LIB_NAME)"

# Compile Rust library (depends on all source files)
$(BUILD_DIR)/memory_rs.o: lib.rs e820.rs pmm.rs paging.rs pat.rs
	$(RUSTC) $< -o $@

clean:
	rm -rf $(BUILD_DIR)
