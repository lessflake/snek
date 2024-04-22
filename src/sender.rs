use crate::error::Result;

use std::fmt::Display;
use std::io;

use async_trait::async_trait;
use serde::Serialize;

#[async_trait]
pub trait Sender {
    async fn send<M>(&mut self, msg: M) -> Result<()>
    where
        M: Display + Serialize + Send + Sync;
}

#[derive(Debug)]
pub struct Webhook {
    url: String,
    client: reqwest::Client,
}

impl Webhook {
    pub fn new(url: &str) -> Self {
        let client = reqwest::Client::new();
        Self {
            url: url.to_string(),
            client,
        }
    }

    pub fn validate_url(url: &str) -> bool {
        const REFERENCE_URL: &str = "https://discordapp.com/api/webhooks/ABCDEFGHIJKLMNOPQR/ABCDEFGHIJKLMNOPQRSTUVWXYZ01234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ01234";
        url.len() == REFERENCE_URL.len() && url[0..35] == REFERENCE_URL[0..35]
    }
}

#[async_trait]
impl Sender for Webhook {
    async fn send<M>(&mut self, msg: M) -> Result<()>
    where
        M: Display + Serialize + Send + Sync,
    {
        log::info!("posting log to discord webhook");
        self.client.post(&self.url).json(&msg).send().await?;
        Ok(())
    }
}

pub struct Write<W: io::Write + Sync + Send> {
    writer: W,
}

impl<W: io::Write + Sync + Send> Write<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }
}

#[async_trait]
impl<W: io::Write + Send + Sync> Sender for Write<W> {
    async fn send<M>(&mut self, msg: M) -> Result<()>
    where
        M: Display + Serialize + Send + Sync,
    {
        writeln!(self.writer, "{}", msg)?;
        Ok(())
    }
}
