extern crate crypto;

use rand::{Rng, OsRng};
use self::crypto::scrypt::{scrypt, ScryptParams};
use self::crypto::aes::KeySize;
use self::crypto::aes_gcm::AesGcm;
use self::crypto::aead::{AeadEncryptor, AeadDecryptor};
use std::fs::File;
use std::path::Path;
use std::str;
use std::io;
use std::io::{Read,Write};

pub fn generate_salt<T>(rng: &mut T, n: usize) -> Vec<u8>
    where T: Rng
{
    let mut data: Vec<u8> = vec![0; n];
    rng.fill_bytes(&mut data);
    data
}

pub fn derive_key(salt: &[u8], password: &str) -> Vec<u8> {
    let mut result = vec![0u8; 32];
    scrypt(password.as_bytes(), salt, &ScryptParams::new(10, 8, 1), &mut result);
    result
}

pub struct EncryptedFileContent {
    salt: Vec<u8>,
    nonce: Vec<u8>,
    tag: Vec<u8>,
    ciphertext: Vec<u8>,
}

pub fn encrypt(plaintext: &str, password: &str) -> EncryptedFileContent {
    let mut rng = OsRng::new().ok().expect("Unable to open crypto RNG");
    
    let salt = generate_salt(&mut rng, 16);
    let key = derive_key(&salt, &password);
    let nonce = generate_salt(&mut rng, 12);
    let mut tag = vec![0u8; 16];
    let aad = b"cred-man";
    
    let mut ciphertext = vec![0u8; plaintext.len()];
    AesGcm::new(KeySize::KeySize256, &key, &nonce, aad)
        .encrypt(plaintext.as_bytes(), &mut ciphertext, &mut tag);

    EncryptedFileContent {
        salt: salt,
        nonce: nonce,
        tag: tag,
        ciphertext: ciphertext,
    }
}

fn read_bytes(source: &mut Read, buffer: &mut [u8]) -> io::Result<()> {
    let mut pos = 0;
    while pos < buffer.len() {
        pos += try!(source.read(buffer.split_at_mut(pos).1));
    }
    Ok(())
}

pub fn write_to_file<P: AsRef<Path>>(file_name: P, data: &EncryptedFileContent) -> io::Result<()> {
    let mut file = try!(File::create(file_name));
    
    try!(file.write_all(&data.salt));
    try!(file.write_all(&data.nonce));
    try!(file.write_all(&data.tag));
    try!(file.write_all(&data.ciphertext));

    Ok(())
}

pub fn parse_file<P: AsRef<Path>>(file_name: P) -> io::Result<EncryptedFileContent> {
    let mut file = try!(File::open(file_name));
    let size = try!(file.metadata()).len() as usize;

    let mut salt = vec![0u8; 16];
    let mut nonce = vec![0u8; 12];
    let mut tag = vec![0u8; 16];
    let mut ciphertext = vec![0u8; size - 16 - 12 - 16];

    try!(read_bytes(&mut file, &mut salt));
    try!(read_bytes(&mut file, &mut nonce));
    try!(read_bytes(&mut file, &mut tag));
    try!(read_bytes(&mut file, &mut ciphertext));

    Ok(EncryptedFileContent {
        salt: salt,
        nonce: nonce,
        tag: tag,
        ciphertext: ciphertext,
    })
}

pub fn decrypt(data: &EncryptedFileContent, password: &str) -> Option<String> {
    let key = derive_key(&data.salt, &password);
    let aad = b"cred-man";

    let mut deciphered = vec![0u8; data.ciphertext.len()];
    
    let success = AesGcm::new(KeySize::KeySize256, &key, &data.nonce, aad)
        .decrypt(&data.ciphertext, &mut deciphered, &data.tag);

    if success {
        Some(str::from_utf8(&deciphered).unwrap().to_string())
    } else {
        None
    }
}
