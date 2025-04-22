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
use serde::de::{self, Deserialize, DeserializeSeed, Deserializer};

macro_rules! with_simple_visit_methods {
    ([$lt:lifetime], $name:ident!($($args:tt)*)) => {
        $name! {
            $($args)*
            visit_bool(v: bool),
            visit_i8(v: i8),
            visit_i16(v: i16),
            visit_i32(v: i32),
            visit_i64(v: i64),
            visit_i128(v: i128),
            visit_u8(v: u8),
            visit_u16(v: u16),
            visit_u32(v: u32),
            visit_u64(v: u64),
            visit_u128(v: u128),
            visit_f32(v: f32),
            visit_f64(v: f64),
            visit_char(v: char),
            visit_str(v: &str),
            visit_borrowed_str(v: &$lt str),
            visit_string(v: String),
            visit_bytes(v: &[u8]),
            visit_borrowed_bytes(v: &$lt [u8]),
            visit_byte_buf(v: Vec<u8>),
            visit_none(),
            visit_unit(),
        }
    };
}

macro_rules! with_deserialize_methods {
    ($name:ident!($($args:tt)*)) => {
        $name! {
            $($args)*
            deserialize_any(),
            deserialize_bool(),
            deserialize_i8(),
            deserialize_i16(),
            deserialize_i32(),
            deserialize_i64(),
            deserialize_u8(),
            deserialize_u16(),
            deserialize_u32(),
            deserialize_u64(),
            deserialize_f32(),
            deserialize_f64(),
            deserialize_char(),
            deserialize_str(),
            deserialize_string(),
            deserialize_bytes(),
            deserialize_byte_buf(),
            deserialize_option(),
            deserialize_unit(),
            deserialize_unit_struct(name: &'static str),
            deserialize_newtype_struct(name: &'static str),
            deserialize_seq(),
            deserialize_tuple(len: usize),
            deserialize_tuple_struct(name: &'static str, len: usize),
            deserialize_map(),
            deserialize_struct(
                name: &'static str,
                fields: &'static [&'static str],
            ),
            deserialize_enum(
                name: &'static str,
                variants: &'static [&'static str],
            ),
            deserialize_identifier(),
            deserialize_ignored_any(),
            deserialize_i128(),
            deserialize_u128(),
        }
    };
}

macro_rules! wrapper_default_visit {
    (
        [$lt:lifetime, $t:ident $(,)?]
        $(,$name:ident($($param:ident: $ty:ty),* $(,)?))*
        $(,)?
    ) => {
        $(fn $name<V, E>(
            mut self,
            visitor: V,
            $($param: $ty,)*
        ) -> ::core::result::Result<Self::Value, E>
        where
            V: ::serde::de::Visitor<$lt, Value = $t>,
            E: ::serde::de::Error,
        {
            self.wrap_result(visitor.$name($($param),*))
        })*
    };
}

macro_rules! wrapper_default_deserialize {
    (
        [$lt:lifetime, $e:ident $(,)?]
        $(,$name:ident($($param:ident: $ty:ty),* $(,)?))*
        $(,)?
    ) => {
        $(fn $name<D, V>(
            mut self,
            deserializer: D,
            $($param: $ty,)*
            visitor: V,
        ) -> ::core::result::Result<V::Value, Self::Error>
        where
            D: ::serde::Deserializer<$lt, Error = $e>,
            V: ::serde::de::Visitor<$lt>,
        {
            let r = deserializer.$name(
                $($param,)*
                Wrapped::new(visitor, self.child()),
            );
            self.wrap_result(r)
        })*
    };
}

pub trait ValueWrapper<'de, T>: Sized {
    type Value;
    type Error<E: de::Error>: de::Error;
    type Child<'s, E: de::Error>: ErrorWrapper<'de, E, Error = Self::Error<E>>
    where
        Self: 's;

    fn child<E: de::Error>(&mut self) -> Self::Child<'_, E>;
    fn wrap(&mut self, v: T) -> Self::Value;
    fn unwrap<E>(&mut self, e: Self::Error<E>) -> Result<Self::Value, E>
    where
        E: de::Error;

    fn wrap_result<E>(
        &mut self,
        r: Result<T, Self::Error<E>>,
    ) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match r {
            Ok(v) => Ok(self.wrap(v)),
            Err(e) => self.unwrap(e),
        }
    }

    fn deserialize_seed<S, D>(
        mut self,
        seed: S,
        deserializer: D,
    ) -> Result<Self::Value, D::Error>
    where
        S: DeserializeSeed<'de, Value = T>,
        D: Deserializer<'de>,
    {
        let r = seed.deserialize(Wrapped::new(deserializer, self.child()));
        self.wrap_result(r)
    }

    fn expecting<V>(
        &self,
        visitor: &V,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    where
        V: de::Visitor<'de, Value = T>,
    {
        visitor.expecting(f)
    }

    fn visit_some<V, D>(
        mut self,
        visitor: V,
        deserializer: D,
    ) -> Result<Self::Value, D::Error>
    where
        V: de::Visitor<'de, Value = T>,
        D: Deserializer<'de>,
    {
        let r = visitor.visit_some(Wrapped::new(deserializer, self.child()));
        self.wrap_result(r)
    }

    fn visit_newtype_struct<V, D>(
        mut self,
        visitor: V,
        deserializer: D,
    ) -> Result<Self::Value, D::Error>
    where
        V: de::Visitor<'de, Value = T>,
        D: Deserializer<'de>,
    {
        let r = visitor
            .visit_newtype_struct(Wrapped::new(deserializer, self.child()));
        self.wrap_result(r)
    }

    fn visit_seq<V, A>(
        mut self,
        visitor: V,
        seq: A,
    ) -> Result<Self::Value, A::Error>
    where
        V: de::Visitor<'de, Value = T>,
        A: de::SeqAccess<'de>,
    {
        let r = visitor.visit_seq(Wrapped::new(seq, self.child()));
        self.wrap_result(r)
    }

    fn visit_map<V, A>(
        mut self,
        visitor: V,
        map: A,
    ) -> Result<Self::Value, A::Error>
    where
        V: de::Visitor<'de, Value = T>,
        A: de::MapAccess<'de>,
    {
        let r = visitor.visit_map(Wrapped::new(map, self.child()));
        self.wrap_result(r)
    }

    fn visit_enum<V, A>(
        mut self,
        visitor: V,
        data: A,
    ) -> Result<Self::Value, A::Error>
    where
        V: de::Visitor<'de, Value = T>,
        A: de::EnumAccess<'de>,
    {
        let r = visitor.visit_enum(Wrapped::new(data, self.child()));
        self.wrap_result(r)
    }

    with_simple_visit_methods!(['de], wrapper_default_visit!(['de, T],));
}

