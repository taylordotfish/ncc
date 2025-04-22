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

use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, BufWriter, Read, Write};
use std::path::Path;
use std::process::{Command, ExitCode};

mod args;
use args::{Args, Usage};

fn must_quote(b: u8) -> bool {
    !matches!(
        b,
        b'A'..=b'Z'
        | b'a'..=b'z'
        | b'0'..=b'9'
        | b'-' | b'_' | b'+' | b'='
        | b':' | b'/' | b'.' | b','
    )
}

fn repr<T, W>(arg: T, writer: &mut W) -> io::Result<()>
where
    T: AsRef<OsStr>,
    W: Write,
{
    let bytes = arg.as_ref().as_encoded_bytes();
    if bytes.is_empty() {
        return writer.write_all(b"\"\"");
    }
    if !bytes.iter().copied().any(must_quote) {
        return writer.write_all(bytes);
    }
    writer.write_all(b"\"")?;
    bytes.iter().copied().try_for_each(|b| match b {
        b'\\' => writer.write_all(b"\\\\"),
        b'"' => writer.write_all(b"\\\""),
        _ => writer.write_all(&[b]),
    })?;
    writer.write_all(b"\"")
}

fn run_verbose<P, A, T>(program: P, args: A) -> Result<(), ()>
where
    P: AsRef<OsStr>,
    A: Copy + IntoIterator<Item = T>,
    T: AsRef<OsStr>,
{
    let program = program.as_ref();
    (|| {
        let mut stdout = BufWriter::new(io::stdout().lock());
        write!(stdout, "> ")?;
        repr(program, &mut stdout)?;
        args.into_iter().try_for_each(|arg| {
            write!(stdout, " ")?;
            repr(arg, &mut stdout)
        })?;
        writeln!(stdout)
    })()
    .map_err(|e| {
        eprintln!("error writing to stdout: {e}");
    })?;

    let status = Command::new(program).args(args).status().map_err(|e| {
        eprintln!("error running `{}`: {e}", program.to_string_lossy());
    })?;
    if status.success() {
        return Ok(());
    }
    let program_str = program.to_string_lossy();
    match status.code() {
        Some(code) => eprintln!(
            "error: `{program_str}` returned non-zero exit code {code}",
        ),
        None => eprintln!(
            "error: `{program_str}` exited unsuccessfully ({status})",
        ),
    }
    Err(())
}

fn ensure_syx<P: AsRef<Path>>(path: P) -> Result<(), ()> {
    let path = path.as_ref();
    let metadata = fs::metadata(path).map_err(|e| {
        eprintln!("error: could not read `{}`: {e}", path.display());
    })?;
    let ft = metadata.file_type();
    if !ft.is_file() {
        eprintln!("error: not a file: {}", path.display());
        return Err(());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        if ft.is_block_device()
            || ft.is_char_device()
            || ft.is_fifo()
            || ft.is_socket()
        {
            // Don't attempt to check anything that isn't a regular file on
            // disk.
            return Ok(());
        }
    }
    match File::open(path).and_then(|f| f.bytes().next().transpose()) {
        Err(e) => {
            eprintln!("error: could not read `{}`: {e}", path.display());
            Err(())
        }
        Ok(Some(0xf0)) => Ok(()),
        Ok(_) => {
            eprintln!("error: not a SysEx file: {}", path.display());
            Err(())
        }
    }
}

fn run() -> Result<(), ()> {
    let mut args = std::env::args_os();
    let arg0 = args.next();
    let bin = arg0
        .as_ref()
        .and_then(|s| Path::new(s).file_name()?.to_str())
        .unwrap_or("ncc-alsa-send");

    let usage = Usage::new(bin);
    let args = Args::parse(args).map_err(|e| {
        eprintln!("error: {e}");
        eprintln!("See `{bin} --help` for usage information.");
    })?;
    let (port, file) = match args {
        Args::Empty => {
            eprintln!("{usage}");
            return Err(());
        }
        Args::Help => {
            println!("{usage}");
            return Ok(());
        }
        Args::Version => {
            println!("ncc-alsa-send {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Args::Run {
            port,
            file,
        } => (port, file),
    };

    ensure_syx(&file)?;
    run_verbose("amidi", ["-p".as_ref(), &*port, "-s".as_ref(), &*file])?;
    run_verbose("amidi", [
        "-p".as_ref(),
        &*port,
        "-d".as_ref(),
        "-t1".as_ref(),
    ])
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
    }
}
