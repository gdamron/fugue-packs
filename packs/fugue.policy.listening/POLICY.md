# Listening

Judge by what is heard, not by how the graph is wired. A connected module or
a control that took its value does not tell you what plays.

## Know what you are reading

The output stage clamps each channel to plus or minus 1, and a mono device
then averages left and right. Previews, renders and spectrum analysis of the
output are read after the clamp, and the spectrum after the mono fold too, so
a clipped peak looks flat and mono cancellation shows, as the speakers will
have it.

## The peak meter reads before the clamp

get_meters reports linear levels where 1.0 is full scale, before the clamp.
A peak above 1.0 is an over that the output flattens: lower the levels. Peak
says how hot, RMS how loud. If audio_clock_ms has not moved since your last
read, the levels are stale. A rising non_finite_samples means a module
produced NaN or infinity: find it before judging anything else.

## A scope shows one module

show_scope shows the signal of the module you name, ahead of the output
stage, so its clipping line is a warning. Confirm with get_meters, because
the sum is what the speakers receive.

## Measure after the change lands

Read the result only once the change has applied. Empty meters mean nothing
is playing and nothing was measured: say so rather than assume.

## Done means heard or measured

Do not call a change done until you have read a meter or scope, or played the
preview, and can say what you observed. Report what you heard, not what you
intended.
