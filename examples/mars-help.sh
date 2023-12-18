#!/bin/sh

{
	if [ -f ~/.config/marswm/keybindings_ext.yaml ]; then
		echo "### CUSTOM KEY BINDINGS ###"
		cat ~/.config/marswm/keybindings_ext.yaml
		echo
	fi

	echo "### DEFAULT KEY BINDINGS ###"
	if [ -f ~/.config/marswm/keybindings.yaml ]; then
		cat ~/.config/marswm/keybindings.yaml
	else
		marswm --print-default-keys
	fi
} | bat -l yaml --paging always

