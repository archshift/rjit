extern crate libc;

use std::mem;
use std::sync::atomic;

mod mmap;
mod extfn;
#[macro_use] pub mod x64asm;
use crate::mmap::MemChunk;
use crate::extfn::ExtFn;

extern {
    fn atomic_write16(p: *mut u16, v: u16);
}

trait InstCopy {
    #[inline]
    fn instcopy<'a>(&mut self, src: impl Iterator<Item=&'a [u8]>);
}

impl InstCopy for [u8] {
    #[inline]
    fn instcopy<'a>(&mut self, src: impl Iterator<Item=&'a [u8]>) {
        let mut pos = 0;
        for slice in src {
            let end = pos + slice.len();
            self[pos..end].copy_from_slice(slice);
            pos = end;
        }
    }
}

struct JitPage {
    page: MemChunk,
    loop_pos: usize,
    prev_page: Option<Box<JitPage>>
}

impl JitPage {
    const CODE_SIZE: usize = MemChunk::PAGE_SIZE - 8;

    pub fn map() -> Self {
        let mut page = MemChunk::filled(x64asm::opcode::NOP);
        let self_jump = assemble!(jmp8 0 0);
        page[..2].instcopy(self_jump);

        JitPage {
            page: page,
            loop_pos: 0,
            prev_page: None
        }
    }

    fn loop_pos(region_start: usize) -> usize {
        // Round up to the nearest word-aligned position
        (region_start + 4) / 4 * 4
    }

    fn address_at(&self, pos: usize) -> *const u8 {
        &self.page[pos] as *const u8
    }

    fn insert_jmp_bridge(&mut self, other: &JitPage) {
        let dst_ptr = other.address_at(0) as usize;
        let src_ptr = self.address_at(Self::CODE_SIZE) as usize;
        
        let jmp_instr = assemble!(jmp dst_ptr src_ptr);
        self.page[Self::CODE_SIZE + 2 .. Self::CODE_SIZE + 7].instcopy(jmp_instr);
    }

    pub fn push_instrs<'a, T: Iterator<Item=&'a [u8]>>(mut self, mut instrs: T) -> JitPage {
        let mut copy_pos = self.loop_pos + 2;

        let mut newpage_instr = None;

        while let Some(instr) = instrs.next() {
            if copy_pos + instr.len() >= Self::CODE_SIZE {
                newpage_instr = Some(instr);
                break;
            }
            self.page[copy_pos .. copy_pos + instr.len()].copy_from_slice(instr);
            copy_pos += instr.len();
        }

        if newpage_instr.is_some() {
            let mut new_page = JitPage::map();
            new_page = new_page.push_instrs(newpage_instr.iter().cloned());
            new_page = new_page.push_instrs(instrs);
            
            self.insert_jmp_bridge(&new_page);
            self.break_loop();

            self.loop_pos = Self::CODE_SIZE;
            new_page.prev_page = Some(Box::new(self));
            new_page
        } else {
            let newloop_pos = Self::loop_pos(copy_pos);
            let jmp_self = assemble!(jmp8 0 0);
            self.page[newloop_pos .. newloop_pos + 2].instcopy(jmp_self);
            self.break_loop();
            self.loop_pos = newloop_pos;
            self
        }
    }

    fn break_loop(&mut self) {
        atomic::fence(atomic::Ordering::SeqCst);
        
        let mut nops = [0u8; 2];
        nops.instcopy(assemble!(nop; nop));
        let word_nop: u16 = unsafe { mem::transmute(nops) };
        let loop_pos = self.loop_pos;
        unsafe {
            atomic_write16(&mut self.page[loop_pos] as *mut u8 as *mut u16, word_nop);
        }
    }

    pub fn curr_addr(&self) -> usize {
        self.address_at(self.loop_pos) as usize
    }
}

impl Drop for JitPage {
    fn drop(&mut self) {
        // Manually drop the rest to avoid a stack overflow
        while let Some(mut x) = self.prev_page.take() {
            self.prev_page = x.prev_page.take();
        }
    }
}


pub struct JitBuffer {
    curr_page: JitPage
}

impl JitBuffer {
    pub fn new() -> Self {
        Self {
            curr_page: JitPage::map()
        }
    }

    pub fn push_instrs<'a, T: Iterator<Item=&'a [u8]>>(&mut self, instrs: T) {
        let mut tmp = unsafe { mem::zeroed() };
        mem::swap(&mut self.curr_page, &mut tmp);
        tmp = tmp.push_instrs(instrs);
        mem::swap(&mut self.curr_page, &mut tmp);
        mem::forget(tmp);
    }

    pub fn start_func(&self) -> ExtFn {
        ExtFn { ptr: self.curr_page.curr_addr() }
    }
}



#[test]
fn test() {
    use std::thread::spawn;
    use std::sync::{Arc, Barrier};
    let currtime = std::time::Instant::now;
    let fence = || atomic::fence(atomic::Ordering::SeqCst);

    let mut jit = JitBuffer::new();
    let func = unsafe { jit.start_func().fn0::<u32>() };

    let b1 = Arc::new(Barrier::new(2));
    let b2 = b1.clone();

    let t = spawn(move || {
        b2.wait();
    
        let out = func();
        let end_time = currtime();
        assert!(out == 20);
        end_time
    });

    b1.wait();
    fence();
    let time = {
        let start = currtime();

        jit.push_instrs(assemble!(
            rex_w; xor ax ax;
            rex_w; addi ax 20;
            ret
        ));

        let end = t.join().unwrap();
        (end - start).subsec_nanos()
    };
    fence();

    spawn(move || {
        println!("Took {}ns", time);

        // Make sure func's in the icache
        func();
        fence();
        let time = {
            // Bench func
            let start = currtime();
            let out = func();
            let end = currtime();
            assert!(out == 20);
            (end - start).subsec_nanos()
        };
        fence();

        println!("Then took {}ns", time);
    }).join().unwrap();
}
