# Clean Audio

A click, pop or glitch is the first thing a listener hears, and it can end
the listening. Keep what Fugue plays as clean as you meant it.

## Leave headroom

Start every voice low. The authoring example gives three mixer channels 0.25
each and a master of 0.8: begin there, and raise a level only after hearing
it. Levels add, so a new voice comes with room made for it by lowering the
others or the master. Delay feedback, reverb and resonance add level of their
own. While building, keep peaks near 0.5; never leave a sustained peak at or
above 1.0.

## Shape every note with an envelope

Do not gate an oscillator straight into the mixer: a note that starts or
stops mid-wave clicks. Pass it through an adsr and a vca with an attack and a
release of a few milliseconds or more.

## Retrigger cleanly

Every onset needs the gate to fall before it rises again. Do not start a new
note on a loud tail of the last one: shorten the note or the release until
the old note has mostly died away, or give the repeat its own voice.

## Ramp, never step

A control may jump to the value it is given. A mixer's level, pan and master
and a vca's level glide over 10 ms, so use them to fade; assume a step for
any other control. Move a sounding control such as pitch, filter
cutoff or a level ahead of the mixer in several small writes, or let an lfo
or adsr move it. Make a large jump where the voice is silent, such as on a
rest.

## Schedule changes

Land a change that must be heard at a musical moment with when, such as
{"kind":"next_bar"}, rather than mid-note. Prefer a control write to a config
change that rebuilds a sounding module, since a rebuild restarts its state.
