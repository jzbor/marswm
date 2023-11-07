# Examples
This chapter contains a couple of example scripts and configurations.
All examples are available on [Github](https://github.com/jzbor/marswm/tree/master/examples).

## Installing Configurations
Most config files go into `~/.config/<program>/<program>.<suffix>`.
For example marswm's main config goes into `~/.config/marswm/marswm.yaml`.

## Installing Scripts
To install a script you will first need to install all of its dependencies.

Then place the script into a directory that is listed in your `$PATH`.
It is suggested to use `~/.local/bin/` to store all of your personal scripts.
You can check if that is in your path with `echo $PATH` and add it to your `~/.profile` otherwise.

You will also have to make the script executable:
```sh
chmod +x <script>
```
