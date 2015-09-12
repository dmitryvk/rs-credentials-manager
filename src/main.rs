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
extern crate rustc_serialize;

use std::cmp;
use std::collections::BTreeMap;
use std::io;
use std::io::Write;
use rustc_serialize::json;
use std::io::Read;
use std::fs;
use std::path::PathBuf;

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
    let rest = unsafe { cmd_line.slice_unchecked(cmp::min(cmd_line.len(), idx + 1), cmd_line.len()) }.trim();
    //println!("cmd = {:?} rest = {:?}", cmd, rest);
    (cmd, rest)
}

struct DbRecord {
    key: String,
    value: Vec<(String, String)>,
}

struct Db {
    data: BTreeMap<String, DbRecord>,
}

#[derive(RustcDecodable, RustcEncodable, Debug)]
struct DbDTO {
    records: Vec<DbRecordDTO>
}

#[derive(RustcDecodable, RustcEncodable, Debug)]
struct DbRecordDTO {
    key: String,
    value: Vec<(String, String)>,
}

fn get_command_handler(cmd: &str) -> Option<fn(&mut Db, &str, &str) -> bool> {
    match cmd {
        "help" => Some(help_cmd),
        "quit" => Some(quit_cmd),
        "add" => Some(add_cmd),
        "get" => Some(get_cmd),
        "list" => Some(list_cmd),
        "find" => Some(find_cmd),
        "del" => Some(del_cmd),
        _ => None
    }
}

fn help_cmd(_: &mut Db, _: &str, _: &str) -> bool {
    println!("Commands:");
    println!(" help");
    println!(" quit");
    println!(" add");
    println!(" get");
    println!(" list");
    println!(" find");
    println!(" del");
    true
}

fn quit_cmd(_: &mut Db, _: &str, _: &str) -> bool {
    false
}


fn del_cmd(db: &mut Db, _: &str, rest_line: &str) -> bool {
    let arg = match rest_line {
        x if x != "" => Some(x.to_string()),
        _ => linenoise::input("Key: ")
    };
    if let Some(key) = arg {
        if key.len() > 0 {
            match db.data.remove(&key) {
                Some(_) => {
                    save_db(db).unwrap();
                    println!("Removed '{:}'", key);
                },
                None => {
                    println!("There is no key '{:}'", key);
                }
            }
        }
    }
    true
}

enum KvResult {
    Done,
    None,
    Some { key: String, val: String },
}

fn get_kv() -> KvResult {
    match linenoise::input("  data key: ").map(|s| s.trim().to_string()) {
        None => KvResult::Done,
        Some(key) => {
            match key {
                ref x if x == "" => KvResult::Done,
                key => {
                    match linenoise::input(&format!("    value for {:}: ", key)) {
                        Some(ref x) if x == "" => KvResult::None,
                        Some(x) => KvResult::Some { key: key, val: x },
                        None => KvResult::None,
                    }
                }
            }
        }
    }
}

fn add_cmd(db: &mut Db, _: &str, rest_line: &str) -> bool {
    let arg = match rest_line {
        x if x != "" => Some(x.to_string()),
        _ => linenoise::input("new key: ")
    };
    if let Some(key) = arg {
        if key.len() > 0 {
            let mut rec = DbRecord { key: key.clone(), value: Vec::new() };
            loop {
                match get_kv() {
                    KvResult::Done => { break; },
                    KvResult::None => { },
                    KvResult::Some { key, val } => {
                        rec.value.push((key, val));
                    }
                }
            }
            println!("inserting '{:}'", key);
            db.data.insert(key, rec);
            println!("now storing {:} keys", db.data.len());
            save_db(db).unwrap();
        }
    }
    true
}

fn get_cmd(db: &mut Db, _: &str, rest_line: &str) -> bool {
    let arg = match rest_line {
        x if x != "" => Some(x.to_string()),
        _ => linenoise::input("find key: ")
    };
    if let Some(key) = arg {
        if key.len() > 0 {
            match db.data.get(&key) {
                None => {
                    println!("there is no match for {:}", &key);
                }
                Some(val) => {
                    println!("Data:");
                    for z in &val.value {
                        println!("{:}: {:}", z.0, z.1);
                    }
                }
            }
        }
    }
    true
}

fn find_cmd(db: &mut Db, _: &str, rest_line: &str) -> bool {
    let arg = match rest_line {
        x if x != "" => Some(x.to_string()),
        _ => linenoise::input("find key: ")
    };
    if let Some(key) = arg {
        if key.len() > 0 {
            for db_key in db.data.keys() {
                if db_key.contains(&key) {
                    println!("{:}", db_key);
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

enum PathKind {
    Main,
    Temp,
    Backup,
}

fn get_db_path(kind: PathKind) -> PathBuf {
    let mut path = std::env::home_dir().unwrap();
    path.push(".local");
    path.push("share");
    path.push("cred-man");
    std::fs::create_dir_all(&path).unwrap();
    path.push(match kind {
        PathKind::Main => "keys.db",
        PathKind::Temp => "keys.tmp.db",
        PathKind::Backup => "keys.backup.db",
    });
    path
}

fn load_db(db: &mut Db) -> io::Result<()> {
    let path = get_db_path(PathKind::Main);
    match fs::File::open(&path) {
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
            println!("Path {} not found", path.to_string_lossy());
            Ok(())
        },
        Err(e) => Err(e),
        Ok(mut f) => {
            let mut contents = "".to_string();
            try!(f.read_to_string(&mut contents));
            let dto: DbDTO = json::decode(&contents).unwrap();
            //println!("db = {:#?}", dto);
            for r in dto.records.into_iter() {
                let k = r.key.clone();
                db.data.insert(k, DbRecord {
                    key: r.key,
                    value: r.value,
                });
            }
            Ok(())
        }
    }
}

fn save_db(db: &mut Db) -> io::Result<()> {
    let main_path = get_db_path(PathKind::Main);
    let backup_path = get_db_path(PathKind::Backup);
    let temp_path = get_db_path(PathKind::Temp);
    match fs::metadata(&main_path) {
        Ok(_) => {
            try!(fs::copy(&main_path, &backup_path));
        },
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => (),
        Err(e) => {
            return Err(e);
        },
    };
    match fs::File::create(&temp_path) {
        Err(e) => Err(e),
        Ok(mut f) => {
            let mut dto = DbDTO {
                records: Vec::new()
            };
            for r in db.data.values() {
                dto.records.push(DbRecordDTO {
                    key: r.key.clone(),
                    value: r.value.clone(),
                });
            }
            let contents = json::encode(&dto).unwrap();
            try!(f.write_all(contents.as_bytes()));
            try!(fs::rename(&temp_path, &main_path));
            Ok(())
        }
    }
}

fn main() {
    if !authenticate() {
        return;
    }
    let mut db = Db { data: BTreeMap::new() };
    match load_db(&mut db) {
        Ok(_) => { println!("loaded"); }
        Err(e) => { println!("error: {:?}", e); panic!(); }
    }
    while let Some(cmd) = linenoise::input("> ") {
        if cmd.len() > 0 {
            if !execute_cmd(&mut db, &cmd) {
                return;
            }
        }
    }
}
