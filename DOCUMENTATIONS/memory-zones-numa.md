# DMA Memory Zones and NUMA Support

## Overview

The Physical Memory Manager (PMM) now supports DMA-capable memory zones and NUMA (Non-Uniform Memory Access) awareness for multi-socket systems. This enables efficient memory allocation for device drivers and optimal performance on multi-socket architectures.

## Memory Zones

### Zone Types

1. **DMA Zone (0-16MB)**
   - ISA DMA compatible
   - Required for legacy devices with 24-bit address limitations
   - Zone ID: 0

2. **DMA32 Zone (16MB-4GB)**
   - 32-bit DMA compatible
   - For devices that can only address the first 4GB
   - Zone ID: 1

3. **Normal Zone (4GB+)**
   - 64-bit capable
   - Standard memory allocation zone
   - Zone ID: 2

4. **HighMem Zone**
   - Very high memory regions
   - Reserved for future use
   - Zone ID: 3

### Zone Boundaries

```rust
pub const DMA_ZONE_END: u64 = 16 * 1024 * 1024;      // 16MB
pub const DMA32_ZONE_END: u64 = 4 * 1024 * 1024 * 1024; // 4GB
```

## Zone-Aware Allocation

### Rust API

```rust
// Allocate from specific zone
let addr = pmm.alloc_frame_in_zone(MemoryZone::Dma)?;

// DMA-capable allocation (convenience wrappers)
let dma_addr = pmm.alloc_dma_frame()?;        // First 16MB
let dma32_addr = pmm.alloc_dma32_frame()?;    // First 4GB (tries DMA, then DMA32)

// Zone statistics
let stats = pmm.zone_stats(MemoryZone::Dma);
let free_mem = pmm.zone_free_memory(MemoryZone::Dma32);
let total_mem = pmm.zone_total_memory(MemoryZone::Normal);
```

### C API

```c
// Allocate from zone (zone: 0=DMA, 1=DMA32, 2=Normal)
uint64_t addr = pmm_alloc_frame_in_zone(0);  // DMA zone

// Convenience functions
uint64_t dma_addr = pmm_alloc_dma_frame();
uint64_t dma32_addr = pmm_alloc_dma32_frame();

// Zone statistics
uint64_t free = pmm_zone_free_memory(0);     // DMA zone free memory
uint64_t total = pmm_zone_total_memory(1);   // DMA32 zone total memory
```

## Contiguous Allocation

For DMA buffers that require physically contiguous memory:

### Rust API

```rust
// Allocate 8 contiguous frames (32KB)
let base_addr = pmm.alloc_frames_contiguous(8, Some(MemoryZone::Dma32))?;

// Free contiguous region
pmm.free_frames_contiguous(base_addr, 8)?;
```

### C API

```c
// Allocate contiguous frames (zone: 0xFF for any zone)
uint64_t base = pmm_alloc_frames_contiguous(8, 1);  // 8 frames in DMA32 zone

// Free contiguous frames
int result = pmm_free_frames_contiguous(base, 8);
```

## NUMA Support

### Current Implementation

NUMA support is currently initialized with a single node (node 0) as a placeholder. Full NUMA topology detection requires ACPI SRAT (System Resource Affinity Table) parsing.

### NUMA API

```rust
// Allocate with NUMA node preference
let addr = pmm.alloc_frame_numa(NumaNode::new(0))?;

// NUMA statistics
let numa_count = pmm.numa_node_count();
let free_mem = pmm.numa_free_memory(NumaNode::new(0));
let total_mem = pmm.numa_total_memory(NumaNode::new(0));
```

### C API

```c
// Allocate with NUMA preference
uint64_t addr = pmm_alloc_frame_numa(0);  // Prefer node 0

// NUMA statistics
uint64_t node_count = pmm_numa_node_count();
uint64_t free = pmm_numa_free_memory(0);
uint64_t total = pmm_numa_total_memory(0);
```

## Allocation Flags

The `AllocFlags` structure provides hints for memory allocation:

```rust
// Default allocation (no preferences)
let flags = AllocFlags::new();

// DMA allocation
let dma_flags = AllocFlags::dma();

// DMA32 allocation with fallback
let dma32_flags = AllocFlags::dma32();

// NUMA-local allocation
let numa_flags = AllocFlags::numa(NumaNode::new(0));
```

