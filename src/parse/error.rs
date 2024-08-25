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
use serde::de;

#[derive(Debug)]
pub struct IgnoredError;

impl Display for IgnoredError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown error")
    }
}

impl de::StdError for IgnoredError {}

impl de::Error for IgnoredError {
    fn custom<T: Display>(_msg: T) -> Self {
        Self
    }
}

#[derive(Debug)]
pub enum WrappedError<E, A> {
    Wrapped(E),
    Other(A),
}

impl<E, A> WrappedError<E, A> {
    pub fn transpose<T>(result: Result<T, Self>) -> Result<Result<T, A>, E> {
        match result {
            Ok(v) => Ok(Ok(v)),
            Err(WrappedError::Wrapped(e)) => Err(e),
            Err(WrappedError::Other(e)) => Ok(Err(e)),
        }
    }
}

impl<E: Display, A: Display> Display for WrappedError<E, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wrapped(e) => write!(f, "{e}"),
            Self::Other(v) => write!(f, "{v}"),
        }
    }
}

impl<E, A> de::StdError for WrappedError<E, A>
where
    E: de::StdError,
    A: fmt::Debug + Display,
{
}

impl<E, A> From<E> for WrappedError<E, A> {
    fn from(e: E) -> Self {
        Self::Wrapped(e)
    }
}

impl<E, A> de::Error for WrappedError<E, A>
where
    E: de::Error,
    A: fmt::Debug + Display,
{
    fn custom<T: Display>(msg: T) -> Self {
        E::custom(msg).into()
    }

    fn invalid_type(
        unexp: de::Unexpected<'_>,
        exp: &dyn de::Expected,
    ) -> Self {
        E::invalid_type(unexp, exp).into()
    }

    fn invalid_value(
        unexp: de::Unexpected<'_>,
        exp: &dyn de::Expected,
    ) -> Self {
        E::invalid_value(unexp, exp).into()
    }

    fn invalid_length(len: usize, exp: &dyn de::Expected) -> Self {
        E::invalid_length(len, exp).into()
    }

    fn unknown_variant(
        variant: &str,
        expected: &'static [&'static str],
    ) -> Self {
        E::unknown_variant(variant, expected).into()
    }

    fn unknown_field(field: &str, expected: &'static [&'static str]) -> Self {
        E::unknown_field(field, expected).into()
    }

    fn missing_field(field: &'static str) -> Self {
        E::missing_field(field).into()
    }

    fn duplicate_field(field: &'static str) -> Self {
        E::duplicate_field(field).into()
    }
}
