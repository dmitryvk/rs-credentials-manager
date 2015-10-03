//! # Intro
//!
//! This is a credentials manager program.
//! It's an interactive command-line application.
//! 
//! # Usage
//!
//! Data is stored in ~/.local/share/credentials-manager (it's stored in clear for now).
//! 

extern crate cred_man_lib;
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
use chrono::naive::datetime::NaiveDateTime;
use cred_man_lib::encrypted_file;
use std::str::FromStr;

fn parse_cmd_line(cmd_line: &str) -> (&str, &str) {
    let idx = cmd_line.find(' ').unwrap_or(cmd_line.len());
    let cmd = unsafe { cmd_line.slice_unchecked(0, idx) };
    let rest = unsafe { cmd_line.slice_unchecked(cmp::min(cmd_line.len(), idx + 1), cmd_line.len()) }.trim();
    //println!("cmd = {:?} rest = {:?}", cmd, rest);
    (cmd, rest)
}

struct DbRecord {
    key: String,
    timestamp: NaiveDateTime,
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
    timestamp: String,
    value: BTreeMap<String, String>,
}

const DTO_TIME_FORMAT: &'static str = "%Y-%m-%dT%H:%M:%S";

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
        "rename" => Some(rename_cmd),
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
    println!(" rename");
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
                timestamp: Local::now().naive_local(),
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

struct RenameCmdArgs {
    from: String,
    to: String,
}

impl RenameCmdArgs {
    fn parse(args_line: &str) -> Option<RenameCmdArgs> {
        let mut args = args_line.split_whitespace().map(|s| s.to_string()).collect::<Vec<_>>();
        if args.len() > 2 {
            return None;
        }
        if args.len() == 0 {
            let k = linenoise::input("old key name: ");
            match k {
                Some(k) => {
                    add_linenoise_history(&k);
                    args.push(k);
                },
                None => { return None; },
            }
        }
        if args.len() == 1 {
            let k = linenoise::input("new key name: ");
            match k {
                Some(k) => {
                    add_linenoise_history(&k);
                    args.push(k);
                },
                None => { return None; },
            }
        }
        let from;
        let to;
        {
            let mut it = args.into_iter();
            from = it.next().unwrap();
            to = it.next().unwrap();
        }
        Some(RenameCmdArgs { from: from, to: to })
    }
}

fn rename_cmd(db: &mut Db, _: &str, args_line: &str) -> bool {
    match RenameCmdArgs::parse(args_line) {
        None => {
            println!("Unexpected input; expected: rename [oldname [newname]]");
        },
        Some(RenameCmdArgs { from, to }) => {
            if let Some(_) = db.data.get(&to) {
                println!("Key {} already exists, not renaming", to);
            } else {
                let cur = db.data.remove(&from);
                match cur {
                    None => {
                        println!("Key {} does not exist", from);
                    },
                    Some(mut v) => {
                        v.key = to.clone();
                        v.timestamp = Local::now().naive_local();
                        db.data.insert(to.clone(), v);
                        save_db(db).unwrap();
                        println!("Renamed {} to {}", from, to);
                    },
                }
            }
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
            timestamp: format!("{}", r.timestamp.format(DTO_TIME_FORMAT)),
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
            timestamp: NaiveDateTime::parse_from_str(&r.timestamp, DTO_TIME_FORMAT).unwrap(),
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
                    println!("Timestamp: {}", val.timestamp.format("%Y-%m-%d %H:%M:%S"));
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

enum ListCmd {
    AllKeys,
    Recent(Option<usize>),
}

impl ListCmd {
    fn parse(args_line: &str) -> Option<Self> {
        let args = args_line.split_whitespace().collect::<Vec<_>>();
        if args.len() == 0 {
            Some(ListCmd::AllKeys)
        } else if args.len() == 1 && args[0] == "recent" {
            Some(ListCmd::Recent(None))
        } else if args.len() == 2 && args[0] == "recent" {
            usize::from_str(args[1]).ok().map(|c| ListCmd::Recent(Some(c)))
        } else {
            None
        }
    }
}

fn list_cmd(db: &mut Db, _: &str, args_line: &str) -> bool {
    let cmd = ListCmd::parse(args_line);
    match cmd {
        Some(ListCmd::AllKeys) => {
            for v in db.data.values() {
                println!("{} ({})", v.key, v.timestamp.format("%Y-%m-%d %H:%M:%S"));
            }
        },
        Some(ListCmd::Recent(opt_count)) => {
            let mut entries = db.data.values().collect::<Vec<_>>();
            entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            let count = match opt_count {
                Some(c) => c,
                None => 10,
            };
            while entries.len() > count {
                let remove_idx = entries.len() - 1;
                entries.remove(remove_idx);
            }
            println!("{} recent keys:", count);
            for v in entries.iter() {
                println!("{} ({})", v.key, v.timestamp.format("%Y-%m-%d %H:%M:%S"));
            }
        },
        None => {
            println!("Unrecognized arguments for list; expected: list [recent [count]]");
        }
    };
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
                            timestamp: NaiveDateTime::parse_from_str(&r.timestamp, DTO_TIME_FORMAT).unwrap(),
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
            timestamp: format!("{}", r.timestamp.format(DTO_TIME_FORMAT)),
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
