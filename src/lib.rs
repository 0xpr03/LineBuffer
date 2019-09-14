use byteorder::{NativeEndian, ReadBytesExt, WriteBytesExt};

/// Circular
pub struct LineBuffer {
    data: Vec<u8>,
    tail: usize,
    elements: usize,
}

impl LineBuffer {
    /// Create new circular buffer of defined size
    pub fn new(max: usize) -> Self {
        Self {
            data: Vec::with_capacity(max),
            elements: 0,
            tail: 0,
        }
    }

    /// Amount of elements in buffer
    pub fn elements(&self) -> usize {
        self.elements
    }

    /// Amount of used bytes in buffer, including metadata
    pub fn len(&self) -> usize {
        self.tail
    }

    /// Length of first element in ringbuffer
    pub fn next_element_len(&self) -> Option<usize> {
        self.data
            .get(0..4)
            .and_then(|mut v| v.read_u32::<NativeEndian>().ok().map(|r| r as usize))
    }

    /// Remove first element in ringbuffer (wrap)
    fn pop(&mut self) -> Option<Vec<u8>> {
        self.next_element_len().map(|chunk_size| {
            self.tail -= chunk_size + 4;
            self.elements -= 1;
            self.data
                .splice(..(chunk_size + 4), vec![])
                .skip(4)
                .collect()
        })
    }

    // Get element at index
    pub fn get(&self, idx: usize) -> Option<&[u8]> {
        if self.elements <= idx {
            return None;
        }
        let mut current_head = 0;
        let mut current_element = 0;
        while current_head < self.len() - 4 {
            // Get the length of the next block
            let element_size = self
                .data
                .get(0..4)
                .and_then(|mut v| v.read_u32::<NativeEndian>().ok().map(|r| r as usize))
                .unwrap();
            if current_element == idx {
                return self
                    .data
                    .get((current_head + 4)..(current_head + element_size + 4));
            }
            current_element += 1;
            current_head += 4 + element_size;
        }
        return None;
    }

    /// Insert element
    pub fn insert(&mut self, mut element: Vec<u8>, ) {
        let e_len = element.len();

        let capacity = self.data.capacity();
        while self.len() + e_len + 4 > capacity {
            self.pop();
        }
        self.data.write_u32::<NativeEndian>(e_len as u32).unwrap();
        self.data.append(&mut element);
        self.tail += 4 + e_len;
        self.elements += 1;
        println!("{:?}", self.data);
    }
}

#[test]
fn buffer_inserts() {
    let mut buffer = LineBuffer::new(100);
    buffer.insert("this is a test".as_bytes().to_vec());
    assert_eq!(buffer.pop().unwrap(), "this is a test".as_bytes().to_vec());
}
#[test]
fn buffer_truncates() {
    let mut buffer = LineBuffer::new(10);
    buffer.insert("foo".as_bytes().to_vec()); // 7 bytes in buffer
    buffer.insert("bar".as_bytes().to_vec()); // overflowed and cleared the first message
    assert_eq!(buffer.pop().unwrap(), "bar".as_bytes().to_vec());
    assert_eq!(buffer.elements(), 0);
}
#[test]
fn buffer_seeks() {
    let mut buffer = LineBuffer::new(100);
    buffer.insert("foo".as_bytes().to_vec());
    buffer.insert("bar".as_bytes().to_vec());
    buffer.insert("baz".as_bytes().to_vec());
    assert_eq!(buffer.get(0), Some("foo".as_bytes()));

    assert_eq!(buffer.get(1), Some("bar".as_bytes()));
    buffer.pop();

    println!("{:?}", std::str::from_utf8(&buffer.data).unwrap());
    assert_eq!(buffer.get(1), Some("baz".as_bytes()));

    assert_eq!(buffer.get(3), None);
}

#[test]
fn buffer_wraps() {
    let mut buffer = LineBuffer::new(20);
    buffer.insert("foo".as_bytes().to_vec());
    dbg!(buffer.next_element_len());
    buffer.insert("bar".as_bytes().to_vec());
    dbg!(buffer.next_element_len());
    buffer.insert("baz".as_bytes().to_vec());
    dbg!(buffer.next_element_len());
    println!("{:?}", std::str::from_utf8(&buffer.data).unwrap());
    buffer.insert("asdasdasd".as_bytes().to_vec());
    println!("{:?}", std::str::from_utf8(&buffer.data).unwrap());
    buffer.insert("123456".as_bytes().to_vec());
    println!("{:?}", std::str::from_utf8(&buffer.data).unwrap());
    buffer.insert("a".as_bytes().to_vec());
    dbg!(buffer.next_element_len());
    buffer.insert("b".as_bytes().to_vec());
    dbg!(buffer.next_element_len());
    buffer.insert("c".as_bytes().to_vec());
    dbg!(buffer.next_element_len());
    println!("{:?}", std::str::from_utf8(&buffer.data).unwrap());
    buffer.pop();
    println!("{:?}", std::str::from_utf8(&buffer.data).unwrap());
}