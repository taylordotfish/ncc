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

use core::fmt;
use serde::de;

#[derive(Clone, Copy, Debug)]
pub struct Visitor {
    pub min: u64,
    pub max: u64,
}

impl<'a> de::Visitor<'a> for Visitor {
    type Value = u64;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "an integer between {} and {}", self.min, self.max)
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let v = v
            .try_into()
            .map_err(|_| E::invalid_value(de::Unexpected::Signed(v), &self))?;
        self.visit_u64(v)
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if (self.min..=self.max).contains(&v) {
            Ok(v)
        } else {
            Err(E::invalid_value(de::Unexpected::Unsigned(v), &self))
        }
    }
}

macro_rules! bounded_uint_define {
    (
        $(
            $(#[$attr:meta])*
            $name:ident($ty:ty, $deserialize:ident)
        ),*
        $(,)?
    ) => {
        $(#[derive(::core::clone::Clone)]
        #[derive(::core::marker::Copy)]
        #[derive(::core::fmt::Debug)]
        pub struct $name<const MIN: $ty, const MAX: $ty>($ty);

        $(#[$attr])*
        impl<const MIN: $ty, const MAX: $ty> $name<MIN, MAX> {
            pub fn get(self) -> $ty {
                self.0
            }
        }

        impl<'a, const MIN: $ty, const MAX: $ty> ::serde::Deserialize<'a>
            for $name<MIN, MAX>
        {
            fn deserialize<D: ::serde::Deserializer<'a>>(
                deserializer: D,
            ) -> ::core::result::Result<Self, D::Error> {
                let visitor = Visitor {
                    min: MIN.into(),
                    max: MAX.into(),
                };
                deserializer.$deserialize(visitor).map(|n| Self(n as $ty))
            }
        })*
    };
}

bounded_uint_define! {
    BoundedU8(u8, deserialize_u8),
    BoundedU16(u16, deserialize_u16),
    BoundedU32(u32, deserialize_u32),
    BoundedU64(u64, deserialize_u64),
}
