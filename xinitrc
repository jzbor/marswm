#test programs
pademelon-tools load-wallpaper
xmodmap - < ~/.config/moonwm/modmap

xclock &
xterm &
sleep 2

exec ./target/debug/marswm
