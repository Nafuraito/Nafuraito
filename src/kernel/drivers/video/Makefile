# Nafuraito Kernel - Video Driver Library
# Builds libvideo.a containing VGA graphics driver

include ../../common.mk

LIB_NAME = libvideo.a
BUILD_DIR = $(KERNEL_BUILD_DIR)/drivers/video

.PHONY: all clean

all: $(BUILD_DIR) $(BUILD_DIR)/$(LIB_NAME)

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(BUILD_DIR)/$(LIB_NAME): $(BUILD_DIR)/vga.o $(BUILD_DIR)/video_rs.o
	$(call create_lib,$@,$^)
	@echo "Built $(LIB_NAME)"

# Compile C sources
$(BUILD_DIR)/vga.o: vga.c
	$(CC) $(CFLAGS) $< -o $@

# Compile Rust library
$(BUILD_DIR)/video_rs.o: lib.rs
	$(RUSTC) $< -o $@

clean:
	rm -rf $(BUILD_DIR)
