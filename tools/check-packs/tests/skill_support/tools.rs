//! The product tool names, commands and links that first-party skills name.
//!
//! check-packs cannot see the product (fyug-product is private), so only the
//! names skills use are listed here, by hand, copied from fyug-product main
//! at 12da546e (2026-11-16). These are not the full listings. Add a name only
//! after finding it there, and say where it comes from.

/// Local server tools, each a `crates/mcp/src/tools/<tool>.rs`.
pub const LOCAL_TOOLS: &[&str] = &["load_invention", "reload_invention", "show_view"];

/// Hosted server tools, `#[tool]` functions of `crates/hosted/src/server.rs`.
pub const HOSTED_TOOLS: &[&str] = &["check_invention", "play_preview", "get_status"];

/// `fyug` subcommands and the flags skills pass them, from `Command` and its
/// argument structs in `crates/cli/src/cli.rs`.
pub const COMMANDS: &[(&str, &[&str])] = &[("fyug render", &["--out", "--seconds", "--to-end"])];

/// Where to install Fyug: `INSTALL_URL` in `crates/hosted/src/server.rs`,
/// which `get_status` returns.
pub const INSTALL_URL: &str = "https://github.com/Ilusiv-media/fyug#install";

/// Local server tools for inspecting, installing and finding developments,
/// each a `crates/mcp/src/tools/<tool>.rs`. A skill test passes these to
/// `assert_names_exist` as its others.
pub const DEVELOPMENT_TOOLS: &[&str] = &[
    "describe_module",
    "get_meters",
    "install_package",
    "list_developments",
    "describe_development",
];

/// Whether `span` is a command line of `COMMANDS` passing only its listed
/// flags, or one of those flags alone.
pub fn is_command(span: &str) -> bool {
    COMMANDS.iter().any(|(command, flags)| {
        let flags_known = |words: &str| {
            words
                .split_whitespace()
                .filter(|word| word.starts_with("--"))
                .all(|flag| flags.contains(&flag))
        };
        match span.strip_prefix(command) {
            Some(rest) => (rest.is_empty() || rest.starts_with(' ')) && flags_known(rest),
            None => flags.contains(&span),
        }
    })
}