pub trait ErrorWrapper<'de, E: de::Error>: Sized {
    type Error: de::Error;
    type Value<T>;
    type Child<'s, T>: ValueWrapper<'de, T, Value = Self::Value<T>>
    where
        Self: 's;

    fn child<T>(&mut self) -> Self::Child<'_, T>;
    fn wrap(&mut self, e: E) -> Self::Error;
    fn unwrap<T>(&mut self, v: Self::Value<T>) -> Result<T, Self::Error>;

    fn wrap_result<T>(
        &mut self,
        r: Result<Self::Value<T>, E>,
    ) -> Result<T, Self::Error> {
        match r {
            Ok(v) => self.unwrap(v),
            Err(e) => Err(self.wrap(e)),
        }
    }

    with_deserialize_methods!(wrapper_default_deserialize!(['de, E],));

    fn is_human_readable<D>(&self, deserializer: &D) -> bool
    where
        D: Deserializer<'de, Error = E>,
    {
        deserializer.is_human_readable()
    }

    fn next_element_seed<A, T>(
        &mut self,
        seq: &mut A,
        seed: T,
    ) -> Result<Option<T::Value>, Self::Error>
    where
        A: de::SeqAccess<'de, Error = E>,
        T: DeserializeSeed<'de>,
    {
        seq.next_element_seed(Wrapped::new(seed, self.child()))
            .transpose()
            .map(|r| self.wrap_result(r))
            .transpose()
    }

    fn seq_size_hint<A>(&self, seq: &A) -> Option<usize>
    where
        A: de::SeqAccess<'de, Error = E>,
    {
        seq.size_hint()
    }

    fn next_key_seed<A, K>(
        &mut self,
        map: &mut A,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error>
    where
        A: de::MapAccess<'de, Error = E>,
        K: DeserializeSeed<'de>,
    {
        map.next_key_seed(Wrapped::new(seed, self.child()))
            .transpose()
            .map(|r| self.wrap_result(r))
            .transpose()
    }

    fn next_value_seed<A, V>(
        &mut self,
        map: &mut A,
        seed: V,
    ) -> Result<V::Value, Self::Error>
    where
        A: de::MapAccess<'de, Error = E>,
        V: DeserializeSeed<'de>,
    {
        let r = map.next_value_seed(Wrapped::new(seed, self.child()));
        self.wrap_result(r)
    }

    fn map_size_hint<A>(&self, map: &A) -> Option<usize>
    where
        A: de::MapAccess<'de, Error = E>,
    {
        map.size_hint()
    }

    #[allow(clippy::type_complexity)]
    fn variant_seed<A, V>(
        mut self,
        data: A,
        seed: V,
    ) -> Result<(V::Value, Wrapped<A::Variant, Self>), Self::Error>
    where
        A: de::EnumAccess<'de, Error = E>,
        V: DeserializeSeed<'de>,
    {
        match data.variant_seed(Wrapped::new(seed, self.child())) {
            Ok((v, variant)) => {
                self.unwrap(v).map(|v| (v, Wrapped::new(variant, self)))
            }
            Err(e) => Err(self.wrap(e)),
        }
    }

    fn unit_variant<A>(mut self, variant: A) -> Result<(), Self::Error>
    where
        A: de::VariantAccess<'de, Error = E>,
    {
        variant.unit_variant().map_err(|e| self.wrap(e))
    }

    fn newtype_variant_seed<A, T>(
        mut self,
        variant: A,
        seed: T,
    ) -> Result<T::Value, Self::Error>
    where
        A: de::VariantAccess<'de, Error = E>,
        T: DeserializeSeed<'de>,
    {
        let r = variant.newtype_variant_seed(Wrapped::new(seed, self.child()));
        self.wrap_result(r)
    }

    fn tuple_variant<A, V>(
        mut self,
        variant: A,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        A: de::VariantAccess<'de, Error = E>,
        V: de::Visitor<'de>,
    {
        let r =
            variant.tuple_variant(len, Wrapped::new(visitor, self.child()));
        self.wrap_result(r)
    }

    fn struct_variant<A, V>(
        mut self,
        variant: A,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        A: de::VariantAccess<'de, Error = E>,
        V: de::Visitor<'de>,
    {
        let r = variant
            .struct_variant(fields, Wrapped::new(visitor, self.child()));
        self.wrap_result(r)
    }
}

#[non_exhaustive]
pub struct Wrapped<T, W> {
    pub inner: T,
    pub wrapper: W,
}

impl<T, W> Wrapped<T, W> {
    pub fn new(inner: T, wrapper: W) -> Self {
        Self {
            inner,
            wrapper,
        }
    }
}

impl<'de, T, W> DeserializeSeed<'de> for Wrapped<T, W>
where
    T: DeserializeSeed<'de>,
    W: ValueWrapper<'de, T::Value>,
{
    type Value = W::Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        self.wrapper.deserialize_seed(self.inner, deserializer)
    }
}

impl<'de, A, W> de::SeqAccess<'de> for Wrapped<A, W>
where
    A: de::SeqAccess<'de>,
    W: ErrorWrapper<'de, A::Error>,
{
    type Error = W::Error;

    fn next_element_seed<T>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        self.wrapper.next_element_seed(&mut self.inner, seed)
    }

    fn size_hint(&self) -> Option<usize> {
        self.wrapper.seq_size_hint(&self.inner)
    }
}

impl<'de, A, W> de::MapAccess<'de> for Wrapped<A, W>
where
    A: de::MapAccess<'de>,
    W: ErrorWrapper<'de, A::Error>,
{
    type Error = W::Error;

    fn next_key_seed<K>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        self.wrapper.next_key_seed(&mut self.inner, seed)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        self.wrapper.next_value_seed(&mut self.inner, seed)
    }

    fn size_hint(&self) -> Option<usize> {
        self.wrapper.map_size_hint(&self.inner)
    }
}

impl<'de, A, W> de::EnumAccess<'de> for Wrapped<A, W>
where
    A: de::EnumAccess<'de>,
    W: ErrorWrapper<'de, A::Error>,
{
    type Error = W::Error;
    type Variant = Wrapped<A::Variant, W>;

    fn variant_seed<V>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        self.wrapper.variant_seed(self.inner, seed)
    }
}

impl<'de, A, W> de::VariantAccess<'de> for Wrapped<A, W>
where
    A: de::VariantAccess<'de>,
    W: ErrorWrapper<'de, A::Error>,
{
    type Error = W::Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        self.wrapper.unit_variant(self.inner)
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        self.wrapper.newtype_variant_seed(self.inner, seed)
    }

    fn tuple_variant<V>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.wrapper.tuple_variant(self.inner, len, visitor)
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.wrapper.struct_variant(self.inner, fields, visitor)
    }
}

