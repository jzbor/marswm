#!/bin/sh

# export RUSTFLAGS="-C prefer-dynamic"
cargo build || exit 1
XEPHYR=$(whereis -b Xephyr | cut -f2 -d' ')
xinit ./xephyr_xinitrc -- \
    "$XEPHYR" \
        :100 \
        -resizeable \
        -screen 800x600 \
        +xinerama \
        -ac \
        # -screen 400x300
        # -host-cursor

