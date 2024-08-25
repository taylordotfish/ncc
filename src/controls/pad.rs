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

use super::def;
use super::{CompileCfg, Control, Optional};
use crate::common::{Channel, Keypress, MidiNote, MidiValue};
use crate::common::{Velocity, VelocityCfg};
use crate::parse;
use crate::parse::config::{ConfigSeed, DeserializeConfig};
use serde::de::value::MapAccessDeserializer;
use serde::de::{self, Deserializer, IntoDeserializer};
use serde::Deserialize;
use std::fmt::{self, Debug, Display};
use std::io::{self, Write};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Behavior {
    Momentary,
    Toggle,
}

impl<'a> Deserialize<'a> for Behavior {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        struct Visitor;

        impl<'a> de::Visitor<'a> for Visitor {
            type Value = Behavior;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "\"momentary\" or \"toggle\"")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match v {
                    "momentary" => Ok(Behavior::Momentary),
                    "toggle" => Ok(Behavior::Toggle),
                    _ => Err(E::invalid_value(de::Unexpected::Str(v), &self)),
                }
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Note {
    pub pitch: MidiNote,
    pub channel: Channel,
    pub velocity: Velocity,
    pub behavior: Behavior,
}

#[derive(Clone, Debug)]
pub struct NoteCfg {
    velocity: VelocityCfg,
}

impl NoteCfg {
    pub const fn new(velocity: VelocityCfg) -> Self {
        Self {
            velocity,
        }
    }
}

impl<'a> DeserializeConfig<'a, NoteCfg> for Note {
    fn deserialize<D>(
        deserializer: D,
        config: &NoteCfg,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        enum Field {
            Pitch,
            Channel,
            Velocity,
            Behavior,
        }

        #[derive(Clone, Copy)]
        struct UnknownField<'a> {
            name: &'a str,
            cfg: &'a NoteCfg,
        }