macro_rules! wrapped_forward_visit {
    (
        $($name:ident($($param:ident: $ty:ty),* $(,)?)),*
        $(,)?
    ) => {
        $(fn $name<E: ::serde::de::Error>(
            self,
            $($param: $ty,)*
        ) -> ::core::result::Result<Self::Value, E> {
            self.wrapper.$name(self.inner $(,$param)*)
        })*
    };
}

impl<'de, V, W> de::Visitor<'de> for Wrapped<V, W>
where
    V: de::Visitor<'de>,
    W: ValueWrapper<'de, V::Value>,
{
    type Value = W::Value;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.wrapper.expecting(&self.inner, f)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        self.wrapper.visit_some(self.inner, deserializer)
    }

    fn visit_newtype_struct<D>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        self.wrapper.visit_newtype_struct(self.inner, deserializer)
    }

    fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        self.wrapper.visit_seq(self.inner, seq)
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        self.wrapper.visit_map(self.inner, map)
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        self.wrapper.visit_enum(self.inner, data)
    }

    with_simple_visit_methods!(['de], wrapped_forward_visit!());
}

macro_rules! wrapped_forward_deserialize {
    (
        [$lt:lifetime]
        $(,$name:ident($($param:ident: $ty:ty),* $(,)?))*
        $(,)?
    ) => {
        $(fn $name<V: ::serde::de::Visitor<$lt>>(
            self,
            $($param: $ty,)*
            visitor: V,
        ) -> ::core::result::Result<V::Value, Self::Error> {
            self.wrapper.$name(self.inner, $($param,)* visitor)
        })*
    };
}

impl<'de, D, W> Deserializer<'de> for Wrapped<D, W>
where
    D: Deserializer<'de>,
    W: ErrorWrapper<'de, D::Error>,
{
    type Error = W::Error;

    with_deserialize_methods!(wrapped_forward_deserialize!(['de],));

    fn is_human_readable(&self) -> bool {
        self.wrapper.is_human_readable(&self.inner)
    }
}

pub fn deserialize<'de, T, D, W>(
    deserializer: D,
    wrapper: W,
) -> Result<W::Value, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
    W: ValueWrapper<'de, T>,
{
    deserialize_seed(core::marker::PhantomData, deserializer, wrapper)
}

pub fn deserialize_seed<'de, T, D, W>(
    seed: T,
    deserializer: D,
    wrapper: W,
) -> Result<W::Value, D::Error>
where
    T: DeserializeSeed<'de>,
    D: Deserializer<'de>,
    W: ValueWrapper<'de, T::Value>,
{
    Wrapped::new(seed, wrapper).deserialize(deserializer)
}

impl<T> ValueWrapper<'_, T> for () {
    type Value = T;
    type Error<E: de::Error> = E;
    type Child<'s, E: de::Error> = Self;

    fn child<E>(&mut self) {}

    fn wrap(&mut self, v: T) -> Self::Value {
        v
    }

    fn unwrap<E>(&mut self, e: E) -> Result<Self::Value, E> {
        Err(e)
    }
}

impl<E> ErrorWrapper<'_, E> for ()
where
    E: de::Error,
{
    type Value<T> = T;
    type Error = E;
    type Child<'s, T> = Self;

    fn child<T>(&mut self) {}

    fn wrap(&mut self, e: E) -> Self::Error {
        e
    }

    fn unwrap<T>(&mut self, v: Self::Value<T>) -> Result<T, Self::Error> {
        Ok(v)
    }
}
