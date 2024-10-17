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

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(clippy::undocumented_unsafe_blocks)]

use serde::de::value::MapAccessDeserializer;
use serde::de::{self, Deserialize, DeserializeSeed, Deserializer};
use std::fmt::{self, Display};
use std::fs::File;
use std::io::{self, BufWriter, IsTerminal, Write};
use std::path::Path;
use std::process::ExitCode;
use std::str::FromStr;

mod ansi;
mod args;
mod common;
mod controls;
mod error;
mod launchkey;
mod launchpad;
mod parse;

use ansi::AnsiWriter;
use error::Error;
use launchkey::flkey as flk;
use launchkey::flkey_mini as flkm;
use launchkey::launchkey_mini_mk3 as lkmm3;
use launchkey::launchkey_mk3 as lkm3;
use launchpad::launchpad_mini_mk3 as lpmm3;
use launchpad::launchpad_x as lpx;
use parse::with_error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Version {
    Two,
}

impl<'a> Deserialize<'a> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        #[derive(Clone, Copy)]
        struct Unsupported(u64);

        impl Display for Unsupported {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "unsupported version `{}`: ", self.0)?;
                write!(f, "version must be 2")
            }
        }

        struct Visitor;

        impl<'a> de::Visitor<'a> for Visitor {
            type Value = Version;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "version number (integer)")
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match v {
                    2 => Ok(Version::Two),
                    _ => Err(E::custom(Unsupported(v))),
                }
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match v.try_into() {
                    Ok(v) => self.visit_u64(v),
                    Err(_) => Err(E::invalid_type(
                        de::Unexpected::Signed(v),
                        &"positive version number",
                    )),
                }
            }
        }

        deserializer.deserialize_u32(Visitor)
    }
}

macro_rules! define_devices {
    () => {
        define_devices! {
            impl
            LaunchkeyMiniMk3Pads("launchkey-mini-mk3-pads", lkmm3::PadMap),
            LaunchkeyMiniMk3Pots("launchkey-mini-mk3-pots", lkmm3::PotMap),
            LaunchkeyMiniMk3Pedal("launchkey-mini-mk3-pedal", lkmm3::PedalMap),
            LaunchkeyMk3Pads("launchkey-mk3-pads", lkm3::PadMap),
            LaunchkeyMk3Pots("launchkey-mk3-pots", lkm3::PotMap),
            LaunchkeyMk3Pedal("launchkey-mk3-pedal", lkm3::PedalMap),
            LaunchkeyMk3Faders("launchkey-mk3-faders", lkm3::FaderMap),
            LaunchpadMiniMk3("launchpad-mini-mk3", lpmm3::Map),
            LaunchpadX("launchpad-x", lpx::Map),
            FlkeyMiniPads("flkey-mini-pads", flkm::PadMap),
            FlkeyMiniPots("flkey-mini-pots", flkm::PotMap),
            FlkeyMiniPedal("flkey-mini-pedal", flkm::PedalMap),
            FlkeyPads("flkey-pads", flk::PadMap),
            FlkeyPots("flkey-pots", flk::PotMap),
            FlkeyPedal("flkey-pedal", flk::PedalMap),
            FlkeyFaders("flkey-faders", flk::FaderMap)
        }
    };

    (impl $($variant:ident($str:literal, $map:ty)),*) => {
        #[derive(Clone, Copy, Debug)]
        enum Device {
            $($variant,)*
        }

        impl FromStr for Device {
            type Err = ();

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($str => Ok(Self::$variant),)*
                    _ => Err(()),
                }
            }
        }

        impl<'a> DeserializeSeed<'a> for Device {
            type Value = CustomMode;

            fn deserialize<D>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'a>,
            {
                match self {
                    $(Self::$variant => Deserialize::deserialize(deserializer)
                        .map(CustomMode::$variant),)*
                }
            }
        }

        #[derive(Clone, Copy, Debug)]
        enum CustomMode {
            $($variant($map),)*
        }

        impl CustomMode {
            pub fn compile<W>(&self, writer: &mut W) -> io::Result<()>
            where
                W: Write,
            {
                match self {
                    $(Self::$variant(m) => m.compile(writer),)*
                }
            }
        }
    };
}

define_devices!();

impl<'a> Deserialize<'a> for Device {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        #[derive(Clone, Copy)]
        struct Unknown<'a>(&'a str);

        impl Display for Unknown<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "unknown device \"{}\"", self.0)?;
                let suffixes: &[_] = match self.0 {
                    "launchkey-mini-mk3" | "flkey-mini" => {
                        &["pads", "pots", "pedal"]
                    }
                    "launchkey-mk3" | "flkey" => {
                        &["pads", "pots", "faders", "pedal"]
                    }
                    _ => return Ok(()),
                };
                write!(f, "; did you mean one of the following?")?;
                for suffix in suffixes {
                    write!(f, "\n* {}-{suffix}", self.0)?;
                }
                Ok(())
            }
        }

        struct Visitor;

        impl<'a> de::Visitor<'a> for Visitor {
            type Value = Device;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "device name (string)")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                v.parse().map_err(|_| E::custom(Unknown(v)))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl<'a> Deserialize<'a> for CustomMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        #[derive(Clone, Copy)]
        struct UnexpectedField<'a> {
            unexp: &'a str,
            exp: &'static str,
        }

        impl Display for UnexpectedField<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    f,
                    "unexpected key `{}`; expected `{}`",
                    self.unexp, self.exp,
                )
            }
        }

        struct FieldSeed(&'static str);

        impl<'a> de::Visitor<'a> for FieldSeed {
            type Value = ();

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "key")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<(), E> {
                if v == self.0 {
                    Ok(())
                } else {
                    Err(E::custom(UnexpectedField {
                        unexp: v,
                        exp: self.0,
                    }))
                }
            }
        }

        impl<'a> DeserializeSeed<'a> for FieldSeed {
            type Value = ();

            fn deserialize<D>(self, deserializer: D) -> Result<(), D::Error>
            where
                D: Deserializer<'a>,
            {
                deserializer.deserialize_str(self)
            }
        }

        struct Visitor;

        impl<'a> de::Visitor<'a> for Visitor {
            type Value = CustomMode;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "version and device")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'a>,
            {
                map.next_key_seed(FieldSeed("version"))?
                    .ok_or_else(|| de::Error::missing_field("version"))?;
                let version: Version = map.next_value()?;
                assert_eq!(version, Version::Two);
                map.next_key_seed(FieldSeed("device"))?
                    .ok_or_else(|| de::Error::missing_field("device"))?;
                let device: Device = map.next_value()?;
                device.deserialize(MapAccessDeserializer::new(map))
            }
        }

        deserializer.deserialize_map(Visitor)
    }
}

