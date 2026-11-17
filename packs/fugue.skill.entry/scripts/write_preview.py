#!/usr/bin/env python3
"""Write a Fyug preview file: the release's single-file player with a piece in it.

    python3 write_preview.py --player fyug-player.html \\
        --invention piece.json --out piece-preview.html \\
        [--modules fyug-player-modules.json]

The output is a copy of the player whose `fyug-invention` script element holds
the invention's JSON, with every "<" written as \\u003c so the text cannot end
the element early. Every other byte of the player is unchanged.

The piece is checked against the release's supported-modules list (`--modules`,
or the list the player carries when it is omitted). A module type the list
lacks is refused by name. On any refusal the script exits non-zero and writes
nothing. It uses only the Python standard library and makes no network request.
"""

import argparse
import json
import os
import re
import sys
import tempfile
from html.parser import HTMLParser

INVENTION_ID = "fyug-invention"
MODULES_ID = "fyug-supported-modules"
MODULES_SCHEMA = "fyug.supported-modules/1"
SCRIPT_END = re.compile("</script", re.IGNORECASE)


class Refusal(Exception):
    """A reason not to write the preview, shown to the user as is."""


def main(argv=None):
    parser = argparse.ArgumentParser(
        prog="write_preview.py",
        description="Put an invention into the Fyug single-file player.",
    )
    parser.add_argument("--player", required=True, help="the release's fyug-player.html")
    parser.add_argument("--invention", required=True, help="the invention JSON file")
    parser.add_argument("--out", required=True, help="where to write the preview file")
    parser.add_argument(
        "--modules",
        help="the release's fyug-player-modules.json (default: the player's own list)",
    )
    args = parser.parse_args(argv)
    try:
        write_preview(args.player, args.invention, args.out, args.modules)
    except Refusal as refusal:
        print(f"write_preview: {refusal}", file=sys.stderr)
        return 1
    print(args.out)
    return 0


def write_preview(player_path, invention_path, out_path, modules_path=None):
    """Check everything, then write the preview file in one step."""
    player = read_text(player_path, "player file")
    elements = script_elements(player)
    invention_spans = [span for span, attrs in elements if attrs.get("id") == INVENTION_ID]
    if not invention_spans:
        raise Refusal(f"the player {player_path} has no {INVENTION_ID} script element")
    if len(invention_spans) > 1:
        raise Refusal(f"the player {player_path} has more than one {INVENTION_ID} element")

    text = read_text(invention_path, "invention file").lstrip("\ufeff")
    invention = parse_json(text, f"the invention {invention_path}")
    if not isinstance(invention, dict):
        raise Refusal(f"the invention {invention_path} is not a JSON object")

    if modules_path is None:
        supported = embedded_modules(player, elements, player_path)
    else:
        supported = read_modules(read_text(modules_path, "supported-modules list"), modules_path)
    missing = unsupported_module_types(invention, supported)
    if missing:
        names = ", ".join(missing)
        raise Refusal(f"the browser preview does not support these module types: {names}")

    for source in (player_path, invention_path, modules_path):
        if source is not None and os.path.exists(out_path) and os.path.samefile(out_path, source):
            raise Refusal(f"the output path is {source} itself; choose another path")
    out_dir = os.path.dirname(os.path.abspath(out_path))
    if not os.path.isdir(out_dir):
        raise Refusal(f"the output folder {out_dir} does not exist")

    # "<" appears in JSON text only inside strings, where \u003c means the
    # same character, so this keeps the piece and cannot close the element.
    safe = text.strip().replace("<", "\\u003c")
    start, end = invention_spans[0]
    preview = player[:start] + safe + player[end:]

    # Write beside the output and move it into place, so a failed write
    # leaves no partial file; give it the mode a plain new file would get.
    handle, temporary = tempfile.mkstemp(dir=out_dir, suffix=".tmp")
    try:
        umask = os.umask(0)
        os.umask(umask)
        os.chmod(temporary, 0o666 & ~umask)
        with os.fdopen(handle, "w", encoding="utf-8", newline="") as file:
            file.write(preview)
        os.replace(temporary, out_path)
    except BaseException as error:
        if os.path.exists(temporary):
            os.remove(temporary)
        if isinstance(error, OSError):
            raise Refusal(f"cannot write {out_path}: {error.strerror}") from error
        raise


