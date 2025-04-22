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

use std::io::{self, Write};

#[derive(Clone, Copy)]
struct FmtNode<'a> {
    fmt: &'static str,
    prev: Option<&'a Self>,
    depth: usize,
}

impl FmtNode<'static> {
    pub const BASE: Self = Self {
        fmt: "",
        prev: None,
        depth: 1,
    };
}

impl<'a> FmtNode<'a> {
    pub fn new(prev: &'a Self, fmt: &'static str) -> Self {
        Self {
            fmt,
            prev: Some(prev),
            depth: prev.depth + 1,
        }
    }

    pub fn depth(&self) -> usize {
        self.depth
    }

    fn emit_body<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        if let Some(prev) = self.prev {
            prev.emit_body(writer)?;
            write!(writer, ";{}", self.fmt)
        } else {
            write!(writer, "\x1b[{}", self.fmt)
        }
    }

    pub fn emit<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.emit_body(writer)?;
        write!(writer, "m")
    }

    pub fn emit_single<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        write!(writer, "\x1b[{}m", self.fmt)
    }
}

pub struct FmtWriter<'a, W: Write> {
    base: &'a mut AnsiWriter<W>,
    node: FmtNode<'a>,
}

impl<W: Write> FmtWriter<'_, W> {
    pub fn with_fmt(&mut self, fmt: &'static str) -> FmtWriter<'_, W> {
        if self.base.depth != self.node.depth() {
            self.base.depth = 0;
        }
        FmtWriter {
            base: self.base,
            node: FmtNode::new(&self.node, fmt),
        }
    }

    pub fn borrow(&mut self) -> FmtWriter<'_, W> {
        FmtWriter {
            base: self.base,
            node: self.node,
        }
    }

    fn activate(&mut self) -> io::Result<()> {
        if self.base.mode == Mode::Plain
            || self.base.depth == self.node.depth()
        {
            return Ok(());
        }
        if self.base.depth == self.node.depth() - 1 {
            self.node.emit_single(&mut self.base.writer)
        } else {
            self.node.emit(&mut self.base.writer)
        }?;
        self.base.depth = self.node.depth();
        Ok(())
    }
}

impl<W: Write> Write for FmtWriter<'_, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.activate()?;
        self.base.writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.base.writer.flush()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    Fancy,
    Plain,
}

#[derive(Clone, Debug)]
pub struct AnsiWriter<W: Write> {
    writer: W,
    /// The depth of the most recently used [`FmtNode`].
    depth: usize,
    mode: Mode,
}

impl<W: Write> AnsiWriter<W> {
    pub fn new(writer: W, mode: Mode) -> Self {
        Self {
            writer,
            depth: 0,
            mode,
        }
    }

    pub fn fancy(writer: W) -> Self {
        Self::new(writer, Mode::Fancy)
    }

    pub fn into_inner(self) -> (W, io::Result<()>) {
        let mut this = std::mem::ManuallyDrop::new(self);
        let r = this.finalize();
        // SAFETY: `this` is `ManuallyDrop` and we don't use it after
        // bitwise-copying `this.writer`.
        (unsafe { std::ptr::read(&this.writer) }, r)
    }

    pub fn with_fmt(&mut self, fmt: &'static str) -> FmtWriter<'_, W> {
        if self.depth != 1 {
            self.depth = 0;
        }
        FmtWriter {
            base: self,
            node: FmtNode::new(&FmtNode::BASE, fmt),
        }
    }

    pub fn borrow(&mut self) -> FmtWriter<'_, W> {
        FmtWriter {
            base: self,
            node: FmtNode::BASE,
        }
    }

    pub fn finalize(&mut self) -> io::Result<()> {
        self.borrow().activate()
    }
}

impl<W: Write> Write for AnsiWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.borrow().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl<W: Write> Drop for AnsiWriter<W> {
    fn drop(&mut self) {
        let _ = self.finalize();
    }
}
