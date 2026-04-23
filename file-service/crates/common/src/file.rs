use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::io::AsyncReadExt;

#[derive(Serialize, Deserialize, Default)]
pub struct FileInfo {
    pub size: u64,
    pub name: String,
    pub path: String,
    pub is_dir: bool,
}

pub const FILE_BLOCK_1M: usize = 1024 * 1024;
pub const FLUSH_TIME: u64 = 100;

impl FileInfo {
    pub async fn new(path: &std::path::Path) -> Result<Self, std::io::Error> {
        let name = get_file_name(path)?;
        let is_dir = path.is_dir();
        let mut size = tokio::fs::metadata(&path).await?.len();
        if is_dir {
            size = 0;
        }
        Ok(FileInfo {
            size,
            name,
            path: path_to_string(path)?,
            is_dir,
        })
    }
}

pub fn path_to_string(file: &Path) -> Result<String, std::io::Error> {
    Ok(file
        .as_os_str()
        .to_str()
        .ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "file name is incorrect",
        ))?
        .to_string())
}

pub fn get_file_name(file: &Path) -> Result<String, std::io::Error> {
    let file_name = file
        .file_name()
        .ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "file name is incorrect",
        ))?
        .to_str()
        .ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "file name contains non-UTF-8 charactors",
        ))?
        .to_string();
    Ok(file_name)
}

pub fn get_file_parent(file: &Path) -> Result<String, std::io::Error> {
    let file_parent = file
        .parent()
        .ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "file name is incorrect",
        ))?
        .as_os_str()
        .to_str()
        .ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "file name contains non-UTF-8 charactors",
        ))?
        .to_string();
    Ok(file_parent)
}

pub async fn read_file_content(
    file_name: String,
) -> Result<tokio::sync::mpsc::Receiver<std::result::Result<Vec<u8>, std::io::Error>>, std::io::Error>
{
    let file = std::path::Path::new(&file_name);
    if file.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::IsADirectory,
            "file if dir, cannot download",
        ));
    }
    let mut f = tokio::fs::OpenOptions::new()
        .read(true)
        .open(file)
        .await
        .unwrap();
    let (sender, receiver) = tokio::sync::mpsc::channel::<Result<Vec<u8>, std::io::Error>>(2);
    tokio::spawn(async move {
        loop {
            let mut content: Vec<u8> = Vec::with_capacity(FILE_BLOCK_1M);
            if let Ok(lens) = f.read_buf(&mut content).await {
                if lens == 0 {
                    break; //EOF
                }
                match sender
                    .send(Ok(content))
                    .await
                    .map_err(std::io::Error::other)
                {
                    Ok(_) => {}
                    Err(e) => {
                        sender.send(Err(e)).await.unwrap_or_default();
                        // return Err(std::io::Error::other(error));
                    }
                }
            } else {
                sender
                    .send(Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("read {:?} got error", file_name),
                    )))
                    .await
                    .unwrap_or_default();
                break;
            }
        }
        //Ok(())
    });
    Ok(receiver)
}
