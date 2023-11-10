//! # Intro
//!
//! This is a credentials manager program.
//! It's an interactive command-line application.
//!
//! # Usage
//!
//! Data is stored in ~/.local/share/credentials-manager.
//!

use chrono::naive::NaiveDateTime;
use chrono::Local;
use cred_man_lib::{Db, DbLoadResult, DbLocation, DbRecord};
use serde::{Deserialize, Serialize};
use std::cmp;
use std::collections::BTreeMap;
use std::io;
use std::io::{Read, Write};
use std::str::FromStr;

fn parse_cmd_line(cmd_line: &str) -> (&str, &str) {
    let idx = cmd_line.find(' ').unwrap_or(cmd_line.len());
    let cmd = cmd_line.get(0..idx).expect("str.get panicked");
    let rest = cmd_line
        .get(cmp::min(cmd_line.len(), idx + 1)..cmd_line.len())
        .expect("str.get panicked")
        .trim();
    (cmd, rest)
}

#[derive(Serialize, Deserialize, Debug)]
struct DbRecordDTO {
    key: String,
    timestamp: String,
    value: BTreeMap<String, String>,
}

const DTO_TIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

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
        "edit" => Some(edit_cmd),
        _ => None,
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
    println!(" edit");
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
        x if !x.is_empty() => Some(x.to_string()),
        _ => linenoise::input("Key: "),
    };
    if let Some(key) = arg {
        add_linenoise_history(&key);
        if !key.is_empty() {
            match db.data.remove(&key) {
                Some(_) => {
                    db.save().unwrap();
                    println!("Removed '{:}'", key);
                }
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
        Some(key) => match key {
            ref x if x.is_empty() => KvResult::Done,
            key => {
                let parts: Vec<_> = key.splitn(2, ' ').map(|s| s.trim().to_string()).collect();
                let real_key = parts[0].clone();
                if parts.len() == 2 {
                    KvResult::Some {
                        key: real_key,
                        val: parts[1].clone(),
                    }
                } else {
                    match linenoise::input(&format!("    value for {:}: ", key)) {
                        Some(ref x) if x.is_empty() => KvResult::None,
                        Some(x) => KvResult::Some { key, val: x },
                        None => KvResult::None,
                    }
                }
            }
        },
    }
}

fn add_cmd(db: &mut Db, _: &str, rest_line: &str) -> bool {
    let arg = match rest_line {
        x if !x.is_empty() => Some(x.to_string()),
        _ => linenoise::input("new key: "),
    };
    if let Some(key) = arg {
        if !key.is_empty() {
            let mut rec = DbRecord {
                key: key.clone(),
                timestamp: Local::now().naive_local(),
                value: BTreeMap::new(),
            };
            loop {
                match get_kv() {
                    KvResult::Done => {
                        break;
                    }
                    KvResult::None => {}
                    KvResult::Some { key, val } => {
                        rec.value.insert(key.clone(), val);
                    }
                }
            }
            db.data.insert(key.clone(), rec);
            println!("inserted '{}', now storing {} keys", key, db.data.len());
            db.save().unwrap();
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
        let mut args = args_line
            .split_whitespace()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        if args.len() > 2 {
            return None;
        }
        if args.is_empty() {
            let k = linenoise::input("old key name: ");
            match k {
                Some(k) => {
                    add_linenoise_history(&k);
                    args.push(k);
                }
                None => {
                    return None;
                }
            }
        }
        if args.len() == 1 {
            let k = linenoise::input("new key name: ");
            match k {
                Some(k) => {
                    add_linenoise_history(&k);
                    args.push(k);
                }
                None => {
                    return None;
                }
            }
        }
        let from;
        let to;
        {
            let mut it = args.into_iter();
            from = it.next().unwrap();
            to = it.next().unwrap();
        }
        Some(RenameCmdArgs { from, to })
    }
}

fn rename_cmd(db: &mut Db, _: &str, args_line: &str) -> bool {
    match RenameCmdArgs::parse(args_line) {
        None => {
            println!("Unexpected input; expected: rename [oldname [newname]]");
        }
        Some(RenameCmdArgs { from, to }) => {
            if db.data.get(&to).is_some() {
                println!("Key {} already exists, not renaming", to);
            } else {
                let cur = db.data.remove(&from);
                match cur {
                    None => {
                        println!("Key {} does not exist", from);
                    }
                    Some(mut v) => {
                        v.key = to.clone();
                        v.timestamp = Local::now().naive_local();
                        db.data.insert(to.clone(), v);
                        db.save().unwrap();
                        println!("Renamed {} to {}", from, to);
                    }
                }
            }
        }
    }
    true
}

#[derive(Debug)]
struct EditCmd {
    key: String,
    op: EditCmdOperation,
}

#[derive(Debug)]
enum EditCmdOperation {
    Del(String),
    Add(String, String),
    Update(String, String),
    Rename(String, String),
}

fn ask_user(prompt: &str, history: bool) -> String {
    let response = linenoise::input(prompt).unwrap();
    if history {
        add_linenoise_history(&response);
    }
    response
}

impl EditCmd {
    fn parse(args_line: &str) -> Option<Self> {
        let args = args_line
            .split_whitespace()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        let mut it = args.into_iter();
        let key = it
            .next()
            .unwrap_or_else(|| ask_user("Enter the key to be edited: ", true));
        if key.is_empty() {
            return None;
        }
        let cmd = it.next().unwrap_or_else(|| {
            ask_user("Enter the edit command (del, add, update, rename): ", true)
        });
        match cmd.as_ref() {
            "del" => {
                let subkey = it
                    .next()
                    .unwrap_or_else(|| ask_user("Enter the subkey: ", true));
                if subkey.is_empty() {
                    return None;
                }
                Some(EditCmd {
                    key,
                    op: EditCmdOperation::Del(subkey),
                })
            }
            "add" => {
                let subkey = it
                    .next()
                    .unwrap_or_else(|| ask_user("Enter the subkey: ", true));
                if subkey.is_empty() {
                    return None;
                }
                let value = it
                    .next()
                    .unwrap_or_else(|| ask_user("Enter the value: ", true));
                if value.is_empty() {
                    return None;
                }
                Some(EditCmd {
                    key,
                    op: EditCmdOperation::Add(subkey, value),
                })
            }
            "update" => {
                let subkey = it
                    .next()
                    .unwrap_or_else(|| ask_user("Enter the subkey: ", true));
                if subkey.is_empty() {
                    return None;
                }
                let value = it
                    .next()
                    .unwrap_or_else(|| ask_user("Enter the value: ", true));
                if value.is_empty() {
                    return None;
                }
                Some(EditCmd {
                    key,
                    op: EditCmdOperation::Update(subkey, value),
                })
            }
            "rename" => {
                let subkey = it
                    .next()
                    .unwrap_or_else(|| ask_user("Enter the subkey: ", true));
                if subkey.is_empty() {
                    return None;
                }
                let new_subkey = it
                    .next()
                    .unwrap_or_else(|| ask_user("Enter the new subkey name: ", true));
                if new_subkey.is_empty() {
                    return None;
                }
                Some(EditCmd {
                    key,
                    op: EditCmdOperation::Rename(subkey, new_subkey),
                })
            }
            _ => None,
        }
    }
}

fn edit_cmd(db: &mut Db, _: &str, args_line: &str) -> bool {
    let cmd = EditCmd::parse(args_line);
    println!("edit cmd: {:?}", cmd);
    match cmd {
        None => {
            println!("Unexepected input. Expected: edit [key [op_type [subkey [arg]]]]");
        }
        Some(cmd) => {
            let should_save: bool;
            let msg: String;
            match db.data.get_mut(&cmd.key) {
                None => {
                    should_save = false;
                    msg = format!("Entry {} not found", cmd.key);
                }
                Some(entry) => match cmd.op {
                    EditCmdOperation::Del(subkey) => match entry.value.remove(&subkey) {
                        Some(_) => {
                            should_save = true;
                            msg = format!("Subkey {} removed", subkey);
                        }
                        None => {
                            should_save = false;
                            msg = format!("Entry {} does not contain subkey {}", cmd.key, subkey);
                        }
                    },
                    EditCmdOperation::Add(subkey, value) => {
                        if entry.value.get(&subkey).is_none() {
                            entry.value.insert(subkey.clone(), value);
                            should_save = true;
                            msg = format!("Added subkey {} for {}", subkey, cmd.key);
                        } else {
                            should_save = false;
                            msg = format!("Subkey {} already exists", subkey);
                        }
                    }
                    EditCmdOperation::Update(subkey, value) => match entry.value.get_mut(&subkey) {
                        None => {
                            should_save = false;
                            msg = format!("Subkey {} does not exist", subkey);
                        }
                        Some(subvalue) => {
                            *subvalue = value;
                            should_save = true;
                            msg = format!("Updated subkey {} for {}", subkey, cmd.key);
                        }
                    },
                    EditCmdOperation::Rename(subkey, newsubkey) => {
                        let cur = entry.value.remove(&subkey);
                        match cur {
                            None => {
                                should_save = false;
                                msg = format!("Subkey {} does not exist", subkey);
                            }
                            Some(cur) => {
                                entry.value.insert(newsubkey.clone(), cur);
                                should_save = true;
                                msg = format!("Renamed subkey {} to {}", subkey, newsubkey);
                            }
                        }
                    }
                },
            }
            if should_save {
                db.save().unwrap();
            }
            println!("{}", msg);
        }
    }
    true
}

fn dump_cmd(db: &mut Db, _: &str, rest_line: &str) -> bool {
    let mut out: Box<dyn Write> = match rest_line {
        x if !x.is_empty() => Box::new(std::fs::File::create(x).unwrap()),
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
    let contents = serde_json::to_string_pretty(&dto).expect("DbRecordDTO is json-serializable");
    out.write_all(contents.as_bytes()).unwrap();
    out.write_all(b"\n").unwrap();
    out.flush().unwrap();

    true
}

fn import_from(db: &mut Db, file_name: &str) -> io::Result<()> {
    let mut contents = String::new();
    let mut f = std::fs::File::open(file_name)?;
    f.read_to_string(&mut contents)?;
    let dto: Vec<DbRecordDTO> = serde_json::from_str(&contents).unwrap();
    for r in dto {
        let v = DbRecord {
            key: r.key,
            timestamp: NaiveDateTime::parse_from_str(&r.timestamp, DTO_TIME_FORMAT).unwrap(),
            value: r.value,
        };
        let k = v.key.clone();
        db.data.insert(k, v);
    }

    db.save().unwrap();

    Ok(())
}

fn import_cmd(db: &mut Db, _: &str, rest_line: &str) -> bool {
    let filename = match rest_line {
        x if !x.is_empty() => x.trim().to_string(),
        _ => {
            let tmp = linenoise::input("Enter filename: ").unwrap();
            add_linenoise_history(&tmp);
            tmp
        }
    };

    match import_from(db, &filename) {
        Ok(()) => {}
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }

    true
}

fn get_cmd(db: &mut Db, _: &str, rest_line: &str) -> bool {
    let arg = match rest_line {
        x if !x.is_empty() => Some(x.to_string()),
        _ => linenoise::input("find key: "),
    };
    if let Some(key) = arg {
        add_linenoise_history(&key);
        if !key.is_empty() {
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
        x if !x.is_empty() => Some(x.to_string()),
        _ => linenoise::input("find key: "),
    };
    if let Some(key) = arg {
        add_linenoise_history(&key);
        if !key.is_empty() {
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
        if args.is_empty() {
            Some(ListCmd::AllKeys)
        } else if args.len() == 1 && args[0] == "recent" {
            Some(ListCmd::Recent(None))
        } else if args.len() == 2 && args[0] == "recent" {
            usize::from_str(args[1])
                .ok()
                .map(|c| ListCmd::Recent(Some(c)))
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
        }
        Some(ListCmd::Recent(opt_count)) => {
            let mut entries = db.data.values().collect::<Vec<_>>();
            entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            let count = opt_count.unwrap_or(10);
            while entries.len() > count {
                let remove_idx = entries.len() - 1;
                entries.remove(remove_idx);
            }
            println!("{} recent keys:", count);
            for v in entries.iter() {
                println!("{} ({})", v.key, v.timestamp.format("%Y-%m-%d %H:%M:%S"));
            }
        }
        None => {
            println!("Unrecognized arguments for list; expected: list [recent [count]]");
        }
    };
    true
}

fn execute_cmd(db: &mut Db, cmd_line: &str) -> bool {
    let (cmd, args) = parse_cmd_line(cmd_line);
    match get_command_handler(cmd) {
        Some(handler) => handler(db, cmd, args),
        None => {
            println!("Unknown command {:}; try `help'", cmd);
            true
        }
    }
}

fn parse_args() -> DbLocation {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        DbLocation::DotLocal
    } else {
        let mut it = args.into_iter();
        let s = it.next().unwrap();
        DbLocation::SpecifiedDirectory(s)
    }
}

fn main() {
    let db_location = parse_args();
    let mut db;
    let password = linenoise::input("Enter password: ").unwrap();
    match Db::load(&db_location, &password) {
        Ok(DbLoadResult::Loaded(loaded_db)) => {
            db = loaded_db;
        }
        Ok(DbLoadResult::WrongPassword) => {
            println!("Wrong password");
            return;
        }
        Err(e) => {
            println!("error: {:}", e);
            return;
        }
    }
    linenoise::clear_screen();
    while let Some(cmd) = linenoise::input("> ") {
        add_linenoise_history(&cmd);
        if !cmd.is_empty() && !execute_cmd(&mut db, &cmd) {
            return;
        }
    }
}
