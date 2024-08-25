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

use super::def;
use super::{CompileCfg, Control, Optional};
use crate::common::{Channel, MidiValue};
use crate::parse;
use serde::de::{self, Deserializer};
use serde::Deserialize;
use std::fmt;
use std::io::{self, Write};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

impl<'a> Deserialize<'a> for Orientation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        struct Visitor;

        impl<'a> de::Visitor<'a> for Visitor {
            type Value = Orientation;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "\"horizontal\" or \"vertical\"")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match v {
                    "horizontal" => Ok(Orientation::Horizontal),
                    "vertical" => Ok(Orientation::Vertical),
                    _ => Err(E::invalid_value(de::Unexpected::Str(v), &self)),
                }
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    Unipolar,
    Bipolar,
}

impl<'a> Deserialize<'a> for Mode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        struct Visitor;

        impl<'a> de::Visitor<'a> for Visitor {
            type Value = Mode;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "\"unipolar\" or \"bipolar\"")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match v {
                    "unipolar" => Ok(Mode::Unipolar),
                    "bipolar" => Ok(Mode::Bipolar),
                    _ => Err(E::invalid_value(de::Unexpected::Str(v), &self)),
                }
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Fader {
    pub orientation: Orientation,
    pub mode: Mode,
    pub color: MidiValue,
    pub cc: MidiValue,
    pub channel: Channel,
}

impl Control for Fader {
    fn compile<W: Write>(
        &self,
        address: u8,
        writer: &mut W,
        config: &CompileCfg,
    ) -> io::Result<()> {
        def::Definition {
            address,
            color: self.color,
            opt: def::Opt::builder(config)
                .channel(self.channel)
                .pad_fader(self.mode)
                .done(),
            payload: def::Payload::CcFull(self.cc),
        }
        .compile(writer)
    }
}

impl<'a> Deserialize<'a> for Optional<Fader> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "kebab-case")]
        #[serde(expecting = "key")]
        enum Field {
            Orientation,
            Mode,
            Color,
            Cc,
            Channel,
        }

        struct Visitor;

        impl<'a> de::Visitor<'a> for Visitor {
            type Value = Optional<Fader>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "horizontal/vertical fader definition (table)")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'a>,
            {
                let mut empty = true;
                let mut orientation = None;
                let mut mode = None;
                let mut color = None;
                let mut cc = None;
                let mut channel = None;
                while let Some(field) = map.next_key()? {
                    empty = false;
                    match field {
                        Field::Orientation => {
                            parse::check_dup(&orientation, "orientation")?;
                            orientation = Some(map.next_value()?);
                        }
                        Field::Mode => {
                            parse::check_dup(&mode, "mode")?;
                            mode = Some(map.next_value()?);
                        }
                        Field::Color => {
                            parse::check_dup(&color, "color")?;
                            color = Some(map.next_value()?);
                        }
                        Field::Cc => {
                            parse::check_dup(&cc, "cc")?;
                            cc = Some(map.next_value()?);
                        }
                        Field::Channel => {
                            parse::check_dup(&channel, "channel")?;
                            channel = Some(map.next_value()?);
                        }
                    }
                }
                if empty {
                    return Ok(Optional::None);
                }
                let missing = de::Error::missing_field;
                Ok(Optional::Some(Fader {
                    orientation: orientation
                        .ok_or_else(|| missing("orientation"))?,
                    mode: mode.unwrap_or(Mode::Unipolar),
                    color: color.ok_or_else(|| missing("color"))?,
                    cc: cc.ok_or_else(|| missing("cc"))?,
                    channel: channel.unwrap_or(Channel::Global),
                }))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Optional::None)
            }

            fn visit_some<D>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'a>,
            {
                deserializer.deserialize_map(self)
            }
        }

        deserializer.deserialize_map(Visitor)
    }
}
