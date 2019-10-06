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
    assert_eq!(buffer.capacity_bytes(), AMOUNT);
    println!(
        "Duration: {} ns for {} entries",
        start.elapsed().as_nanos(),
        max
    );
    let input = buffer
        .get(((max - 1) as u32).try_into().unwrap())
        .unwrap()
        .to_owned();
    let (int_bytes, _) = input.split_at(std::mem::size_of::<u32>());
    let data = u32::from_le_bytes(int_bytes.try_into().unwrap());
    println!("entries {:?} {:?}", buffer.get(0), data);
}

#[test]
#[ignore]
fn perf_from_file() {
    unimplemented!();
}
