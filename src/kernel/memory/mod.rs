//! Memory Management Module
//!
//! This module provides memory management functionality for the kernel:
//! - E820 memory map parsing (BIOS memory detection)
//! - Physical memory management (planned)
//! - Virtual memory management (planned)
//! - Heap allocation (planned)

#![allow(unused)]

pub mod e820;

// Re-export commonly used types
pub use self::e820::{E820Entry, E820Type, MemoryMap};
