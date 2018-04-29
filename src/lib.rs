extern crate libc;

use std::mem;
use std::ptr;
use std::slice;
use std::sync::atomic;

const NOP: u8 = 0x90;
const JMP_SELF: [u8; 2] = [0xEB, 0xFE];
fn JMP_REL(rel: i32) -> [u8; 5] {
    let rb: [u8; 4] = unsafe { mem::transmute(rel - 5) };
    [0xe9, rb[0], rb[1], rb[2], rb[3]]
}

extern {
    fn atomic_write16(p: *mut u16, v: u16);
}

type JitInstr<'a> = &'a [u8];

pub struct JitPage {
    page: &'static mut [u8],
    loop_pos: usize,
    prev_page: Option<Box<JitPage>>
}

impl JitPage {
    const PAGE_SIZE: usize = 0x10; // TODO: For testing that page "resizing" works
    const CODE_SIZE: usize = Self::PAGE_SIZE - 8;

    pub fn map() -> Self {
        let slice = unsafe {
            let buf = libc::mmap(ptr::null_mut(), Self::PAGE_SIZE, libc::PROT_WRITE | libc::PROT_EXEC,
                                 libc::MAP_PRIVATE | libc::MAP_ANON, 0, 0) as *mut u8;
            println!("Mapping JIT page at {:016X}!", buf as u64);
            ptr::write_bytes(buf, NOP, Self::PAGE_SIZE);
            slice::from_raw_parts_mut(buf, Self::PAGE_SIZE)
        };
        slice[..2].copy_from_slice(&JMP_SELF);

        JitPage {
            page: slice,
            loop_pos: 0,
            prev_page: None
        }
    }

    fn loop_pos(region_start: usize) -> usize {
        // Round up to the nearest word-aligned position
        (region_start + 2) / 4 * 4
    }

    fn rel_to(&self, other_address: usize) -> i32 {
        let target_addr = self.page.as_ptr() as usize;
        let src_addr = other_address;
        let out = target_addr.wrapping_sub(src_addr) as isize;
        assert!(out >= (i32::min_value() as isize) && out <= (i32::max_value() as isize));
        out as i32
    }

    fn address_at(&self, pos: usize) -> usize {
        &self.page[pos] as *const u8 as usize
    }

    pub fn push_instrs(mut self, instrs: &[JitInstr]) -> JitPage {
        let mut copy_pos = self.loop_pos + 2;

        for (i, instr) in instrs.iter().enumerate() {
            if copy_pos + instr.len() >= Self::CODE_SIZE {
                let mut new_page = JitPage::map().push_instrs(&instrs[i..]);
                let jmp_instr = JMP_REL(new_page.rel_to(self.address_at(Self::CODE_SIZE)));
                self.page[Self::CODE_SIZE .. Self::CODE_SIZE + 5].copy_from_slice(&jmp_instr);

                self.break_loop();
                self.loop_pos = Self::CODE_SIZE;
                new_page.prev_page = Some(Box::new(self));
                return new_page;
            }
            self.page[copy_pos .. copy_pos + instr.len()].copy_from_slice(instr);
            copy_pos += instr.len();
        }

        let newloop_pos = Self::loop_pos(copy_pos);
        self.page[newloop_pos .. newloop_pos + 2].copy_from_slice(&JMP_SELF);
        self.break_loop();
        self.loop_pos = newloop_pos;
        self
    }

    fn break_loop(&mut self) {
        atomic::fence(atomic::Ordering::SeqCst);

        let word_nop: u16 = unsafe { mem::transmute([NOP, NOP]) };
        let loop_pos = self.loop_pos;
        unsafe {
            atomic_write16(&mut self.page[loop_pos] as *mut u8 as *mut u16, word_nop);
        }
    }

    pub fn func(&self) -> extern fn() {
        return unsafe { mem::transmute(self.page.as_ptr()) };
    }
}

impl Drop for JitPage {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.page.as_mut_ptr() as *mut libc::c_void, Self::PAGE_SIZE);
        }
    }
}

#[test]
fn test() {
    let mut page = JitPage::map();
    let func = page.func();

    let t = ::std::thread::spawn(move || {
        let func: fn() -> u32 = unsafe { mem::transmute(func) };
        let out = func();
        let end_time = ::std::time::Instant::now();
        assert!(out == 20);
        println!("Found {:08X}", out);
        println!("Exiting execution thread...");
        end_time
    });

    ::std::thread::sleep_ms(10);
    let start = ::std::time::Instant::now();

    page = page.push_instrs(&[
        &[0x48, 0x31, 0xc0], // xor eax, eax
        &[0x48, 0x83, 0xc0, 0x14], // add eax, 20
        &[0xc3] // ret
    ]);

    let end = t.join().unwrap();
    println!("Took {}ns", (end - start).subsec_nanos());

    ::std::thread::spawn(move || {
        let func: fn() -> u32 = unsafe { mem::transmute(func) };
        let start_time = ::std::time::Instant::now();
        let out = func();
        let end_time = ::std::time::Instant::now();
        println!("Then took {}ns", (end_time - start_time).subsec_nanos());
        assert!(out == 20);
    }).join().unwrap();
}
