#!/bin/sh

WALLPAPER_FILE="$HOME/.background-image"

xwallpaper --zoom "$WALLPAPER_FILE"

xev -root -event randr \
	| grep --line-buffered XRROutputChangeNotifyEvent \
	| while read -r; do
	xwallpaper --zoom "$WALLPAPER_FILE"
done
