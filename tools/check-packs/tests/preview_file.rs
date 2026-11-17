//! The entry skill's preview file script, `scripts/write_preview.py`, run
//! with `python3` against a small stand-in for the release's single-file
//! player. The stand-in has the real player's insertion point: an empty
//! `fyug-invention` script element after the `fyug-supported-modules` list,
//! with the page's own script after it. The real player and its list are
//! release files and are never checked in here.

mod policy_support;
mod skill_support;

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use policy_support::{code_spans, flat, pack, repository};
use serde_json::{json, Value};
use skill_support::{section, skill};
use tempfile::TempDir;

const SKILL: &str = "fugue.skill.entry";
const EMPTY: &str = r#"<script type="application/json" id="fyug-invention"></script>"#;

/// A stand-in player, built like the release's: the insertion point, the
/// supported-modules list the page carries, and a page script that names
/// the element too.
fn stand_in_player() -> String {
    let modules = modules_list(&["oscillator", "vca", "dac"]);
    format!(
        r#"<!doctype html>
<!--
  Stand-in player. Put an invention's JSON in the fyug-invention script element.
  <script type="application/json" id="fyug-invention"></script>
-->
<html lang="en">
  <head><meta charset="utf-8" /><title>Stand-in player</title></head>
  <body>
    <header><h1>Fyug — Ünïcödé</h1><span class="badge">Preview</span></header>
    <script type="application/json" id="fyug-supported-modules">
      {modules}
    </script>
    {EMPTY}
    <script>
      const text = document.getElementById("fyug-invention").textContent;
      if (text.length < 1) console.log("no piece");
    </script>
  </body>
</html>"#
    )
}

fn modules_list(types: &[&str]) -> String {
    let modules: Vec<Value> = types.iter().map(|t| json!({ "type": t })).collect();
    json!({
        "schema": "fyug.supported-modules/1",
        "platform": "browser",
        "product_version": "2026.11.1",
        "core_revision": "0".repeat(40),
        "modules": modules,
    })
    .to_string()
}

/// A tone through a VCA, original and trivially public.
fn piece() -> Value {
    json!({
        "version": "1.0.0",
        "title": "A tone",
        "modules": [
            { "id": "osc", "type": "oscillator", "config": { "frequency": 220 } },
            { "id": "amp", "type": "vca", "config": { "level": 0.2 } },
            { "id": "dac", "type": "dac" }
        ],
        "connections": [
            { "from": "osc", "from_port": "audio", "to": "amp", "to_port": "audio" },
            { "from": "amp", "from_port": "audio", "to": "dac", "to_port": "audio" }
        ]
    })
}

fn script() -> PathBuf {
    pack(SKILL).join("scripts/write_preview.py")
}

struct Run {
    dir: TempDir,
}