## Utility Functions

### Memory Usage Monitoring

```rust
// Get overall usage percentage
let usage = pmm.memory_usage_percent();  // 0-100

// Check if memory is critically low (>90% used)
if pmm.is_memory_critical() {
    // Handle low memory condition
}

// Zone-specific usage
let zone_usage = pmm.zone_usage_percent(MemoryZone::Dma);
if pmm.is_zone_critical(MemoryZone::Dma) {
    // DMA zone critically low
}

// Fragmentation estimate
let frag = pmm.fragmentation_estimate();  // 0-100 percentage
```

### C API

```c
// Memory usage
uint8_t usage = pmm_memory_usage_percent();
int is_critical = pmm_is_memory_critical();  // 1 if critical, 0 otherwise
```

## E820 Zone Helpers

Extended E820 memory map functionality:

```rust
// Get total memory in a zone from E820 map
let zone_mem = memory_map.zone_memory(MemoryZone::Dma);

// Count regions in a zone
let region_count = memory_map.zone_region_count(MemoryZone::Dma32);

// Check zone overlap for an entry
if let Some((base, length)) = entry.zone_overlap(MemoryZone::Dma) {
    // Entry overlaps with DMA zone
}
```

## Implementation Details

### Zone Statistics Tracking

Each zone maintains:
- `total_frames`: Total frames in the zone
- `free_frames`: Currently free frames
- `last_alloc_index`: Optimization hint for allocation

Zone statistics are updated during:
- PMM initialization (calculating zone boundaries)
- Frame allocation (decrementing free_frames)
- Frame deallocation (incrementing free_frames)

### NUMA Statistics Tracking

Each NUMA node tracks:
- `total_frames`: Total frames on the node
- `free_frames`: Currently free frames on the node

Current implementation:
- Defaults to single NUMA node (node 0)
- Placeholder for future ACPI SRAT parsing
- All memory attributed to node 0

### Contiguous Allocation Algorithm

1. Determine search range based on zone constraint
2. Scan bitmap for contiguous free frames
3. When found, mark all frames as used
4. Update global and per-zone statistics
5. Return base physical address

Complexity: O(n) where n is the number of frames in the search range

## Future Enhancements

1. **ACPI SRAT Parsing**
   - Detect actual NUMA topology
   - Map memory regions to NUMA nodes
   - Enable true NUMA-local allocation

2. **Advanced Allocation Strategies**
   - Best-fit for contiguous allocations
   - Buddy allocator for reducing fragmentation
   - Zone watermarks and emergency reserves

3. **Memory Compaction**
   - Defragmentation support
   - Page migration for contiguous allocation

4. **Hot-plug Support**
   - Dynamic memory addition/removal
   - Zone boundary adjustments

## Usage Examples

### DMA Buffer Allocation for Device Driver

```rust
// Allocate 64KB contiguous DMA buffer (16 frames)
let buffer_addr = pmm.alloc_frames_contiguous(16, Some(MemoryZone::Dma32))?;

// Use buffer for DMA operation
setup_dma_transfer(buffer_addr, 64 * 1024);

// Free when done
pmm.free_frames_contiguous(buffer_addr, 16)?;
```

### Monitoring Memory Pressure

```c
void check_memory_health(void) {
    uint8_t usage = pmm_memory_usage_percent();
    
    if (usage > 80) {
        printf("WARNING: Memory usage at %d%%\n", usage);
    }
    
    if (pmm_is_memory_critical()) {
        printf("CRITICAL: Memory critically low!\n");
        // Trigger garbage collection, cache eviction, etc.
    }
    
    // Check specific zones
    uint64_t dma_free = pmm_zone_free_memory(0);
    if (dma_free < (1024 * 1024)) {  // Less than 1MB
        printf("WARNING: DMA zone critically low\n");
    }
}
```

## References

- E820 Memory Map: `src/kernel/memory/e820.rs`
- Physical Memory Manager: `src/kernel/memory/pmm.rs`
- Module Exports: `src/kernel/memory/mod.rs`
- Library Interface: `src/kernel/memory/lib.rs`