        impl Display for UnknownField<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let allowed = [
                    Some("pitch"),
                    Some("channel"),
                    self.cfg.velocity.fixed_allowed().then_some("velocity"),
                    Some("behavior"),
                ];
                write!(
                    f,
                    "unknown key `{}`; expected {}",
                    self.name.escape_default(),
                    parse::one_of(allowed.into_iter().flatten()),
                )
            }
        }

        struct FieldVisitor<'a> {
            cfg: &'a NoteCfg,
        }

        impl<'a> de::Visitor<'a> for FieldVisitor<'_> {
            type Value = Field;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "key")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match v {
                    "pitch" => Ok(Field::Pitch),
                    "channel" => Ok(Field::Channel),
                    "velocity" => Ok(Field::Velocity),
                    "behavior" => Ok(Field::Behavior),
                    _ => Err(E::custom(UnknownField {
                        name: v,
                        cfg: self.cfg,
                    })),
                }
            }
        }

        impl<'a> DeserializeConfig<'a, NoteCfg> for Field {
            fn deserialize<D>(
                deserializer: D,
                config: &NoteCfg,
            ) -> Result<Self, D::Error>
            where
                D: Deserializer<'a>,
            {
                deserializer.deserialize_str(FieldVisitor {
                    cfg: config,
                })
            }
        }

        struct Visitor<'a> {
            cfg: &'a NoteCfg,
        }

        impl Visitor<'_> {
            fn visit_note<'a, T, E, F>(self, v: T, unexp: F) -> Result<Note, E>
            where
                T: Copy + IntoDeserializer<'a, E>,
                E: de::Error,
                F: FnOnce(T) -> de::Unexpected<'a>,
            {
                if self.cfg.velocity.variable_allowed() {
                    Ok(Note {
                        pitch: MidiNote::deserialize(v.into_deserializer())?,
                        channel: Channel::Global,
                        velocity: Velocity::Variable,
                        behavior: Behavior::Momentary,
                    })
                } else {
                    Err(E::invalid_type(unexp(v), &self))
                }
            }
        }

        impl<'a> de::Visitor<'a> for Visitor<'_> {
            type Value = Note;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    f,
                    "note definition ({})",
                    if self.cfg.velocity.variable_allowed() {
                        "integer, string, or table"
                    } else {
                        "table"
                    },
                )
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_note(v, de::Unexpected::Unsigned)
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match v.try_into() {
                    Ok(v) => self.visit_u64(v),
                    Err(_) => {
                        Err(E::invalid_type(de::Unexpected::Signed(v), &self))
                    }
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_note(v, de::Unexpected::Str)
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'a>,
            {
                let mut pitch = None;
                let mut channel = None;
                let mut velocity = None;
                let mut behavior = None;
                while let Some(field) =
                    map.next_key_seed(ConfigSeed::new(self.cfg))?
                {
                    match field {
                        Field::Pitch => {
                            parse::check_dup(&pitch, "pitch")?;
                            pitch = Some(map.next_value()?);
                        }
                        Field::Channel => {
                            parse::check_dup(&channel, "channel")?;
                            channel = Some(map.next_value()?);
                        }
                        Field::Velocity => {
                            parse::check_dup(&velocity, "velocity")?;
                            let seed = ConfigSeed::new(&self.cfg.velocity);
                            velocity = Some(map.next_value_seed(seed)?);
                        }
                        Field::Behavior => {
                            parse::check_dup(&behavior, "behavior")?;
                            behavior = Some(map.next_value()?);
                        }
                    }
                }
                let missing = de::Error::missing_field;
                let velocity = if let Some(v) = velocity {
                    v
                } else if self.cfg.velocity.variable_allowed() {
                    Velocity::Variable
                } else {
                    return Err(missing("velocity"));
                };
                Ok(Note {
                    pitch: pitch.ok_or_else(|| missing("pitch"))?,
                    channel: channel.unwrap_or(Channel::Global),
                    velocity,
                    behavior: behavior.unwrap_or(Behavior::Momentary),
                })
            }
        }

        deserializer.deserialize_map(Visitor {
            cfg: config,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Cc {
    pub number: MidiValue,
    pub channel: Channel,
    pub off: MidiValue,
    pub on: MidiValue,
    pub behavior: Behavior,
}

impl<'a> Deserialize<'a> for Cc {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        fn channel_default() -> Channel {
            Channel::Global
        }

        fn off_default() -> MidiValue {
            MidiValue::MIN
        }

        fn on_default() -> MidiValue {
            MidiValue::MAX
        }

        fn behavior_default() -> Behavior {
            Behavior::Momentary
        }

        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        #[serde(expecting = "cc definition (table)")]
        struct Fields {
            number: MidiValue,
            #[serde(default = "channel_default")]
            channel: Channel,
            #[serde(default = "off_default")]
            off: MidiValue,
            #[serde(default = "on_default")]
            on: MidiValue,
            #[serde(default = "behavior_default")]
            behavior: Behavior,
        }

        impl From<Fields> for Cc {
            fn from(f: Fields) -> Self {
                Self {
                    number: f.number,
                    channel: f.channel,
                    off: f.off,
                    on: f.on,
                    behavior: f.behavior,
                }
            }
        }

        struct Visitor;

        impl<'a> de::Visitor<'a> for Visitor {
            type Value = Cc;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "cc definition (integer or table)")
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Cc {
                    number: MidiValue::deserialize(v.into_deserializer())?,
                    channel: channel_default(),
                    off: off_default(),
                    on: on_default(),
                    behavior: behavior_default(),
                })
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match v.try_into() {
                    Ok(v) => self.visit_u64(v),
                    Err(_) => {
                        Err(E::invalid_type(de::Unexpected::Signed(v), &self))
                    }
                }
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'a>,
            {
                Fields::deserialize(MapAccessDeserializer::new(map))
                    .map(Into::into)
            }
        }

        deserializer.deserialize_map(Visitor)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Prog {
    pub number: MidiValue,
    pub channel: Channel,
}

impl<'a> Deserialize<'a> for Prog {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        fn channel_default() -> Channel {
            Channel::Global
        }

        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        #[serde(expecting = "program change definition (table)")]
        struct Fields {
            number: MidiValue,
            #[serde(default = "channel_default")]
            channel: Channel,
        }

        impl From<Fields> for Prog {
            fn from(f: Fields) -> Self {
                Self {
                    number: f.number,
                    channel: f.channel,
                }
            }
        }

        struct Visitor;

        impl<'a> de::Visitor<'a> for Visitor {
            type Value = Prog;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "program chanel definition (integer or table)")
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Prog {
                    number: MidiValue::deserialize(v.into_deserializer())?,
                    channel: channel_default(),
                })
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match v.try_into() {
                    Ok(v) => self.visit_u64(v),
                    Err(_) => {
                        Err(E::invalid_type(de::Unexpected::Signed(v), &self))
                    }
                }
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'a>,
            {
                Fields::deserialize(MapAccessDeserializer::new(map))
                    .map(Into::into)
            }
        }

        deserializer.deserialize_map(Visitor)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PadAction {
    Note(Note),
    Cc(Cc),
    Prog(Prog),
    Key(Keypress),
}

