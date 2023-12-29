use std::fs::{create_dir_all, remove_file, write};
use std::io::Result;
use std::path::Path;

pub fn delete_file(files_dir: &str, filename: &String) -> Result<()> {
    let folder = Path::new(files_dir);
    let path = folder.join(filename);
    remove_file(path)
}

pub fn save_file(files_dir: &str, filename: &String, data: &Vec<u8>) -> Result<()> {
    let folder = Path::new(files_dir);
    create_dir_all(folder)?;
    let path = folder.join(filename);
    write(path, data)
}

pub fn get_files(files_dir: &str) -> Result<Vec<String>> {
    Path::new(files_dir).read_dir().map(|dir| {
        dir.flat_map(|res| res.map(|e| e.file_name().into_string()))
            .flatten()
            .collect::<Vec<String>>()
    })
}
