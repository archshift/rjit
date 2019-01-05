use std::cell::Cell;

pub mod opcode {
    pub const ADD8: u8   = 0x00;
    pub const ADD: u8    = 0x01;
    pub const OR8: u8   = 0x08;
    pub const OR: u8    = 0x09;
    pub const ADC8: u8   = 0x10;
    pub const ADC: u8    = 0x11;
    pub const SBB8: u8   = 0x18;
    pub const SBB: u8    = 0x19;
    pub const AND8: u8   = 0x20;
    pub const AND: u8    = 0x21;
    pub const SUB8: u8   = 0x28;
    pub const SUB: u8    = 0x29;
    pub const XOR8: u8   = 0x30;
    pub const XOR: u8    = 0x31;
    pub const CMP8: u8   = 0x38;
    pub const CMP: u8    = 0x39;
    pub const REX_W: u8  = 0x48;
    pub const ARITH8: u8 = 0x80;
    pub const ARITH: u8  = 0x81;
    pub const NOP: u8    = 0x90;
    pub const RET: u8    = 0xC3;
    pub const JMP: u8    = 0xE9;
    pub const JMP8: u8   = 0xEB;

    pub enum ExtOp {
        Add = 0,
        Or = 1,
        Adc = 2,
        Sbb = 3,
        And = 4,
        Sub = 5,
        Xor = 6,
        Cmp = 7
    }
}

type Register = u8;
#[allow(non_upper_case_globals)]
pub mod register {
    pub const ax: u8 = 0;
    pub const cx: u8 = 1;
    pub const dx: u8 = 2;
    pub const bx: u8 = 3;
    pub const sp: u8 = 4;
    pub const bp: u8 = 5;
    pub const si: u8 = 6;
    pub const di: u8 = 7;
}

mod rm {
    pub const fn regreg(dst: super::Register, src: super::Register) -> u8 {
        (3 << 6) | ((src as u8) << 3) | (dst as u8)
    }
    pub const fn extop(op: super::opcode::ExtOp, reg2: super::Register) -> u8 {
        (3 << 6) | ((op as u8) << 3) | (reg2 as u8)
    }
}

fn as_bytes<'a, T: Copy>(t: &'a T) -> &'a [u8] {
    use std::{mem, slice::from_raw_parts};
    let size = mem::size_of::<T>();
    unsafe { from_raw_parts(t as *const T as *const u8, size) }
}

macro_rules! add {
    () => { 0 };
    ($a:expr) => { $a };
    ($a:expr, $($rest:expr),*) => { $a + add!($($rest),*) }
}

macro_rules! copy_all {
    ($dst:expr, $start:expr) => {};
    ($dst:expr, $start:expr, $val:expr $(,$rest:expr)*) => {{
        let size: usize = ::std::mem::size_of_val(&$val);
        let bytes = as_bytes(&$val);
        $dst[$start..$start+size]
            .copy_from_slice(bytes);
        copy_all!($dst, $start + size $(,$rest)*);
    }}
}

macro_rules! assemble_inner {
    ($buf:expr; $opcode:ident $($arg:expr)*) => {{
        let size = add!(1, $( ::std::mem::size_of_val(&$arg) ),*);
        $buf[0] = $crate::x64asm::opcode::$opcode;
        copy_all!($buf, 1 $(,$arg)*);
        size
    }};
}

macro_rules! def_arith {
    ( $( $name:ident / $name8:ident: $op:ident / $op8:ident ),* ) => { $(
        #[inline]
        pub fn $name(buf: &mut [u8], to: Register, with: Register) -> usize {
            let rm = rm::regreg(to, with);
            assemble_inner!(buf; $op rm)
        }

        #[inline]
        pub fn $name8(buf: &mut [u8], to: Register, with: Register) -> usize {
            let rm = rm::regreg(to, with);
            assemble_inner!(buf; $op8 rm)
        }
    )* }
}

macro_rules! def_arith_imm {
    ( $( $name:ident / $name8:ident: $op:ident ),* ) => { $(
        #[inline]
        pub fn $name(buf: &mut [u8], to: Register, operand: u32) -> usize {
            let rm = rm::extop(opcode::ExtOp::$op, to);
            assemble_inner!(buf; ARITH rm operand)
        }

        #[inline]
        pub fn $name8(buf: &mut [u8], to: Register, operand: u8) -> usize {
            let rm = rm::extop(opcode::ExtOp::$op, to);
            assemble_inner!(buf; ARITH8 rm operand)
        }
    )* }
}

