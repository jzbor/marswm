# MARSWM Project
`marswm` aims to be the rusty successor to [moonwm](https://github.com/jzbor/moonwm).
In addition to the window manager this repository also contains the [library](./libmars) it is built on, an accompanying [status bar](./marsbar) and an [ipc client](./mars-relay) to control the window manager from external scripts.

You can find documentation on how to configure the window manger on [crates.io](https://docs.rs/crate/marswm) or in the [Github repo](https://github.com/jzbor/marswm/tree/master/marswm/README.md)

*DISCLAIMER: Although already usable this is still in development. The library API as well as the window manager itself might be subject to frequent changes.*

## The Components

### marswm

`marswm`'s goal is to have a simple tiling window manager with tiling and workspaces for myself.

Features:
* dwm-style layouts
* workspaces (similar to i3 or bspwm)
* IPC using X11 atoms (`mars-relay`)
* YAML for configuration and key bindings

### libmars

`libmars` aims expose xlib's ugly sides through a nice rusty interface that makes it easier to implement custom window managers.
It is still not very mature and mainly targeted to suit `marswm`'s needs, but it should be great for writing simple, personal window managers once the API is somewhat stable and documented.
Although not currently planned a wayland backend (as well as other backends) would be possible to implement due to the libraries modular concept.

### mars-relay

`mars-relay` lets you control EWMH-compliant X11 window managers and can be used as an IPC-client for `marswm` and a lot of other window managers.


## Installation (with package manager)
See [installation.md](./docs/installation.md).

For a guide on how to setup a working desktop environment with marswm as base take a look at [the quickstart guide](https://jzbor.de/marswm/quickstart.html).


## Building from Source
You have to install the following libraries natively: `libX11`, `libXft`, `libXinerama`, `libXrandr`.

Then you can use cargo to build the binaries.

```sh
# development build
cargo build
# release build
cargo build --release
```

The binary files will be in `target/debug` or `target/release` depending on your build type.