#[derive(Clone, Copy, Debug)]
pub struct Pad {
    pub color: MidiValue,
    pub action: PadAction,
}

impl Control for Pad {
    fn compile<W: Write>(
        &self,
        address: u8,
        writer: &mut W,
        config: &CompileCfg,
    ) -> io::Result<()> {
        match self.action {
            PadAction::Note(v) => def::Definition {
                address,
                color: self.color,
                opt: def::Opt::builder(config)
                    .channel(v.channel)
                    .behavior(v.behavior)
                    .velocity(v.velocity)
                    .aftertouch(config.aftertouch)
                    .done(),
                payload: def::Payload::Note {
                    pitch: v.pitch,
                    velocity: v.velocity,
                },
            },
            PadAction::Cc(v) => def::Definition {
                address,
                color: self.color,
                opt: def::Opt::builder(config)
                    .channel(v.channel)
                    .behavior(v.behavior)
                    .done(),
                payload: def::Payload::Cc {
                    number: v.number,
                    min: v.off,
                    max: v.on,
                },
            },
            PadAction::Prog(v) => def::Definition {
                address,
                color: self.color,
                opt: def::Opt::builder(config)
                    .channel(v.channel)
                    .behavior(Behavior::Toggle)
                    .done(),
                payload: def::Payload::Prog(v.number),
            },
            PadAction::Key(v) => def::Definition {
                address,
                color: self.color,
                opt: def::Opt::builder(config).key(v).done(),
                payload: def::Payload::Key(v.code),
            },
        }
        .compile(writer)
    }
}

#[derive(Clone, Debug)]
pub struct PadCfg {
    keypress: bool,
    velocity: VelocityCfg,
    color: Option<MidiValue>,
    name: &'static str,
}

impl PadCfg {
    pub const fn new(velocity: VelocityCfg) -> Self {
        Self {
            keypress: false,
            velocity,
            color: None,
            name: "pad",
        }
    }

    pub const fn keypress(mut self, allowed: bool) -> Self {
        self.keypress = allowed;
        self
    }

    pub const fn color(mut self, fixed_color: MidiValue) -> Self {
        self.color = Some(fixed_color);
        self
    }

    pub const fn name(mut self, name: &'static str) -> Self {
        self.name = name;
        self
    }
}

impl<'a> DeserializeConfig<'a, PadCfg> for Pad {
    fn deserialize<D>(
        deserializer: D,
        config: &PadCfg,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        match DeserializeConfig::deserialize(
            deserializer,
            &OptionalPadCfg {
                required: true,
                pad: config,
            },
        )? {
            Optional::None => unreachable!(),
            Optional::Some(pad) => Ok(pad),
        }
    }
}

struct OptionalPadCfg<'a> {
    required: bool,
    pad: &'a PadCfg,
}

impl OptionalPadCfg<'_> {
    fn note(&self) -> NoteCfg {
        NoteCfg::new(self.pad.velocity)
    }
}

