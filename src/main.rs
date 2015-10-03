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
extern crate rand;
extern crate chrono;

use std::cmp;
use std::collections::BTreeMap;
use std::io;
use rustc_serialize::json;
use std::fs;
use std::path::PathBuf;
use std::io::{Read,Write};
use chrono::Local;

mod encrypted_file;

fn parse_cmd_line(cmd_line: &str) -> (&str, &str) {
    let idx = cmd_line.find(' ').unwrap_or(cmd_line.len());
    let cmd = unsafe { cmd_line.slice_unchecked(0, idx) };
    let rest = unsafe { cmd_line.slice_unchecked(cmp::min(cmd_line.len(), idx + 1), cmd_line.len()) }.trim();
    //println!("cmd = {:?} rest = {:?}", cmd, rest);
    (cmd, rest)
}

struct DbRecord {
    key: String,
    value: BTreeMap<String, String>,
}

struct Db {
    data: BTreeMap<String, DbRecord>,
    password: String,
}

impl Db {
    fn new() -> Db {
        Db {
            data: BTreeMap::new(),
            password: String::new(),
        }
    }
}

#[derive(RustcDecodable, RustcEncodable, Debug)]
struct DbRecordDTO {
    key: String,
    value: BTreeMap<String, String>,
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
        "dump" => Some(dump_cmd),
        "import" => Some(import_cmd),
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
    println!(" dump");
    println!(" import");
    true
}

fn quit_cmd(_: &mut Db, _: &str, _: &str) -> bool {
    false
}

fn add_linenoise_history(line: &str) {
    if !line.trim().is_empty() {
        linenoise::history_add(line);
    }
}

