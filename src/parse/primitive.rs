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

use core::fmt::{self, Display};
use serde::de::{self, Deserializer};

mod detail {
    use super::Deserializer;

    pub trait Primitive: Sized {
        fn deserialize<'a, D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'a>;
    }
}

pub trait Primitive: detail::Primitive {}

struct IntVisitor<T> {
    min: T,
    max: T,
}

impl<T> de::Visitor<'_> for IntVisitor<T>
where
    T: Display + TryFrom<u64> + TryFrom<i64>,
{
    type Value = T;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "an integer between {} and {}", self.min, self.max)
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.try_into()
            .map_err(|_| E::invalid_value(de::Unexpected::Unsigned(v), &self))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.try_into()
            .map_err(|_| E::invalid_value(de::Unexpected::Signed(v), &self))
    }
}

impl detail::Primitive for bool {
    fn deserialize<'a, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        struct Visitor;

        impl de::Visitor<'_> for Visitor {
            type Value = bool;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "`true` or `false`")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(v)
            }
        }

        deserializer.deserialize_bool(Visitor)
    }
}

impl Primitive for bool {}

macro_rules! primitive_impl_for_int {
    ($($deserialize:ident($ty:ty)),* $(,)?) => {
        $(impl detail::Primitive for $ty {
            fn deserialize<'a, D: ::serde::Deserializer<'a>>(
                deserializer: D,
            ) -> ::core::result::Result<Self, D::Error> {
                let visitor = IntVisitor {
                    min: <$ty>::MIN,
                    max: <$ty>::MAX,
                };
                deserializer.$deserialize(visitor)
            }
        }
        impl Primitive for $ty {})*
    };
}

primitive_impl_for_int! {
    deserialize_u8(u8),
    deserialize_u16(u16),
    deserialize_u32(u32),
    deserialize_u64(u64),
    deserialize_i8(i8),
    deserialize_i16(i16),
    deserialize_i32(i32),
    deserialize_i64(i64),
}

pub fn deserialize<'a, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Primitive,
    D: Deserializer<'a>,
{
    T::deserialize(deserializer)
}
