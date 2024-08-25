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
* Launchpad X: full support
* Launchpad Mini \[MK3]: full support

[^1]: Tested on real hardware.

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

[examples]: https://github.com/taylordotfish/ncc/tree/0.1.0/examples/

Installing custom modes
-----------------------

To install custom modes on your device, the compiled `.syx` file needs to be
sent to the device as MIDI (and the response from the device must be read). The
way to do this depends on your operating system. [A guide is available][alsa]
for GNU/Linux systems using ALSA.

[alsa]: https://github.com/taylordotfish/ncc/blob/0.1.0/doc/alsa.md
