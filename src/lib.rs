//! # linebuffer
//!
//! A circular-/ringbuffer for dynamic sized elements.
//!
//! It's created specifically for storing line-like data in a upcounting fashion.
//!
//! ## Example
//!
//! ```rust
//! use linebuffer::{typenum, LineBuffer};
//!
//! // create a buffer of max 2048 entries/lines and 512KB data cache
//! // with the additional flag type ()
//! let mut buffer: LineBuffer<(), typenum::U2048> = LineBuffer::new(512_000);
//!
//! let data = String::from("Some data stuff");
//! buffer.insert(data.as_bytes(),());
//! assert_eq!(buffer.get(0),Some((data.as_bytes(), &())));
//! ```
//!
//! ## Details
//!
//! When creating a linebuffer the amount of elements(lines) and the data size is specified.  
//! This means for 8 elements and a data size of 16 the buffer will wrap when either 8 elements or more than 16 bytes were written.
//! If we would insert 8 elements of 4 bytes, our buffer would thus already wrap after 4 elements.
//!
//! Please note that the element amount is stack allocated currently. Consequently setting a high amount of elements can lead to stack overflow.
//!
use ::std::fmt::Debug;
use ::std::iter::Iterator;
use arraydeque::{self, ArrayDeque, Wrapping};
pub use generic_array::typenum;
use generic_array::{ArrayLength, GenericArray};
/// Circular Line Buffer
pub struct LineBuffer<T, B>
where
    T: Debug,
    B: ArrayLength<Entry<T>>,
{
    data: Vec<u8>,
    book_keeping: BookKeeping<T, B>,
    /// pointing to next free space in data array
    tail: usize,
    /// total amount of inserted items
    elements: usize,
    /// total written bytes, including wrapped bytes
    written_bytes: usize,
}

/// Iterator over entries in LineBuffer
///
/// Created by calling .iter() on LineBuffer
#[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
pub struct Iter<'a, T: Debug> {
    capacity: usize,
    written_bytes: usize,
    first_run: bool,
    data: &'a [u8],
    len: usize,
    iter_book: arraydeque::Iter<'a, Entry<T>>,
}

impl<'a, T> Iterator for Iter<'a, T>
where
    T: Debug,
{
    type Item = (&'a [u8], &'a T);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let mut next;
        if self.first_run {
            if self.written_bytes >= self.capacity {
                loop {
                    next = self.iter_book.next();
                    match next {
                        Some(entry) => {
                            if entry.start >= self.written_bytes - self.capacity {
                                break;
                            }
                        }
                        None => break,
                    }
                }
            } else {
                next = self.iter_book.next();
            }
            self.first_run = true;
        } else {
            next = self.iter_book.next();
        }

        if let Some(entry) = next {
            let start = entry.start % self.capacity;
            return Some((&self.data[start..start + entry.length], &entry.addition));
        }
        None
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len / 2, Some(self.len))
    }
}

/// Simple book keeping index
///
/// Doesn't handle validation
struct BookKeeping<T, B>
where
    T: Debug,
    B: ArrayLength<Entry<T>>,
{
    index: ArrayDeque<GenericArray<Entry<T>, B>, Wrapping>,
}

impl<T, B> BookKeeping<T, B>
where
    T: Debug,
    B: ArrayLength<Entry<T>>,
{
    fn new() -> Self {
        Self {
            index: ArrayDeque::new(),
        }
    }

    #[cfg(test)]
    pub fn print_index(&self) {
        dbg!(&self.index);
    }

    /// Upper bound amount of items
    ///
    /// Real value varies depending on amount of valid entries
    #[inline]
    fn length_max(&self) -> usize {
        self.index.len()
    }

    #[inline]
    fn iter(&self) -> arraydeque::Iter<Entry<T>> {
        self.index.iter()
    }

    /// Capacity of elements that can be hold.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.index.capacity()
    }

    #[inline]
    fn append(&mut self, addition: T, start: usize, length: usize) {
        self.index.push_back(Entry {
            start,
            length,
            addition,
        });
    }

    #[inline]
    fn get(&self, idx: usize, current_max: usize) -> Option<&Entry<T>> {
        // calculate total position based on "floating window" of elements in buffer
        let min = if current_max < self.index.capacity() {
            0 // no wrap till now
        } else {
            current_max - self.index.capacity()
        };
        let pos = if idx >= min { idx - min } else { idx };
        self.index.get(pos)
    }
}

