# MARSWM

A rusty successor to [moonwm](https://github.com/jzbor/moonwm) - my attempt at implementing a window manager with its own library.

*DISCLAIMER: This is still in development. The library API as well as the window manager itself might be subject to frequent changes. Neither can be considered stable yet.*

## The Components

### marswm

`marswm`'s goal is to have a simple tiling window manager with tiling and workspaces for myself.
However currently neither workspaces, nor tiling, nor multi-monitor support is implemented.

### libmars

`libmars` aims expose xlib's ugly sides through a nice rusty interface that makes it easier to implement custom window managers.
Although not currently planned a wayland backend (as well as other backends) would be possible to implement due to the libraries modular concept.
