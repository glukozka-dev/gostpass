use libgost_rs::Kuznechik;
use libgost_rs::kdf_gostr3411_2012_256;
use serde::{Deserialize, Serialize};
use std::fs::{read, write};

#[derive(Serialize, Deserialize)]
pub struct DBentry {
    login: String,
    password: String,
    url: String,
}

fn read_file(path_to_file: String) -> Vec<u8> {
    read(&path_to_file).unwrap_or_else(|e| {
        eprintln!("Error while reading file {}: {}", path_to_file, e);
        Vec::new()
    })
}

fn write_file(path_to_file: String, data: Vec<u8>) {
    write(&path_to_file, data).unwrap_or_else(|e| {
        eprintln!("Error while writing file {}: {}", path_to_file, e);
    })
}

fn read_entries(data: Vec<u8>) -> Vec<DBentry>{
    serde_json::from_slice::<Vec<DBentry>>(&data)
        .unwrap_or_else(|e| {
            eprintln!("Error JSON deserialization: {}", e);
            Vec::new()
        })
}

fn entries_to_bytes(data: Vec<DBentry>) -> Vec<u8> {
    serde_json::to_vec(&data).unwrap_or_else(|e| {
        eprintln!("Error JSON serialization: {}", e);
        Vec::new()
    })
}

pub fn read_db(path_to_file: String, key: String) -> Vec<DBentry> {
    let ciphertext = read_file(path_to_file);
    let seed = &kdf_gostr3411_2012_256(key.as_bytes(), b"Seed generation", &[0x00, 0xff, 0x00, 0xa1, 0xbc]);
    let iv = kdf_gostr3411_2012_256(key.as_bytes(), b"IV generation", &[0x11, 0xaa, 0x00, 0x1a, 0xff]).to_vec();
    let kuznechik = Kuznechik::new_from_kdf(key.as_bytes(), String::from("Database key").as_bytes(), seed);
    let plaintext = kuznechik.decrypt_cbc(ciphertext, iv).concat();
    read_entries(plaintext)
}

pub fn write_db(path_to_file: String, key: String, db_data: Vec<DBentry>) {
    let seed = &kdf_gostr3411_2012_256(key.as_bytes(), b"Seed generation", &[0x00, 0xff, 0x00, 0xa1, 0xbc]);
    let iv = kdf_gostr3411_2012_256(key.as_bytes(), b"IV generation", &[0x11, 0xaa, 0x00, 0x1a, 0xff]).to_vec();
    let kuznechik = Kuznechik::new_from_kdf(key.as_bytes(), String::from("Database key").as_bytes(), seed);
    let plaintext = entries_to_bytes(db_data);
    let ciphertext = kuznechik.encrypt_cbc(plaintext, iv).concat();
    write_file(path_to_file, ciphertext);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_operations_test() {
        let entries = vec![
        DBentry {
            login: "user1".to_string(),
            password: "pass1".to_string(),
            url: "url1".to_string(),
        },
        DBentry {
            login: "user2".to_string(),
            password: "pass2".to_string(),
            url: "url2".to_string(),
        },
        ];

        let new_entry = DBentry {
            login: "user3".to_string(),
            password: "pass3".to_string(),
            url: "url3".to_string(),
        };
        println!("Origin db = {}", serde_json::to_string(&entries).unwrap());
        write_db("/home/user/Tests/test.db".to_string(), "12345".to_string(), entries);
        let mut new_entries = read_db("/home/user/Tests/test.db".to_string(),"12345".to_string() );
        println!("Write db, than read it = {}", serde_json::to_string(&new_entries).unwrap());
        new_entries.push(new_entry);
        write_db("/home/user/Tests/test.db".to_string(), "12345".to_string(), new_entries);
        let check_entries = read_db("/home/user/Tests/test.db".to_string(),"12345".to_string() );
        println!("Added 1 new entry, than write it and read it = {}", serde_json::to_string(&check_entries).unwrap());

    }
}