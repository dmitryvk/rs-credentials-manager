#![warn(
    clippy::cargo,
    clippy::pedantic,
    // Extra restrictions:
    clippy::create_dir,
    clippy::dbg_macro,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::todo,
    clippy::undocumented_unsafe_blocks,
    clippy::unimplemented,
    clippy::unwrap_used,
)]
#![allow(
    clippy::cargo_common_metadata,
    clippy::cast_precision_loss,
    clippy::if_not_else,
    clippy::multiple_crate_versions,
    clippy::implicit_hasher,
    clippy::new_without_default,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc
)]

use std::path::PathBuf;

use app_state::AppState;
use clap::Parser;
use cred_man_lib::DbLocation;

mod app_state;
mod ui;

fn main() {
    let args = CliArgs::parse();
    let mut app_state = AppState::new(args.db_location());
    ui::ui_main(&mut app_state);
}

#[derive(Debug, Parser)]
struct CliArgs {
    /// Path to the database
    db_path: Option<PathBuf>,
}

impl CliArgs {
    fn db_location(&self) -> DbLocation {
        match &self.db_path {
            Some(db_path) => DbLocation::SpecifiedDirectory(db_path.clone()),
            None => DbLocation::DotLocal,
        }
    }
}