impl<'a> DeserializeConfig<'a, OptionalPadCfg<'_>> for Optional<Pad> {
    fn deserialize<D>(
        deserializer: D,
        config: &OptionalPadCfg<'_>,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        enum Field {
            Color,
            Note,
            Cc,
            Prog,
            Keypress,
        }

        #[derive(Clone, Copy)]
        struct UnknownField<'a> {
            name: &'a str,
            cfg: &'a PadCfg,
        }

        impl Display for UnknownField<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                if self.name == "color" {
                    return write!(
                        f,
                        "the color of this {} cannot be customized",
                        self.cfg.name,
                    );
                }
                let allowed = [
                    self.cfg.color.is_none().then_some("color"),
                    Some("note"),
                    Some("cc"),
                    Some("prog"),
                    self.cfg.keypress.then_some("keypress"),
                ];
                write!(
                    f,
                    "unknown key `{}`; expected {}",
                    self.name.escape_default(),
                    parse::one_of(allowed.into_iter().flatten()),
                )
            }
        }

        struct FieldVisitor<'a> {
            cfg: &'a PadCfg,
        }

        impl<'a> de::Visitor<'a> for FieldVisitor<'_> {
            type Value = Field;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "key")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match v {
                    "color" if self.cfg.color.is_none() => Ok(Field::Color),
                    "note" => Ok(Field::Note),
                    "cc" => Ok(Field::Cc),
                    "prog" => Ok(Field::Prog),
                    "keypress" if self.cfg.keypress => Ok(Field::Keypress),
                    _ => Err(E::custom(UnknownField {
                        name: v,
                        cfg: self.cfg,
                    })),
                }
            }
        }

        impl<'a> DeserializeConfig<'a, PadCfg> for Field {
            fn deserialize<D>(
                deserializer: D,
                config: &PadCfg,
            ) -> Result<Self, D::Error>
            where
                D: Deserializer<'a>,
            {
                deserializer.deserialize_str(FieldVisitor {
                    cfg: config,
                })
            }
        }

        #[derive(Clone, Copy)]
        struct DuplicateAction<'a> {
            old: &'a PadAction,
            new: &'a Field,
        }

        impl Display for DuplicateAction<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let old = match self.old {
                    PadAction::Note(_) => "note",
                    PadAction::Cc(_) => "cc",
                    PadAction::Prog(_) => "prog",
                    PadAction::Key(_) => "keypress",
                };
                let new = match self.new {
                    Field::Note => "note",
                    Field::Cc => "cc",
                    Field::Prog => "prog",
                    Field::Keypress => "keypress",
                    _ => unreachable!(),
                };
                if old == new {
                    write!(f, "duplicate key `{old}`")
                } else {
                    write!(f, "conflicting keys: `{old}` and `{new}` ")?;
                    write!(f, "(only one action allowed)")
                }
            }
        }

        #[derive(Clone, Copy)]
        struct MissingAction<'a> {
            cfg: &'a PadCfg,
        }

        impl Display for MissingAction<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let allowed = ["note", "cc", "prog"]
                    .into_iter()
                    .chain(self.cfg.keypress.then_some("keypress"));
                write!(f, "missing action: expected one of these keys: ")?;
                allowed.enumerate().try_for_each(|(i, key)| {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", key.escape_default())
                })
            }
        }

        struct Visitor<'a> {
            cfg: &'a OptionalPadCfg<'a>,
        }

        impl<'a> de::Visitor<'a> for Visitor<'_> {
            type Value = Optional<Pad>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{} definition (table)", self.cfg.pad.name)
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'a>,
            {
                let mut empty = true;
                let mut color = self.cfg.pad.color;
                let mut action = None;
                while let Some(field) =
                    map.next_key_seed(ConfigSeed::new(self.cfg.pad))?
                {
                    empty = false;
                    let check_action = || {
                        if let Some(a) = &action {
                            Err(de::Error::custom(DuplicateAction {
                                old: a,
                                new: &field,
                            }))
                        } else {
                            Ok(())
                        }
                    };
                    match field {
                        Field::Color => {
                            parse::check_dup(&color, "color")?;
                            color = Some(map.next_value()?);
                        }
                        Field::Note => {
                            check_action()?;
                            let cfg = self.cfg.note();
                            let seed = ConfigSeed::new(&cfg);
                            let note = map.next_value_seed(seed)?;
                            action = Some(PadAction::Note(note));
                        }
                        Field::Cc => {
                            check_action()?;
                            action = Some(PadAction::Cc(map.next_value()?));
                        }
                        Field::Prog => {
                            check_action()?;
                            action = Some(PadAction::Prog(map.next_value()?));
                        }
                        Field::Keypress => {
                            check_action()?;
                            action = Some(PadAction::Key(map.next_value()?));
                        }
                    }
                }
                if empty && !self.cfg.required {
                    return Ok(Optional::None);
                }
                let missing = de::Error::missing_field;
                Ok(Optional::Some(Pad {
                    color: color.ok_or_else(|| missing("color"))?,
                    action: action.ok_or_else(|| {
                        de::Error::custom(MissingAction {
                            cfg: self.cfg.pad,
                        })
                    })?,
                }))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if self.cfg.required {
                    Err(E::invalid_type(de::Unexpected::Option, &self))
                } else {
                    Ok(Optional::None)
                }
            }

            fn visit_some<D>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'a>,
            {
                deserializer.deserialize_map(self)
            }
        }

        let visitor = Visitor {
            cfg: config,
        };
        if config.required {
            deserializer.deserialize_map(visitor)
        } else {
            deserializer.deserialize_option(visitor)
        }
    }
}

impl<'a> DeserializeConfig<'a, PadCfg> for Optional<Pad> {
    fn deserialize<D>(
        deserializer: D,
        config: &PadCfg,
    ) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        DeserializeConfig::deserialize(
            deserializer,
            &OptionalPadCfg {
                required: false,
                pad: config,
            },
        )
    }
}
