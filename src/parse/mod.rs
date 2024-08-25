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
use core::marker::PhantomData;
use serde::de::{self, Deserialize, DeserializeSeed, Deserializer};

pub mod bounded;
pub mod config;
pub mod error;
pub mod primitive;
pub mod slice;
pub mod with_error;
pub mod wrap;

pub fn try_none<'a, T>() -> Option<T>
where
    T: Deserialize<'a>,
{
    try_none_seed(PhantomData)
}

pub fn try_none_seed<'a, T>(seed: T) -> Option<T::Value>
where
    T: DeserializeSeed<'a>,
{
    struct NoneDeserializer;

    impl<'a> Deserializer<'a> for NoneDeserializer {
        type Error = error::IgnoredError;

        fn deserialize_any<V>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error>
        where
            V: de::Visitor<'a>,
        {
            visitor.visit_none()
        }

        serde::forward_to_deserialize_any! {
            <V: Visitor<'a>>
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str
            string bytes byte_buf option unit unit_struct newtype_struct seq
            tuple tuple_struct map struct enum identifier ignored_any
        }
    }

    seed.deserialize(NoneDeserializer).ok()
}

pub fn one_of<'a, C>(items: C) -> impl Display
where
    C: Clone + IntoIterator<Item = &'a str>,
{
    struct OneOf<C>(C);

    impl<'a, C> Display for OneOf<C>
    where
        C: Clone + IntoIterator<Item = &'a str>,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let mut items = self.0.clone().into_iter();
            let Some(first) = items.next() else {
                return write!(f, "nothing");
            };
            let Some(second) = items.next() else {
                return write!(f, "`{}`", first.escape_default());
            };
            let Some(third) = items.next() else {
                return write!(
                    f,
                    "`{}` or `{}`",
                    first.escape_default(),
                    second.escape_default(),
                );
            };
            write!(f, "one of: {}", first.escape_default())?;
            [second, third]
                .into_iter()
                .chain(items)
                .try_for_each(|s| write!(f, ", {s}"))
        }
    }

    OneOf(items)
}

pub fn check_dup<T, E>(v: &Option<T>, name: &'static str) -> Result<(), E>
where
    E: de::Error,
{
    if v.is_some() {
        Err(E::duplicate_field(name))
    } else {
        Ok(())
    }
}
