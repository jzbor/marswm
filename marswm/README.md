# MARSWM
A modern window manager featuring dynamic tiling (rusty successor to [moonwm](https://github.com/jzbor/moonwm)).

![An example image](https://i.imgur.com/1cBa1Hf.png)

The [YAML](https://yaml.org/) format is used for configuration with the default file path being `~/.config/marswm/marswm.yaml`.
You can get the default configuration with `marswm --print-default-config`.

## Multi-Monitor Setups and Workspaces
The window manager supports multi-monitor setups, although they are not as well tested as they probably should be for daily usage.
Every (non-overlapping) monitor gets its own set of workspaces, which is also exposed as such to other applications like status bars.
You can configure the number of the primary monitor and secondary monitors with the `primary_workspaces` and the `secondary_workspaces` option respectively.

It is suggested to use a relatively low number of workspaces for secondary monitors as they might clutter your bar otherwise.


## Initial Window Placement
You can specify where windows should be placed initially (applies to floating windows only).
Possible settings are:
* `center` - center the window on the screen
* `pointer` - place the window below the pointer
* `wherever` - don't care about placing the window at a special position

The corresponding setting is called `initial_placement`.


## Layouts
`marswm` supports dynamic tiling and takes a lot of inspiration for it from [dwm](https://dwm.suckless.org).

Currently the following layouts are supported:
* `floating` - the clients are not automatically tiled in any way and can be freely positioned by the user
* `stack` - other windows are tiled vertically to the right of the main windows
* `bottom-stack` - other windows are tiled horizontally below the main windows
* `monocle` - all window are stacked on top of each other and fill the whole area
* `deck` - other windows are stacked to the right of the main windows on top of each other
* `dynamic` - this one is a little more complicated and is described in more detail down below

You can influence the layout of the windows with different parameters.
All of the following options belong in the `layout` section:
* `default` - specifies the default layout for new workspaces
* `gap_width` - size of the gap between windows and between the windowing area and the screen edge
* `main_ratio` - share of space that the main windows take on the screen
* `nmain` - how many windows the main area contains on a new workspace

Some of these values can be changed at runtime through respective key bindings.

### The `dynamic` Layout
As the name suggest the dynamic layout can be used to implement a variety of different layouts.
It is configured by these two parameters (also in the `layout` section of the configuration file):
* `stack_position` - specifies where the stack windows should be placed in relation to the main windows
* `stack_mode` - describes whether the stack windows should be in a `split` or `deck` configuration


## Theming
You can configure different parts of how `marswm` looks in the `theming` section of the configuration file.

These attributes influence the coloring of window borders:
* `active_color` - frame color of currently focused window
* `inactive_color` - frame color of unfocused windows
* `border_color` - color of the inner and outer border around the frame

*Note: Although they may look very weird in the output of `marswm --print-default-config` colors can simply be written as hex values (like `0x1a2b3c`).*

To show a window's title at the top of its frame use these settings:
* `show_title` - a boolean value determining whether the title is shown or not
* `font` - the font that is used for drawing the title

Attributes specifying width are all in pixels:
* `frame_width` - tuple describing the width of the frame on each side (excluding inner and outer borders)
* `inner_border_width` - inner border between the window content and frame
* `outer_border_width` - outer border around the window frame
* `title_vpadding` - vertical padding for title
* `title_hpadding` - horizontal padding for title

There is also a sub-section for the border configuration of windows that usually don't want to be decorated.
It is part of the general `theming` section and is called `no_decoration`.
The values `frame_width`, `inner_border_width` and `outer_border_width` are available and work the same as with normal windows.


## Key Bindings
`marswm` comes with a set of default key bindings.
Call `marswm --print-default-keys` to get an overview of them.

In contrast to the other sections of this manual the keybindings are not configured in the default configuration file.
Instead they are read from a separate YAML file (usually in `~/.config/marswm/keybindings.yaml`).
The bindings in that file will overwrite the default bindings.
If you wish to just extend the default key bindings by some custom ones you can use the file `~/.config/marswm/keybindings_ext.yaml` which will then get merged with the default key bindings.

A key binding entry consists of a list of `modifers`, the `key` you want to bind as well as an `action` to execute as soon as a key is pressed.
Here is an example:
```YAML
- modifiers: [Mod4, Shift]
  key: '1'
  action: !move-workspace 0
```

You can find documentation for actions [in the docs](bindings::BindingAction) or [in the source code](src/bindings.rs).


## Button Bindings
Button actions can be configured similarly to key bindings in the files `~/.config/marswm/buttonbindings.yaml` and `~/.config/marswm/buttonbindings_ext.yaml` respectively.
`marswm --print-default-buttons` tells you the button bindings installed by default.

The `targets` field specifies which window areas should be used for the button event.
Possible values are `window`, `frame` and `root`.
The `action`s are the same as used for key bindings.

Here is an example:
```YAML
- modifiers: [Mod4, Shift]
  button: 2
  targets: [WindowFrame, ClientWindow]
  action: close-client
```

You can find documentation for actions [in the docs](bindings::BindingAction) or [in the source code](src/bindings.rs).


## Window Rules
It is possible to configure the state of newly mapped windows with window rules.
The file `~/.config/marswm/rules.yaml` may contain a list of such rules.
The rules consist of an `identifier` part as well as configuration options and a list of `actions` to apply on matching windows.

For example:
```YAML
- identifiers:
    application: 'thunderbird'
  actions: [ !move-workspace 5 ]
```

You can find documentation for actions [in the docs](bindings::BindingAction) or [in the source code](src/bindings.rs).


### Identifiers:
* `application` - name of the application (second string of the `WM_CLASS` property on X11)
* `title` - window title

### Configuration Options:
* `actions` - list of binding actions to execute for the new window
* `floating` - specify whether a window should initially be tiled or floating
* `ignore_window` - leads to the window not being managed by the window manager
* `initial_placement` - allows overwriting the placement value in your general configuration
* `workspace` - set to the workspace you would prefer the application to launch on

