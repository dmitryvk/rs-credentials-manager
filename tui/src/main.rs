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
