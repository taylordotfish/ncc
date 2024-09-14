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

use crate::common::{MidiValue, Name, VelocityCfg};
use crate::controls::pad::{Pad, PadAction, PadCfg};
use crate::controls::pot::{PickupCfg, Pot, PotCfg};
use crate::controls::{self, Control, Optional, Pedal};
use crate::parse::config::{ConfigSeed, DeserializeConfig};
use crate::parse::{self, slice};
use serde::de::{self, Deserializer};
use serde::Deserialize;
use std::fmt;
use std::io::{self, Write};

const COMPILE_CONFIG: controls::CompileCfg = controls::CompileCfgRequired {
    null_channel: 1,
}
.into_cfg();

fn write_header<W>(writer: &mut W, device_id: u8) -> io::Result<()>
where
    W: Write,
{
    writer.write_all(b"\xf0\x00\x20\x29\x02")?;
    writer.write_all(&[device_id])?;
    writer.write_all(b"\x05\x00\x45")
}

#[derive(Clone, Copy, Debug)]
struct StandardParams {
    pub map_type: u8,
    pub active_color: u8,
    pub any_notes: bool,
    pub name: Name,
}

fn any_notes<'a, T>(pads: T) -> bool
where
    T: IntoIterator,
    T::Item: Into<Option<&'a Pad>>,
{
    pads.into_iter()
        .filter_map(Into::into)
        .any(|p| matches!(p.action, PadAction::Note(_)))
}

impl StandardParams {
    pub fn compile<W>(self, writer: &mut W) -> io::Result<()>
    where
        W: Write,
    {
        writer.write_all(&[
            self.map_type,
            0x7f,
            0x00,
            self.active_color,
            0x04,
            match self.any_notes {
                true => 0x42,
                false => 0x40,
            },
            0x07,
            match self.any_notes {
                true => 0x33,
                false => 0,
            },
            0x20,
        ])?;
        self.name.compile(writer)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PadMap {
    name: Name,
    active_color: MidiValue,
    pads: [Optional<Pad>; Self::NUM_PADS],
}

impl PadMap {
    pub const NUM_PADS: usize = 16;

    pub fn compile<W>(&self, device_id: u8, writer: &mut W) -> io::Result<()>
    where
        W: Write,
    {
        write_header(writer, device_id)?;
        StandardParams {
            map_type: 0x01,
            active_color: self.active_color.value(),
            any_notes: any_notes(&self.pads),
            name: self.name,
        }
        .compile(writer)?;
        for (i, pad) in self.pads.iter().enumerate() {
            pad.compile(i as u8, writer, &COMPILE_CONFIG)?;
        }
        for (i, _) in self.pads.iter().enumerate() {
            writer.write_all(&[0x60, i as u8])?;
        }
        writer.write_all(b"\xf7")
    }
}

#[derive(Clone, Debug)]
pub struct PadMapCfg {
    keypress: bool,
}

impl PadMapCfg {
    pub const fn new() -> Self {
        Self {
            keypress: false,
        }
    }

    pub const fn keypress(mut self, allowed: bool) -> Self {
        self.keypress = allowed;
        self
    }
}

impl<'a> DeserializeConfig<'a, PadMapCfg> for PadMap {
    fn deserialize<D>(
        deserializer: D,
        config: &PadMapCfg,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "kebab-case")]
        #[serde(expecting = "key")]
        enum Field {
            Name,
            ActiveColor,
            Pads,
        }

        struct Visitor<'a> {
            cfg: &'a PadMapCfg,
        }

