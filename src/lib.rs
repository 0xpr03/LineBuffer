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

#[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
pub struct Iter<'a, T: Debug> {
    len: usize,
    data: &'a [u8],
    iter_book: arraydeque::Iter<'a, Entry<T>>,
}

impl<'a, T> Iterator for Iter<'a, T>
where
    T: Debug,
{
    type Item = &'a [u8];

    #[inline]
    fn next(&mut self) -> Option<&'a [u8]> {
        if let Some(entry) = self.iter_book.next() {
            return Some(&self.data[entry.start..entry.start + entry.length]);
        }
        None
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
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

    #[inline]
    fn iter(&self) -> arraydeque::Iter<Entry<T>> {
        self.index.iter()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.index.capacity()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.index.len()
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
        let min = match current_max < self.index.capacity() {
            true => 0, // no wrap till now
            false => current_max - self.index.capacity(),
        };
        let pos = if idx >= min { idx - min } else { idx };
        self.index.get(pos)
    }
}

// not supposed to be public..
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
    /// Note that this is not the amount of lines (entries).
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
    #[inline]
    pub fn iter(&self) -> Iter<T> {
        Iter {
            data: &self.data,
            len: self.len(),
            iter_book: self.book_keeping.iter(),
        }
    }

    /// Total amount of inserted elements
    pub fn elements(&self) -> usize {
        self.elements
    }

    /// Amount of entries in buffer, including metadata
    #[inline]
    pub fn len(&self) -> usize {
        self.book_keeping.len()
    }

    /// Capacity of lines
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

    // Get element at index, idx counting up since first element inserted.
    pub fn get(&self, idx: usize) -> Option<&[u8]> {
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
        // dbg!(entry);
        if let Some(entry) = entry {
            // by checking that it is contained in let n = total_byte_count_written_into_ringbuffer; [n - buffer_size, n)
            // dbg!(idx);
            // dbg!(entry.start);
            // dbg!(self.written_bytes);
            // dbg!(self.capacity_bytes());
            // prevent underflow when writteb_bytes < capacity, otherwise check withing range
            if self.written_bytes < self.capacity_bytes() || entry.start >= self.written_bytes - self.capacity_bytes() {
                // let start = entry.start - (self.written_bytes - self.capacity_bytes());
                let start = entry.start % self.capacity_bytes();
                // dbg!(start);
                return Some(&self.data[start..start + entry.length]);
            }
        }
        None
    }

    /// Insert element and an additional value, can be used as flag
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
        self.book_keeping.append(addition, self.written_bytes, e_len);
        self.elements += 1;
        self.written_bytes += e_len;
    }
}

#[test]
fn insert_simple() {
    let mut buffer: LineBuffer<i32, typenum::U8> = LineBuffer::new(8);
    for i in 0..8 {
        buffer.insert(format!("{}", i).as_bytes(), 0);
    }
    // dbg!(String::from_utf8_lossy(buffer.get(0).unwrap()));
    for i in 0..8 {
        // dbg!(i);
        // dbg!(buffer.get(i));
        assert_eq!(buffer.get(i), Some(format!("{}", i).as_bytes()));
    }
    assert_eq!(buffer.get(8), None);
}

#[test]
fn insert_overflow_index() {
    let mut buffer: LineBuffer<i32, typenum::U8> = LineBuffer::new(8);
    for i in 0..8 {
        buffer.insert(format!("{}", i).as_bytes(), 0);
    }
    buffer.insert(format!("{}", 8).as_bytes(), 0);
    assert_eq!(buffer.get(0), None);
    assert_eq!(buffer.get(1), Some(format!("{}", 1).as_bytes()));
    for i in 1..9 {
        // dbg!(String::from_utf8_lossy(buffer.get(i).unwrap()));
        assert_eq!(buffer.get(i), Some(format!("{}", i).as_bytes()));
    }
}

#[test]
fn insert_overflow_full() {
    let mut buffer: LineBuffer<(), typenum::U8> = LineBuffer::new(8);
    for i in 0..100 {
        buffer.insert(format!("{}", i).as_bytes(), ());
    }
    for i in 1..96 {
        // dbg!(String::from_utf8_lossy(buffer.get(i).unwrap()));
        // dbg!(i);
        assert_eq!(buffer.get(i), None);
    }
    // dbg!(buffer.get_all_data());
    for i in 96..100 {
        // dbg!(i);
        assert_eq!(buffer.get(i), Some(format!("{}", i).as_bytes()));
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
        assert_eq!(buffer.get(i), Some(format!("{}", i + 10).as_bytes()));
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
        assert_eq!(buffer.get(i), Some(format!("{}", i + 10).as_bytes()));
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
        assert_eq!(buffer.get(i), Some(format!("{}", i + 10).as_bytes()));
    }
    assert_eq!(buffer.get(8), None);
}

#[test]
fn insert_empty() {
    let mut buffer: LineBuffer<(), typenum::U8> = LineBuffer::new(9);
    buffer.insert(format!("{}",21).as_bytes(),());
    let empty = [0; 0];
    buffer.insert(&empty, ());
    assert_eq!(buffer.get(0), Some(format!("{}", 21).as_bytes()));
    assert_eq!(buffer.get(1), Some(&empty[0..0]));
}

#[test]
fn iter_test() {
    let mut buffer: LineBuffer<(), typenum::U8> = LineBuffer::new(9);
    for i in 0..
    buffer.insert(format!("{}"))
}