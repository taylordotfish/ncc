/*
 * Copyright (C) 2024 taylor.fish <contact@taylor.fish>
 *
 * This file is part of ncc.
 *
 * ncc is free software: you can redistribute it and/or modify it under
 * the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * ncc is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
 * or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General
 * Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public
 * License along with ncc. If not, see <https://www.gnu.org/licenses/>.
 */

use super::ansi::AnsiWriter;
use super::parse;
use super::Input;
use serde::de;
use std::fmt::{self, Display};
use std::io::{self, Write};
use std::ops::Range;
use toml::de::Error as TomlError;

/// Gets the first line of `s`, up to but not including `'\n'`.
///
/// Returns the entire string if no line terminator is found.
fn first_line(s: &str) -> &str {
    s.split_once('\n').map_or(s, |p| p.0)
}

/// Writes a character and returns its displayed width.
///
/// Writes `c` (or an equivalent string) to `writer` and returns the
/// approximate displayed width (in columns) of what was written.
fn write_char_sized<W: Write>(c: char, writer: &mut W) -> io::Result<usize> {
    if c == '\t' {
        write!(writer, "    ").map(|_| 4)
    } else if c.is_ascii_control() {
        Ok(0)
    } else {
        write!(writer, "{c}").map(|_| 1)
    }
}

#[derive(Debug)]
pub enum Unexpected {
    Bool(bool),
    Unsigned(u64),
    Signed(i64),
    Float(f64),
    Char(char),
    Str(Box<str>),
    Bytes(Vec<u8>),
    Unit,
    Option,
    NewtypeStruct,
    Seq,
    Map,
    Enum,
    UnitVariant,
    NewtypeVariant,
    TupleVariant,
    StructVariant,
    Other(Box<str>),
}

impl From<de::Unexpected<'_>> for Unexpected {
    fn from(unexp: de::Unexpected<'_>) -> Self {
        match unexp {
            de::Unexpected::Bool(v) => Self::Bool(v),
            de::Unexpected::Unsigned(v) => Self::Unsigned(v),
            de::Unexpected::Signed(v) => Self::Signed(v),
            de::Unexpected::Float(v) => Self::Float(v),
            de::Unexpected::Char(v) => Self::Char(v),
            de::Unexpected::Str(v) => Self::Str(v.into()),
            de::Unexpected::Bytes(v) => Self::Bytes(v.into()),
            de::Unexpected::Unit => Self::Unit,
            de::Unexpected::Option => Self::Option,
            de::Unexpected::NewtypeStruct => Self::NewtypeStruct,
            de::Unexpected::Seq => Self::Seq,
            de::Unexpected::Map => Self::Map,
            de::Unexpected::Enum => Self::Enum,
            de::Unexpected::UnitVariant => Self::UnitVariant,
            de::Unexpected::NewtypeVariant => Self::NewtypeVariant,
            de::Unexpected::TupleVariant => Self::TupleVariant,
            de::Unexpected::StructVariant => Self::StructVariant,
            de::Unexpected::Other(v) => Self::Other(v.to_string().into()),
        }
    }
}

impl Display for Unexpected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Use TOML terminology (e.g., "array" instead of "sequence").
        // Note, however, that not all variants are possible with TOML.
        match self {
            Self::Bool(v) => write!(f, "boolean `{v}`"),
            Self::Unsigned(v) => write!(f, "integer `{v}`"),
            Self::Signed(v) => write!(f, "integer `{v}`"),
            Self::Float(v) => write!(f, "floating-point number `{v}`"),
            Self::Char(v) => write!(f, "character '{v}'"),
            Self::Str(v) => write!(f, "string \"{}\"", v.escape_default()),
            Self::Bytes(v) => {
                write!(f, "byte sequence \"{}\"", v.escape_ascii())
            }
            Self::Unit => write!(f, "unit value"),
            Self::Option => write!(f, "optional value"),
            Self::NewtypeStruct => write!(f, "newtype struct"),
            Self::Seq => write!(f, "array"),
            Self::Map => write!(f, "table"),
            Self::Enum => write!(f, "enum"),
            Self::UnitVariant => write!(f, "unit variant"),
            Self::NewtypeVariant => write!(f, "newtype variant"),
            Self::TupleVariant => write!(f, "tuple variant"),
            Self::StructVariant => write!(f, "struct variant"),
            Self::Other(v) => write!(f, "{v}"),
        }
    }
}

