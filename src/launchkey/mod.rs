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

macro_rules! define_device_maps {
    (PadMap) => {
        #[derive(Clone, Copy, Debug)]
        pub struct PadMap(super::common::PadMap);

        impl PadMap {
            pub fn compile<W>(&self, writer: &mut W) -> std::io::Result<()>
            where
                W: std::io::Write,
            {
                self.0.compile(DEVICE_ID, writer)
            }
        }

        impl<'a> serde::Deserialize<'a> for PadMap {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'a>,
            {
                crate::parse::config::DeserializeConfig::deserialize(
                    deserializer,
                    &PAD_CONFIG,
                )
                .map(Self)
            }
        }
    };

    (PotMap) => {
        #[derive(Clone, Copy, Debug)]
        pub struct PotMap(super::common::PotMap);

        impl PotMap {
            pub fn compile<W>(&self, writer: &mut W) -> std::io::Result<()>
            where
                W: std::io::Write,
            {
                self.0.compile(DEVICE_ID, writer)
            }
        }

        impl<'a> serde::Deserialize<'a> for PotMap {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'a>,
            {
                crate::parse::config::DeserializeConfig::deserialize(
                    deserializer,
                    &POT_CONFIG,
                )
                .map(Self)
            }
        }
    };

    (PedalMap) => {
        #[derive(Clone, Copy, Debug, serde::Deserialize)]
        #[serde(transparent)]
        pub struct PedalMap(super::common::PedalMap);

        impl PedalMap {
            pub fn compile<W>(&self, writer: &mut W) -> std::io::Result<()>
            where
                W: std::io::Write,
            {
                self.0.compile(DEVICE_ID, writer)
            }
        }
    };

    (FaderMap) => {
        #[derive(Clone, Copy, Debug, serde::Deserialize)]
        #[serde(transparent)]
        pub struct FaderMap(super::common::FaderMap);

        impl FaderMap {
            pub fn compile<W>(&self, writer: &mut W) -> std::io::Result<()>
            where
                W: std::io::Write,
            {
                self.0.compile(DEVICE_ID, writer)
            }
        }
    };

    ($name:ident) => {
        compile_error!(concat!("invalid map name: ", stringify!($name)));
    };

    ($($name:ident),+ $(,)?) => {
        $(define_device_maps!($name);)+
    };
}

pub mod launchkey_mk3 {
    use super::common::{PadMapCfg, PotMapCfg};
    use crate::controls::pot::PickupCfg;

    const DEVICE_ID: u8 = 0x0f;

    pub(super) const PAD_CONFIG: PadMapCfg = PadMapCfg::new().keypress(true);
    pub(super) const POT_CONFIG: PotMapCfg =
        PotMapCfg::new().pickup(PickupCfg::GlobalAllowed);

    define_device_maps!(PadMap, PotMap, PedalMap, FaderMap);
}

pub mod launchkey_mini_mk3 {
    use super::common::{PadMapCfg, PotMapCfg};
    use crate::controls::pot::PickupCfg;

    const DEVICE_ID: u8 = 0x0b;

    pub(super) const PAD_CONFIG: PadMapCfg = PadMapCfg::new().keypress(false);
    pub(super) const POT_CONFIG: PotMapCfg =
        PotMapCfg::new().pickup(PickupCfg::BinaryOnly);

    define_device_maps!(PadMap, PotMap, PedalMap);
}

pub mod flkey {
    use super::launchkey_mk3::{PAD_CONFIG, POT_CONFIG};

    const DEVICE_ID: u8 = 0x11;

    define_device_maps!(PadMap, PotMap, PedalMap, FaderMap);
}

pub mod flkey_mini {
    use super::launchkey_mini_mk3::{PAD_CONFIG, POT_CONFIG};

    const DEVICE_ID: u8 = 0x10;

    define_device_maps!(PadMap, PotMap, PedalMap);
}