/// Implementation detail, currently leaked by generic declaration
#[derive(Debug)]
pub struct Entry<T>
where
    T: Debug,
{
    start: usize,
    length: usize,
    addition: T,
}

impl<T, B> LineBuffer<T, B>
where
    T: Debug,
    B: ArrayLength<Entry<T>>,
{
    /// Create new circular buffer of defined data size (bytes)
    ///
    /// Note that this is not the amount of entries (lines).
    /// LineBuffer will wrap after reaching max bytes or the max amount of lines specified.
    pub fn new(max: usize) -> Self {
        Self {
            data: vec![0; max],
            elements: 0,
            tail: 0,
            book_keeping: BookKeeping::new(),
            written_bytes: 0,
        }
    }

    /// Debugging only
    #[cfg(test)]
    pub fn get_all_data(&self) -> String {
        self.book_keeping.print_index();
        String::from_utf8_lossy(&self.data).to_string()
    }

    /// Returns an iterator over the elements
    ///
    /// Note that the first iteration step has some overhead to skip invalid entries.
    #[inline]
    pub fn iter(&self) -> Iter<T> {
        Iter {
            data: &self.data,
            first_run: true,
            len: self.book_keeping.length_max(),
            written_bytes: self.written_bytes,
            iter_book: self.book_keeping.iter(),
            capacity: self.capacity_bytes(),
        }
    }

    /// Total amount of inserted elements
    pub fn elements(&self) -> usize {
        self.elements
    }

    /// Capacity of elements (lines)
    #[inline]
    pub fn capacity(&self) -> usize {
        self.book_keeping.capacity()
    }

    /// Capacity of total bytes
    ///
    /// Note that due to fragmentation it is not
    /// trivially possible to get the amount of free bytes
    #[inline]
    pub fn capacity_bytes(&self) -> usize {
        self.data.len()
    }

    /// Get element at index, idx counting up since first element inserted.
    pub fn get(&self, idx: usize) -> Option<(&[u8], &T)> {
        // idx > seen lines
        if self.elements <= idx {
            return None;
        }
        // idx < min elements
        if self.elements() > self.book_keeping.capacity()
            && self.elements - self.book_keeping.capacity() > idx
        {
            return None;
        }
        let entry = self.book_keeping.get(idx, self.elements());
        if let Some(entry) = entry {
            // by checking that it is contained in let n = total_byte_count_written_into_ringbuffer; [n - buffer_size, n)
            // prevent underflow when writteb_bytes < capacity, otherwise check withing range
            if self.written_bytes < self.capacity_bytes()
                || entry.start >= self.written_bytes - self.capacity_bytes()
            {
                // let start = entry.start - (self.written_bytes - self.capacity_bytes());
                let start = entry.start % self.capacity_bytes();
                return Some((&self.data[start..start + entry.length], &entry.addition));
            }
        }
        None
    }

    /// Insert element at the front and an additional value, which can be used as flag
    pub fn insert(&mut self, element: &[u8], addition: T) {
        let e_len = element.len();
        let offset;
        // calculate position in data
        if self.tail + e_len > self.capacity_bytes() {
            offset = 0;
            // add wrapped data to written_bytes, otherwise position calculation by modulo won't work
            self.written_bytes += self.capacity_bytes() - self.tail;
            self.tail = e_len;
        } else {
            offset = self.tail;
            self.tail += e_len;
        }
        self.data[offset..self.tail].copy_from_slice(element);
        self.book_keeping
            .append(addition, self.written_bytes, e_len);
        self.elements += 1;
        self.written_bytes += e_len;
    }
}

#[test]
fn insert_simple() {
    let mut buffer: LineBuffer<i32, typenum::U8> = LineBuffer::new(8);
    for i in 0..8 {
        buffer.insert(format!("{}", i).as_bytes(), i);
    }
    for i in 0..8 {
        assert_eq!(
            buffer.get(i),
            Some((format!("{}", i).as_bytes(), &(i as i32)))
        );
    }
    assert_eq!(buffer.get(8), None);
}

#[test]
fn insert_overflow_index() {
    let mut buffer: LineBuffer<i32, typenum::U8> = LineBuffer::new(8);
    for i in 0..8 {
        buffer.insert(format!("{}", i).as_bytes(), i);
    }
    buffer.insert(format!("{}", 8).as_bytes(), 8);
    assert_eq!(buffer.get(0), None);
    assert_eq!(buffer.get(1), Some((format!("{}", 1).as_bytes(), &1)));
    for i in 1..9 {
        assert_eq!(
            buffer.get(i),
            Some((format!("{}", i).as_bytes(), &(i as i32)))
        );
    }
}