impl Run {
    fn new(player: Option<&str>, invention: &str) -> Run {
        let dir = TempDir::new().unwrap();
        if let Some(player) = player {
            fs::write(dir.path().join("player.html"), player).unwrap();
        }
        fs::write(dir.path().join("piece.json"), invention).unwrap();
        Run { dir }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    fn write(&self, extra: &[&str]) -> Output {
        let mut command = Command::new("python3");
        command.arg("-I").arg(script()).current_dir(self.dir.path());
        command.args(["--player", "player.html", "--invention", "piece.json"]);
        command.args(["--out", "preview.html"]).args(extra);
        command.output().expect("python3 runs")
    }

    fn output(&self) -> Option<String> {
        fs::read_to_string(self.path("preview.html")).ok()
    }
}

/// The text of the `fyug-invention` element (the last, after the stand-in's
/// commented-out copy), read as a browser does: to
/// the first `</script`, whatever case.
fn inserted(page: &str) -> &str {
    let start = page.rfind(r#"id="fyug-invention">"#).unwrap() + r#"id="fyug-invention">"#.len();
    let end = start + page[start..].to_lowercase().find("</script").unwrap();
    &page[start..end]
}

fn script_tags(page: &str) -> (usize, usize) {
    let lower = page.to_lowercase();
    (
        lower.matches("<script").count(),
        lower.matches("</script").count(),
    )
}

fn assert_refused(run: &Run, output: &Output, names: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "should refuse: {stderr}");
    assert!(
        stderr.contains(names),
        "the message names `{names}`: {stderr}"
    );
    assert!(run.output().is_none(), "no file is written");
}

/// The output is the player with the piece as the element's text, and every
/// other byte unchanged, line endings included.
#[test]
fn the_script_writes_the_player_with_the_piece_inside() {
    for player in [stand_in_player(), stand_in_player().replace('\n', "\r\n")] {
        let text = serde_json::to_string_pretty(&piece()).unwrap();
        let run = Run::new(Some(&player), &text);
        // An absolute output path, as an agent would give.
        let out = run.path("preview.html");
        let output = run.write(&["--out", out.to_str().unwrap()]);
        assert!(output.status.success(), "{output:?}");

        let page = run.output().expect("the preview file is written");
        let (before, after) = player.rsplit_once(EMPTY).unwrap();
        let open = &EMPTY[..EMPTY.len() - "</script>".len()];
        let body = page
            .strip_prefix(before)
            .and_then(|rest| rest.strip_prefix(open))
            .and_then(|rest| rest.strip_suffix(after))
            .and_then(|rest| rest.strip_suffix("</script>"))
            .expect("only the element's text changed");
        assert_eq!(body, inserted(&page));
        assert_eq!(serde_json::from_str::<Value>(body).unwrap(), piece());
    }
}

/// `</script>`, `<!--` and `<script` in the piece's text cannot end the
/// element early: the page keeps its elements and the piece reads back.
#[test]
fn the_piece_cannot_end_its_element_early() {
    let mut hostile = piece();
    hostile["title"] = json!("x</script><script>alert(1)</SCRIPT > <!-- <script");
    hostile["description"] = json!("a < b, </scrip, <\\/script>");
    let run = Run::new(Some(&stand_in_player()), &hostile.to_string());
    let output = run.write(&[]);
    assert!(output.status.success(), "{output:?}");

    let page = run.output().unwrap();
    assert_eq!(script_tags(&page), script_tags(&stand_in_player()));
    let text = inserted(&page);
    assert!(!text.contains('<'), "{text}");
    assert_eq!(serde_json::from_str::<Value>(text).unwrap(), hostile);
    let after = &page[page.rfind(text).unwrap() + text.len()..];
    assert!(after.contains("getElementById"), "the page script follows");
}

/// A module type off the supported list is refused by name, whether the
/// list is the release file or the player's own, and in a development too.
#[test]
fn an_unsupported_module_type_is_refused_by_name() {
    let mut piece = piece();
    piece["modules"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "id": "out", "type": "rtmp_sink" }));
    let run = Run::new(Some(&stand_in_player()), &piece.to_string());
    assert_refused(&run, &run.write(&[]), "rtmp_sink");

    fs::write(run.path("modules.json"), modules_list(&["dac"])).unwrap();
    let output = run.write(&["--modules", "modules.json"]);
    assert_refused(&run, &output, "oscillator, vca, rtmp_sink");
    // A list given with --modules is the one checked, not the player's own.
    let wider = modules_list(&["oscillator", "vca", "dac", "rtmp_sink"]);
    fs::write(run.path("modules.json"), wider).unwrap();
    let output = run.write(&["--modules", "modules.json"]);
    assert!(output.status.success(), "{output:?}");
    assert!(run.output().unwrap().contains("rtmp_sink"));

    let voice = |inner: &str| {
        json!({
            "developments": [{ "name": "voice", "definition": {
                "modules": [{ "id": "g", "type": inner }]
            }}],
            "modules": [{ "id": "v", "type": "voice" }, { "id": "dac", "type": "dac" }]
        })
        .to_string()
    };
    let run = Run::new(Some(&stand_in_player()), &voice("oscillator"));
    assert!(
        run.write(&[]).status.success(),
        "a development's name is a type"
    );
    let run = Run::new(Some(&stand_in_player()), &voice("granular"));
    assert_refused(&run, &run.write(&[]), "granular");
}

/// Bad input is refused, naming the problem, and nothing is written.
#[test]
fn bad_input_is_refused_without_writing() {
    let piece = piece().to_string();
    let run = Run::new(Some(&stand_in_player()), "{\"modules\": [");
    assert_refused(&run, &run.write(&[]), "not valid JSON");

    let run = Run::new(None, &piece);
    assert_refused(
        &run,
        &run.write(&[]),
        "player file player.html does not exist",
    );

    let run = Run::new(Some(&stand_in_player().replace(EMPTY, "")), &piece);
    assert_refused(&run, &run.write(&[]), "no fyug-invention script element");

    // A refusal leaves an earlier preview at the output path as it was.
    fs::write(run.path("preview.html"), "an earlier preview").unwrap();
    assert!(!run.write(&[]).status.success());
    assert_eq!(run.output().unwrap(), "an earlier preview");

    let run = Run::new(Some(&stand_in_player()), "[1, 2]");
    assert_refused(&run, &run.write(&[]), "not a JSON object");

    let run = Run::new(Some(&stand_in_player()), &piece);
    fs::write(run.path("modules.json"), "[]").unwrap();
    assert_refused(&run, &run.write(&["--modules", "modules.json"]), "not a");

    let twice = stand_in_player().replace(EMPTY, &format!("{EMPTY}\n{EMPTY}"));
    let run = Run::new(Some(&twice), &piece);
    assert_refused(&run, &run.write(&[]), "more than one fyug-invention");

    let run = Run::new(Some(&stand_in_player()), &piece);
    let output = run.write(&["--out", "missing/preview.html"]);
    assert!(!output.status.success() && !run.path("missing").exists());
    assert!(String::from_utf8_lossy(&output.stderr).contains("does not exist"));

    // The output never replaces one of the script's own inputs.
    let run = Run::new(Some(&stand_in_player()), &piece);
    for input in ["player.html", "piece.json"] {
        let before = fs::read(run.path(input)).unwrap();
        let output = run.write(&["--out", input]);
        assert!(!output.status.success(), "{output:?}");
        assert_eq!(fs::read(run.path(input)).unwrap(), before, "{input}");
    }
}

