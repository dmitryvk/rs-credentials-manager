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
    clippy::missing_errors_doc
)]

use chrono::naive::NaiveDateTime;
use chrono::Local;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::PathBuf;

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
            password,
            location,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct DbRecordDTO {
    key: String,
    timestamp: String,
    value: BTreeMap<String, String>,
}

const DTO_TIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

#[derive(Clone)]
pub enum DbLocation {
    DotLocal,
    SpecifiedDirectory(PathBuf),
}

#[derive(Clone, Copy)]
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
        }
        DbLocation::SpecifiedDirectory(dir) => {
            path = dir.clone();
        }
    }
    path.push(match kind {
        PathKind::Main => "keys.db".to_string(),
        PathKind::Temp => "keys.tmp.db".to_string(),
        PathKind::Backup => format!("keys.backup.{}.db", Local::now().format("%Y%m%d_%H%M%S")),
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
                println!(
                    "Path {} not found, will create new database",
                    path.to_string_lossy()
                );
                let dir = path.parent().expect("get_db_path returns path with dir");
                std::fs::create_dir_all(dir)?;
                let db = Db::new(password.to_owned(), location.clone());
                Ok(DbLoadResult::Loaded(db))
            }
            Err(e) => Err(e),
            Ok(_) => {
                let data = encrypted_file::parse_file(&path)?;
                match encrypted_file::decrypt(&data, password) {
                    None => Ok(DbLoadResult::WrongPassword),
                    Some(contents) => {
                        let dto: Vec<DbRecordDTO> =
                            serde_json::from_str(&contents).map_err(|e| {
                                io::Error::new(
                                    io::ErrorKind::Other,
                                    format!("Db contains invalid json: {e}"),
                                )
                            })?;
                        let mut db = Db::new(password.to_owned(), location.clone());
                        for r in dto {
                            let k = r.key.clone();
                            db.data.insert(
                                k,
                                DbRecord {
                                    key: r.key,
                                    timestamp: NaiveDateTime::parse_from_str(
                                        &r.timestamp,
                                        DTO_TIME_FORMAT,
                                    ).map_err(|e| {
                                        io::Error::new(
                                            io::ErrorKind::Other,
                                            format!("Db contains invalid json: invalid timestamp \"{}\": {e}", r.timestamp),
                                        )
                                    })?,
                                    value: r.value,
                                },
                            );
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
        match fs::metadata(&main_path) {
            Ok(_) => {
                fs::copy(&main_path, backup_path)?;
            }
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => {
                return Err(e);
            }
        }
        let mut dto: Vec<DbRecordDTO> = Vec::new();
        for r in self.data.values() {
            dto.push(DbRecordDTO {
                key: r.key.clone(),
                timestamp: format!("{}", r.timestamp.format(DTO_TIME_FORMAT)),
                value: r.value.clone(),
            });
        }
        let contents = serde_json::to_string(&dto).expect("DbRecordDTO is json-serializable");
        let data = encrypted_file::encrypt(&contents, &self.password);
        encrypted_file::write_to_file(&temp_path, &data)?;
        fs::rename(&temp_path, &main_path)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn db_recorddto_is_serializable() {
        serde_json::to_string(&DbRecordDTO {
            key: String::new(),
            timestamp: String::new(),
            value: BTreeMap::new(),
        })
        .expect("DbRecordDTO should be serializable");
    }
}
