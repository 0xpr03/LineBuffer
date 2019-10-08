use linebuffer::{typenum, LineBuffer};
use std::convert::TryInto;
use std::time::*;

#[test]
#[ignore]
fn perf_simple() {
    const AMOUNT: usize = 512_000;
    let mut buffer: LineBuffer<(), typenum::U2048> = LineBuffer::new(AMOUNT);
    let start = Instant::now();
    let max: u32 = 1_000_000_000;
    for i in 0..max {
        buffer.insert(&i.to_ne_bytes(), ());
    }
    let nanos = start.elapsed().as_nanos();
    assert_eq!(buffer.capacity_bytes(), AMOUNT);
    println!("Duration: {} ns for {} entries", nanos, max);

    // let bytes: u128 = (max * 4) as u128;
    // let ms = nanos / 1_000_000;
    // println!("{} Byte in, {} B/ms",bytes, (bytes / ms) );

    let expected: u32 = max - 1;
    assert_eq!(
        buffer.get((max - 1) as usize),
        Some((&(expected.to_ne_bytes()[..]), &()))
    );
}

#[test]
#[ignore]
fn perf_from_file() {
    unimplemented!();
}