def read_text(path, what):
    try:
        with open(path, "rb") as file:
            data = file.read()
    except FileNotFoundError:
        raise Refusal(f"the {what} {path} does not exist") from None
    except OSError as error:
        raise Refusal(f"cannot read the {what} {path}: {error.strerror}") from None
    try:
        return data.decode("utf-8")
    except UnicodeDecodeError:
        raise Refusal(f"the {what} {path} is not UTF-8 text") from None


def parse_json(text, what):
    def refuse_constant(name):
        raise ValueError(f"{name} is not JSON")

    try:
        return json.loads(text, parse_constant=refuse_constant)
    except ValueError as error:
        raise Refusal(f"{what} is not valid JSON: {error}") from None


class ScriptFinder(HTMLParser):
    """Finds each script element's attributes and the span of its text."""

    def __init__(self, text):
        super().__init__(convert_charrefs=False)
        self.text = text
        self.line_starts = [0] + [found.end() for found in re.finditer("\n", text)]
        self.scripts = []

    def handle_starttag(self, tag, attrs):
        if tag != "script":
            return
        line, column = self.getpos()
        start = self.line_starts[line - 1] + column + len(self.get_starttag_text())
        # A script's text runs to the first "</script", whatever it holds.
        found = SCRIPT_END.search(self.text, start)
        end = found.start() if found else len(self.text)
        # A browser keeps the first of a repeated attribute.
        first = {}
        for name, value in attrs:
            first.setdefault(name, value)
        self.scripts.append(((start, end), first))


def script_elements(player):
    finder = ScriptFinder(player)
    finder.feed(player)
    finder.close()
    return finder.scripts


def embedded_modules(player, elements, player_path):
    for (start, end), attrs in elements:
        if attrs.get("id") == MODULES_ID:
            return read_modules(player[start:end], f"the list in {player_path}")
    raise Refusal(
        f"the player {player_path} carries no supported-modules list; pass --modules"
    )


def read_modules(text, where):
    listing = parse_json(text, f"the supported-modules list {where}")
    if not (
        isinstance(listing, dict)
        and listing.get("schema") == MODULES_SCHEMA
        and isinstance(listing.get("modules"), list)
    ):
        raise Refusal(f"{where} is not a {MODULES_SCHEMA} supported-modules list")
    return {
        entry["type"]
        for entry in listing["modules"]
        if isinstance(entry, dict) and isinstance(entry.get("type"), str)
    }


def unsupported_module_types(invention, supported):
    """The module types the piece builds that `supported` lacks, each once.

    Follows the engine's development rules, as the player does: a type named
    by an inline development counts as supported, and a development's own
    modules are checked when a module builds it, with the names declared
    before it in scope. A development loaded from a path is left to the
    engine.
    """
    missing = []
    built = set()

    def collect(document, enclosing):
        scope = dict(enclosing)
        for development in as_list(document.get("developments")):
            if not isinstance(development, dict):
                continue
            name = development.get("name")
            if not isinstance(name, str) or name in scope:
                continue
            definition = development.get("definition")
            scope[name] = (definition if isinstance(definition, dict) else None, dict(scope))
        for module in as_list(document.get("modules")):
            if not isinstance(module, dict) or not isinstance(module.get("type"), str):
                continue
            kind = module["type"]
            if kind in scope:
                definition, declared_scope = scope[kind]
                if definition is not None and id(definition) not in built:
                    built.add(id(definition))
                    collect(definition, declared_scope)
            elif kind not in supported and kind not in missing:
                missing.append(kind)

    collect(invention, {})
    return missing


def as_list(value):
    return value if isinstance(value, list) else []


if __name__ == "__main__":
    sys.exit(main())
