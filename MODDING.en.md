# Experimental mods

This branch is a laboratory. It does not touch the `traduccion-pt` branch.

## Mods

Each mod lives in `game/extraido/mods/<id>/` and needs a `mod.ini`:

```ini
name=My mod
version=0.1
enabled=1
author=Your name
description=What the mod changes and what it is for
```

Since 0.4.0, mods are **disabled by default**. The global switch is stored in
`mods/enabled.flag`; if `mods/disabled.flag` exists, the latter wins and the game starts without mods.

- `data/` is overlaid on top of `game/extraido/data/`. Before overwriting, a copy is saved to `mods/.backup/<id>/`.
- If you set `enabled=0`, the engine restores the backed-up files.
- On Android, the visual section is in **Settings → Mods**: the **Enable mods** switch starts off. Turn it on to load mods; the choice is kept. Changes apply when the game restarts.
- On PC, create `game/extraido/mods/enabled.flag` to enable mods and edit `enabled=1`/`enabled=0` in each `mod.ini`. To turn them all off, create `mods/disabled.flag`.

## In-game menu

- If at least one mod is active, the engine creates `mods/menu.txt` and `mods/active.flag`.
- With active mods, a **Mods** button appears in the top-right corner of the title screen; pressing it opens a full-size screen of its own, like the Options one, with the list of mods.
- In the in-game menu you can click **CRT** or **Game Boy** to turn them on/off live. The other mods are still enabled from Settings → Mods on Android or by editing `mod.ini` on PC.
- Only **Back** closes the list; the rest of the title screen stays locked while it is open.


## CRT mod

A mod can enable the CRT post-process with `effect=crt` in its `mod.ini`. The values are
optional; any that are missing use the default value. The effect applies to the whole game,
including title, menus and map.

```ini
name=CRT
enabled=1
effect=crt

curvature=0.055   # screen curvature
scanline=0.40     # scanline intensity
scanline_count=360 # number of lines over the height
mask=0.60         # RGB phosphor mask
aberration=0.0035 # color separation at the edges
vignette=0.35     # darkening of the corners
blur=0.0          # pixel smoothing
noise=0.014       # grain
brightness=1.18
saturation=1.08
glow=0.15         # brightness of the light areas
corner=0.07       # corner radius
```

`0` disables the corresponding part; `1` is the maximum. Changes apply on restart.


## Game Boy mod

Enables the four-tone filter with `effect=gb`:

```ini
name=Game Boy
enabled=1
effect=gb

strength=1.0   # blend with the original image
dither=0.25    # ordered dithering, mostly in mid-tones
contrast=0.90
brightness=1.02
```

The filter shares the same pass as the CRT, so both can be active at the same time.

It can optionally apply a lo-fi audio effect to the whole mix:

```ini
audio=gb
audio_bits=6
audio_rate=8000
audio_mono=1
```

## Testing

```bash
cd /home/klisee/pokemon-reloaded-experimental
cargo run -p gm8emulator --release -- "game/extraido/Proyecto Reloaded The Last Beta 1.9.2.2 Full.exe" -3
```

## In-game manager (fork)

- Mods are prepared **before** the game's scripts are compiled: you no longer need to open the game twice.
- The **Mods** button on the title screen always appears as long as the global switch is active and at least one mod exists (even if all of them are OFF).
- Any mod (including those imported from the .zip) is enabled/disabled by tapping it. Those in `data/` apply instantly; those that bring `scripts/` carry a `*` and ask you to restart the game.
- Disabling a mod also deletes the files it had created in `data/`.
- More than 7 mods: `<` `>` buttons to change page.