def_arith! {
    adc/adc8: ADC/ADC8,
    add/add8: ADD/ADD8,
    and/and8: AND/AND8,
    cmp/cmp8: CMP/CMP8,
    or/or8:   OR/OR8,
    sbb/sbb8: SBB/SBB8,
    sub/sub8: SUB/SUB8,
    xor/xor8: XOR/XOR8
}

def_arith_imm! {
    adci/adc8i: Adc,
    addi/add8i: Add,
    andi/and8i: And,
    cmpi/cmp8i: Cmp,
    ori /or8i:  Or,
    sbbi/sbb8i: Sbb,
    subi/sub8i: Sub,
    xori/xor8i: Xor
}

#[inline] pub fn jmp(buf: &mut [u8], to: usize, from: usize) -> usize {
    let rel: u32 = to.wrapping_sub(from).wrapping_sub(5) as u32;
    assemble_inner!(buf; JMP rel)
}

#[inline] pub fn jmp8(buf: &mut [u8], to: usize, from: usize) -> usize {
    let rel: u8 = to.wrapping_sub(from).wrapping_sub(2) as u8;
    assemble_inner!(buf; JMP8 rel)
}

#[inline] pub fn rex_w(buf: &mut [u8]) -> usize {
    assemble_inner!(buf; REX_W)
}

#[inline] pub fn ret(buf: &mut [u8]) -> usize {
    assemble_inner!(buf; RET)
}

#[inline] pub fn nop(buf: &mut [u8]) -> usize {
    assemble_inner!(buf; NOP)
}

pub struct Assembly<DAT: Copy> {
    pub dat: DAT,
    pos: Cell<usize>
}

impl<T: Copy> Assembly<T> {
    pub fn create(dat: T) -> Self {
        Self {
            dat,
            pos: Cell::new(0),
        }
    }
}

impl<'a, T: Copy> Iterator for &'a Assembly<T> {
    type Item = &'a [u8];
    fn next(&mut self) -> Option<&'a [u8]> {
        let pos = self.pos.get();
        let buf = as_bytes(&self.dat);
        let inst_size = buf[pos] as usize;
        if inst_size == 0 {
            None
        } else {
            let out = &buf[pos + 1 .. pos + 1 + inst_size];
            self.pos.set(pos + 1 + inst_size);
            Some(out)
        }
    }
}

#[macro_export]
macro_rules! count_idents {
    () => { 0 };
    ($what:ident $(, $rest:ident)*) => { 1 + count_idents!( $($rest),* ) }
}

#[macro_export]
macro_rules! assemble_to_buf {
    ($buf:expr, $pos:expr;) => {};
    ($buf:expr, $pos:expr; ( $inst:ident $($arg:expr)* ) $($rest:tt)* ) => {{
        let size = $crate::x64asm::$inst(&mut $buf[$pos + 1..], $($arg),* );
        $buf[$pos] = size as u8;
        assemble_to_buf!($buf, $pos + 1 + size; $($rest)*);
    }}
}

#[macro_export]
macro_rules! assemble {
    ( $( $inst:ident $($arg:expr)* );* ) => {&{
        #[allow(unused_imports)]
	use $crate::x64asm::register::*;

        const BUF_LEN: usize = 13 * count_idents!( $( $inst ),* );
        let mut buf = [0u8; BUF_LEN];
        assemble_to_buf!(buf, 0; $(( $inst $($arg)* ))* );
        $crate::x64asm::Assembly::create(buf)
    }};
}


#[test]
fn test_assemble_macro() {
    let nops = assemble!(nop; nop; nop; jmp 0 0; nop);
    let nop_buf = as_bytes(&nops.dat);
    assert_eq!(&nop_buf[..15],
               &[1, 0x90,
                 1, 0x90,
                 1, 0x90,
                 5, 0xE9, 0xFB, 0xFF, 0xFF, 0xFF,
                 1, 0x90,
                 0]);
}

#[test]
fn test_asm_iter() {
    let nops = assemble!(nop; nop; nop; jmp 0 0; nop);
    let nop_vec: Vec<&[u8]> = nops.collect();
    assert_eq!(&nop_vec[..],
               &[
                    &[0x90][..],
                    &[0x90][..],
                    &[0x90][..],
                    &[0xE9, 0xFB, 0xFF, 0xFF, 0xFF][..],
                    &[0x90][..]
               ]);
}

#[test]
fn test_arith_regreg() {
    let mut buf = [0u8;12];
    let size = xor(&mut buf, register::ax, register::bx);
    assert_eq!(&buf[..size], &[0x31, 0xd8]);
}

