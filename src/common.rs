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

use super::parse::bounded::BoundedU8;
use super::parse::config::DeserializeConfig;
use super::parse::error::IgnoredError;
use super::parse::primitive;
use serde::Deserialize;
use serde::de::value::MapAccessDeserializer;
use serde::de::{self, Deserializer, IntoDeserializer};
use std::fmt::{self, Debug, Display};
use std::io::{self, Write};
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(from = "BoundedU8<1, 16>")]
pub struct MidiChannel(u8);

impl MidiChannel {
    pub const fn new(channel: u8) -> Option<Self> {
        match channel {
            1..=16 => Some(Self(channel - 1)),
            _ => None,
        }
    }

    pub fn number(self) -> u8 {
        self.0 + 1
    }

    pub fn raw_value(self) -> u8 {
        self.0
    }
}

impl From<BoundedU8<1, 16>> for MidiChannel {
    fn from(v: BoundedU8<1, 16>) -> Self {
        Self(v.get() - 1)
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(from = "BoundedU8<0, 127>")]
pub struct MidiValue(u8);

impl MidiValue {
    pub const MIN: Self = Self(0);
    pub const MAX: Self = Self(127);

    pub const fn new(value: u8) -> Option<Self> {
        if value <= 127 {
            Some(Self(value))
        } else {
            None
        }
    }

    pub fn value(self) -> u8 {
        self.0
    }
}

impl From<BoundedU8<0, 127>> for MidiValue {
    fn from(v: BoundedU8<0, 127>) -> Self {
        Self(v.get())
    }
}

#[derive(Clone, Copy, Debug)]
struct NoteName(pub MidiValue);

impl FromStr for NoteName {
    type Err = NoteNameParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();
        let key = match chars.next() {
            Some('c' | 'C') => 0,
            Some('d' | 'D') => 2,
            Some('e' | 'E') => 4,
            Some('f' | 'F') => 5,
            Some('g' | 'G') => 7,
            Some('a' | 'A') => 9,
            Some('b' | 'B') => 11,
            _ => return Err(NoteNameParseError),
        };
        let offset = match chars.clone().next() {
            Some('#') => 1,
            Some('b') => -1,
            _ => 0,
        };
        if offset != 0 {
            chars.next();
        }
        let octave: i8 =
            chars.as_str().parse().map_err(|_| NoteNameParseError)?;
        let pitch = 12 * (i16::from(octave) + 1) + key + offset;
        u8::try_from(pitch)
            .ok()
            .and_then(MidiValue::new)
            .map(Self)
            .ok_or(NoteNameParseError)
    }
}

#[derive(Clone, Copy, Debug)]
struct NoteNameParseError;

impl Display for NoteNameParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid note name")
    }
}

impl<'a> Deserialize<'a> for NoteName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        struct Visitor;

        impl<'a> de::Visitor<'a> for Visitor {
            type Value = NoteName;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "\"C-1\" to \"G9\"")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                v.parse().map_err(|_| {
                    E::invalid_value(de::Unexpected::Str(v), &self)
                })
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MidiNote(pub MidiValue);

impl MidiNote {
    pub fn value(self) -> u8 {
        self.0.value()
    }
}

impl From<NoteName> for MidiNote {
    fn from(n: NoteName) -> Self {
        Self(n.0)
    }
}

impl<'a> Deserialize<'a> for MidiNote {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        struct Visitor;

