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

use std::cmp;
use std::collections::BTreeMap;

static CORRECT_PASSWORD: &'static str = "123";

fn authenticate() -> bool {
  let password = linenoise::input("Enter password: ");
  match password {
    None => { false }
    Some(p) => {
      if p == CORRECT_PASSWORD {
        println!("OK");
        true
      } else {
        println!("Wrong password");
        false
      }
    }
  }
}

fn parse_cmd_line(cmd_line: &str) -> (&str, &str) {
  let idx = cmd_line.find(' ').unwrap_or(cmd_line.len());
  let cmd = unsafe { cmd_line.slice_unchecked(0, idx) };
  let rest = unsafe { cmd_line.slice_unchecked(cmp::min(cmd_line.len(), idx + 1), cmd_line.len()) };
  (cmd, rest)
}

struct DbRecord {
  key: String,
  value: String,
}

struct Db {
  data: BTreeMap<String, DbRecord>,
}

fn get_command_handler(cmd: &str) -> Option<fn(&mut Db, &str, &str) -> bool> {
  match cmd {
    "help" => Some(help_cmd),
    "quit" => Some(quit_cmd),
    "add" => Some(add_cmd),
    "get" => Some(get_cmd),
    "list" => Some(list_cmd),
    _ => None
  }
}

fn help_cmd(_: &mut Db, _: &str, _: &str) -> bool {
  println!("Commands:");
  println!(" help");
  println!(" quit");
  println!(" add");
  println!(" list");
  true
}

fn quit_cmd(_: &mut Db, _: &str, _: &str) -> bool {
  false
}

fn add_cmd(db: &mut Db, _: &str, _: &str) -> bool {
  if let Some(key) = linenoise::input("new key: ") {
    if key.len() > 0 {
      if let Some(val) = linenoise::input(&format!("value for {:}: ", key)) {
        if val.len() > 0 {
          println!("inserting '{:}' => '{:}'", key, val);
          let rec = DbRecord { key: key.clone(), value: val };
          db.data.insert(key, rec);
          println!("now storing {:} keys", db.data.len());
        }
      }
    }
  }
  true
}

fn get_cmd(db: &mut Db, _: &str, _: &str) -> bool {
  if let Some(key) = linenoise::input("find key: ") {
    if key.len() > 0 {
      match db.data.get(&key) {
        None => {
          println!("there is no match for {:}", &key);
        }
        Some(val) => {
          println!("the value is:\n{:}", val.value);
        }
      }
    }
  }
  true
}

fn list_cmd(db: &mut Db, _: &str, _: &str) -> bool {
  for key in db.data.keys() {
    println!("{:}", key);
  }
  true
}

fn execute_cmd(db: &mut Db, cmd_line: &str) -> bool {
  let (cmd, args) = parse_cmd_line(cmd_line);
  match get_command_handler(cmd) {
    Some(handler) => { handler(db, cmd, args) }
    None => {
      println!("Unknown command {:}; try `help'", cmd);
      true
    }
  }
}

fn main() {
  if !authenticate() {
    return;
  }
  let mut db = Db { data: BTreeMap::new() };
  while let Some(cmd) = linenoise::input("> ") {
    if cmd.len() > 0 {
      if !execute_cmd(&mut db, &cmd) {
        return;
      }
    }
  }
}
