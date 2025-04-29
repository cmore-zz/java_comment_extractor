use clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::io::{self, Read};

mod buffered_char_reader;
use buffered_char_reader::BufferedCharReader;

mod output_writer;
use output_writer::OutputWriter;

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
    let mut output_writer = OutputWriter::new(stdout.lock());

    process(&mut char_reader, &mut output_writer, args.preserve_strings)?;
    Ok(())
}

fn process<R: Read, W: io::Write>(
    reader: &mut BufferedCharReader<R>,
    output: &mut OutputWriter<W>,
    preserve_strings: bool,
) -> io::Result<()> {
    let mut state = State::Normal;

    while let Some(c) = reader.next_char()? {
        match state {
            State::Normal => match c {
                '/' => match reader.peek_char()? {
                    Some('/') => {
                        let _ = reader.next_char();
                        output.write_str("  ")?;
                        state = State::LineComment;
                    }
                    Some('*') => {
                        let _ = reader.next_char();
                        output.write_str("  ")?;
                        while let Some('*') = reader.peek_char()? {
                            let _ = reader.next_char();
                            output.write_char(' ')?;
                        }
                        state = State::BlockComment;
                        if maybe_close_block_comment(reader)? {
                            state = State::Normal;
                        }
                    }
                    _ => {
                        output.write_char(' ')?;
                    }
                },
                '"' => {
                    if let Some(next1) = reader.peek_char()? {
                        if next1 == '"' {
                            let _ = reader.next_char();
                            if let Some(next2) = reader.peek_char()? {
                                if next2 == '"' {
                                    let _ = reader.next_char();
                                    output.write_n_spaces(3)?;
                                    state = State::TextBlockLiteral;
                                    continue;
                                } else {
                                    output.write_n_spaces(2)?;
                                    state = State::Normal;
                                    continue;
                                }
                            } else {
                                output.write_n_spaces(2)?;
                                state = State::Normal;
                                continue;
                            }
                        }
                    }
                    output.write_char(' ')?;
                    state = State::StringLiteral;
                }
                '\'' => {
                    output.write_char(' ')?;
                    state = State::CharLiteral;
                }
                '\n' => output.write_char('\n')?,
                _ => output.write_char(' ')?,
            },
            State::LineComment => match c {
                '\n' => {
                    output.write_char('\n')?;
                    state = State::Normal;
                }
                _ => output.write_char(c)?,
            },
            State::BlockComment => match c {
                '*' => {
                    if let Some('/') = reader.peek_char()? {
                        let _ = reader.next_char();
                        output.write_str("  ")?;
                        state = State::Normal;
                        continue;
                    } else {
                        output.write_char('*')?;
                    }
                }
                '\n' => {
                    output.write_char('\n')?;
                    if maybe_close_block_comment(reader)? {
                        state = State::Normal;
                        continue;
                    }
                }
                _ => output.write_char(c)?,
            },
            State::StringLiteral => match c {
                '\\' => {
                    if let Some(escaped) = reader.next_char()? {
                        if preserve_strings {
                            output.write_char(escaped)?;
                        } else {
                            output.write_char(' ')?;
                        }
                    }
                }
                '"' => {
                    output.write_char(' ')?;
                    state = State::Normal;
                    continue;
                }
                '\n' => {
                    output.write_char('\n')?;
                    state = State::Normal;
                }
                _ => {
                    if preserve_strings {
                        output.write_char(c)?;
                    } else {
                        output.write_char(' ')?;
                    }
                }
            },
            State::TextBlockLiteral => match c {
                '"' => {
                    if let Some(next1) = reader.peek_char()? {
                        if next1 == '"' {
                            let _ = reader.next_char();
                            if let Some(next2) = reader.peek_char()? {
                                if next2 == '"' {
                                    let _ = reader.next_char();
                                    output.write_n_spaces(3)?;
                                    state = State::Normal;
                                    continue;
                                }
                            }
                        }
                    }
                    if preserve_strings {
                        output.write_char('"')?;
                    } else {
                        output.write_char(' ')?;
                    }
                }
                '\\' => {
                    if let Some(escaped) = reader.next_char()? {
                        if preserve_strings {
                            output.write_char(escaped)?;
                        } else {
                            output.write_char(' ')?;
                        }
                    }
                }
                '\n' => output.write_char('\n')?,
                _ => {
                    if preserve_strings {
                        output.write_char(c)?;
                    } else {
                        output.write_char(' ')?;
                    }
                }
            },
            State::CharLiteral => match c {
                '\\' => {
                    output.write_char(' ')?;
                    if let Some(_) = reader.next_char()? {
                        output.write_char(' ')?;
                    }
                }
                '\'' => {
                    output.write_char(' ')?;
                    state = State::Normal;
                }
                '\n' => {
                    output.write_char('\n')?;
                    state = State::Normal;
                }
                _ => output.write_char(' ')?,
            },
        }
    }

    Ok(())
}

fn maybe_close_block_comment<R: Read>(reader: &mut BufferedCharReader<R>) -> io::Result<bool> {
    while let Some(c) = reader.peek_char()? {
        if c == ' ' || c == '\t' {
            let _ = reader.next_char();
        } else {
            break;
        }
    }
    if let Some('*') = reader.peek_char()? {
        let _ = reader.next_char();
        if let Some('/') = reader.peek_char()? {
            let _ = reader.next_char();
            return Ok(true);
        } else if let Some(' ') = reader.peek_char()? {
            let _ = reader.next_char();
        }
    }
    Ok(false)
}




