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
    clippy::missing_errors_doc,
    clippy::unnecessary_wraps
)]

use cred_man_lib::encrypted_file;

use std::fs::File;
use std::io::Write;

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 1 && args.len() != 2 {
        println!("Expected arguments: path to file");
        std::process::exit(1);
    }
    let f = || -> Result<(), DecryptError> {
        let plaintext = do_decrypt(&args[0])?;
        if args.len() == 1 {
            println!("{plaintext}");
        } else {
            let mut f = File::create(&args[1])?;
            f.write_all(plaintext.as_bytes())?;
        }
        Ok(())
    };
    if let Err(e) = f() {
        println!("{e}");
        std::process::exit(1);
    }
}

#[derive(Debug)]
enum DecryptError {
    WrongPassword,
    IoError(std::io::Error),
}

impl From<std::io::Error> for DecryptError {
    fn from(e: std::io::Error) -> DecryptError {
        DecryptError::IoError(e)
    }
}

impl std::fmt::Display for DecryptError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            &DecryptError::WrongPassword => write!(f, "Wrong password"),
            DecryptError::IoError(e) => write!(f, "{e}"),
        }
    }
}

fn do_decrypt(path: &str) -> Result<String, DecryptError> {
    let encrypted_data = encrypted_file::parse_file(path)?;
    let password = linenoise::input("Enter password: ").expect("stdio should be successful");
    let maybe_plaintext = encrypted_file::decrypt(&encrypted_data, &password);
    match maybe_plaintext {
        Some(plaintext) => Ok(plaintext),
        None => Err(DecryptError::WrongPassword),
    }
}
