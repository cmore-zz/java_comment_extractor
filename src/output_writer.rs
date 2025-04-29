// output_writer.rs
use std::io::{self, Write};

pub struct OutputWriter<W: Write> {
    writer: W,
    encode_buf: [u8; 4],
}

impl<W: Write> OutputWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            encode_buf: [0u8; 4],
        }
    }

    pub fn write_char(&mut self, c: char) -> io::Result<()> {
        let encoded = c.encode_utf8(&mut self.encode_buf);
        self.writer.write_all(encoded.as_bytes())
    }

    pub fn write_str(&mut self, s: &str) -> io::Result<()> {
        self.writer.write_all(s.as_bytes())
    }

    pub fn write_n_spaces(&mut self, n: usize) -> io::Result<()> {
        // Write in chunks to avoid allocating a huge string
        const CHUNK: &str = "                "; // 16 spaces
        let mut remaining = n;
        while remaining > 0 {
            let len = remaining.min(CHUNK.len());
            self.writer.write_all(&CHUNK.as_bytes()[..len])?;
            remaining -= len;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
