use std::ops::{Index, IndexMut};
use std::slice;
use std::ptr;

use libc;

pub(crate) struct MemChunk {
    ptr: *mut u8,
}

impl MemChunk {
    pub const PAGE_SIZE: usize = 0x10000; // TODO: For testing that page "resizing" works

    pub unsafe fn uninit() -> Self {
        let perms = libc::PROT_WRITE | libc::PROT_EXEC;
        let flags = libc::MAP_PRIVATE | libc::MAP_ANON;
        let ptr = libc::mmap(ptr::null_mut(), Self::PAGE_SIZE, perms, flags, 0, 0) as *mut u8;
        Self { ptr: ptr }
    }


    pub fn filled(fill: u8) -> Self {
        unsafe {
            let chunk = Self::uninit();
            chunk.ptr.write_bytes(fill, Self::PAGE_SIZE);
            chunk
        }
    }
}

impl<T: slice::SliceIndex<[u8]>> Index<T> for MemChunk {
    type Output = <[u8] as Index<T>>::Output;

    fn index(&self, idx: T) -> &Self::Output {
        unsafe { &slice::from_raw_parts(self.ptr, Self::PAGE_SIZE)[idx] }
    }
}

impl<T: slice::SliceIndex<[u8]>> IndexMut<T> for MemChunk {
    fn index_mut(&mut self, idx: T) -> &mut Self::Output {
        unsafe { &mut slice::from_raw_parts_mut(self.ptr, Self::PAGE_SIZE)[idx] }
    }
}

impl Drop for MemChunk {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.ptr as *mut libc::c_void, Self::PAGE_SIZE);
        }
    }
}
