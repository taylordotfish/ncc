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

use core::marker::PhantomData;
use serde::de::{DeserializeSeed, Deserializer};

pub trait DeserializeConfig<'a, C>: Sized {
    fn deserialize<D>(deserializer: D, config: &C) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>;
}

pub struct ConfigSeed<'a, T, C> {
    config: &'a C,
    _phantom: PhantomData<fn() -> T>,
}

impl<'a, T, C> ConfigSeed<'a, T, C> {
    pub fn new(config: &'a C) -> Self {
        Self {
            config,
            _phantom: PhantomData,
        }
    }
}

impl<T, C> core::ops::Deref for ConfigSeed<'_, T, C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        self.config
    }
}

impl<'a, T, C> DeserializeSeed<'a> for ConfigSeed<'_, T, C>
where
    T: DeserializeConfig<'a, C>,
{
    type Value = T;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'a>,
    {
        T::deserialize(deserializer, self.config)
    }
}
