# Nafuraito Kernel - Common Build Configuration
# This file is included by all kernel library Makefiles

# Toolchain
RUSTC = rustc --emit obj -C panic=abort --target x86_64-unknown-linux-gnu \
        -C code-model=kernel -C relocation-model=static \
        -C opt-level=2 -C overflow-checks=no -C debug-assertions=no

ASM = nasm
LD = ld
AR = ar
CC = gcc

# Common flags
CFLAGS = -m64 -nostdlib -fno-stack-protector -ffreestanding -c -Wall -Werror -Wpedantic \
         -fno-builtin -mno-red-zone -fno-pic -fno-pie

ASMFLAGS = -f elf64

# Build directory (relative to workspace root)
ROOT_BUILD_DIR = $(realpath $(dir $(lastword $(MAKEFILE_LIST)))../../build)
KERNEL_BUILD_DIR = $(ROOT_BUILD_DIR)/kernel

# Create static library from object files
# Usage: $(call create_lib,libname,object_files)
define create_lib
	$(AR) rcs $1 $2
endef
