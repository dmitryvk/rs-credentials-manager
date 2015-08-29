//! # Intro
//!
//! This is a credentials manager program.
//! It's an interactive command-line application.
//! 
//! # Usage
//!
//! Data is stored in ~/.local/share/credentials-manager (it's stored in clear for now).
//! 

extern crate linenoise;

static CORRECT_PASSWORD: &'static str = "123";

fn main() {
  let password = linenoise::input("Enter password: ");
  match password {
    None => { return; }
    Some(p) => {
      if p == CORRECT_PASSWORD {
        println!("OK");
      } else {
        println!("Wrong password");
      }
    }
  }
}