// test cornercase from 1 -> 2 bytes of data where the written_bytes check doesn't work
#[test]
fn insert_overflow_border() {
    let mut buffer: LineBuffer<i32, typenum::U8> = LineBuffer::new(9);
    for i in 0..12 {
        buffer.insert(format!("{}", i).as_bytes(), i);
    }
    // dbg!(buffer.get_all_data());
    // buffer content: 910115678, idx < min elements has to prevent this
    for i in 0..5 {
        assert_eq!(buffer.get(i), None);
    }

    for i in 5..12 {
        assert_eq!(
            buffer.get(i),
            Some((format!("{}", i).as_bytes(), &(i as i32)))
        );
    }
    assert_eq!(buffer.get(12), None);
}

#[test]
fn insert_overflow_full() {
    let mut buffer: LineBuffer<(), typenum::U8> = LineBuffer::new(8);
    for i in 0..100 {
        buffer.insert(format!("{}", i).as_bytes(), ());
    }
    for i in 1..96 {
        assert_eq!(buffer.get(i), None);
    }
    for i in 96..100 {
        assert_eq!(buffer.get(i), Some((format!("{}", i).as_bytes(), &())));
    }
    for i in 100..200 {
        assert_eq!(buffer.get(i), None);
    }
}

#[test]
fn insert_elements_less_capacity() {
    let mut buffer: LineBuffer<(), typenum::U8> = LineBuffer::new(8);
    for i in 0..4 {
        // use two byte entries
        buffer.insert(format!("{}", i + 10).as_bytes(), ());
    }
    for i in 0..4 {
        assert_eq!(buffer.get(i), Some((format!("{}", i + 10).as_bytes(), &())));
    }
    assert_eq!(buffer.get(4), None);
}

// found underflow in BookKeeping::get window calc
#[test]
fn insert_elements_uneven_capacity() {
    let mut buffer: LineBuffer<(), typenum::U8> = LineBuffer::new(9);
    for i in 0..4 {
        // use two byte entries
        buffer.insert(format!("{}", i + 10).as_bytes(), ());
    }
    for i in 0..4 {
        assert_eq!(buffer.get(i), Some((format!("{}", i + 10).as_bytes(), &())));
    }
    assert_eq!(buffer.get(4), None);
}

#[test]
fn insert_elements_uneven_capacity_wrap() {
    let mut buffer: LineBuffer<(), typenum::U8> = LineBuffer::new(9);
    for i in 0..8 {
        // use two byte entries
        buffer.insert(format!("{}", i + 10).as_bytes(), ());
    }
    for i in 0..4 {
        assert_eq!(buffer.get(i), None);
    }
    for i in 4..8 {
        assert_eq!(buffer.get(i), Some((format!("{}", i + 10).as_bytes(), &())));
    }
    assert_eq!(buffer.get(8), None);
}

#[test]
fn insert_empty() {
    let mut buffer: LineBuffer<(), typenum::U8> = LineBuffer::new(9);
    buffer.insert(format!("{}", 21).as_bytes(), ());
    let empty = [0; 0];
    buffer.insert(&empty, ());
    assert_eq!(buffer.get(0), Some((format!("{}", 21).as_bytes(), &())));
    assert_eq!(buffer.get(1), Some((&empty[0..0], &())));
}

#[test]
fn iter_test_simple() {
    let mut buffer: LineBuffer<i32, typenum::U8> = LineBuffer::new(9);
    for i in 0..8 {
        buffer.insert(format!("{}", i).as_bytes(), i);
    }
    let mut i: i32 = 0;
    for (data, flag) in buffer.iter() {
        assert_eq!(data, format!("{}", i).as_bytes());
        assert_eq!(*flag, i);
        i += 1;
    }
    assert_eq!(i, 8);
}

#[test]
fn iter_test_wrap() {
    let mut buffer: LineBuffer<i32, typenum::U8> = LineBuffer::new(9);
    for i in 0..16 {
        buffer.insert(format!("{}", i).as_bytes(), i);
    }
    let mut i: i32 = 12;
    for (data, flag) in buffer.iter() {
        assert_eq!(*flag, i);
        assert_eq!(data, format!("{}", i).as_bytes());
        i += 1;
    }
    assert_eq!(i, 16);
}