        impl<'a> de::Visitor<'a> for Visitor<'_> {
            type Value = PadMap;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "pad map (table)")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'a>,
            {
                let mut name = None;
                let mut active_color = None;
                let mut pads = None;
                while let Some(field) = map.next_key()? {
                    match field {
                        Field::Name => {
                            parse::check_dup(&name, "name")?;
                            name = Some(map.next_value()?);
                        }
                        Field::ActiveColor => {
                            parse::check_dup(&active_color, "active-color")?;
                            active_color = Some(map.next_value()?);
                        }
                        Field::Pads => {
                            parse::check_dup(&pads, "pads")?;
                            let cfg = PadCfg::new(VelocityCfg::Any)
                                .keypress(self.cfg.keypress);
                            let b = map.next_value_seed(slice::Seed::new(
                                || ConfigSeed::new(&cfg),
                                PadMap::NUM_PADS,
                            ))?;
                            pads = Some(*Box::try_from(b).unwrap());
                        }
                    }
                }
                let missing = de::Error::missing_field;
                Ok(PadMap {
                    name: name.unwrap_or_else(Name::empty),
                    active_color: active_color
                        .ok_or_else(|| missing("active-color"))?,
                    pads: pads.unwrap_or([Optional::None; PadMap::NUM_PADS]),
                })
            }
        }

        deserializer.deserialize_map(Visitor {
            cfg: config,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PotMap {
    name: Name,
    pots: [Pot; Self::NUM_POTS],
}

impl PotMap {
    pub const NUM_POTS: usize = 8;

    pub fn compile<W>(&self, device_id: u8, writer: &mut W) -> io::Result<()>
    where
        W: Write,
    {
        const BASE_ADDR: u8 = 0x38;
        write_header(writer, device_id)?;
        StandardParams {
            map_type: 0x00,
            active_color: 0x1a,
            any_notes: false,
            name: self.name,
        }
        .compile(writer)?;
        for (i, pot) in self.pots.iter().enumerate() {
            pot.compile(BASE_ADDR + i as u8, writer, &COMPILE_CONFIG)?;
        }
        for (i, _) in self.pots.iter().enumerate() {
            writer.write_all(&[0x60, BASE_ADDR + i as u8])?;
        }
        writer.write_all(b"\xf7")
    }
}

#[derive(Clone, Debug)]
pub struct PotMapCfg {
    pickup: PickupCfg,
}

impl PotMapCfg {
    pub const fn new() -> Self {
        Self {
            pickup: PickupCfg::BinaryOnly,
        }
    }

    pub const fn pickup(mut self, cfg: PickupCfg) -> Self {
        self.pickup = cfg;
        self
    }
}

impl<'a> DeserializeConfig<'a, PotMapCfg> for PotMap {
    fn deserialize<D>(
        deserializer: D,
        config: &PotMapCfg,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "kebab-case")]
        #[serde(expecting = "key")]
        enum Field {
            Name,
            Pots,
        }

        struct Visitor<'a> {
            cfg: &'a PotMapCfg,
        }

        impl<'a> de::Visitor<'a> for Visitor<'_> {
            type Value = PotMap;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "pot map (table)")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'a>,
            {
                let mut name = None;
                let mut pots = None;
                while let Some(field) = map.next_key()? {
                    match field {
                        Field::Name => {
                            parse::check_dup(&name, "name")?;
                            name = Some(map.next_value()?);
                        }
                        Field::Pots => {
                            parse::check_dup(&pots, "pots")?;
                            let cfg = PotCfg::new().pickup(self.cfg.pickup);
                            let b = map.next_value_seed(slice::Seed::new(
                                || ConfigSeed::new(&cfg),
                                PotMap::NUM_POTS,
                            ))?;
                            pots = Some(*Box::try_from(b).unwrap());
                        }
                    }
                }
                let missing = de::Error::missing_field;
                Ok(PotMap {
                    name: name.unwrap_or_else(Name::empty),
                    pots: pots.ok_or_else(|| missing("pots"))?,
                })
            }
        }

        deserializer.deserialize_map(Visitor {
            cfg: config,
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(transparent)]
pub struct PedalMap {
    pedal: Pedal,
}

impl PedalMap {
    pub fn compile<W>(&self, device_id: u8, writer: &mut W) -> io::Result<()>
    where
        W: Write,
    {
        const ADDR: u8 = 0x78;
        write_header(writer, device_id)?;
        writer.write_all(&[0x02, 0x00, 0x00, 0x1a, 0x20])?;
        Name::empty().compile(writer)?;
        self.pedal.compile(ADDR, writer, &COMPILE_CONFIG)?;
        writer.write_all(&[0xf7])
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FaderMap {
    name: Name,
    active_color: MidiValue,
    faders: [Optional<Pot>; Self::NUM_FADERS],
    buttons: [Optional<Pad>; Self::NUM_BUTTONS],
}

impl FaderMap {
    pub const NUM_FADERS: usize = 9;
    pub const NUM_BUTTONS: usize = 9;

    pub fn compile<W>(&self, device_id: u8, writer: &mut W) -> io::Result<()>
    where
        W: Write,
    {
        const FADER_ADDR: u8 = 0x50;
        const BUTTON_ADDR: u8 = 0x28;
        write_header(writer, device_id)?;
        StandardParams {
            map_type: 0x03,
            active_color: self.active_color.value(),
            any_notes: any_notes(&self.buttons),
            name: self.name,
        }
        .compile(writer)?;
        for (i, fader) in self.faders.iter().enumerate() {
            fader.compile(FADER_ADDR + i as u8, writer, &COMPILE_CONFIG)?;
        }
        for (i, button) in self.buttons.iter().enumerate() {
            button.compile(BUTTON_ADDR + i as u8, writer, &COMPILE_CONFIG)?;
        }
        for (i, _) in self.faders.iter().enumerate() {
            writer.write_all(&[0x60, FADER_ADDR + i as u8])?;
        }
        for (i, _) in self.buttons.iter().enumerate() {
            writer.write_all(&[0x60, BUTTON_ADDR + i as u8])?;
        }
        writer.write_all(b"\xf7")
    }
}

impl<'a> Deserialize<'a> for FaderMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "kebab-case")]
        #[serde(expecting = "key")]
        enum Field {
            Name,
            ActiveColor,
            Faders,
            Buttons,
        }

        struct ButtonElemSeed {
            normal: PadCfg,
            last: PadCfg,
        }

        impl<'a, 'b> slice::ElementSeed<'a> for &'b ButtonElemSeed {
            type Seed = ConfigSeed<'b, Optional<Pad>, PadCfg>;

            fn get(&self, index: usize) -> Self::Seed {
                ConfigSeed::new(if index == FaderMap::NUM_BUTTONS - 1 {
                    &self.last
                } else {
                    &self.normal
                })
            }
        }

        const LAST_BUTTON_COLOR: MidiValue = match MidiValue::new(2) {
            Some(v) => v,
            None => unreachable!(),
        };

        struct Visitor;

        impl<'a> de::Visitor<'a> for Visitor {
            type Value = FaderMap;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "fader & button map (table)")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'a>,
            {
                let mut name = None;
                let mut active_color = None;
                let mut faders = None;
                let mut buttons = None;
                while let Some(field) = map.next_key()? {
                    match field {
                        Field::Name => {
                            parse::check_dup(&name, "name")?;
                            name = Some(map.next_value()?);
                        }
                        Field::ActiveColor => {
                            parse::check_dup(&active_color, "active-color")?;
                            active_color = Some(map.next_value()?);
                        }
                        Field::Faders => {
                            parse::check_dup(&faders, "faders")?;
                            let cfg = PotCfg::new()
                                .pickup(PickupCfg::GlobalAllowed)
                                .name("fader");
                            let b = map.next_value_seed(slice::Seed::new(
                                || ConfigSeed::new(&cfg),
                                FaderMap::NUM_FADERS,
                            ))?;
                            faders = Some(*Box::try_from(b).unwrap());
                        }
                        Field::Buttons => {
                            parse::check_dup(&buttons, "buttons")?;
                            let cfg = PadCfg::new(VelocityCfg::FixedOnly)
                                .keypress(true)
                                .name("button");
                            let elem = ButtonElemSeed {
                                normal: cfg.clone(),
                                last: cfg.color(LAST_BUTTON_COLOR),
                            };
                            let b = map.next_value_seed(slice::Seed::new(
                                &elem,
                                FaderMap::NUM_BUTTONS,
                            ))?;
                            buttons = Some(*Box::try_from(b).unwrap());
                        }
                    }
                }
                let missing = de::Error::missing_field;
                Ok(FaderMap {
                    name: name.unwrap_or_else(Name::empty),
                    active_color: active_color
                        .ok_or_else(|| missing("active-color"))?,
                    faders: faders.unwrap_or_default(),
                    buttons: buttons.unwrap_or_default(),
                })
            }
        }

        deserializer.deserialize_map(Visitor)
    }
}
