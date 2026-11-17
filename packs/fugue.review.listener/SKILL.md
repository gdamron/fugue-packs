---
name: fugue-review-listener
description: Listen to the Fugue piece that is loaded and give musical feedback with one suggested change. Use when the user asks how the piece sounds, what to change, or to review, critique or check the mix ("does the bass mask the kick?", "what would make this better?"). Not for writing a new piece or playing one, and not where the local Fyug tools are missing.
---

# Review listener

A critique by ear. Hear the piece as it stands, measure what plays, say what
you heard against what the music is meant to do, and propose one change.

## Triggers

- A request to listen to, review or critique the piece, or to check the mix.
- A question about how it sounds or what to change next.

## Required policies

- [Surfaces](policies/fugue.policy.surfaces.md): whether this surface has
  the audio preview.
- [Listening](policies/fugue.policy.listening.md): what to measure and how
  to read it.
- [Musical intent](policies/fugue.policy.musical-intent.md): the bar the
  feedback is judged against.
- [Clean audio](policies/fugue.policy.clean-audio.md): for diagnosing
  clipping, clicks and masking.

## Inputs

- The loaded piece, and what the user says it is meant to do.
- The user's question, if it names a voice or a problem.

## Outputs

- What you heard and measured, in musical terms, and where it parts from the
  piece's intent.
- One proposed change, with how you will check it. Nothing is changed until
  the user agrees.

## Procedure

1. Recognize the surface as the surfaces policy says. The audio preview is
   the local server's, so off the local route, stop.
2. While the piece plays, read `get_meters` for the whole output and
   `show_scope` on the voice in question, as the listening policy says.
3. Get the bounded audio preview. It renders only a paused piece, from its
   start, as its document stands, without live control changes. Ask before
   pausing music the user is playing, then call `pause_invention`, then
   `show_view` with view "audio" (10 seconds unless you ask for more, at most
   30), then `resume_invention`. Listen to the WAV the reply names where your
   host can play it to you.
4. Say what you heard against the musical-intent policy, using the
   clean-audio policy to name a fault: what the music sets out to do, what it
   does, and where the two part (the bass hides the kick, the hats never
   vary). Say which parts you heard and which you only measured.
5. Propose one change, the one that closes the widest gap, and say what you
   will listen for after it.

## Stop conditions

Stop, say why, and say what the user can do next.

- **No audio preview on this surface.** The surfaces policy decides: only
  the local route has one. On the hosted route the user can hear
  `play_preview` and tell you what they heard; elsewhere, say what to
  install or connect.
- **Nothing is loaded.** `show_view` refuses; ask which piece to hear.
- **The piece cannot be rendered offline.** `show_view` says why (it records
  or streams, say). Critique from the meters and scope alone, and say so.
- **The user does not want the music paused.** Critique from the meters and
  scope alone, and say the preview was not heard.
- **Your host cannot play the WAV to you.** Do not claim to have heard it:
  report what you measured, and ask the user to listen and say what they
  hear.
