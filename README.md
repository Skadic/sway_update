# sway_update

An update script to sync eww widgets with sway.

## Install

```
cargo install --path .
```

## Usage

To listen for sway events, run
```
sway_update <events>
```

The events are the same as the ones that can be used in `swaymsg -t subscribe -m '[<events>]'`.
So if you want to listen for `workspace` events and `shutdown` events, you call:

```
sway_update workspace shutdown
```

## Issues

There is an issue where when listening for both `workspace` and `window` events, sometimes `window` events aren't received when changing workspace at the same time.
