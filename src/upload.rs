use std::path::Path;

use serde::Deserialize;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

use crate::error::Result;

#[derive(Debug)]
struct Uploader {
    url: String,
    client: reqwest::Client,
}

pub async fn push(url: &str, path: impl AsRef<Path>) -> Result<Response> {
    let upload = upload_file(url, path).await?;
    Ok(upload)
}

#[derive(Debug, Clone, Deserialize)]
pub struct Response {
    pub id: String,
    pub permalink: String,
}

async fn read_file(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let mut file = File::open(path).await?;
    let mut file_buf = Vec::new();
    file.read_to_end(&mut file_buf).await?;
    Ok(file_buf)
}

async fn upload_file(url: &str, path: impl AsRef<Path>) -> Result<Response> {
    let path = path.as_ref();
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap();
    let file = read_file(path).await?;

    let part = reqwest::multipart::Part::bytes(file)
        .file_name(file_name)
        .mime_str("application/octet-stream")?;

    let form = reqwest::multipart::Form::new()
        .text("json", "1")
        .part("file", part);

    let url = format!("{}uploadContent", url);

    let res = reqwest::Client::new()
        .post(&url)
        .multipart(form)
        .send()
        .await?
        .json()
        .await?;

    Ok(res)
}
