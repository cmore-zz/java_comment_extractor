use clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::io::{self, Read};
mod buffered_char_reader;
use buffered_char_reader::BufferedCharReader;

/// A simple Java comment and optional string extractor
#[derive(Parser)]
struct Args {
    /// Path to Java source file (use "-" to read from stdin)
    input: Option<PathBuf>,

    /// Preserve string contents (otherwise mask with whitespace)
    #[arg(long)]
    preserve_strings: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Normal,
    LineComment,
    BlockComment,
    StringLiteral,
    TextBlockLiteral,
    CharLiteral,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let reader: Box<dyn Read> = match args.input.as_ref().and_then(|p| p.to_str()) {
        Some("-") | None => Box::new(io::stdin()),
        Some(path_str) => Box::new(fs::File::open(path_str)?),
    };
    let mut char_reader = BufferedCharReader::new(reader);

    let stdout = io::stdout();
    let mut output = stdout.lock();

    process(&mut char_reader, &mut output, args.preserve_strings)?;
    Ok(())
}

fn process<R: Read, W: io::Write>(
    reader: &mut BufferedCharReader<R>,
    output: &mut W,
    preserve_strings: bool,
) -> io::Result<()> {
    let mut state = State::Normal;

    while let Some(c) = reader.next_char()? {
        match state {
            State::Normal => match c {
                '/' => {
                    if peek_is(reader, '/')? {
                        let _ = reader.next_char()?;
                        write_str(output, "  ")?;
                        state = State::LineComment;
                    } else if peek_is(reader, '*')? {
                        let _ = reader.next_char()?;
                        write_str(output, "  ")?;
                        while peek_is(reader, '*')? {
                            let _ = reader.next_char()?;
                            write_char(output, ' ')?;
                        }
                        state = State::BlockComment;
                        if maybe_close_block_comment(reader) {
                            state = State::Normal;
                        }
                    } else {
                        write_char(output, ' ')?;
                    }
                }
                '"' => {
                    if peek_is(reader, '"')? {
                        let _ = reader.next_char()?;
                        if peek_is(reader, '"')? {
                            let _ = reader.next_char()?;
                            write_str(output, "   ")?;
                            state = State::TextBlockLiteral;
                            continue;
                        } else {
                            write_str(output, "  ")?;
                            state = State::Normal;
                            continue;
                        }
                    }
                    write_char(output, ' ')?;
                    state = State::StringLiteral;
                }
                '\'' => {
                    write_char(output, ' ')?;
                    state = State::CharLiteral;
                }
                '\n' => {
                    write_char(output, '\n')?;
                }
                _ => {
                    write_char(output, ' ')?;
                }
            },
            State::LineComment => match c {
                '\n' => {
                    write_char(output, '\n')?;
                    state = State::Normal;
                }
                _ => write_char(output, c)?,
            },
            State::BlockComment => match c {
                '*' => {
                    if peek_is(reader, '/')? {
                        let _ = reader.next_char()?;
                        write_str(output, "  ")?;
                        state = State::Normal;
                        continue;
                    } else {
                        write_char(output, '*')?;
                    }
                }
                '\n' => {
                    write_char(output, '\n')?;
                    if maybe_close_block_comment(reader) {
                        state = State::Normal;
                        continue;
                    }
                }
                _ => write_char(output, c)?,
            },
            State::StringLiteral => match c {
                '\\' => {
                    if let Some(escaped) = reader.next_char()? {
                        if preserve_strings {
                            write_char(output, escaped)?;
                        } else {
                            write_char(output, ' ')?;
                        }
                    }
                }
                '"' => {
                    write_char(output, ' ')?;
                    state = State::Normal;
                    continue;
                }
                '\n' => {
                    write_char(output, '\n')?;
                    state = State::Normal;
                }
                _ => {
                    if preserve_strings {
                        write_char(output, c)?;
                    } else {
                        write_char(output, ' ')?;
                    }
                }
            },
            State::TextBlockLiteral => match c {
                '"' => {
                    if peek_is(reader, '"')? {
                        let _ = reader.next_char()?;
                        if peek_is(reader, '"')? {
                            let _ = reader.next_char()?;
                            write_str(output, "   ")?;
                            state = State::Normal;
                            continue;
                        }
                    }
                    if preserve_strings {
                        write_char(output, '"')?;
                    } else {
                        write_char(output, ' ')?;
                    }
                }
                '\\' => {
                    if let Some(escaped) = reader.next_char()? {
                        if preserve_strings {
                            write_char(output, escaped)?;
                        } else {
                            write_char(output, ' ')?;
                        }
                    }
                }
                '\n' => {
                    write_char(output, '\n')?;
                }
                _ => {
                    if preserve_strings {
                        write_char(output, c)?;
                    } else {
                        write_char(output, ' ')?;
                    }
                }
            },
            State::CharLiteral => match c {
                '\\' => {
                    write_char(output, ' ')?;
                    if let Some(_) = reader.next_char()? {
                        write_char(output, ' ')?;
                    }
                }
                '\'' => {
                    write_char(output, ' ')?;
                    state = State::Normal;
                }
                '\n' => {
                    write_char(output, '\n')?;
                    state = State::Normal;
                }
                _ => {
                    write_char(output, ' ')?;
                }
            },
        }
    }

    Ok(())
}

/// Helper function to check and handle block comment closure after a newline
fn maybe_close_block_comment<R: Read>(reader: &mut BufferedCharReader<R>) -> bool {
    while matches!(reader.peek_char(), Ok(Some(' ' | '\t'))) {
        let _ = reader.next_char();
    }
    if matches!(reader.peek_char(), Ok(Some('*'))) {
        let _ = reader.next_char();
        if matches!(reader.peek_char(), Ok(Some('/'))) {
            let _ = reader.next_char();
            return true;
        } else if matches!(reader.peek_char(), Ok(Some(' '))) {
            let _ = reader.next_char();
        }
    }
    false
}

fn write_char<W: io::Write>(out: &mut W, c: char) -> io::Result<()> {
    let mut temp = [0u8; 4];
    let encoded = c.encode_utf8(&mut temp);
    out.write_all(encoded.as_bytes())
}

fn write_str<W: io::Write>(out: &mut W, s: &str) -> io::Result<()> {
    out.write_all(s.as_bytes())
}

fn peek_is<R: Read>(reader: &mut BufferedCharReader<R>, expected: char) -> io::Result<bool> {
    Ok(matches!(reader.peek_char()?, Some(c) if c == expected))
}



