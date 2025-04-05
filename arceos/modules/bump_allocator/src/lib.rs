#![no_std]

use core::alloc::Layout;
use core::ptr::NonNull;
use allocator::{AllocResult, AllocError};
use allocator::{BaseAllocator, ByteAllocator, PageAllocator};

extern crate alloc;

/// Early memory allocator
/// Use it before formal bytes-allocator and pages-allocator can work!
/// This is a double-end memory range:
/// - Alloc bytes forward
/// - Alloc pages backward
///
/// [ bytes-used | avail-area | pages-used ]
/// |            | -->    <-- |            |
/// start       b_pos        p_pos       end
///
/// For bytes area, 'count' records number of allocations.
/// When it goes down to ZERO, free bytes-used area.
/// For pages area, it will never be freed!
///
pub struct EarlyAllocator<const PAGE_SIZE: usize> {
    start:  usize,
    end:    usize,
    count:  usize,
    b_pos:  usize,
    p_pos:  usize,
}

impl<const PAGE_SIZE: usize> EarlyAllocator<PAGE_SIZE> {
    pub const fn new() -> Self {
        Self { start: 0, end: 0, count: 0, b_pos: 0, p_pos: 0 }
    }

    fn alloc_bytes(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        let next = align_up(self.b_pos, layout.align());
        let end = next + layout.size();
        if end > self.end {
            alloc::alloc::handle_alloc_error(layout)
        } else {
            self.b_pos = end;
            self.count += 1;
            NonNull::new(next as *mut u8).ok_or(AllocError::NoMemory)
        }
    }
}

impl<const PAGE_SIZE: usize> BaseAllocator for EarlyAllocator<PAGE_SIZE> {
    fn init(&mut self, start: usize, size: usize) {
        self.start = start;
        self.end = start + size;
        self.b_pos = start;
        self.p_pos = self.end;
    }

    fn add_memory(&mut self, _start: usize, _size: usize) -> AllocResult {
        unimplemented!();
    }
}

impl<const PAGE_SIZE: usize> ByteAllocator for EarlyAllocator<PAGE_SIZE> {
    fn total_bytes(&self) -> usize {
        self.end - self.start
    }
    fn used_bytes(&self) -> usize {
        self.b_pos - self.start
    }
    fn available_bytes(&self) -> usize {
        self.p_pos - self.b_pos
    }
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        self.alloc_bytes(layout)
    }

    fn dealloc(&mut self, _ptr: NonNull<u8>, _layout: Layout) {
        self.count -= 1;
        if self.count == 0 {
            self.b_pos = self.start;
        }
    }
}

impl<const PAGE_SIZE: usize> PageAllocator for EarlyAllocator<PAGE_SIZE> {
    const PAGE_SIZE: usize = PAGE_SIZE;

    fn alloc_pages(&mut self, num_pages: usize, align_pow2: usize) -> AllocResult<usize> {
        assert_eq!(align_pow2 % PAGE_SIZE, 0);
        assert_eq!(num_pages, 1);
        let size = num_pages * PAGE_SIZE;
        let next = align_down(self.p_pos - size, align_pow2);
        if next <= self.b_pos {
            return Err(AllocError::NoMemory);
        } else {
            self.p_pos = next;
            Ok(next)
        }
    }

    fn dealloc_pages(&mut self, _pos: usize, _num_pages: usize) {
        unimplemented!()
    }

    fn used_pages(&self) -> usize {
        (self.end - self.p_pos) / PAGE_SIZE
    }

    fn available_pages(&self) -> usize {
        (self.p_pos - self.b_pos) / PAGE_SIZE
    }

    fn total_pages(&self) -> usize {
        (self.end - self.start) / PAGE_SIZE
    }
}

#[inline]
const fn align_down(pos: usize, align: usize) -> usize {
    pos & !(align - 1)
}

#[inline]
const fn align_up(pos: usize, align: usize) -> usize {
    (pos + align - 1) & !(align - 1)
}