#[derive(Clone, Copy)]
struct Input<'a> {
    pub path: &'a Path,
    pub text: &'a str,
}

fn run() -> Result<(), ()> {
    use args::{Args, PathArg, Usage};
    let mut args = std::env::args_os();
    let arg0 = args.next();
    let bin = arg0
        .as_ref()
        .and_then(|s| Path::new(s).file_name()?.to_str())
        .unwrap_or("ncc");

    let usage = Usage::new(bin);
    let args = Args::parse(args).map_err(|e| {
        eprintln!("error: {e}");
        eprintln!("See `{bin} --help` for usage information.");
    })?;
    let args = match args {
        Args::Empty => {
            eprintln!("{usage}");
            return Err(());
        }
        Args::Help => {
            println!("{usage}");
            return Ok(());
        }
        Args::Version => {
            println!("{}", args::Version::new());
            return Ok(());
        }
        Args::Compile(a) => a,
    };

    let in_canon = args.in_path.try_canonicalize();
    let out_canon = args.out_path.try_canonicalize();
    if match (&in_canon, &out_canon) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    } {
        eprintln!("error: input file and output file are the same");
        eprintln!("input would be overwritten by output");
        return Err(());
    }

    let text = match &args.in_path {
        PathArg::Stdio => {
            io::read_to_string(io::stdin().lock()).map_err(|e| {
                eprintln!("error: could not read from stdin: {e}");
            })?
        }
        PathArg::Path(p) => std::fs::read_to_string(p).map_err(|e| {
            eprintln!("error: could not read `{}`: {e}", p.display());
        })?,
    };
    let input = Input {
        path: match &args.in_path {
            PathArg::Stdio => "<stdin>".as_ref(),
            PathArg::Path(p) => p,
        },
        text: &text,
    };

    let result = with_error::deserialize(toml::Deserializer::new(&text));
    let mode: CustomMode = result.map_err(|(err, toml_err)| {
        let stderr = io::stderr().lock();
        let mode = if cfg!(unix) && stderr.is_terminal() {
            ansi::Mode::Fancy
        } else {
            ansi::Mode::Plain
        };
        let mut w = AnsiWriter::new(BufWriter::new(stderr), mode);
        Error::new(err, toml_err)
            .show(&mut w, input)
            .and_then(|_| w.flush())
            .expect("error writing to stderr");
    })?;

    match &args.out_path {
        PathArg::Stdio => {
            let mut w = BufWriter::new(io::stdout().lock());
            mode.compile(&mut w).and_then(|_| w.flush()).map_err(|e| {
                eprintln!("error writing to stdout: {e}");
            })
        }
        PathArg::Path(p) => {
            let f = File::create(p).map_err(|e| {
                eprintln!("error: could not create `{}`: {e}", p.display());
            })?;
            let mut w = BufWriter::new(f);
            mode.compile(&mut w).and_then(|_| w.flush()).map_err(|e| {
                eprintln!("error writing to `{}`: {e}", p.display());
            })
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
    }
}

/// This function silences dead code warnings for items that are an intentional
/// part of the public interface of a module or type, but are currently unused
/// in this crate.
///
/// The advantage of silencing the warnings this way instead of adding
/// `#[allow(dead_code)]` to each of the items is that unused code in the
/// private implementation of each item will still be detected.
#[allow(dead_code, unused_imports)]
fn allow_unused<T>()
where
    for<'a> T: Deserialize<'a> + Deserializer<'a> + Write,
    for<'a> T: parse::wrap::ValueWrapper<'a, T>,
{
    let _ = ansi::AnsiWriter::<T>::fancy;
    let _ = ansi::AnsiWriter::<T>::into_inner;
    let _ = ansi::FmtWriter::<T>::borrow;
    let _ = ansi::FmtWriter::<T>::with_fmt;
    let _ = common::MidiChannel::new;
    let _ = common::MidiChannel::number;
    let _ = parse::bounded::BoundedU16::<0, 0>::get;
    let _ = parse::bounded::BoundedU32::<0, 0>::get;
    let _ = parse::bounded::BoundedU64::<0, 0>::get;
    let _ = parse::error::WrappedError::<T, T>::transpose::<T>;
    let _ = parse::slice::deserialize::<T, T>;
    let _ = parse::try_none::<T>;
    let _ = parse::wrap::deserialize::<T, T, T>;
    use crate::controls::{Pad as _, Pot as _};
}
