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
use crate::controls::pad::{Pad, PadCfg};
use crate::controls::pad_fader::{Fader, Orientation};
use crate::controls::{self, Control, Optional};
use crate::parse::config::ConfigSeed;
use crate::parse::{self, primitive, slice};
use serde::de::{self, Deserializer};
use serde::Deserialize;
use std::fmt::{self, Display};
use std::io::{self, Write};

const COMPILE_CONFIG: controls::CompileCfg = controls::CompileCfgRequired {
    null_channel: 0,
}
.into_cfg()
.aftertouch(true);

#[derive(Clone, Copy, Debug)]
enum Transposition {
    Enabled,
    Disabled,
}

impl<'a> Deserialize<'a> for Transposition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        primitive::deserialize(deserializer).map(|v| match v {
            true => Self::Enabled,
            false => Self::Disabled,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Map {
    name: Name,
    active_color: MidiValue,
    pads: [Optional<Pad>; Self::NUM_PADS],
    faders: [Optional<Fader>; Self::SIDE_LEN],
    /// Octave transposition
    trans_oct: Transposition,
    /// Semitone transposition
    trans_step: Transposition,
}

impl Map {
    /// The number of rows and columns in the square grid of pads.
    pub const SIDE_LEN: usize = 8;
    pub const NUM_PADS: usize = Self::SIDE_LEN * Self::SIDE_LEN;

    fn fader_orientation(&self) -> Option<Orientation> {
        self.faders
            .iter()
            .filter_map(|f| f.as_ref())
            .map(|f| f.orientation)
            .next()
    }

    pub fn compile<W>(&self, device_id: u8, writer: &mut W) -> io::Result<()>
    where
        W: Write,
    {
        writer.write_all(b"\xf0\x00\x20\x29\x02")?;
        writer.write_all(&[device_id])?;
        writer.write_all(b"\x20\x00\x45\x40\x7f\x20")?;
        self.name.compile(writer)?;
        writer.write_all(&[0x21, 0x01, 0x00])?;
        for (i, pad) in self.pads.iter().enumerate() {
            pad.compile(i as u8, writer, &COMPILE_CONFIG)?;
        }
        for (i, fader) in self.faders.iter().enumerate() {
            let addr = (Self::NUM_PADS + i) as u8;
            fader.compile(addr, writer, &COMPILE_CONFIG)?;
        }
        writer.write_all(&[
            0x00,
            self.active_color.value(),
            0x01,
            self.active_color.value(),
            0x02,
            0x00,
            0x06,
            0x00,
            0x07,
            match self.trans_oct {
                Transposition::Enabled => 0x64,
                Transposition::Disabled => 0,
            },
            0x08,
            0x00,
            0x05,
            match self.fader_orientation() {
                Some(Orientation::Vertical) => 0x01,
                _ => 0x00,
            },
            0x04,
            match self.trans_step {
                Transposition::Enabled => 0x42,
                Transposition::Disabled => 0x40,
            },
        ])?;
        writer.write_all(b"\xf7")
    }
}

fn check_conflict<'a, P, F, E>(pads: P, faders: F) -> Result<(), E>
where
    P: Into<Option<&'a [Optional<Pad>; Map::NUM_PADS]>>,
    F: Into<Option<&'a [Optional<Fader>; Map::SIDE_LEN]>>,
    E: de::Error,
{
    #[derive(Clone, Copy)]
    enum Slot {
        Fader(Orientation),
        Pad,
    }

    struct FmtFader {
        index: usize,
        orientation: Orientation,
    }

    impl Display for FmtFader {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self.orientation {
                Orientation::Horizontal => write!(
                    f,
                    "fader {} (horizontal)",
                    self.index / Map::SIDE_LEN + 1,
                ),
                Orientation::Vertical => write!(
                    f,
                    "fader {} (vertical)",
                    self.index % Map::SIDE_LEN + 1,
                ),
            }
        }
    }

    #[derive(Clone, Copy)]
    struct Conflict {
        index: usize,
        slots: [Slot; 2],
    }

    impl Display for Conflict {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let fader = |orientation| FmtFader {
                index: self.index,
                orientation,
            };
            match self.slots {
                [Slot::Fader(o1), Slot::Fader(o2)] => {
                    write!(f, "{} and {} intersect", fader(o1), fader(o2))
                }
                [Slot::Pad, Slot::Fader(o)] | [Slot::Fader(o), Slot::Pad] => {
                    write!(
                        f,
                        "cannot define both pad {} and {}, \
                        which includes pad {0}",
                        self.index + 1,
                        fader(o),
                    )
                }
                [Slot::Pad, Slot::Pad] => unreachable!(),
            }
        }
    }

    fn conflict<E: de::Error>(index: usize, slots: [Slot; 2]) -> E {
        E::custom(Conflict {
            index,
            slots,
        })
    }

    let pads = pads.into().map(|a| &a[..]).unwrap_or_default();
    let faders = faders.into().map(|a| &a[..]).unwrap_or_default();
    let mut slots = [None; Map::NUM_PADS];

    for (fader_index, fader) in faders.iter().enumerate() {
        let Optional::Some(fader) = fader else {
            continue;
        };
        let new = Slot::Fader(fader.orientation);
        match fader.orientation {
            Orientation::Horizontal => {
                for (i, slot) in slots
                    .iter_mut()
                    .enumerate()
                    .skip(fader_index * Map::SIDE_LEN)
                    .take(Map::SIDE_LEN)
                {
                    if let Some(old) = slot.replace(new) {
                        return Err(conflict(i, [old, new]));
                    }
                }
            }
            Orientation::Vertical => {
                for row in 0..Map::SIDE_LEN {
                    let i = row * Map::SIDE_LEN + fader_index;
                    if let Some(old) = slots[i].replace(new) {
                        return Err(conflict(i, [old, new]));
                    }
                }
            }
        }
    }
    for (i, (slot, pad)) in slots.iter_mut().zip(pads).enumerate() {
        let Optional::Some(_) = pad else {
            continue;
        };
        let new = Slot::Pad;
        if let Some(old) = slot.replace(new) {
            return Err(conflict(i, [old, new]));
        }
    }
    Ok(())
}

