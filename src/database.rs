use libgost_rs::Kuznechik;
use libgost_rs::kdf_gostr3411_2012_256;
use serde::{Deserialize, Serialize};
use std::fs::{read, write};

// Константа для магического числа (незашифрованная часть)
const MAGIC_HEADER: &[u8] = b"GOSTDB";
// Версия формата
const FORMAT_VERSION: u8 = 1;

#[derive(Serialize, Deserialize, Clone)]
pub struct DBentry {
    pub login: String,
    pub password: String,
    pub url: String,
}

fn write_file(path_to_file: String, data: Vec<u8>) {
    write(&path_to_file, data).unwrap_or_else(|e| {
        eprintln!("Error while writing file {}: {}", path_to_file, e);
    })
}

fn read_entries(data: Vec<u8>) -> Vec<DBentry>{
     if data.is_empty() {
        return Vec::new();
    }
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

pub fn new_db(path_to_file: String, key: String) {
    let seed = &kdf_gostr3411_2012_256(key.as_bytes(), b"Seed generation", &[0x00, 0xff, 0x00, 0xa1, 0xbc]);
    let iv = kdf_gostr3411_2012_256(key.as_bytes(), b"IV generation", &[0x11, 0xaa, 0x00, 0x1a, 0xff]).to_vec();
    let kuznechik = Kuznechik::new_from_kdf(key.as_bytes(), String::from("Database key").as_bytes(), seed);
    
    // Создаем заголовок для шифрования
    let header = vec![FORMAT_VERSION];
    let encrypted_header = kuznechik.encrypt_cbc(header, iv.clone()).concat();
    
    let empty_data = Vec::new();
    let ciphertext = kuznechik.encrypt_cbc(empty_data, iv).concat();
    
    let mut file_data = MAGIC_HEADER.to_vec();
    file_data.extend(encrypted_header);
    file_data.extend(ciphertext);
    
    write_file(path_to_file, file_data);
}

pub fn read_db(path_to_file: String, key: String) -> Result<Vec<DBentry>, DbError> {
    let file_content = read(&path_to_file)
        .map_err(|_| DbError::FileReadError)?;
    
    // Проверяем магическое число (незашифрованная часть)
    if file_content.len() < MAGIC_HEADER.len() || &file_content[..MAGIC_HEADER.len()] != MAGIC_HEADER
    {
        return Err(DbError::InvalidHeader);
    }
    
    let seed = &kdf_gostr3411_2012_256(key.as_bytes(), b"Seed generation", &[0x00, 0xff, 0x00, 0xa1, 0xbc]);
    let iv = kdf_gostr3411_2012_256(key.as_bytes(), b"IV generation", &[0x11, 0xaa, 0x00, 0x1a, 0xff]).to_vec();
    let kuznechik = Kuznechik::new_from_kdf(key.as_bytes(), String::from("Database key").as_bytes(), seed);
    
    // Извлекаем и расшифровываем заголовок
    let header_start = MAGIC_HEADER.len();
    let header_end = header_start + 16; // Kuznechik encrypt_cbc возвращает блоки по 16 байт
    
    if file_content.len() < header_end {
        return Err(DbError::InvalidHeader);
    }
    
    let encrypted_header = file_content[header_start..header_end].to_vec();
    let header_bytes = kuznechik.decrypt_cbc(encrypted_header, iv.clone()).concat();
    
    // Проверяем версию формата
    if header_bytes.is_empty() || header_bytes[0] != FORMAT_VERSION {
        return Err(DbError::InvalidHeader);
    }
    
    // Извлекаем и расшифровываем данные
    let ciphertext = file_content[header_end..].to_vec();
    let plaintext = kuznechik.decrypt_cbc(ciphertext, iv).concat();
    
    Ok(read_entries(plaintext))
}

pub fn write_db(path_to_file: String, key: String, db_data: Vec<DBentry>) {
    let seed = &kdf_gostr3411_2012_256(key.as_bytes(), b"Seed generation", &[0x00, 0xff, 0x00, 0xa1, 0xbc]);
    let iv = kdf_gostr3411_2012_256(key.as_bytes(), b"IV generation", &[0x11, 0xaa, 0x00, 0x1a, 0xff]).to_vec();
    let kuznechik = Kuznechik::new_from_kdf(key.as_bytes(), String::from("Database key").as_bytes(), seed);
    
    // Создаем и шифруем заголовок
    let header = vec![FORMAT_VERSION];
    let encrypted_header = kuznechik.encrypt_cbc(header, iv.clone()).concat();
    
    // Шифруем данные
    let plaintext = entries_to_bytes(db_data);
    let ciphertext = kuznechik.encrypt_cbc(plaintext, iv).concat();
    
    // Формируем файл
    let mut file_data = MAGIC_HEADER.to_vec();
    file_data.extend(encrypted_header);
    file_data.extend(ciphertext);
    
    write_file(path_to_file, file_data);
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum DbError {
    FileReadError,
    InvalidHeader,
    DecryptionError,
    JsonError,
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
        new_db("/home/user/Tests/test.db".to_string(), "12345".to_string());
        let check_empty_db = read_db("/home/user/Tests/test.db".to_string(),"12345".to_string() );
        println!("Check empty db = {}", serde_json::to_string(&check_empty_db).unwrap());
        println!("Origin db = {}", serde_json::to_string(&entries).unwrap());
        write_db("/home/user/Tests/test.db".to_string(), "12345".to_string(), entries);
        let mut new_entries = read_db("/home/user/Tests/test.db".to_string(),"12345".to_string() ).unwrap();
        println!("Write db, than read it = {}", serde_json::to_string(&new_entries).unwrap());
        new_entries.push(new_entry);
        write_db("/home/user/Tests/test.db".to_string(), "12345".to_string(), new_entries);
        let check_entries = read_db("/home/user/Tests/test.db".to_string(),"12345".to_string() );
        println!("Added 1 new entry, than write it and read it = {}", serde_json::to_string(&check_entries).unwrap());

    }
}