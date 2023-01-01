# MARSWM

A rusty successor to [moonwm](https://github.com/jzbor/moonwm) - my attempt at implementing a window manager with its own library.

*DISCLAIMER: Although already usable this is still in development. The library API as well as the window manager itself might be subject to frequent changes.*

## The Components

### marswm

`marswm`'s goal is to have a simple tiling window manager with tiling and workspaces for myself.

Features:
* dwm-style layouts
* workspaces (like i3 - unlike dwm)
* IPC using X11 atoms (`mars-relay`)
* YAML for configuration and key bindings

### libmars

`libmars` aims expose xlib's ugly sides through a nice rusty interface that makes it easier to implement custom window managers.
It is still not very mature and mainly targeted to suit `marswm`'s needs, but it should be great for writing simple, personal window managers once the API is somewhat stable and documented.
Although not currently planned a wayland backend (as well as other backends) would be possible to implement due to the libraries modular concept.

### mars-relay

`mars-relay` lets you control EWMH-compliant X11 window managers and can be used as an IPC-client for `marswm` and a lot of other window managers.
