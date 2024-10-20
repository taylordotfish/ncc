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

use super::error::WrappedError;
use super::wrap;
use serde::de::{self, Deserialize, DeserializeSeed, Deserializer};

struct Wrapper<'f, F>(&'f mut Option<F>);

impl<'a, F, T> wrap::ValueWrapper<'a, T> for Wrapper<'_, F>
where
    F: de::Error,
{
    type Value = T;
    type Error<E: de::Error> = WrappedError<F, E>;
    type Child<'s, E: de::Error>
        = Wrapper<'s, F>
    where
        Self: 's;

    fn child<E>(&mut self) -> Wrapper<'_, F> {
        Wrapper(self.0)
    }

    fn wrap(&mut self, v: T) -> Self::Value {
        v
    }

    fn unwrap<E>(&mut self, e: Self::Error<E>) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match e {
            WrappedError::Wrapped(e) => {
                *self.0 = Some(e);
                Err(E::custom(""))
            }
            WrappedError::Other(e) => Err(e),
        }
    }
}

impl<'a, F, E> wrap::ErrorWrapper<'a, E> for Wrapper<'_, F>
where
    F: de::Error,
    E: de::Error,
{
    type Value<T> = T;
    type Error = WrappedError<F, E>;
    type Child<'s, T>
        = Wrapper<'s, F>
    where
        Self: 's;

    fn child<T>(&mut self) -> Wrapper<'_, F> {
        Wrapper(self.0)
    }

    fn wrap(&mut self, e: E) -> Self::Error {
        WrappedError::Other(e)
    }

    fn unwrap<T>(&mut self, v: Self::Value<T>) -> Result<T, Self::Error> {
        Ok(v)
    }
}

pub fn deserialize<'a, E, T, D>(
    deserializer: D,
) -> Result<T, (Option<E>, D::Error)>
where
    E: de::Error,
    T: Deserialize<'a>,
    D: Deserializer<'a>,
{
    deserialize_seed(core::marker::PhantomData, deserializer)
}

pub fn deserialize_seed<'a, E, T, D>(
    seed: T,
    deserializer: D,
) -> Result<T::Value, (Option<E>, D::Error)>
where
    E: de::Error,
    T: DeserializeSeed<'a>,
    D: Deserializer<'a>,
{
    let mut error = None;
    wrap::deserialize_seed(seed, deserializer, Wrapper(&mut error))
        .map_err(|e| (error, e))
}
