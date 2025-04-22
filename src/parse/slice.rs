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

use super::try_none_seed;
use core::fmt::{self, Display};
use core::marker::PhantomData;
use serde::de::{self, Deserialize, DeserializeSeed, Deserializer};

#[derive(Clone, Copy)]
struct UnknownKey<'a> {
    name: &'a str,
    max: usize,
}

impl Display for UnknownKey<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown key `{}`; expected an integer between 1 and {}",
            self.name.escape_default(),
            self.max,
        )
    }
}

struct IndexKey {
    max: usize,
}

impl de::Visitor<'_> for IndexKey {
    type Value = usize;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "an integer between 1 and {}", self.max)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let b = v.as_bytes();
        (matches!(b, [b'1'..=b'9', ..]) && b.iter().all(u8::is_ascii_digit))
            .then_some(v)
            .and_then(|v| v.parse().ok())
            .and_then(|n| (1..=self.max).contains(&n).then(|| n - 1))
            .ok_or_else(|| {
                de::Error::custom(UnknownKey {
                    name: v,
                    max: self.max,
                })
            })
    }
}

impl<'a> DeserializeSeed<'a> for IndexKey {
    type Value = usize;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_str(self)
    }
}

#[derive(Clone, Copy)]
struct MissingItem(usize);

impl Display for MissingItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "missing item {} in sequence", self.0 + 1)
    }
}

#[derive(Clone, Copy)]
struct DuplicateItem(usize);

impl Display for DuplicateItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "duplicate item in sequence: {}", self.0 + 1)
    }
}

pub trait ElementSeed<'a> {
    type Seed: DeserializeSeed<'a>;
    fn get(&self, index: usize) -> Self::Seed;
}

pub type ElementType<'a, S> =
    <<S as ElementSeed<'a>>::Seed as DeserializeSeed<'a>>::Value;

impl<'a, F, T> ElementSeed<'a> for F
where
    F: Fn() -> T,
    T: DeserializeSeed<'a>,
{
    type Seed = T;

    fn get(&self, _index: usize) -> Self::Seed {
        self()
    }
}

impl<'a, T> ElementSeed<'a> for PhantomData<T>
where
    T: Deserialize<'a>,
{
    type Seed = Self;

    fn get(&self, _index: usize) -> Self::Seed {
        Self
    }
}

pub struct Seed<S> {
    element: S,
    len: usize,
}

impl<S> Seed<S> {
    pub fn new(element: S, len: usize) -> Self {
        Self {
            element,
            len,
        }
    }
}

impl<T> Seed<PhantomData<T>> {
    pub fn basic(len: usize) -> Self {
        Self::new(PhantomData, len)
    }
}

impl<'a, S> de::Visitor<'a> for Seed<S>
where
    S: ElementSeed<'a>,
{
    type Value = Box<[ElementType<'a, S>]>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a sequence of {} items", self.len)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'a>,
    {
        let mut items = Vec::with_capacity(self.len);
        for i in 0..self.len {
            let seed = || self.element.get(i);
            let none = || try_none_seed(seed());
            if let Some(item) = seq.next_element_seed(seed())?.or_else(none) {
                items.push(item);
            } else {
                return Err(de::Error::invalid_length(i, &self));
            }
        }
        let mut extra = 0;
        while seq.next_element::<de::IgnoredAny>()?.is_some() {
            extra += 1;
        }
        if extra > 0 {
            return Err(de::Error::invalid_length(self.len + extra, &self));
        }
        Ok(items.into_boxed_slice())
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'a>,
    {
        let mut items = Vec::new();
        items.resize_with(self.len, || None);
        let key = || IndexKey {
            max: self.len,
        };
        let seed = |i| self.element.get(i);
        while let Some(i) = map.next_key_seed(key())? {
            if items[i].replace(map.next_value_seed(seed(i))?).is_some() {
                return Err(de::Error::custom(DuplicateItem(i)));
            }
        }
        for (i, slot) in items.iter_mut().enumerate() {
            if slot.is_some() {
            } else if let Some(item) = try_none_seed(seed(i)) {
                *slot = Some(item);
            } else {
                return Err(de::Error::custom(MissingItem(i)));
            }
        }
        Ok(items.into_iter().map(Option::unwrap).collect())
    }
}

impl<'a, S> DeserializeSeed<'a> for Seed<S>
where
    S: ElementSeed<'a>,
{
    type Value = Box<[<S::Seed as DeserializeSeed<'a>>::Value]>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_seq(self)
    }
}

pub fn deserialize<'a, T, D>(
    deserializer: D,
    len: usize,
) -> Result<Box<[T]>, D::Error>
where
    T: Deserialize<'a>,
    D: Deserializer<'a>,
{
    deserialize_seed(PhantomData, deserializer, len)
}

pub fn deserialize_seed<'a, S, D>(
    seed: S,
    deserializer: D,
    len: usize,
) -> Result<Box<[ElementType<'a, S>]>, D::Error>
where
    S: ElementSeed<'a>,
    D: Deserializer<'a>,
{
    Seed::new(seed, len).deserialize(deserializer)
}
