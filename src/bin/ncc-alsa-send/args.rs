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

use std::ffi::OsString;
use std::fmt::{self, Display};
use std::ops::ControlFlow;

const USAGE: &str = "\
<port> <file>

Sends the MIDI SysEx message in <file> to the ALSA MIDI device on port <port>.

Options:
  -h, --help     Show this help message
  -v, --version  Show program version
";

pub struct Usage<'a> {
    bin: &'a str,
}

impl<'a> Usage<'a> {
    pub fn new(bin: &'a str) -> Self {
        Self {
            bin,
        }
    }
}

impl Display for Usage<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Usage: {} {}", self.bin, USAGE.trim_end())
    }
}

pub enum Args {
    Empty,
    Help,
    Version,
    Run {
        port: OsString,
        file: OsString,
    },
}

impl Args {
    pub const NUM_POSITIONAL: usize = 2;

    pub fn parse<A>(args: A) -> Result<Self, ArgsError>
    where
        A: IntoIterator<Item = OsString>,
    {
        Parser {
            args: args.into_iter(),
            options_done: false,
            num_positional: 0,
            port: None,
            file: None,
        }
        .parse()
    }
}

pub enum ArgsError {
    UnknownShort(char),
    NonAsciiShort(OsString),
    UnknownLong(OsString),
    Unexpected(OsString),
    MissingArgs,
}

impl Display for ArgsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownShort(c) => write!(f, "unknown option: -{c}"),
            Self::NonAsciiShort(s) | Self::UnknownLong(s) => {
                write!(f, "unknown option: {}", s.to_string_lossy())
            }
            Self::Unexpected(s) => {
                write!(f, "unexpected argument: {}", s.to_string_lossy())
            }
            Self::MissingArgs => write!(
                f,
                "missing arguments (expected {})",
                Args::NUM_POSITIONAL,
            ),
        }
    }
}

type ArgsResult = Result<Args, ArgsError>;

impl<C> From<Args> for ControlFlow<ArgsResult, C> {
    fn from(a: Args) -> Self {
        Self::Break(Ok(a))
    }
}

impl<C> From<ArgsError> for ControlFlow<ArgsResult, C> {
    fn from(e: ArgsError) -> Self {
        Self::Break(Err(e))
    }
}

struct Parser<A> {
    args: A,
    options_done: bool,
    num_positional: usize,
    port: Option<OsString>,
    file: Option<OsString>,
}

impl<A: Iterator<Item = OsString>> Parser<A> {
    fn parse(mut self) -> ArgsResult {
        let mut any = false;
        while let Some(arg) = self.args.next() {
            any = true;
            if let ControlFlow::Break(r) = self.arg(arg) {
                return r;
            }
        }
        if !any {
            return Ok(Args::Empty);
        }
        if self.num_positional < Args::NUM_POSITIONAL {
            return Err(ArgsError::MissingArgs);
        }
        Ok(Args::Run {
            port: self.port.unwrap(),
            file: self.file.unwrap(),
        })
    }

    fn arg(&mut self, arg: OsString) -> ControlFlow<ArgsResult> {
        let bytes = arg.as_encoded_bytes();
        if self.options_done || arg == "-" {
        } else if bytes.starts_with(b"--") {
            return self.long_opt(arg);
        } else if bytes.starts_with(b"-") {
            return self.short_opts(arg);
        }
        match self.num_positional {
            0 => self.port = Some(arg),
            1 => self.file = Some(arg),
            _ => return ArgsError::Unexpected(arg).into(),
        }
        self.num_positional += 1;
        ControlFlow::Continue(())
    }

    fn short_opts(&mut self, opts: OsString) -> ControlFlow<ArgsResult> {
        // `opts` starts with '-', so the first short option is at index 1.
        if let Some(&b) = opts.as_encoded_bytes().get(1) {
            if !b.is_ascii() {
                return ArgsError::NonAsciiShort(opts).into();
            }
            match char::from(b) {
                'h' => Args::Help.into(),
                'v' => Args::Version.into(),
                c => ArgsError::UnknownShort(c).into(),
            }
        } else {
            ControlFlow::Continue(())
        }
    }

    fn long_opt(&mut self, opt: OsString) -> ControlFlow<ArgsResult> {
        match opt.to_str().unwrap_or("") {
            "--" => {
                self.options_done = true;
                ControlFlow::Continue(())
            }
            "--help" => Args::Help.into(),
            "--version" => Args::Version.into(),
            _ => ArgsError::UnknownLong(opt).into(),
        }
    }
}
