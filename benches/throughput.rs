#[macro_use]
extern crate criterion;
extern crate rjit;

use criterion::*;

use rjit::*;
use std::thread;
use std::mem;
use std::sync::mpsc::sync_channel;

pub fn write_throughput(b: &mut Bencher) {
    let mut jit = JitBuffer::new();
    let func = jit.start_func().f();

    let t = thread::spawn(move || {
        let func: fn() -> u32 = unsafe { mem::transmute(func) };
        func();
    });

    b.iter(|| {
        jit.push_instrs(assemble!(xor ax ax));
    });

    jit.push_instrs(assemble!(ret));
    t.join().unwrap();
}

pub fn write_latency(b: &mut Bencher) {
    let (tx, rx) = sync_channel(0);

    let t = thread::spawn(move || {
        let mut jit = JitBuffer::new();
        while let Ok(()) = tx.send( jit.start_func().f() ) {
            jit.push_instrs(assemble!(ret));
        }
    });

    let setup = || {
        rx.recv().unwrap()
    };

    let routine = |func: extern fn()| {
        func();
    };

    b.iter_with_setup(setup, routine);
}

pub fn exec_throughput(b: &mut Bencher, amount: &usize) {
    let (func_tx, func_rx) = sync_channel(0);

    let amount = *amount;

    let t = thread::spawn(move || {
        let mut jit = JitBuffer::new();
        while let Ok(()) = func_tx.send( jit.start_func().f() ) {
            for _ in 0..amount {
                jit.push_instrs(assemble!(xor ax ax));
            }
            jit.push_instrs(assemble!(ret));
        }
    });
    
    let setup = || {
        func_rx.recv().unwrap()
    };

    let routine = |func: extern fn()| {
        func();
    };

    b.iter_with_setup(setup, routine);
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("instruction write throughput", write_throughput);
    c.bench_function("instruction write latency", write_latency);
    c.bench_function_over_inputs("instruction exec throughput", exec_throughput, vec![0x400usize, 0x800, 0x4000, 0x8000]);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
