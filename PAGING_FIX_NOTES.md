# Paging Map/Unmap Implementation Issues

## Problem
The `paging_map_page` function crashes with GPF before entering the function body.

## Root Cause
The crash occurs when calling the Rust `map_page()` function from the C wrapper. Testing shows:
- ✅ C calling convention works (simple test functions succeed)
- ✅ Creating VirtAddr/PhysAddr/PageTableFlags works
- ❌ Calling `map_page()` Rust function crashes immediately

## Suspected Issues
1. Stack alignment or calling convention mismatch
2. Compiler optimization causing issues with unsafe code
3. Complex Rust types being passed on stack incorrectly

## Solution Options

### Option 1: Implement directly in C wrapper (RECOMMENDED)
Don't call separate Rust function - implement page table walk directly in the C wrapper:

```rust
#[no_mangle]
pub unsafe extern "C" fn paging_map_page(virt_addr: u64, phys_addr: u64, flags: u64) -> i32 {
    // Get manager
    let manager = match get_paging_manager() {
        Some(m) => m,
        None => return -1,
    };
    
    // Create addresses inline
    let virt = VirtAddr::new(virt_addr);
    let phys = PhysAddr::new(phys_addr);
    
    // Validate
    if (phys_addr & 0xFFF) != 0 {
        return -1;  // Not aligned
    }
    
    // Get root table and do page table walk HERE
    // Don't call another Rust function
    let root_phys = manager.root_table_phys();
    
    // Implement walk directly...
    // [rest of implementation]
    
    0  // Success
}
```

### Option 2: Use #[inline(always)] on map_page
Force the compiler to inline the function to avoid calling convention issues.

### Option 3: Rewrite in C/Assembly
Create a pure C or assembly implementation for page table manipulation.

## Next Steps
1. Implement Option 1 by moving page table walk logic into C wrapper
2. Test incrementally
3. Once working, can refactor if needed
