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

use std::ffi::{OsStr, OsString};
use std::fmt::{self, Display};
use std::ops::ControlFlow;
use std::path::PathBuf;

const USAGE: &str = "\
[options] <input>

Compiles the custom mode in the TOML file <input> and writes the
resulting MIDI SysEx message to a file.

By default, if the input filename ends in `.toml`, the output filename
is obtained by replacing `.toml` with `.syx`. Otherwise, `.syx` is
appended to the filename.

Options:
  -o <file>      Write the compiled SysEx to <file>
  -h, --help     Show this help message
  -v, --version  Show program version
";

#[derive(Debug)]
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

const VERSION_MESSAGE: &str = "\
Copyright (C) 2024-2025 taylor.fish <contact@taylor.fish>
Licensed under the GNU Affero General Public License, version 3 or
later; see <https://www.gnu.org/licenses/>. There is NO WARRANTY;
see the license for details.
";

#[derive(Debug)]
pub struct Version(());

impl Version {
    pub fn new() -> Self {
        Self(())
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ncc {}", env!("CARGO_PKG_VERSION"))?;
        write!(f, "{}", VERSION_MESSAGE.trim_end())
    }
}

#[derive(Debug)]
pub enum PathArg {
    Stdio,
    Path(PathBuf),
}

impl PathArg {
    pub fn try_canonicalize(&self) -> Option<PathBuf> {
        if let Self::Path(p) = self {
            p.canonicalize().ok()
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct CompileArgs {
    pub in_path: PathArg,
    pub out_path: PathArg,
}

#[derive(Debug)]
pub enum Args {
    /// Program was invoked without any arguments.
    Empty,
    /// `-h` or `--help` was present.
    Help,
    /// `-v` or `--version` was present.
    Version,
    /// Typical usage: compile input, write to output.
    Compile(CompileArgs),
}

impl Args {
    const NUM_POSITIONAL: usize = 1;

    pub fn parse<A>(args: A) -> Result<Self, ArgsError>
    where
        A: IntoIterator<Item = OsString>,
    {
        Parser {
            args: args.into_iter(),
            options_done: false,
            num_positional: 0,
            in_path: None,
            out_path: None,
        }
        .parse()
    }
}

#[derive(Debug)]
pub enum ArgsError {
    UnknownShort(char),
    DuplicateShort(char),
    IncompleteShort(char),
    NonAsciiShort(OsString),
    UnknownLong(OsString),
    Unexpected(OsString),
    MissingArgs,
    MissingStdinOutput,
}

impl Display for ArgsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownShort(c) => write!(f, "unknown option: -{c}"),
            Self::DuplicateShort(c) => write!(f, "duplicate option: -{c}"),
            Self::IncompleteShort(c) => {
                write!(f, "missing argument for option -{c}")
            }
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
            Self::MissingStdinOutput => {
                write!(f, "-o must be specified when input is `-`")
            }
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

#[derive(Clone, Copy, Eq, PartialEq)]
enum NextShort {
    #[allow(dead_code)]
    /// Treat the next character in the argument as another short option.
    NextChar,
    /// Skip the rest of the current argument.
    SkipRest,
}

struct Parser<A> {
    args: A,
    options_done: bool,
    num_positional: usize,
    in_path: Option<PathArg>,
    out_path: Option<PathArg>,
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
        debug_assert_eq!(self.num_positional, Args::NUM_POSITIONAL);
        let in_path = self.in_path.unwrap();
        let out_path = if let Some(p) = self.out_path {
            p
        } else {
            let PathArg::Path(p) = &in_path else {
                return Err(ArgsError::MissingStdinOutput);
            };
            PathArg::Path(if p.extension().is_some_and(|x| x == "toml") {
                p.with_extension("syx")
            } else {
                let mut out = p.clone();
                out.as_mut_os_string().push(".syx");
                out
            })
        };
        Ok(Args::Compile(CompileArgs {
            in_path,
            out_path,
        }))
    }

    fn to_path<S>(&self, s: S) -> PathArg
    where
        S: Into<PathBuf> + for<'a> PartialEq<&'a str>,
    {
        if s == "-" && !self.options_done {
            PathArg::Stdio
        } else {
            PathArg::Path(s.into())
        }
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
            0 => self.in_path = Some(self.to_path(arg)),
            _ => return ArgsError::Unexpected(arg).into(),
        }
        self.num_positional += 1;
        ControlFlow::Continue(())
    }

    fn short_opts(&mut self, opts: OsString) -> ControlFlow<ArgsResult> {
        let bytes = opts.as_encoded_bytes();
        for (i, b) in bytes.iter().copied().enumerate().skip(1) {
            if !b.is_ascii() {
                return ArgsError::NonAsciiShort(opts).into();
            }
            // SAFETY: Because `b` is ASCII, it is necessarily a valid
            // non-empty UTF-8 substring, so it is safe to split the string
            // immediately after this byte.
            let rest = unsafe {
                OsStr::from_encoded_bytes_unchecked(&bytes[i + 1..])
            };
            if self.short_opt(b.into(), rest)? == NextShort::SkipRest {
                break;
            }
        }
        ControlFlow::Continue(())
    }

    fn short_opt(
        &mut self,
        opt: char,
        rest: &OsStr,
    ) -> ControlFlow<ArgsResult, NextShort> {
        match opt {
            'o' => {
                if self.out_path.is_some() {
                    return ArgsError::DuplicateShort(opt).into();
                }
                let arg = if !rest.is_empty() {
                    rest.to_owned()
                } else if let Some(arg) = self.args.next() {
                    arg
                } else {
                    return ArgsError::IncompleteShort(opt).into();
                };
                self.out_path = Some(self.to_path(arg));
                ControlFlow::Continue(NextShort::SkipRest)
            }
            'h' => Args::Help.into(),
            'v' => Args::Version.into(),
            _ => ArgsError::UnknownShort(opt).into(),
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
