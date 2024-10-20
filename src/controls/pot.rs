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
use crate::parse::config::{ConfigSeed, DeserializeConfig};
use serde::Deserialize;
use serde::de::{self, Deserializer};
use std::fmt;
use std::io::{self, Write};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Pickup {
    Global,
    Yes,
    No,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PickupCfg {
    GlobalAllowed,
    BinaryOnly,
}

impl PickupCfg {
    pub fn global_allowed(self) -> bool {
        self == Self::GlobalAllowed
    }
}

impl<'a> DeserializeConfig<'a, PickupCfg> for Pickup {
    fn deserialize<D>(
        deserializer: D,
        config: &PickupCfg,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        struct Visitor<'a> {
            cfg: &'a PickupCfg,
        }

        impl<'a> de::Visitor<'a> for Visitor<'_> {
            type Value = Pickup;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                if self.cfg.global_allowed() {
                    write!(f, "true, false, or \"global\"")
                } else {
                    write!(f, "`true` or `false`")
                }
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(match v {
                    true => Pickup::Yes,
                    false => Pickup::No,
                })
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match v {
                    "global" if self.cfg.global_allowed() => {
                        Ok(Pickup::Global)
                    }
                    "true" => Ok(Pickup::Yes),
                    "false" => Ok(Pickup::No),
                    _ => Err(E::invalid_value(de::Unexpected::Str(v), &self)),
                }
            }
        }

        deserializer.deserialize_str(Visitor {
            cfg: config,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Pot {
    pub channel: Channel,
    pub cc: MidiValue,
    pub min: MidiValue,
    pub max: MidiValue,
    pub pickup: Pickup,
}

impl Control for Pot {
    fn compile<W: Write>(
        &self,
        address: u8,
        writer: &mut W,
        config: &CompileCfg,
    ) -> io::Result<()> {
        def::Definition {
            address,
            color: MidiValue::MIN,
            opt: def::Opt::builder(config)
                .channel(self.channel)
                .pickup(self.pickup)
                .done(),
            payload: def::Payload::Cc {
                number: self.cc,
                min: self.min,
                max: self.max,
            },
        }
        .compile(writer)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PotCfg {
    pickup: PickupCfg,
    name: &'static str,
}

impl PotCfg {
    pub const fn new() -> Self {
        Self {
            pickup: PickupCfg::BinaryOnly,
            name: "pot",
        }
    }

    pub const fn pickup(mut self, cfg: PickupCfg) -> Self {
        self.pickup = cfg;
        self
    }

    pub const fn name(mut self, name: &'static str) -> Self {
        self.name = name;
        self
    }
}

impl<'a> DeserializeConfig<'a, PotCfg> for Pot {
    fn deserialize<D>(
        deserializer: D,
        config: &PotCfg,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        match DeserializeConfig::deserialize(deserializer, &OptionalPotCfg {
            required: true,
            pot: config,
        })? {
            Optional::None => unreachable!(),
            Optional::Some(pot) => Ok(pot),
        }
    }
}

struct OptionalPotCfg<'a> {
    required: bool,
    pot: &'a PotCfg,
}

impl OptionalPotCfg<'_> {
    fn pickup(&self) -> &PickupCfg {
        &self.pot.pickup
    }
}

impl<'a> DeserializeConfig<'a, OptionalPotCfg<'_>> for Optional<Pot> {
    fn deserialize<D>(
        deserializer: D,
        config: &OptionalPotCfg,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "kebab-case")]
        #[serde(expecting = "key")]
        enum Field {
            Channel,
            Cc,
            Min,
            Max,
            Pickup,
        }

        struct Visitor<'a> {
            cfg: &'a OptionalPotCfg<'a>,
        }

        impl<'a> de::Visitor<'a> for Visitor<'_> {
            type Value = Optional<Pot>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{} definition (table)", self.cfg.pot.name)
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'a>,
            {
                let mut empty = true;
                let mut channel = None;
                let mut cc = None;
                let mut min = None;
                let mut max = None;
                let mut pickup = None;
                while let Some(field) = map.next_key()? {
                    empty = false;
                    match field {
                        Field::Channel => {
                            parse::check_dup(&channel, "channel")?;
                            channel = Some(map.next_value()?);
                        }
                        Field::Cc => {
                            parse::check_dup(&cc, "cc")?;
                            cc = Some(map.next_value()?);
                        }
                        Field::Min => {
                            parse::check_dup(&min, "min")?;
                            min = Some(map.next_value()?);
                        }
                        Field::Max => {
                            parse::check_dup(&max, "max")?;
                            max = Some(map.next_value()?);
                        }
                        Field::Pickup => {
                            parse::check_dup(&pickup, "pickup")?;
                            let seed = ConfigSeed::new(self.cfg.pickup());
                            pickup = Some(map.next_value_seed(seed)?);
                        }
                    }
                }
                if empty && !self.cfg.required {
                    return Ok(Optional::None);
                }
                let missing = de::Error::missing_field;
                Ok(Optional::Some(Pot {
                    channel: channel.unwrap_or(Channel::Global),
                    cc: cc.ok_or_else(|| missing("cc"))?,
                    min: min.unwrap_or(MidiValue::MIN),
                    max: max.unwrap_or(MidiValue::MAX),
                    // Use `Global` as the default even on devices that don't
                    // have a global pickup setting; this matches the behavior
                    // of the official software, which doesn't provide a global
                    // pickup option for pots yet still initializes them in
                    // that state.
                    pickup: pickup.unwrap_or(Pickup::Global),
                }))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if self.cfg.required {
                    Err(E::invalid_type(de::Unexpected::Option, &self))
                } else {
                    Ok(Optional::None)
                }
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

        let visitor = Visitor {
            cfg: config,
        };
        if config.required {
            deserializer.deserialize_map(visitor)
        } else {
            deserializer.deserialize_option(visitor)
        }
    }
}

impl<'a> DeserializeConfig<'a, PotCfg> for Optional<Pot> {
    fn deserialize<D>(
        deserializer: D,
        config: &PotCfg,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        DeserializeConfig::deserialize(deserializer, &OptionalPotCfg {
            required: false,
            pot: config,
        })
    }
}
