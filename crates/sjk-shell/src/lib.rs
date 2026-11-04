//! Portable client-shell primitives.
//!
//! This crate contains no renderer, window-system, filesystem-location, or
//! legacy-network policy. Applications register their cvars and commands,
//! select an explicit persistence path, and present the console however they
//! choose.

#![warn(missing_docs)]

mod alias;
mod bind;
mod command;
mod command_buffer;
mod config;
mod config_saver;
mod config_watch;
pub mod console_socket;
mod cvar;
pub mod key_names;
pub mod local_time;
mod revision;
pub mod secret;

mod shell;
mod shell_error;

pub use alias::{
    ALIAS_FILE_NAME, AliasError, AliasTable, MAX_ALIAS_COMMAND_BYTES, MAX_ALIAS_NAME_BYTES,
    MAX_ALIASES,
};
pub use bind::{BindError, BindState, BindTable, MAX_KEY_COMBOS};
pub use command::{CommandError, CommandRegistry, split_commands, tokenize};
pub use command_buffer::{
    CommandBuffer, CommandBufferError, CommandFileResolver, MAX_COMMAND_BUFFER_BYTES,
    MAX_EXEC_DEPTH, NoCommandFiles,
};
pub use config::{ConfigError, decode_config_text, load_config, save_config};
pub use config_saver::CONFIG_SAVE_DELAY;
pub use cvar::{Cvar, CvarChange, CvarDefinition, CvarError, CvarFlags, CvarRegistry, CvarValue};
pub use shell::{
    ArgumentSource, CommandSource, CompletionKey, ConsoleLine, ConsoleLineKind, MAP_EXTENSION,
    Shell,
};
pub use shell_error::ShellError;

/// Counts allocator calls inside `assert_no_alloc` blocks (`cvar.rs`'s frame reads).
#[cfg(all(test, debug_assertions))]
#[global_allocator]
static ALLOCATOR: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;