/// The script imports only the standard library, none of its network
/// modules, as `python3` itself reports.
#[test]
fn the_script_imports_only_the_standard_library() {
    let check = r#"
import ast, sys
stdlib = getattr(sys, "stdlib_module_names", None)
if stdlib is None:  # before Python 3.10
    import pkgutil, sysconfig
    lib = sysconfig.get_paths()["stdlib"]
    stdlib = set(sys.builtin_module_names) | {m.name for m in pkgutil.iter_modules([lib])}
tree = ast.parse(open(sys.argv[1]).read())
names = set()
for node in ast.walk(tree):
    if isinstance(node, ast.Import):
        names |= {alias.name.split(".")[0] for alias in node.names}
    elif isinstance(node, ast.ImportFrom):
        names.add((node.module or "").split(".")[0])
    elif isinstance(node, ast.Call) and getattr(node.func, "id", "") == "__import__":
        names.add("__import__")
forbidden = {"urllib", "http", "socket", "ssl", "ftplib", "smtplib", "asyncio", "xmlrpc", "webbrowser", "subprocess"}
bad = sorted(n for n in names if n not in stdlib or n in forbidden)
print(" ".join(bad))
sys.exit(1 if bad else 0)
"#;
    let output = Command::new("python3")
        .args(["-I", "-c", check])
        .arg(script())
        .output()
        .expect("python3 runs");
    assert!(output.status.success(), "{output:?}");
}

/// Runs the script isolated from site-packages, with every socket call
/// recorded in a marker file and refused.
const NETWORK_GUARD: &str = r#"
import os, runpy, socket, sys
marker = os.environ["PREVIEW_NETWORK_MARKER"]
def refuse(*args, **kwargs):
    with open(marker, "a") as f:
        f.write("network request\n")
    raise OSError("the network is disabled in this test")
socket.getaddrinfo = refuse
socket.create_connection = refuse
socket.socket.connect = refuse
socket.socket.connect_ex = refuse
socket.socket.sendto = refuse
script = sys.argv[1]
sys.argv = sys.argv[1:]
runpy.run_path(script, run_name="__main__")
"#;

/// Neither a preview nor a refusal makes a network request when run.
#[test]
fn the_script_makes_no_network_request() {
    for player in [Some(stand_in_player()), None] {
        let run = Run::new(player.as_deref(), &piece().to_string());
        let marker = run.path("network.log");
        let output = Command::new("python3")
            .args(["-I", "-S", "-c", NETWORK_GUARD])
            .arg(script())
            .args(["--player", "player.html", "--invention", "piece.json"])
            .args(["--out", "preview.html"])
            .current_dir(run.dir.path())
            .env("PREVIEW_NETWORK_MARKER", &marker)
            .output()
            .expect("python3 runs");
        assert_eq!(output.status.success(), player.is_some(), "{output:?}");
        assert!(!marker.exists(), "a network request: {output:?}");
    }
}

/// The command the skill gives works as written against the stand-in.
#[test]
fn the_skill_runs_the_script_as_the_script_expects() {
    let skill = skill(SKILL);
    let plain = flat(section(&skill.body, "Procedure"))
        .split("**Plain chat:**")
        .nth(1)
        .unwrap()
        .to_string();
    let command = code_spans(&plain)
        .into_iter()
        .find(|span| span.starts_with("python3 scripts/write_preview.py "))
        .expect("the plain chat step runs the script");

    let run = Run::new(Some(&stand_in_player()), &piece().to_string());
    fs::copy(run.path("player.html"), run.path("fyug-player.html")).unwrap();
    fs::write(
        run.path("fyug-player-modules.json"),
        modules_list(&["oscillator", "vca", "dac"]),
    )
    .unwrap();
    let output = Command::new("python3")
        .arg("-I")
        .arg(script())
        .args(command.split_whitespace().skip(2))
        .current_dir(run.dir.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(run.path("piece-preview.html").is_file());
}

/// Neither a player nor a supported-modules list is checked in, whatever
/// its name; a test's stand-in player stays small.
#[test]
fn no_release_player_file_is_checked_in() {
    let root = repository();
    let listed = Command::new("git")
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ])
        .current_dir(&root)
        .output()
        .expect("git runs");
    assert!(listed.status.success());
    let mut problems = Vec::new();
    for name in String::from_utf8(listed.stdout).unwrap().split('\0') {
        let Ok(bytes) = fs::read(root.join(name)) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        let in_tests = name.starts_with("tools/check-packs/tests/");
        let player = text.contains("fyug-invention") && text.to_lowercase().contains("<html");
        if player && !(in_tests && bytes.len() <= 64 * 1024) {
            problems.push(format!("{name} holds a player"));
        }
        let schema = serde_json::from_slice::<Value>(&bytes)
            .ok()
            .and_then(|v| v.get("schema").and_then(Value::as_str).map(str::to_string));
        if schema.as_deref() == Some("fyug.supported-modules/1") {
            problems.push(format!("{name} is a supported-modules list"));
        }
    }
    assert!(problems.is_empty(), "{problems:#?}");
}
