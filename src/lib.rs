use ::std::fmt::Debug;
use ::std::iter::Iterator;
use arraydeque::{self, ArrayDeque, Wrapping};
pub use generic_array::typenum;
use generic_array::{ArrayLength, GenericArray};
/// Circular
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
}

#[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
pub struct Iter<'a, T: Debug> {
    len: usize,
    data: &'a [u8],
    iter_book: arraydeque::Iter<'a, Entry<T>>
}

impl<'a, T> Iterator for Iter<'a, T>
where T: Debug {
    type Item = &'a [u8];

    #[inline]
    fn next(&mut self) -> Option<&'a [u8]> {
        if let Some(entry) = self.iter_book.next() {
            return Some(&self.data[entry.start..entry.start+entry.length]);
        }
        None
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

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

    fn append(&mut self, addition: T, start: usize, length: usize) {
        self.index.push_back(Entry {
            start,
            length,
            valid: true,
            addition,
        });
    }

    fn get(&self, idx: usize, current_max: usize) -> Option<&Entry<T>> {
        let min = current_max - self.index.capacity();
        let pos = if idx >= min { idx - min } else { idx };
        self.index.get(pos)
    }

    fn invalidate_until(&mut self, start: usize, length: usize) {
        let end = start + length;
        let mut found = false;
        for entry in self.index.iter_mut() {
            if entry.valid {
                if entry.start >= start && entry.start < end {
                    entry.valid = false;
                    found = true;
                } else if found {
                    break; // early return
                }
            }
        }
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
    valid: bool,
    addition: T,
}

impl<T, B> LineBuffer<T, B>
where
    T: Debug,
    B: ArrayLength<Entry<T>>,
{
    /// Create new circular buffer of defined size (bytes)
    ///
    /// Note that the capacity includes book keeping
    pub fn new(max: usize) -> Self {
        Self {
            data: vec![0; max],
            elements: 0,
            tail: 0,
            book_keeping: BookKeeping::new(),
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
            iter_book: self.book_keeping.iter()
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

    // Get element at index
    pub fn get(&self, idx: usize) -> Option<&[u8]> {
        if self.elements <= idx {
            return None;
        }
        if self.elements - self.book_keeping.capacity() > idx {
            return None;
        }
        let entry = self.book_keeping.get(idx, self.elements());
        if let Some(entry) = entry {
            if entry.valid {
                return Some(&self.data[entry.start..entry.start + entry.length]);
            }
        }
        None
    }

    /// Insert element
    pub fn insert(&mut self, element: &[u8], addition: T) {
        let e_len = element.len();
        let offset;
        let length = e_len;
        if self.tail + e_len > self.capacity_bytes() {
            offset = 0;
            self.tail = length;
        } else {
            offset = self.tail;
            self.tail += length;
        }
        self.data[offset..self.tail].copy_from_slice(&element);
        self.book_keeping.invalidate_until(offset, length);
        self.book_keeping.append(addition, offset, length);
        self.elements += 1;
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
    // dbg!(buffer.get_all_data());
    assert_eq!(buffer.get(0), None);
    // assert_eq!(buffer.get(1), Some(format!("{}", 1).as_bytes()));
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