impl<'a> Deserialize<'a> for Map {
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
            Pads,
            Faders,
            #[serde(rename = "octave-transposition")]
            TransOct,
            #[serde(rename = "semitone-transposition")]
            TransStep,
        }

        struct Visitor;

        impl<'a> de::Visitor<'a> for Visitor {
            type Value = Map;

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
                let mut faders = None;
                let mut trans_oct = None;
                let mut trans_step = None;
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
                            let cfg = PadCfg::new(VelocityCfg::VariableOnly)
                                .keypress(true);
                            let b = map.next_value_seed(slice::Seed::new(
                                || ConfigSeed::new(&cfg),
                                Map::NUM_PADS,
                            ))?;
                            pads = Some(*Box::try_from(b).unwrap());
                            check_conflict(&pads, &faders)?;
                        }
                        Field::Faders => {
                            parse::check_dup(&faders, "faders")?;
                            let b = map.next_value_seed(
                                slice::Seed::basic(Map::SIDE_LEN),
                            )?;
                            faders = Some(*Box::try_from(b).unwrap());
                            check_conflict(&pads, &faders)?;
                        }
                        Field::TransOct => {
                            parse::check_dup(
                                &trans_oct,
                                "octave-transposition",
                            )?;
                            trans_oct = Some(map.next_value()?);
                        }
                        Field::TransStep => {
                            parse::check_dup(
                                &trans_step,
                                "semitone-transposition",
                            )?;
                            trans_step = Some(map.next_value()?);
                        }
                    }
                }
                let missing = de::Error::missing_field;
                Ok(Map {
                    name: name.unwrap_or_else(Name::empty),
                    active_color: active_color
                        .ok_or_else(|| missing("active-color"))?,
                    pads: pads.unwrap_or([Optional::None; Map::NUM_PADS]),
                    faders: faders.unwrap_or([Optional::None; Map::SIDE_LEN]),
                    trans_oct: trans_oct.unwrap_or(Transposition::Enabled),
                    trans_step: trans_step.unwrap_or(Transposition::Enabled),
                })
            }
        }

        deserializer.deserialize_map(Visitor)
    }
}