#[derive(Debug)]
pub enum DeserializationError {
    Custom(Box<str>),
    InvalidType(Unexpected, Box<str>),
    InvalidValue(Unexpected, Box<str>),
    InvalidLength(usize, Box<str>),
    UnknownVariant(Box<str>, &'static [&'static str]),
    UnknownField(Box<str>, &'static [&'static str]),
    MissingField(&'static str),
    DuplicateField(&'static str),
}

impl Display for DeserializationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Custom(s) => write!(f, "{s}"),
            Self::InvalidType(v, exp) => {
                write!(f, "invalid type: {v}; expected {exp}")
            }
            Self::InvalidValue(v, exp) => {
                write!(f, "invalid value: {v}; expected {exp}")
            }
            Self::InvalidLength(len, exp) => write!(
                f,
                "invalid length: {len} {}; expected {exp}",
                if *len == 1 {
                    "element"
                } else {
                    "elements"
                },
            ),
            Self::UnknownVariant(s, exp) | Self::UnknownField(s, exp) => {
                write!(
                    f,
                    "unknown key `{}`; expected {}",
                    s.escape_default(),
                    parse::one_of(exp.iter().copied()),
                )
            }
            Self::MissingField(s) => write!(f, "missing key `{s}`"),
            Self::DuplicateField(s) => write!(f, "duplicate key `{s}`"),
        }
    }
}

impl std::error::Error for DeserializationError {}

impl de::Error for DeserializationError {
    fn custom<T: Display>(msg: T) -> Self {
        Self::Custom(msg.to_string().into())
    }

    fn invalid_type(v: de::Unexpected<'_>, exp: &dyn de::Expected) -> Self {
        Self::InvalidType(v.into(), exp.to_string().into())
    }

    fn invalid_value(v: de::Unexpected<'_>, exp: &dyn de::Expected) -> Self {
        Self::InvalidValue(v.into(), exp.to_string().into())
    }

    fn invalid_length(len: usize, exp: &dyn de::Expected) -> Self {
        Self::InvalidLength(len, exp.to_string().into())
    }

    fn unknown_variant(variant: &str, exp: &'static [&'static str]) -> Self {
        Self::UnknownVariant(variant.into(), exp)
    }

    fn unknown_field(field: &str, exp: &'static [&'static str]) -> Self {
        Self::UnknownField(field.into(), exp)
    }

    fn missing_field(field: &'static str) -> Self {
        Self::MissingField(field)
    }

    fn duplicate_field(field: &'static str) -> Self {
        Self::DuplicateField(field)
    }
}

pub struct Error {
    de: Option<DeserializationError>,
    toml: TomlError,
}

impl Error {
    pub fn new(de: Option<DeserializationError>, toml: TomlError) -> Self {
        Self {
            de,
            toml,
        }
    }

    pub fn span(&self) -> Option<Range<usize>> {
        self.toml.span()
    }

    pub fn show<W: Write>(
        &self,
        writer: &mut AnsiWriter<W>,
        input: Input<'_>,
    ) -> io::Result<()> {
        show(self, writer, input)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(e) = &self.de {
            write!(f, "{e}")
        } else {
            write!(f, "TOML syntax error")?;
            let mut lines = self.toml.message().lines();
            lines.by_ref().take(2).try_for_each(|s| write!(f, ": {s}"))?;
            lines.try_for_each(|s| write!(f, "\n{s}"))
        }
    }
}

