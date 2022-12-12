#test programs
pademelon-tools load-wallpaper
xmodmap - < ~/.config/moonwm/modmap

xclock &
xterm &
sleep 1

# moonwm &
# wm_pid="$!"
# sleep 1
# moonctl tag 4
# sleep 1
# kill "$wm_pid"
# sleep 2

exec ./target/debug/marswm
