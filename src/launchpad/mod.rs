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

mod common;

macro_rules! define_device_map {
    () => {
        #[derive(Clone, Copy, Debug)]
        pub struct Map(super::common::Map);

        impl Map {
            pub fn compile<W>(&self, writer: &mut W) -> std::io::Result<()>
            where
                W: std::io::Write,
            {
                self.0.compile(DEVICE_ID, writer)
            }
        }

        impl<'a> serde::Deserialize<'a> for Map {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'a>,
            {
                serde::Deserialize::deserialize(deserializer).map(Self)
            }
        }
    };
}

pub mod launchpad_mini_mk3 {
    const DEVICE_ID: u8 = 0x0d;

    define_device_map!();
}

pub mod launchpad_x {
    const DEVICE_ID: u8 = 0x0c;

    define_device_map!();
}
