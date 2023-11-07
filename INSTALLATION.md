# Installation

## Archlinux ([AUR](https://aur.archlinux.org/packages/marswm))
```sh
paru -S marswm
# or
yay -S marswm
```

[`marswm-git`](https://aur.archlinux.org/packages/marswm) is also available as the development version.


## NetBSD ([Official repositories](https://pkgsrc.se/wm/marswm/))
```sh
pkgin install marswm
```

or, if you prefer to build it from source

```sh
cd /usr/pkgsrc/wm/marswm
make install
```


## Nix
`marswm` is currently not officially packaged for Nix.
You can use the derivation in [`examples/default.nix`](./examples/default.nix) to install it on your machine.
Make sure to update the version number and hash accordingly.


## Other ([cargo](https://crates.io/crates/marswm))
This guide shows installation for a standard Linux distribution that supports the Standard File Hierarchy.

For non-standard distributions (e.g. `doas` instead of `sudo`, no FHS-support) you will have to change some things.
But in that case chances are you know what to do anyway.

First make sure you have the following libraries installed natively via your package manager: `libX11`, `libXft`, `libXinerama`, `libXrandr`.
Make sure to also include their development version if your distribution splits up packages in this manner.

Then you can build and install `marswm` and its components with cargo:
```sh
sudo cargo install --root=/usr/local/ marswm marsbar mars-relay
```

To run `marswm` directly from your display manager of choice you will have to add a `.desktop` file.
You can copy [`./marswm.desktop`](./marswm.desktop), but make sure to replace `PATH` with your actual path (e.g. `/usr/local/bin`).
Usually it goes into `/usr/share/xsessions`.
