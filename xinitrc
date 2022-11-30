#test programs
pademelon-tools load-wallpaper
xmodmap - < ~/.config/moonwm/modmap

xclock &
xterm &
sleep 1

exec ./target/debug/marswm
