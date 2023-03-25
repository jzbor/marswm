# MARSBAR

`marsbar` is a minimalist status bar for `marswm` and other X11/EWMH Window Managers.

The configuration should be stored in the [YAML](https://yaml.org/) format at  `~/.config/marswm/marsbar.yaml`
You can get the default configuration with `marsbar --print-default-config`.

## The Status Script
You can set the status on the right side of the bar with a custom skript or program.
On X11 it uses the custom property `_MARS_STATUS` on the root window.
You can use any program to set it, but `mars-relay` also supports the `set-status` command:
```sh
mars-relay set-status "Today is $(date +%F)"
```

You also have the possibility to use multiple **modules** for different metrics.
They are separated by a special character, the default is currently `0x1f`.
In a shell script you could use it like so:
```sh
load="$(cut -d' ' -f1 /proc/load)"
date="$(date +%F)"
status="$(printf "%s\x1f%s" "load: $load" "date: $date")"
mars-relay set-status "$status"
```

The script/program is expected to update the status on its own.
It can either be started by your own startup scripts/systemd/etc. or by `marsbar` itself.
To launch the script with `marsbar` you have to make sure the script is executable (`chmod +x`).
Then you can add it to the config file under the option `status_cmd`.

### Button Actions
`marsbar` also lets you handle button clicks for those status blocks.
These are handled by a script/program which can be a different executable or just the same as used for status updates.
Place the path to the executable under the `action_cmd` option in the config file.

When a button is pressed that executable is called with the environment variables `BLOCK` and `BUTTON` are set:
* `$BLOCK` contains the index of the status block that was clicked
* `$BUTTON` contains the index number of the mouse button that generated the event


## Theming
Theming is available under the `style` subsection in the configuration file.

This section might look something like this:
```yaml
style:
  background: 0x262626             # background color of the bar
  expand_workspace_widgets: false  # make all workspace widgets the same width
  height: 31                       # height of the whole bar
  font: FiraCode:size=12           # font of text surfaces (as xft name)
  workspaces:
    foreground: 0x262626           # foreground (text) color of the workspace widget
    inner_background: 0x5F87AF     # background of the individual workspaces
    outer_background: 0x262626     # background *around* the individual workspaces
    padding_horz: 0                # horizontal padding around the workspaces
    padding_vert: 0                # vertical padding around the workspaces
    text_padding_horz: 10          # horizontal padding around the text
    text_padding_vert: 4           # vertical padding around the text
    spacing: 0                     # spacing between the individual workspaces
  title:
    foreground: 0xBCBCBC           # foreground (text) color
    background: 0x262626           # background color of the text widget
  status:
    foreground: 0x262626           # foreground (text) color
    inner_background: 0xAF5F5F     # background of the individual blocks
    outer_background: 0x262626     # background *around* the individual blocks
    padding_horz: 4                # horizontal padding around the blocks
    padding_vert: 4                # vertical padding around the blocks
    text_padding_horz: 5           # horizontal padding around the text
    text_padding_vert: 0           # vertical padding around the text
    spacing: 4                     # spacing between the individual blocks
```

*Note: Although they may look very weird in the output of `marsbar --print-default-config` colors can simply be written as hex values (like `0x1a2b3c`).*

