extern crate linenoise;
extern crate rustc_serialize;
extern crate chrono;

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use chrono::Local;
use chrono::naive::NaiveDateTime;
use rustc_serialize::json;

pub mod encrypted_file;

pub struct DbRecord {
    pub key: String,
    pub timestamp: NaiveDateTime,
    pub value: BTreeMap<String, String>,
}

pub struct Db {
    pub data: BTreeMap<String, DbRecord>,
    password: String,
    location: DbLocation,
}

impl Db {
    fn new(password: String, location: DbLocation) -> Db {
        Db {
            data: BTreeMap::new(),
            password: password,
            location: location,
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


#[derive(Clone)]
pub enum DbLocation {
    DotLocal,
    SpecifiedDirectory(String),
}

pub enum PathKind {
    Main,
    Temp,
    Backup,
}

fn get_db_path(kind: PathKind, location: &DbLocation) -> PathBuf {
    let mut path;
    match location {
        &DbLocation::DotLocal => {
            path = dirs::home_dir().expect("home_dir failed");
            path.push(".local");
            path.push("share");
            path.push("cred-man");
        },
        &DbLocation::SpecifiedDirectory(ref dir) => {
            path = PathBuf::from(&dir);
        },
    }
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

pub enum DbLoadResult {
    Loaded(Db),
    WrongPassword,
}

impl Db {
    pub fn load(location: &DbLocation, password: &str) -> io::Result<DbLoadResult> {
        let path = get_db_path(PathKind::Main, location);
        match fs::metadata(&path) {
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
                println!("Path {} not found, will create new database", path.to_string_lossy());
                let db = Db::new(password.to_owned(), location.clone());
                Ok(DbLoadResult::Loaded(db))
            },
            Err(e) => Err(e),
            Ok(_) => {
                let data = encrypted_file::parse_file(&path)?;
                match encrypted_file::decrypt(&data, &password) {
                    None => {
                        Ok(DbLoadResult::WrongPassword)
                    },
                    Some(contents) => {
                        let dto: Vec<DbRecordDTO> = json::decode(&contents).unwrap();
                        let mut db = Db::new(password.to_owned(), location.clone());
                        //println!("db = {:#?}", dto);
                        for r in dto.into_iter() {
                            let k = r.key.clone();
                            db.data.insert(k, DbRecord {
                                key: r.key,
                                timestamp: NaiveDateTime::parse_from_str(&r.timestamp, DTO_TIME_FORMAT).unwrap(),
                                value: r.value,
                            });
                        }
                        Ok(DbLoadResult::Loaded(db))
                    }
                }
            }
        }
    }

    pub fn save(&self) -> io::Result<()> {
        let main_path = get_db_path(PathKind::Main, &self.location);
        let backup_path = get_db_path(PathKind::Backup, &self.location);
        let temp_path = get_db_path(PathKind::Temp, &self.location);
        //println!("main_path: {:?}", main_path);
        match fs::metadata(&main_path) {
            Ok(_) => {
                //println!("copy main to backup");
                fs::copy(&main_path, &backup_path)?;
            },
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => {
                return Err(e);
            },
        };
        let mut dto: Vec<DbRecordDTO> = Vec::new();
        for r in self.data.values() {
            dto.push(DbRecordDTO {
                key: r.key.clone(),
                timestamp: format!("{}", r.timestamp.format(DTO_TIME_FORMAT)),
                value: r.value.clone(),
            });
        }
        let contents = json::encode(&dto).unwrap();
        //println!("contents: {}", contents);
        let data = encrypted_file::encrypt(&contents, &self.password);
        //println!("encrypted");
        encrypted_file::write_to_file(&temp_path, &data)?;
        //println!("wrote to {:?}", temp_path);
        fs::rename(&temp_path, &main_path)?;
        Ok(())
    }
}
