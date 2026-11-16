# Authoring Fugue Inventions

How an agent creates and changes a Fugue invention, and plays it, through the
`fugue` tools. Each tool's description adds its arguments and limits. Rules
for reaching Fugue over MCP are under "MCP transport" at the end.

## Author documents, conduct with controls

Fugue is a music creation tool. Plan structural creation and edits as a
document; use live controls to conduct the music once it plays.

## Discover only what you need

describe_module_types({}) returns a terse index of module types (builtins
only while idle; a loaded invention adds its developments).
describe_module({"type":"mixer","config":{"levels":[0.25,0.25,0.25],"master":0.8}})
shows the ports and starting controls that config gives; pass module_id
instead to read a running module. Results are not cached: ask again after a
load, reload, swap or removal.

## Write one complete invention

For a new piece, write one complete invention document (version, modules with
supported configs, connections) and load it with load_invention. A document's
modules use id, type and config; add_module takes id, module_type and config.
Put intended starting values in config instead of setting each in a separate
call, and check them with describe_module.

## Config keys are control names

A config key that sets a control has the control's name: an oscillator uses
waveform: "sine" and frequency: 220. A mixer's config uses
levels: [0.25,0.25,0.25] and master: 0.8, while its live keys are level.0,
level.1 and so on.

## Import reusable instruments by ref

Reusable instruments are developments from installed packages and the
inventions workspace. list_developments returns each with an exact ref;
describe_development reports its inputs, outputs and controls without loading
or playing it. Copy the ref unchanged into the document,
"developments":[{"name":"my_pad","ref":RETURNED_REF}], choosing a name no
builtin type or other development uses, then instantiate modules with
"type":"my_pad". Listing registers nothing, and neither inspection nor import
installs a missing package. Saving keeps the reference rather than flattening
the instrument. Ask again after stale_cursor or stale_reference.

## Start examples by ref

list_examples finds complete playable examples, each with an exact ref;
describe_example reports its dependencies and assets without playing it. Pass
the ref unchanged to load_invention to start one.

## Edit the whole document against its revision

For a structural edit, retrieve the current document with get_invention, make
the intended change to the complete document, and send it with
reload_invention. Never reconstruct a document from list_modules or
list_connections. Pass the snapshot's source_path and revision as
expected_revision: a stale revision is refused, so retrieve again and rebase
the edit. A reload without expected_revision is unconditional. A snapshot is
a point in time, not a synchronized copy. reload_invention has no ref:
references name catalog content, not documents to edit.

## Reload keeps what did not change

A reload diffs the documents: unchanged modules keep their phase and state,
supported scalar config changes become control writes, and other config,
type or development changes rebuild the modules they touch. A reload that
cannot be applied is refused with nothing changed. A clean load, which
restarts state, happens only when nothing is running.

## Targeted edits to large inventions

When resending the whole document costs more than the change (a new voice
wired into a mixer, a filter inserted into a chain, several starting values
retuned together), use apply_edits: its edits apply in order, all or nothing,
in one revision. Chain each call on the previous result's revision.
reload_invention stays the default when you hold the document.

## Read before retrying

When the daemon connection drops mid-call, an edit is resent only when it
cannot apply twice, so an edit that returns normally applied exactly once. A
reload, edit batch or example load marked recovered applied before its reply
was lost, and its details went with the reply: inspect with get_invention. An
"Outcome unknown" error, including after a daemon restart, means the change
may or may not have applied: read get_invention or list_controls before
retrying. Live gestures are never resent.

## Conduct with set_controls

Read live keys and ranges first (list_controls). Use set_controls for a
gesture that moves several controls and set_control for one. A batch runs in
order without rollback: earlier writes stay if a later one fails. Each write's
ack is applied (with its value) or pending (with a request that
control_changed names when it applies). intent "perform" plays a value
without saving it into the document.

## Land changes on the beat

Pass when, such as {"kind":"next_bar"}, to land a control write, swap_module,
apply_edits or reload_invention in musical time. Nothing changes until it
lands; follow the request with get_request or poll_events, and give it
ttl_ms to let it expire instead of landing late. One timed edit waits at a
time. Another structural edit cancels it, and so does an authored write (not
intent "perform") to a module it rebuilds or removes. An authored write to a
control it sets as it lands is kept: the edit lands without its own write to
that control, or is cancelled if it cannot. Perform writes, and authored
writes to other controls of modules it keeps, never cancel it, except a
schedule written to a scheduler it re-resolves or whose targets it rebuilds
or removes. Between a timed write and a timed edit, whichever lands later
wins. Once an edit is adopted, read get_invention before the next document
edit.

## Surgical tools for surgical changes

Keep add_module (with full supported config), remove_module, swap_module,
connect and disconnect for small changes. create_invention makes an empty
canvas with a DAC. stop_invention unloads the piece; pause_invention silences
it with position and values held, and edits still apply, until
resume_invention. save_invention writes the document, references intact, on
the daemon's filesystem.

## Listen to the result

Check what plays with get_meters (peak and RMS levels) or show_scope (one
module's waveform).

## The person plays too

A value the person changed after you set it (list_changed_controls lists
them) is their choice: do not set yours back. When they say "this" or "here",
read get_focus instead of asking.

## Show what changed

After an edit, show_view shows the person what it changed, such as a piano
roll or a slider for a value you guessed. Controls you show stay pinned on
every surface; list_pins and unpin_controls manage them. describe_piece_view
returns a page to publish beside the chat that keeps the piece in view.

## Packages

list_packages lists built-in and installed packages without a registry.
install_package installs one, with its dependencies, from a local directory,
GitHub or a registry. Installed versions never change: an identical repeat
reports already_installed, and other bytes under the same version fail with
integrity_mismatch.

## MCP transport

These rules hold only for agents that reach Fugue through the MCP server.

- load_invention takes exactly one of path, json or ref, and reload_invention
  one of path, json or snapshot_path. json is a serialized JSON string, not
  an object: use it for a self-contained document when the agent host does
  not share the server's filesystem.
- path is read by the MCP server, and the daemon must reach it and what it
  references. Relative development and $asset references need a file-backed
  document. A file local to the agent is not uploaded.
- get_invention takes delivery="inline" (64 KiB) or delivery="file" with a
  new absolute path on the MCP host (16 MiB); use file delivery only when
  your file tools reach that host. Edit the file's document, then call
  reload_invention(snapshot_path=path), which resolves its original
  references on the daemon wherever the snapshot is stored. Existing files
  are never overwritten, and an oversized retrieval fails rather than
  truncating.
- save_invention writes on the daemon's filesystem and does not download a
  file to the agent host.
- A call the host cancelled or timed out is like "Outcome unknown": read
  before retrying.
- Examples and developments work whether or not the host surfaces MCP
  resources.