fn del_cmd(db: &mut Db, _: &str, rest_line: &str) -> bool {
    let arg = match rest_line {
        x if x != "" => Some(x.to_string()),
        _ => linenoise::input("Key: ")
    };
    if let Some(key) = arg {
        add_linenoise_history(&key);
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
                    let parts: Vec<_> = key.splitn(2, ' ').map(|s| s.trim().to_string()).collect();
                    let real_key = parts[0].clone();
                    if parts.len() == 2 {
                        KvResult::Some { key: real_key, val: parts[1].clone() }
                    } else {
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
}

fn add_cmd(db: &mut Db, _: &str, rest_line: &str) -> bool {
    let arg = match rest_line {
        x if x != "" => Some(x.to_string()),
        _ => linenoise::input("new key: ")
    };
    if let Some(key) = arg {
        if key.len() > 0 {
            let mut rec = DbRecord {
                key: key.clone(),
                value: BTreeMap::new()
            };
            loop {
                match get_kv() {
                    KvResult::Done => { break; },
                    KvResult::None => { },
                    KvResult::Some { key, val } => {
                        rec.value.insert(key.clone(), val);
                    }
                }
            }
            db.data.insert(key.clone(), rec);
            println!("inserted '{}', now storing {} keys",
                     key,
                     db.data.len());
            save_db(db).unwrap();
        }
    }
    true
}

fn dump_cmd(db: &mut Db, _: &str, rest_line: &str) -> bool {
    let mut out: Box<Write> = match rest_line {
        x if x != "" => Box::new(std::fs::File::create(&x).unwrap()),
        _ => Box::new(std::io::stdout()),
    };
    let mut dto = Vec::new();
    for r in db.data.values() {
        dto.push(DbRecordDTO {
            key: r.key.clone(),
            value: r.value.clone(),
        });
    }
    let contents = json::as_pretty_json(&dto).to_string();
    out.write_all(contents.as_bytes()).unwrap();
    out.write_all(b"\n").unwrap();
    out.flush().unwrap();
    
    true
}

fn import_from(db: &mut Db, file_name: &str) -> io::Result<()> {
    let mut contents = String::new();
    let mut f = try!(std::fs::File::open(file_name));
    try!(f.read_to_string(&mut contents));
    let dto: Vec<DbRecordDTO> = json::decode(&contents).unwrap();
    for r in dto {
        let v = DbRecord {
            key: r.key,
            value: r.value,
        };
        let k = v.key.clone();
        db.data.insert(k, v);
    }

    save_db(db).unwrap();

    Ok(())
}

fn import_cmd(db: &mut Db, _: &str, rest_line: &str) -> bool {
    let filename = match rest_line {
        x if x != "" => x.trim().to_string(),
        _ => {
            let tmp = linenoise::input("Enter filename: ").unwrap();
            add_linenoise_history(&tmp);
            tmp
        }
    };

    match import_from(db, &filename) {
        Ok(()) => { },
        Err(e) => {
            println!("Error: {:?}", e);
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
        add_linenoise_history(&key);
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
        add_linenoise_history(&key);
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
        PathKind::Main => "keys.db".to_string(),
        PathKind::Temp => "keys.tmp.db".to_string(),
        PathKind::Backup => format!(
            "keys.backup.{}.db",
            Local::now().format("%Y%m%d_%H%M%S").to_string()
                )
    });
    path
}

enum DbLoadResult {
    Loaded(Db),
    WrongPassword,
}

fn load_db() -> io::Result<DbLoadResult> {
    let path = get_db_path(PathKind::Main);
    let password = linenoise::input("Enter password: ").unwrap();
    linenoise::clear_screen();
    match fs::metadata(&path) {
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
            println!("Path {} not found, will create new database", path.to_string_lossy());
            let mut db = Db::new();
            db.password = password;
            Ok(DbLoadResult::Loaded(db))
        },
        Err(e) => Err(e),
        Ok(_) => {
            let data = try!(encrypted_file::parse_file(&path));
            match encrypted_file::decrypt(&data, &password) {
                None => {
                    println!("Wrong password!");
                    let ans = linenoise::input("Recreate db (yes/N)? ").unwrap();
                    if ans.to_lowercase() == "yes" {
                        let mut db = Db::new();
                        db.password = password;
                        Ok(DbLoadResult::Loaded(db))
                    } else {
                        Ok(DbLoadResult::WrongPassword)
                    }
                },
                Some(contents) => {
                    let dto: Vec<DbRecordDTO> = json::decode(&contents).unwrap();
                    let mut db = Db::new();
                    //println!("db = {:#?}", dto);
                    for r in dto.into_iter() {
                        let k = r.key.clone();
                        db.data.insert(k, DbRecord {
                            key: r.key,
                            value: r.value,
                        });
                    }
                    db.password = password;
                    Ok(DbLoadResult::Loaded(db))
                }
            }
        }
    }
}

fn save_db(db: &mut Db) -> io::Result<()> {
    let main_path = get_db_path(PathKind::Main);
    let backup_path = get_db_path(PathKind::Backup);
    let temp_path = get_db_path(PathKind::Temp);
    //println!("main_path: {:?}", main_path);
    match fs::metadata(&main_path) {
        Ok(_) => {
            //println!("copy main to backup");
            try!(fs::copy(&main_path, &backup_path));
        },
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => (),
        Err(e) => {
            return Err(e);
        },
    };
    let mut dto: Vec<DbRecordDTO> = Vec::new();
    for r in db.data.values() {
        dto.push(DbRecordDTO {
            key: r.key.clone(),
            value: r.value.clone(),
        });
    }
    let contents = json::encode(&dto).unwrap();
    //println!("contents: {}", contents);
    let data = encrypted_file::encrypt(&contents, &db.password);
    //println!("encrypted");
    try!(encrypted_file::write_to_file(&temp_path, &data));
    //println!("wrote to {:?}", temp_path);
    try!(fs::rename(&temp_path, &main_path));
    Ok(())
}

fn main() {
    let mut db;
    match load_db() {
        Ok(DbLoadResult::Loaded(loaded_db)) => { db = loaded_db; },
        Ok(DbLoadResult::WrongPassword) => { return; },
        Err(e) => { println!("error: {:}", e); return; },
    }
    while let Some(cmd) = linenoise::input("> ") {
        add_linenoise_history(&cmd);
        if cmd.len() > 0 {
            if !execute_cmd(&mut db, &cmd) {
                return;
            }
        }
    }
}
