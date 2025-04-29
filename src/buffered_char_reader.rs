// buffered_char_reader.rs
use std::io::{self, BufRead, BufReader, Read};

pub struct BufferedCharReader<R: Read> {
    reader: BufReader<R>,
    buf: String,
    pos: usize,
    peeked: Option<char>,
}

impl<R: Read> BufferedCharReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::with_capacity(4096, reader),
            buf: String::new(),
            pos: 0,
            peeked: None,
        }
    }

    fn fill_buf_if_needed(&mut self) -> io::Result<()> {
        while self.pos >= self.buf.len() {
            self.buf.clear();
            self.pos = 0;
            let bytes_read = self.reader.read_line(&mut self.buf)?;
            if bytes_read == 0 {
                break;
            }
        }
        Ok(())
    }

    pub fn next_char(&mut self) -> io::Result<Option<char>> {
        if let Some(c) = self.peeked.take() {
            return Ok(Some(c));
        }
        self.fill_buf_if_needed()?;
        if self.pos >= self.buf.len() {
            return Ok(None);
        }
        let c = self.buf[self.pos..].chars().next().unwrap();
        self.pos += c.len_utf8();
        Ok(Some(c))
    }

    pub fn peek_char(&mut self) -> io::Result<Option<char>> {
        if self.peeked.is_none() {
            self.peeked = self.next_char()?;
        }
        Ok(self.peeked)
    }
}
