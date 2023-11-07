#/bin/sh

set +o nounset

SEPARATOR='\x1f'
BATTERY_PATH="$(find /sys/class/power_supply -maxdepth 1 -mindepth 1 | grep -i bat | head -n 1)"


### HELPERS

confirmation_submenu () {
	printf "\n\tYou sure?\n\t\t%s" "$1"
}

gen_media_menu () {
	for player in $(playerctl -l); do
		gen_player_menu "$player"
	done
}

gen_player_menu () {
	echo "$1"
	printf "\t%s - %s\n" "$(property_for_player "$1" title)" "$(property_for_player "$1" artist)"
	printf "\t%s\tplayerctl play-pause -p \"%s\"\n" "$(play_pause_label "$1")" "$1"
	printf "\tnext\tplayerctl next -p \"%s\"\n" "$1"
	printf "\tprevious\tplayerctl prev -p \"%s\"\n" "$1"
}

pa_volume () {
	pactl get-sink-volume @DEFAULT_SINK@ | grep "Volume" | sed 's/.*\/\s*\(.*\) \s*\/.*/\1/;'
}

pa_muted () {
	if pactl get-sink-mute @DEFAULT_SINK@ | grep no > /dev/null; then
		return 1
	else
		return 0
	fi
}

pa_loop () {
	pactl subscribe | grep --line-buffered "Event 'change' on sink " | while read -r _; do
		update_blocks
	done
}

play_pause_label () {
	if [ "$(playerctl status -p "$1")" = "Playing" ]; then
		echo "pause"
	else
		echo "play"
	fi
}

property_for_player () {
	playerctl metadata -p "$1" | grep "xesam:$2" | sed 's/^\([a-zA-Z]*\) xesam:\([a-zA-Z]*\) *\(.*\)/\3/'
}

audio_menu () {
	SINK_MENU="$(pactl list sinks | grep "Name: \|Description:" \
		| sed 'N; s/\t*Name: \(.*\)\n\t*Description: \(.*\)/\t\2\tpactl set-default-sink \1/')"
	SOURCE_MENU="$(pactl list sources | grep "Name: \|Description:" \
		| sed 'N; s/\t*Name: \(.*\)\n\t*Description: \(.*\)/\t\2\tpactl set-default-source \1/')"
	printf "Change default output\n%s\nChange default input\n%s" "$SINK_MENU" "$SOURCE_MENU" | xmenu | sh
}

media_menu () {
	gen_media_menu | xmenu | sh
}

system_menu () {
SYSTEM_MENU="Logout $(confirmation_submenu 'pkill marswm')
Suspend $(confirmation_submenu 'systemctl suspend')
Poweroff $(confirmation_submenu poweroff)
Reboot $(confirmation_submenu reboot)

Output Profile
$(find ~/.screenlayout -type f | sed 's/^\(.*\)\/\(.*\)\(\.sh\)/\t\2\tsh \1\/\2\3/')"
	echo "$SYSTEM_MENU" | xmenu | sh
}


### BUTTON HANDLERS

battery_button () {
	profile="$(powerprofilesctl list | sed '/^   /d;/^$/d;s/\(.*\):/\1/' | xmenu | sed 's/.* //')"
	if [ -n "$profile" ]; then
		powerprofilesctl set "$profile"
	fi
	# case "$BUTTON" in
	# 	1) pademelon-widgets ppd-dialog ;;
	# esac
}

volume_button () {
	case "$BUTTON" in
		1) media_menu ;;
		2) pactl set-sink-mute @DEFAULT_SINK@ toggle ;;
		3) audio_menu ;;
		4) pactl set-sink-volume @DEFAULT_SINK@ +5% \
			&& canberra-gtk-play -i audio-volume-change ;;
		5) pactl set-sink-volume @DEFAULT_SINK@ -5% \
			&& canberra-gtk-play -i audio-volume-change ;;
	esac
}

date_button () {
	case "$BUTTON" in
		3) system_menu ;;
		*) notify-send "$(date)" ;;
	esac
}


### STATUS BLOCKS

volume_block () {
	if pa_muted; then
		printf 'volume: muted'
	else
		printf 'volume: %s' "$(pa_volume)"
	fi
}

battery_block () {
	if [ -e "$BATTERY_PATH" ]; then
		status="$(cat "$BATTERY_PATH/status")"
		if [ "$status" = 'Charging' ]; then
			printf 'charging: %s' "$(cat "$BATTERY_PATH/capacity")%"
		else
			printf 'battery: %s' "$(cat "$BATTERY_PATH/capacity")%"
		fi
	else
		echo "plugged in"
	fi
}

date_block () {
	printf 'date: %s' "$(date +%H:%M)"
}

blocks () {
	printf "%s$SEPARATOR" "$(volume_block)"
	printf "%s$SEPARATOR" "$(battery_block)"
	printf "%s" "$(date_block)"
}

update_blocks () {
	mars-relay set-status "$(blocks)"
}


loop () {
	(pa_loop) &

	while true; do
		update_blocks
		sleep 10
	done
}

if [ "$1" = "action" ]; then
	case "$BLOCK" in
		0) volume_button ;;
		1) battery_button ;;
		2) date_button ;;
		_) echo unhandled ;;
	esac
else
	loop
fi