        impl<'a> de::Visitor<'a> for Visitor {
            type Value = MidiNote;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "0-127 or \"C-1\" to \"G9\"")
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                MidiValue::deserialize(v.into_deserializer()).map(MidiNote)
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match v.try_into() {
                    Ok(v) => self.visit_u64(v),
                    Err(_) => {
                        Err(E::invalid_type(de::Unexpected::Signed(v), &self))
                    }
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match NoteName::deserialize(v.into_deserializer()) {
                    Ok(name) => Ok(name.into()),
                    Err(IgnoredError) => {
                        Err(E::invalid_value(de::Unexpected::Str(v), &self))
                    }
                }
            }
        }

        deserializer.deserialize_u8(Visitor)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Velocity {
    Fixed(MidiValue),
    Variable,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VelocityCfg {
    FixedOnly,
    VariableOnly,
    Any,
}

impl VelocityCfg {
    pub const fn fixed_allowed(self) -> bool {
        matches!(self, Self::FixedOnly | Self::Any)
    }

    pub const fn variable_allowed(self) -> bool {
        matches!(self, Self::VariableOnly | Self::Any)
    }
}

impl<'a> DeserializeConfig<'a, VelocityCfg> for Velocity {
    fn deserialize<D>(
        deserializer: D,
        config: &VelocityCfg,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        struct Visitor<'a> {
            cfg: &'a VelocityCfg,
        }

        impl<'a> de::Visitor<'a> for Visitor<'_> {
            type Value = Velocity;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self.cfg {
                    VelocityCfg::FixedOnly => {
                        write!(f, "an integer between 0 and 127")
                    }
                    VelocityCfg::VariableOnly => {
                        write!(f, "\"variable\"")
                    }
                    VelocityCfg::Any => {
                        write!(f, "0-127 or \"variable\"")
                    }
                }
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if self.cfg.fixed_allowed() {
                    MidiValue::deserialize(v.into_deserializer())
                        .map(Velocity::Fixed)
                } else {
                    Err(E::invalid_type(de::Unexpected::Unsigned(v), &self))
                }
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match v.try_into() {
                    Ok(v) => self.visit_u64(v),
                    Err(_) => {
                        Err(E::invalid_type(de::Unexpected::Signed(v), &self))
                    }
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if !self.cfg.variable_allowed() {
                    Err(E::invalid_type(de::Unexpected::Str(v), &self))
                } else if v == "variable" {
                    Ok(Velocity::Variable)
                } else {
                    Err(E::invalid_value(de::Unexpected::Str(v), &self))
                }
            }
        }

        deserializer.deserialize_any(Visitor {
            cfg: config,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Channel {
    Fixed(MidiChannel),
    Global,
}

impl<'a> Deserialize<'a> for Channel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        struct Visitor;

        impl<'a> de::Visitor<'a> for Visitor {
            type Value = Channel;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "1-16 or \"global\"")
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                MidiChannel::deserialize(v.into_deserializer())
                    .map(Channel::Fixed)
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match v.try_into() {
                    Ok(v) => self.visit_u64(v),
                    Err(_) => {
                        Err(E::invalid_type(de::Unexpected::Signed(v), &self))
                    }
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if v == "global" {
                    Ok(Channel::Global)
                } else {
                    Err(E::invalid_value(de::Unexpected::Str(v), &self))
                }
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

#[derive(Clone, Copy)]
pub struct Name {
    bytes: [u8; Self::MAX_LEN],
    len: u8,
}

impl Name {
    pub const MAX_LEN: usize = 16;

    pub fn empty() -> Self {
        Self {
            bytes: Default::default(),
            len: 0,
        }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes[..self.len.into()]
    }

    pub fn as_str(&self) -> &str {
        debug_assert!(self.bytes.is_ascii());
        // SAFETY: Contents are always ASCII.
        unsafe { std::str::from_utf8_unchecked(self.bytes()) }
    }

    pub fn compile<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: Write,
    {
        writer.write_all(&[self.len])?;
        writer.write_all(self.bytes())
    }
}

impl Debug for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}

impl Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl<'a> Deserialize<'a> for Name {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        struct Visitor;

        impl<'a> de::Visitor<'a> for Visitor {
            type Value = Name;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "ASCII string, at most {} characters", Name::MAX_LEN)
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                v.parse()
                    .map_err(|e| E::invalid_value(de::Unexpected::Str(v), &e))
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl FromStr for Name {
    type Err = NameParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.is_ascii() {
            return Err(NameParseError::NonAscii);
        }
        if s.len() > 16 {
            return Err(NameParseError::Overflow);
        }
        let mut bytes = [0; 16];
        bytes[..s.len()].copy_from_slice(s.as_bytes());
        Ok(Self {
            bytes,
            len: s.len() as _,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum NameParseError {
    Overflow,
    NonAscii,
}

impl Display for NameParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overflow => {
                write!(f, "too long (max {} characters)", Name::MAX_LEN)
            }
            Self::NonAscii => write!(f, "must be ASCII"),
        }
    }
}

impl de::Expected for NameParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overflow => {
                write!(f, "no more than {} characters", Name::MAX_LEN)
            }
            Self::NonAscii => write!(f, "only ASCII characters"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Keypress {
    /// USB HID keycode.
    pub code: u8,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl<'a> Deserialize<'a> for Keypress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        #[serde(expecting = "keypress definition (table)")]
        struct Fields {
            #[serde(deserialize_with = "primitive::deserialize")]
            keycode: u8,
            #[serde(default, deserialize_with = "primitive::deserialize")]
            ctrl: bool,
            #[serde(default, deserialize_with = "primitive::deserialize")]
            shift: bool,
            #[serde(default, deserialize_with = "primitive::deserialize")]
            alt: bool,
        }

        impl From<Fields> for Keypress {
            fn from(f: Fields) -> Self {
                Self {
                    code: f.keycode,
                    ctrl: f.ctrl,
                    shift: f.shift,
                    alt: f.alt,
                }
            }
        }

        struct Visitor;

        impl<'a> de::Visitor<'a> for Visitor {
            type Value = Keypress;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "0-255 or map with keycode and modifiers")
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Keypress {
                    code: primitive::deserialize(v.into_deserializer())?,
                    ctrl: false,
                    shift: false,
                    alt: false,
                })
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match v.try_into() {
                    Ok(v) => self.visit_u64(v),
                    Err(_) => {
                        Err(E::invalid_type(de::Unexpected::Signed(v), &self))
                    }
                }
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'a>,
            {
                Fields::deserialize(MapAccessDeserializer::new(map))
                    .map(Into::into)
            }
        }

        deserializer.deserialize_map(Visitor)
    }
}
