#[macro_use]
extern crate criterion;
extern crate rjit;

use criterion::*;

use rjit::*;
use std::thread;
use std::mem;
use std::sync::atomic::{AtomicBool, Ordering};

pub fn send_instrs(b: &mut Bencher) {
    let mut page = Some(JitPage::map());
    let func = page.as_ref().unwrap().func();

    let t = thread::spawn(move || {
        let func: fn() -> u32 = unsafe { mem::transmute(func) };
        func();
    });

    thread::sleep_ms(10);

    b.iter(|| {
        page = Some(page.take().unwrap().push_instrs(&[
            &[0x48, 0x31, 0xc0, 0xc3], // xor eax, eax
        ]));
    });

    page.take().unwrap().push_instrs(&[&[0xc3]]);
    t.join().unwrap();
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("instruction sending", send_instrs);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
