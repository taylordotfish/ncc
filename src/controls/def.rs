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

use super::CompileCfg;
use super::{pad, pad_fader, pot};
use crate::common::{Channel, Keypress, MidiNote, MidiValue, Velocity};
use std::io::{self, Write};

#[derive(Clone, Copy, Debug)]
pub enum Payload {
    Note {
        pitch: MidiNote,
        velocity: Velocity,
    },
    Cc {
        number: MidiValue,
        min: MidiValue,
        max: MidiValue,
    },
    Prog(MidiValue),
    Key(u8),
    CcFull(MidiValue),
}

impl Payload {
    pub fn id(&self) -> u8 {
        match self {
            Self::Note {
                ..
            } => 0x01,
            Self::CcFull(_)
            | Self::Cc {
                ..
            } => 0x02,
            Self::Prog(_) => 0x03,
            Self::Key(_) => 0x0c,
        }
    }

    pub fn len(&self) -> u8 {
        match self {
            Self::Note {
                ..
            } => 3,
            Self::Cc {
                ..
            } => 4,
            Self::Prog(_) => 4,
            Self::Key(_) => 2,
            Self::CcFull(_) => 3,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Opt {
    b1: u8,
    b2: u8,
}

impl Opt {
    pub fn builder(config: &CompileCfg) -> OptBuilder<'_> {
        OptBuilder {
            b1: 0,
            b2: 0,
            cfg: config,
        }
    }
}

#[derive(Clone, Copy)]
pub struct OptBuilder<'a> {
    b1: u8,
    b2: u8,
    cfg: &'a CompileCfg,
}

impl OptBuilder<'_> {
    pub fn done(self) -> Opt {
        Opt {
            b1: self.b1,
            b2: self.b2,
        }
    }

    pub fn channel(mut self, value: Channel) -> Self {
        self.b1 |= match value {
            Channel::Fixed(c) => c.raw_value(),
            Channel::Global => self.cfg.null_channel,
        };
        self.b2 |= match value {
            Channel::Fixed(_) => 0,
            Channel::Global => 0x40,
        };
        self
    }

    pub fn key(mut self, key: Keypress) -> Self {
        if key.ctrl {
            self.b1 |= 0x01;
        }
        if key.shift {
            self.b1 |= 0x02;
        }
        if key.alt {
            self.b1 |= 0x04;
        }
        self
    }

    pub fn behavior(mut self, value: pad::Behavior) -> Self {
        self.b1 |= match value {
            pad::Behavior::Momentary => 0,
            pad::Behavior::Toggle => 0x20,
        };
        self
    }

    pub fn pickup(mut self, value: pot::Pickup) -> Self {
        self.b1 |= match value {
            pot::Pickup::Global => 0,
            pot::Pickup::Yes => 0x10,
            pot::Pickup::No => 0x20,
        };
        self
    }

    pub fn aftertouch(mut self, value: bool) -> Self {
        self.b1 |= (value as u8) << 4;
        self
    }

    pub fn velocity(mut self, value: Velocity) -> Self {
        self.b2 |= match value {
            Velocity::Fixed(_) => 4,
            Velocity::Variable => 0,
        };
        self
    }

    pub fn pad_fader(mut self, mode: pad_fader::Mode) -> Self {
        self.b2 |= match mode {
            pad_fader::Mode::Unipolar => 0x08,
            pad_fader::Mode::Bipolar => 0x09,
        };
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Definition {
    pub address: u8,
    pub color: MidiValue,
    pub opt: Opt,
    pub payload: Payload,
}

impl Definition {
    pub fn compile<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: Write,
    {
        writer.write_all(&[
            0x45 + self.payload.len(),
            self.address,
            self.payload.id(),
            self.color.value(),
            0x00,
            self.opt.b1,
            self.opt.b2,
        ])?;
        match self.payload {
            Payload::Note {
                pitch,
                velocity,
            } => writer.write_all(&[
                0x00,
                match velocity {
                    Velocity::Fixed(v) => v.value(),
                    Velocity::Variable => 0,
                },
                pitch.value(),
            ]),
            Payload::Cc {
                number,
                min,
                max,
            } => writer.write_all(&[
                0x00,
                number.value(),
                max.value(),
                min.value(),
            ]),
            Payload::Prog(number) => writer.write_all(&[
                0x00, //
                0x00,
                number.value(),
                number.value(),
            ]),
            Payload::Key(code) => writer.write_all(&[
                code >> 7, //
                code & 0x7f,
            ]),
            Payload::CcFull(number) => writer.write_all(&[
                0x00, //
                number.value(),
                0x7f,
            ]),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Empty {
    pub address: u8,
}

impl Empty {
    pub fn compile<W>(&self, writer: &mut W) -> io::Result<()>
    where
        W: Write,
    {
        writer.write_all(&[0x40, self.address])
    }
}
