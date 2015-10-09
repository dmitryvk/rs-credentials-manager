extern crate linenoise;
extern crate cred_man_lib;

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
        let plaintext = try!(do_decrypt(&args[0]));
        if args.len() == 1 {
            println!("{}", plaintext);
        } else {
            let mut f = try!(File::create(&args[1]));
            try!(f.write_all(plaintext.as_bytes()));
        }
        Ok(())
    };
    match f() {
        Err(e) => {
            println!("{}", e);
            std::process::exit(1);
        },
        Ok(()) => {
        },
    }
}

#[derive(Debug)]
enum DecryptError {
    WrongPassword,
    IoError(std::io::Error)
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
            &DecryptError::IoError(ref e) => write!(f, "{}", e),
        }
    }
}

fn do_decrypt(path: &str) -> Result<String, DecryptError> {
    let encrypted_data = try!(encrypted_file::parse_file(&path));
    let password = linenoise::input("Enter password: ").unwrap();
    let maybe_plaintext = encrypted_file::decrypt(&encrypted_data, &password);
    match maybe_plaintext {
        Some(plaintext) => Ok(plaintext),
        None => Err(DecryptError::WrongPassword),
    }
}
