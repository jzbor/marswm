# Quick Start Guide
This guide shows you how to build a fully functional desktop environment with marswm.

## Assumptions
There are a few assumptions made in these scripts with regards to your system in this guide and in the scripts:
* `marswm`, `marsbar` and `mars-relay` are already installed (see ["Installation"](/installation.html))
* You use either [PulseAudio](https://www.freedesktop.org/wiki/Software/PulseAudio/) or [pipewire](https://pipewire.org/) with `pactl` installed (probably in the `pulseaudio` package)
* Your distro uses [systemd](https://systemd.io/)

No worries, if one of those does not apply to your setup.
It should be straightforward to adjust the scripts for example for a non-systemd distro, but you have to do so on your own.


## Startup Script
First of all it makes sense to create a startup script in which we can put programs to run when our WM starts.
Create a file called `mars-startup` in your `$PATH`) (e.g. `~/.local/bin/mars-startup`):
```sh
#/bin/sh

# helper function to detect if a program is already running
is_running () {
	pgrep --uid "$UID" "$1" > /dev/null
}

# load default layout (use arandr to set it)
[ -f ~/.screenlayout/default.sh ] && /bin/sh ~/.screenlayout/default.sh;

# programs to automatically start
is_running marsbar || marsbar &
```

We will add further lines to this script later on.


## Status Script
`marsbar` can use scripts to display status information and generate menus.

You can find an example script [`mars-status`](/examples/mars-status.html) in examples section.
Read [Examples/Installing Scripts](examples.html#installing-scripts) for more information on how to install it.

Now you can add the script to your `marsbar` config in `~/.config/marswm/marsbar.yaml`:
```yaml
status_cmd: "mars-status"
action_cmd: "mars-status action"
```


## Wallpaper
This repo also contains a simple script to set your wallpaper: [`wallpaper-daemon`](examples/wallpaper-daemon.md).
It automatically adjusts your wallpaper whenever your screen configuration changes.
Read [Examples/Installing Scripts](examples.html#installing-scripts) for more information on how to install it.


Now we can add it to our autostart script:
```sh
is_running wallpaper-daem || wallpaper-daemon &
```

It will load whatever wallpaper you put in `~/.background-image`.


## Application Menu(s)
Another script provided in `examples/` is [`xdg-xmenu.py`](examples/xdg-xmenu.md).
Read [Examples/Installing Scripts](examples.html#installing-scripts) for more information on how to install it.

Now we can add it to our button bindings.
It is suggested to put it in `~/.config/marswm/buttonbindings_ext.yaml` as this way it does not interfere with other default bindings:
```yaml
- modifiers: []
  button: 3
  targets: [root]
  action: !execute xdg-xmenu -m | xmenu | /bin/sh
```
Make sure to restart the WM for the bindings to take effect.

After installing the script you can generate the icon cache it by running `xdg-xmenu -f`.
Now you should be able to access the menu when right-clicking the desktop.

### Rofi / dmenu
You will probably also want a keyboard-driven option to access your applications.
By default `marswm` comes with keybindings for [`Rofi`](https://github.com/davatorium/rofi) preconfigured.
Make sure to install and customize it to your liking, then you should be able to run it by pressing `MOD + d`.

A lightweight alternative is [dmenu](https://tools.suckless.org/dmenu/), but you will have to add your own keybindings for it to work properly.


## Audio, Media and Brightness Key Bindings
A modern desktop should also provide working key bindings for audio, media and brightness control, so let's add these (`~/.config/marswm/keybindings_ext.yaml`):
```yaml
# Volume Control
- key: XF86AudioRaiseVolume
  action: !execute pactl set-sink-volume @DEFAULT_SINK@ +5% && canberra-gtk-play -i audio-volume-change
- key: XF86AudioLowerVolume
  action: !execute pactl set-sink-volume @DEFAULT_SINK@ -5% && canberra-gtk-play -i audio-volume-change
- key: XF86AudioMute
  action: !execute pactl set-sink-mute @DEFAULT_SINK@ toggle
- key: XF86AudioMicMute
  action: !execute pactl set-source-mute @DEFAULT_SOURCE@ toggle

# Media Control
- key: XF86AudioPlay
  action: !execute playerctl play-pause -p Lollypop,spotify
- key: XF86AudioPause
  action: !execute playerctl play-pause -p Lollypop,spotify
- key: XF86AudioPrev
  action: !execute playerctl previous -p Lollypop,spotify
- key: XF86AudioNext
  action: !execute playerctl next -p Lollypop,spotify

# Brightness Control
- key: XF86MonBrightnessUp
  action: !execute light -A 10
- key: XF86MonBrightnessDown
  action: !execute light -U 10
```

Note that these keybindings depend on `pactl`, `playerctl` and `light`, so make sure to install these.


## Screenshots
Surely you will want to be able to take screenshots, so lets set up a key binding for them.
We will use [`maim`](https://github.com/naelstrof/maim), `tee` and `xclip` so make sure to have them installed.
You will also want to create a directory for your screenshots (e.g. `~/Pictures/Screenshots`).

The key binding (add to `~/.config/marswm/keybindings_ext.yaml`) looks like this:
```yaml
# Screenshots
- modifiers: [ Mod4 ]
  key: Print
  action: !execute maim -s | tee "$HOME/Pictures/Screenshots/$(date '+%Y-%m-%d_%H-%M-%S.png')" | xclip -selection clipboard -t image/png -i
```

Now once you press `Alt + Print` you will be able to select an area to take a screenshot from.
The image will be saved and copied to your clipboard.
You may want to test this once to make sure everything works.


## Touch Gestures
There is an [example config](examples/touchegg.md) for [Touchégg](https://github.com/JoseExposito/touchegg) provided along with this repo.
Copy the file to `~/.config/touchegg/touchegg.conf`.
Then [install and setup](https://github.com/JoseExposito/touchegg#installation) touchegg (it requires a server daemon to run, as well as a client).

You will then be able to cycle through workspaces using three fingers on your touchpad, as well as accessing the window menu (swipe down).


## Additional Suggestions
These are additional programs suggested to complete your desktop setup:
* `nmapplet` - applet for managing your network (e.g. Wifi setup) (only works if your machine uses NetworkManager)
* `blueman` + `blueman-applet` - GUI and applet for managing your Bluetooth connections
* `arandr` - GUI to setup screen configurations