fn show<W: Write>(
    error: &Error,
    writer: &mut AnsiWriter<W>,
    input: Input<'_>,
) -> io::Result<()> {
    let path = input.path;
    let text = input.text;
    let Some(span) = error.span().and_then(|mut span| {
        // Make sure the span ends on a char boundary.
        span.end = (span.end..text.len())
            .find(|&i| text.is_char_boundary(i))
            .unwrap_or(text.len());
        // Don't print context if the error spans the entire file, minus
        // whitespace.
        if text[..span.start]
            .bytes()
            .chain(text[span.end..].bytes())
            .all(|b| b.is_ascii_whitespace())
        {
            return None;
        }
        // Multiline spans that start at the very beginning of the file may not
        // be specific enough to be useful, so omit them.
        if span.start == 0 && text[span.clone()].lines().nth(1).is_some() {
            return None;
        }
        Some(span)
    }) else {
        writeln!(writer, "error in {}:", path.display())?;
        return writeln!(writer.with_fmt("1"), "{error}");
    };

    let (start, end) = (span.start, span.end);
    let start_lineno = text[..start].matches('\n').count() + 1;
    // Byte index of the start of the line that contains `start`.
    let start_line_i = text[..start].rfind('\n').map_or(0, |n| n + 1);
    let start_col = text[start_line_i..start].chars().count() + 1;

    let until_end = text[..end].strip_suffix('\n').unwrap_or(&text[..end]);
    let end_lineno = until_end[start..].matches('\n').count() + start_lineno;
    // Byte index of the start of the line that contains `end`.
    let end_line_i = until_end.rfind('\n').map_or(0, |n| n + 1);
    // Ensure (start..end) is nonempty so we print at least one caret.
    let end = end.max(start + 1);

    writeln!(
        writer,
        "error in {}, line {start_lineno}, column {start_col}:",
        path.display(),
    )?;
    writeln!(writer.with_fmt("1"), "{error}")?;

    // Width of left column of context (contains line numbers).
    let width = 2 + std::iter::successors(Some(end_lineno), |&n| Some(n / 10))
        .take_while(|&n| n > 0)
        .count();

    #[derive(Clone, Copy, Eq, PartialEq)]
    #[allow(clippy::enum_variant_names)]
    enum State {
        BeforeSpan,
        InSpan,
        AfterSpan,
    }

    // Write first line of context:
    write!(writer, "{start_lineno:>width$} | ")?;
    let start_line = first_line(&text[start_line_i..]);
    let mut num_space = 0;
    let mut num_caret = 0;
    // Index of the character directly after the last printed character.
    let mut next_i = start_line_i;
    let mut state = State::BeforeSpan;
    let mut wf = writer.borrow();
    for (i, c) in start_line.char_indices() {
        let abs_i = start_line_i + i;
        next_i = abs_i + c.len_utf8();
        if state == State::BeforeSpan {
            if next_i > start {
                state = State::InSpan;
                wf = writer.with_fmt("1;31");
            } else {
                num_space += write_char_sized(c, &mut wf)?;
                continue;
            }
        }
        if state == State::InSpan {
            if abs_i >= end {
                state = State::AfterSpan;
                wf = writer.borrow();
            } else {
                num_caret += write_char_sized(c, &mut wf)?;
                continue;
            }
        }
        write_char_sized(c, &mut wf)?;
    }
    write!(writer, "\n{:width$} | ", "")?;
    if (start..end).contains(&next_i) {
        num_caret += 1;
    }
    (0..num_space).try_for_each(|_| write!(writer, " "))?;
    if num_caret > 0 {
        let mut wf = writer.with_fmt("1;31");
        write!(wf, "^")?;
        (1..num_caret).try_for_each(|_| write!(wf, "~"))?;
    }
    writeln!(writer)?;

    // Write middle line of context or indicate omitted lines.
    match end_lineno - start_lineno {
        0 => return Ok(()),
        1 => {}
        2 => {
            let lineno = start_lineno + 1;
            let i = start_line_i + start_line.len() + 1;
            let line = first_line(&text[i..]);
            write!(writer, "{lineno:>width$} | ")?;
            writeln!(writer.with_fmt("1;31"), "{line}")?;
        }
        n => {
            writeln!(writer, "{:>width$} | ({} lines omitted)", "...", n - 1)?;
        }
    }

    // Write last line of context:
    write!(writer, "{end_lineno:>width$} | ")?;
    let end_line = first_line(&text[end_line_i..]);
    let mut num_caret = 0;
    // Index of the character directly after the last printed character.
    let mut next_i = end_line_i;
    let mut state = State::InSpan;
    let mut wf = writer.with_fmt("1;31");
    for (i, c) in end_line.char_indices() {
        let abs_i = end_line_i + i;
        next_i = abs_i + c.len_utf8();
        if state == State::InSpan {
            if abs_i >= end {
                state = State::AfterSpan;
                wf = writer.borrow();
            } else {
                num_caret += write_char_sized(c, &mut wf)?;
                continue;
            }
        }
        write_char_sized(c, &mut wf)?;
    }
    write!(writer, "\n{:width$} | ", "")?;
    if (start..end).contains(&next_i) {
        num_caret += 1;
    }
    if num_caret > 0 {
        let mut wf = writer.with_fmt("1;31");
        (0..num_caret).try_for_each(|_| write!(wf, "~"))?;
    }
    writeln!(writer)?;
    writer.finalize()
}
