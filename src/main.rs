use clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::io::{self, Read};

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
    let input = match args.input.as_ref().and_then(|p| p.to_str()).filter(|s| *s != "-") {
        Some(path_str) => fs::read_to_string(path_str)?,
        None => {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            buffer
        }
    };
    let output = process(&input, args.preserve_strings);
    println!("{}", output);
    Ok(())
}

fn process(input: &str, preserve_strings: bool) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut state = State::Normal;

    while let Some(c) = chars.next() {
        match state {
            State::Normal => {
                match c {
                    '/' => {
                        match chars.peek() {
                            Some('/') => {
                                chars.next();
                                output.push_str("  ");
                                state = State::LineComment;
                            }
                            Some('*') => {
                                chars.next();
                                output.push_str("  ");
                                while let Some('*') = chars.peek() {
                                    chars.next();
                                    output.push(' ');
                                }
                                state = State::BlockComment;
                                if maybe_close_block_comment(&mut chars) {
                                    state = State::Normal;
                                }
                            }
                            _ => {
                                output.push(' ');
                            }
                        }
                    }
                    '"' => {
                        if let Some(&next1) = chars.peek() {
                            if next1 == '"' {
                                chars.next(); // consume second quote
                                if let Some(&next2) = chars.peek() {
                                    if next2 == '"' {
                                        chars.next(); // consume third quote
                                        // It's a triple quote: start text block
                                        output.push(' ');
                                        output.push(' ');
                                        output.push(' ');
                                        state = State::TextBlockLiteral;
                                        continue;
                                    } else {
                                        // Only two quotes: empty string!
                                        output.push(' ');
                                        output.push(' ');
                                        // Immediately back to normal
                                        state = State::Normal;
                                        continue;
                                    }
                                } else {
                                    // Second quote but then EOF — weird, but same, treat as empty string
                                    output.push(' ');
                                    output.push(' ');
                                    state = State::Normal;
                                    continue;
                                }
                            }
                        }
                        // Only one quote
                        output.push(' ');
                        state = State::StringLiteral;
                    }                    
                    '\'' => {
                        output.push(' ');
                        state = State::CharLiteral;
                    }
                    '\n' => {
                        output.push('\n');
                    }
                    _ => {
                        output.push(' ');
                    }
                }
            }
            State::LineComment => {
                match c {
                    '\n' => {
                        output.push('\n');
                        state = State::Normal;
                    }
                    _ => output.push(c),
                }
            }
            State::BlockComment => {
                match c {
                    '*' => {
                        if let Some('/') = chars.peek() {
                            chars.next();
                            output.push(' ');
                            output.push(' ');
                            state = State::Normal;
                            continue;
                        } else {
                            output.push('*');
                        }
                    }
                    '\n' => {
                        output.push('\n');
                        if maybe_close_block_comment(&mut chars) {
                            state = State::Normal;
                            continue;
                        }
                    }
                    _ => output.push(c),
                }
            }
            State::StringLiteral => {
                match c {
                    '\\' => {
                        if let Some(escaped) = chars.next() {
                            if preserve_strings {
                                output.push(escaped);
                            } else {
                                output.push(' ');
                            }
                        }
                    }
                    '"' => {
                        output.push(' ');
                        state = State::Normal;
                        continue;
                    }
                    '\n' => {
                        output.push('\n');
                        state = State::Normal;
                    }
                    _ => {
                        if preserve_strings {
                            output.push(c);
                        } else {
                            output.push(' ');
                        }
                    }
                }
            }
            State::TextBlockLiteral => {
                match c {
                    '"' => {
                        if let Some(&next1) = chars.peek() {
                            if next1 == '"' {
                                chars.next();
                                if let Some(&next2) = chars.peek() {
                                    if next2 == '"' {
                                        chars.next();
                                        // Closing triple quote detected
                                        // Mask closing """ always
                                        output.push(' ');
                                        output.push(' ');
                                        output.push(' ');
                                        state = State::Normal;
                                        continue;
                                    }
                                }
                            }
                        }
                        // A lone quote inside a text block? Just treat it as content
                        if preserve_strings {
                            output.push('"');
                        } else {
                            output.push(' ');
                        }
                    }
                    '\\' => {
                        if let Some(escaped) = chars.next() {
                            if preserve_strings {
                                output.push(escaped);
                            } else {
                                output.push(' ');
                            }
                        }
                    }
                    '\n' => {
                        output.push('\n');
                    }
                    _ => {
                        if preserve_strings {
                            output.push(c);
                        } else {
                            output.push(' ');
                        }
                    }
                }
            }
            State::CharLiteral => {
                match c {
                    '\\' => {
                        output.push(' ');
                        if let Some(_) = chars.next() {
                            output.push(' ');
                        }
                    }
                    '\'' => {
                        output.push(' ');
                        state = State::Normal;
                    }
                    '\n' => {
                        output.push('\n');
                        state = State::Normal;
                    }
                    _ => {
                        output.push(' ');
                    }
                }
            }
        }
    }

    output
}


/// Helper function to check and handle block comment closure after a newline
fn maybe_close_block_comment<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) -> bool {
    while let Some(' ' | '\t') = chars.peek() {
        chars.next();
    }
    if let Some('*') = chars.peek() {
        chars.next();
        if let Some('/') = chars.peek() {
            chars.next();
            return true;
        } else if let Some(' ') = chars.peek() {
            chars.next();
        }
    }
    false
}
