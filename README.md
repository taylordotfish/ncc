<!-- This file is automatically generated from .misc/README.m4. -->
ncc
===

ncc compiles text-based configuration files into custom modes for Novation MIDI
devices.

The behavior and appearance of controls on the device are specified using
[TOML] files, which are then compiled by ncc into MIDI SysEx messages that can
be sent to the hardware to apply the custom mode.

[TOML]: https://toml.io

*ncc is not affiliated with Novation or its parent, Focusrite plc.*

Supported devices
-----------------

* Launchkey \[MK3]: full support
* Launchkey Mini \[MK3]: full support, tested[^1]
* FLkey: full support
* FLkey Mini: full support
* Launchpad X: full support, tested[^1]
* Launchpad Mini \[MK3]: full support

[^1]: Tested on real hardware.

Installation
------------

Ensure [Rust][rust-inst] 1.74 or later is installed. Then install with Cargo:

[rust-inst]: https://www.rust-lang.org/tools/install

```bash
cargo install ncc
```

This will install a binary named `ncc` in `~/.cargo/bin`. If that directory is
in your `PATH`, you can run the program simply by typing `ncc` in your shell:

```console
$ ncc --version
ncc 0.1.3
```

<details>
<summary>Manual installation</summary>

To compile and install ncc manually, ensure the following dependencies are
installed:

* [Rust] 1.74 or later
* [Git]

[Rust]: https://www.rust-lang.org
[Git]: https://git-scm.com

Download the source code:

```bash
git clone https://github.com/taylordotfish/ncc
cd ncc
```

Build and install the program:

```bash
cargo install --path .
```

Alternatively, you can build and run ncc locally without installing:

```console
$ cargo build --release
$ ./target/release/ncc --version
ncc 0.1.4-dev
```
</details>

Usage
-----

See `ncc --help` for detailed usage information. The simplest use of ncc is
`ncc <file>`, which compiles the TOML file `<file>` into a SysEx file with the
same name but ending in `.syx`:

```console
$ cd examples/launchkey-mini-mk3
$ ls example-pads*
example-pads.toml
$ ncc example-pads.toml
$ ls example-pads*
example-pads.toml  example-pads.syx
```

See the [examples] directory for a demonstration of how to write custom modes
for ncc.

[examples]: examples/

Installing custom modes
-----------------------

To install custom modes on your device, the compiled `.syx` file needs to be
sent to the device as MIDI (and the response from the device must be read). The
way to do this depends on your operating system. [A guide is available][alsa]
for GNU/Linux systems using ALSA.

[alsa]: doc/alsa.md

License
-------

ncc is licensed under version 3 of the GNU Affero General Public License, or
(at your option) any later version. See [LICENSE](LICENSE).

The example `.toml` files in the [examples] directory have additionally been
released to the public domain using [CC0].

[examples]: examples/
[CC0]: https://creativecommons.org/publicdomain/zero/1.0/

Contributing
------------

Pull requests are welcome. By contributing to ncc, you agree that your
contribution may be used under the terms of ncc’s license.
