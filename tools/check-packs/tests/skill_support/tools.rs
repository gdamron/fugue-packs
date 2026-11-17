//! The product tool and command names that first-party skills name.
//!
//! check-packs cannot see the product (fyug-product is private), so the names
//! a skill uses are listed here by hand, copied from fyug-product main at
//! 12da546e (2026-11-16). Add a name only after finding it there, and say
//! where it comes from.

/// Tools of the local server, `crates/mcp/src/tools/<tool>.rs`.
pub const LOCAL_TOOLS: &[&str] = &["load_invention", "reload_invention", "show_view"];

/// Tools of the hosted server, the `#[tool]` functions of
/// `crates/hosted/src/server.rs`.
pub const HOSTED_TOOLS: &[&str] = &["check_invention", "play_preview"];

/// `fyug` subcommands, from `Command` in `crates/cli/src/cli.rs`. A span that
/// starts with one of these is a command line.
pub const COMMANDS: &[&str] = &["fyug render"];
