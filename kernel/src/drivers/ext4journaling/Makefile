# Nafuraito Kernel - EXT4 Journaling Driver Library
# Builds libext4journaling.a containing the EXT4 filesystem driver

include ../../common.mk

LIB_NAME = libext4journaling.a
BUILD_DIR = $(KERNEL_BUILD_DIR)/drivers/ext4journaling

.PHONY: all clean

all: $(BUILD_DIR) $(BUILD_DIR)/$(LIB_NAME)

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/$(LIB_NAME): $(BUILD_DIR)/ext4journaling_rs.o
	$(call create_lib,$@,$^)
	@echo "Built $(LIB_NAME)"

# Compile Rust library
$(BUILD_DIR)/ext4journaling_rs.o: ext4journaling.rs
	$(RUSTC) $< -o $@

clean:
	rm -rf $(BUILD_DIR)
