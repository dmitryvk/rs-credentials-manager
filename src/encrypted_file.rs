use getrandom::getrandom;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::path::Path;
use std::str;

#[must_use]
pub fn generate_salt(n: usize) -> Vec<u8> {
    let mut data: Vec<u8> = vec![0; n];
    getrandom(&mut data).expect("getrandom() failed");
    data
}

#[must_use]
pub fn derive_key(salt: &[u8], password: &str) -> Vec<u8> {
    use scrypt::Params;
    let mut result = vec![0u8; 32];
    scrypt::scrypt(
        password.as_bytes(),
        salt,
        &Params::new(14, 8, 1, 32).unwrap(),
        &mut result,
    )
    .unwrap();
    result
}

#[allow(clippy::module_name_repetitions)]
pub struct EncryptedFileContent {
    salt: Vec<u8>,
    nonce: Vec<u8>,
    tag: Vec<u8>,
    ciphertext: Vec<u8>,
}

pub fn encrypt(plaintext: &str, password: &str) -> EncryptedFileContent {
    use aes_gcm::{
        aead::{Aead, OsRng, Payload},
        AeadCore, Aes256Gcm, Key, KeyInit,
    };
    let salt = generate_salt(16);
    let key = derive_key(&salt, password);
    let key = Key::<Aes256Gcm>::from_slice(&key);
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    // let mut tag = vec![0u8; 16];
    let aad = b"cred-man";
    let result = cipher
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext.as_bytes(),
                aad,
            },
        )
        .expect("should not be fallible");
    let tag = result[(result.len() - 16)..].to_vec();
    let ciphertext = result[..(result.len() - 16)].to_vec();

    EncryptedFileContent {
        salt,
        nonce: nonce.to_vec(),
        tag,
        ciphertext,
    }
}

fn read_bytes(source: &mut dyn Read, buffer: &mut [u8]) -> io::Result<()> {
    let mut pos = 0;
    while pos < buffer.len() {
        pos += source.read(buffer.split_at_mut(pos).1)?;
    }
    Ok(())
}

const CRED_MAN_MAGIC: &[u8] = b"CREDMAN";

const CRED_MAN_VERSION: i32 = 1;

#[must_use]
#[allow(clippy::cast_sign_loss)]
pub fn i32_to_bytes(x: i32) -> [u8; 4] {
    let v: [u8; 4] = [
        ((x >> 24) & 0xFF) as u8,
        ((x >> 16) & 0xFF) as u8,
        ((x >> 8) & 0xFF) as u8,
        (x & 0xFF) as u8,
    ];
    v
}

pub fn write_to_file<P: AsRef<Path>>(file_name: P, data: &EncryptedFileContent) -> io::Result<()> {
    let mut file = File::create(file_name)?;

    file.write_all(CRED_MAN_MAGIC)?;
    file.write_all(&i32_to_bytes(CRED_MAN_VERSION))?;
    file.write_all(&data.salt)?;
    file.write_all(&data.nonce)?;
    file.write_all(&data.tag)?;
    file.write_all(&data.ciphertext)?;

    Ok(())
}

pub fn parse_file<P: AsRef<Path>>(file_name: P) -> io::Result<EncryptedFileContent> {
    let mut file = File::open(file_name)?;
    #[allow(clippy::cast_possible_truncation)]
    let size = file.metadata()?.len() as usize;

    let mut magic = vec![0u8; CRED_MAN_MAGIC.len()];
    let mut ver_bytes = [0u8; 4];
    let mut salt = vec![0u8; 16];
    let mut nonce = vec![0u8; 12];
    let mut tag = vec![0u8; 16];
    let mut ciphertext = vec![0u8; size - CRED_MAN_MAGIC.len() - 4 - 16 - 12 - 16];

    read_bytes(&mut file, &mut magic)?;
    read_bytes(&mut file, &mut ver_bytes)?;
    read_bytes(&mut file, &mut salt)?;
    read_bytes(&mut file, &mut nonce)?;
    read_bytes(&mut file, &mut tag)?;
    read_bytes(&mut file, &mut ciphertext)?;

    if magic != CRED_MAN_MAGIC {
        return Err(io::Error::new(io::ErrorKind::Other, "MAGIC mismatch"));
    }
    #[allow(clippy::cast_lossless)]
    let ver: i32 = ((ver_bytes[0] as i32) << 24)
        | ((ver_bytes[1] as i32) << 16)
        | ((ver_bytes[2] as i32) << 8)
        | (ver_bytes[3] as i32);

    if ver > 1 {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Unsupported credentials database version: {ver}"),
        ));
    }

    Ok(EncryptedFileContent {
        salt,
        nonce,
        tag,
        ciphertext,
    })
}

#[must_use]
pub fn decrypt(data: &EncryptedFileContent, password: &str) -> Option<String> {
    use aes_gcm::{
        aead::{Aead, Nonce, Payload},
        Aes256Gcm, Key, KeyInit,
    };
    let key = derive_key(&data.salt, password);
    let aad = b"cred-man";
    let mut ciphertext = Vec::with_capacity(data.tag.len() + data.ciphertext.len());
    ciphertext.extend_from_slice(&data.ciphertext);
    ciphertext.extend_from_slice(&data.tag);

    let key = Key::<Aes256Gcm>::from_slice(&key);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::<Aes256Gcm>::from_slice(&data.nonce);
    let plaintext = cipher
        .decrypt(
            nonce,
            Payload {
                msg: &ciphertext,
                aad,
            },
        )
        .ok()?;

    let plaintext = String::from_utf8(plaintext).ok()?;

    Some(plaintext)
}
