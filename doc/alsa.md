Installing custom modes on GNU/Linux
====================================

On GNU/Linux systems using ALSA, compiled custom modes (`.syx`) can be sent to
your Novation MIDI device as follows:

1. Check if `amidi` is installed by running `amidi --version` in a terminal. If
   you see `command not found`, install [alsa-utils] \(likely available in your
   distribution).

   [alsa-utils]: https://github.com/alsa-project/alsa-utils

2. Ensure the device is connected to your computer, and that no software is
   using the device. If JACK is running and using ALSA MIDI, it must be
   stopped.

3. Make sure the device is in custom mode. If the device supports multiple
   custom modes, select the mode that you want to overwrite.

4. In a terminal, run `amidi --list-devices`. Find the line that corresponds to
   your device. It should look something like:

   ```
   IO  hw:3,0,0  Launchkey Mini MK3
   ```

   If there are multiple lines for your device, use the first, but try others
   if that doesn’t work.

5. Run `ncc-alsa-send <port> <file>`, where `<port>` is the second column in
   the line of `amidi` output above (`hw:3,0,0`), and `<file>` is the compiled
   `.syx` file. For example:

   ```bash
   ncc-alsa-send hw:3,0,0 my-custom-mode.syx
   ```

   (`ncc-alsa-send` is installed in the same location as `ncc`. If that
   location is in your `PATH`, `ncc-alsa-send` should be available.)

   This is essentially a wrapper around the following commands, which, if
   desired, can be run manually instead:

   ```
   amidi --port <port> --send <file>
   amidi --port <port> --dump --timeout=1
   ```

   The second command ensures that the response from the device is read after
   the custom mode is transferred; otherwise, the change may not take effect.
