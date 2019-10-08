#### LineBuffer - ringbuffer but for elements of different sizes

This crate is specifically for the following use case:

- high throughput of data
- infrequent read of entries or the whole buffer
- entries are distinguishable arrays of bytes
- data has dynamic size
- numbering is infinite

You can use it for example to buffer the stdout of a process per line.  
It allows setting the amount of last lines to store and the size of bytes before wrapping.

#### Example

```rust
use linebuffer::{typenum, LineBuffer};

// create a buffer of max 2048 entries/lines and 512KB data cache
// with the additional flag type ()
let mut buffer: LineBuffer<(), typenum::U2048> = LineBuffer::new(512_000);

let data = String::from("Some data stuff");
buffer.insert(data.as_bytes(),());
assert_eq!((buffer.get(0),Some(data.as_bytes(), &())));
```
