# Nafuraito Kernel - PIC Driver Library
# Builds libpic.a containing Programmable Interrupt Controller driver

include ../../common.mk

LIB_NAME = libpic.a
BUILD_DIR = $(KERNEL_BUILD_DIR)/drivers/pic

.PHONY: all clean

all: $(BUILD_DIR) $(BUILD_DIR)/$(LIB_NAME)

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/$(LIB_NAME): $(BUILD_DIR)/pic_rs.o
	$(call create_lib,$@,$^)
	@echo "Built $(LIB_NAME)"

# Compile Rust library
$(BUILD_DIR)/pic_rs.o: lib.rs
	$(RUSTC) $< -o $@

clean:
	rm -rf $(BUILD_DIR)
