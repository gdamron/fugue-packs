# Surfaces

Fyug reaches you by different routes, and only some of them play a piece
well. Work out which one you are on before you choose how to play.

## Recognize your surface

Use what you can observe, and stop at the first match.

1. The local Fyug tools are present (for example load_invention): local.
2. Only the hosted tools are present (check_invention, play_preview): hosted.
3. A shell is present, but no Fyug tools: shell.
4. None of these: plain chat.

A shell is on the user's machine or in a cloud agent session, such as Claude
Code on the web or Codex cloud, which has no speakers. If you cannot tell,
ask.

## Take the route for your surface

- **Local:** play and edit through the local tools, and use views
  (show_view) where the host shows them. Never use the browser preview.
- **Shell on the user's machine:** offer the native install and wait for the
  answer. The install procedure is not part of this policy.
- **Shell in a cloud session:** do not install for playback. Render offline
  with fyug render where fyug is already present, or use the preview artifact
  where the host shows one.
- **Hosted:** check the document with check_invention, fix what it reports,
  then call play_preview. The user taps Play to hear it. get_status returns
  the page to install Fyug locally.
- **Plain chat:** use the preview artifact where the host can run the entry
  skill's script and show an HTML file. Otherwise say what to connect (the
  hosted connector, https://mcp.fyug.fm/mcp) or install.

## Know what the browser preview lacks

The hosted route and the preview artifact have no local engine, so they
offer none of these:

- file, RTMP or YouTube outputs, or hardware output
- live controls, or saving the piece
- module types outside the preview's supported list, or bundled
  developments and samples; the preview refuses them by name

## Say where you are

The browser preview is limited: it has a reduced feature set and may drop
out. Say so plainly, and promise it nothing more. If there is no sound on a
phone, the silent switch may be on.

Tell the user which route you are using. When you are on the preview and a
native route exists (hosted or plain chat), offer it. Never change route
without saying so: if the preview fails or drops out, say that, and offer the
native route instead of moving on silently.
