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

use std::io::{self, Write};

mod def;
pub mod pad;
pub mod pad_fader;
pub mod pedal;
pub mod pot;

pub use pad::Pad;
pub use pedal::Pedal;
pub use pot::Pot;

pub trait Control {
    fn compile<W: Write>(
        &self,
        address: u8,
        writer: &mut W,
        config: &CompileCfg,
    ) -> io::Result<()>;
}

#[derive(Clone, Copy, Debug)]
pub enum Optional<T> {
    None,
    Some(T),
}

impl<T> Optional<T> {
    pub fn as_ref(&self) -> Option<&T> {
        match self {
            Self::None => None,
            Self::Some(v) => Some(v),
        }
    }
}

impl<'a, T> From<&'a Optional<T>> for Option<&'a T> {
    fn from(opt: &'a Optional<T>) -> Self {
        opt.as_ref()
    }
}

impl<T> Default for Optional<T> {
    fn default() -> Self {
        Self::None
    }
}

impl<T: Control> Control for Optional<T> {
    fn compile<W: Write>(
        &self,
        address: u8,
        writer: &mut W,
        config: &CompileCfg,
    ) -> io::Result<()> {
        match self {
            Self::None => def::Empty {
                address,
            }
            .compile(writer),
            Self::Some(c) => c.compile(address, writer, config),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CompileCfg {
    aftertouch: bool,
    null_channel: u8,
}

impl CompileCfg {
    pub const fn new(params: CompileCfgRequired) -> Self {
        Self {
            null_channel: params.null_channel,
            aftertouch: false,
        }
    }

    /// Whether pads support aftertouch.
    pub const fn aftertouch(mut self, supported: bool) -> Self {
        self.aftertouch = supported;
        self
    }
}

#[derive(Debug)]
pub struct CompileCfgRequired {
    /// The 0-based channel number used when channel configuration is
    /// "global".
    pub null_channel: u8,
}

impl CompileCfgRequired {
    pub const fn into_cfg(self) -> CompileCfg {
        CompileCfg::new(self)
    }
}
