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
use super::{CompileCfg, Control};
use crate::common::{Channel, MidiValue};
use serde::Deserialize;
use std::io::{self, Write};

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(expecting = "pedal definition (table)")]
pub struct Pedal {
    pub cc: MidiValue,
}

impl Control for Pedal {
    fn compile<W: Write>(
        &self,
        address: u8,
        writer: &mut W,
        config: &CompileCfg,
    ) -> io::Result<()> {
        def::Definition {
            address,
            color: MidiValue::MIN,
            opt: def::Opt::builder(config).channel(Channel::Global).done(),
            payload: def::Payload::Cc {
                number: self.cc,
                min: MidiValue::MIN,
                max: MidiValue::MAX,
            },
        }
        .compile(writer)
    }
}
