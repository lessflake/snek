use crate::{core::UploadedLog, error::Result, log as logg, target::Target};

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use serde::{de::DeserializeOwned, Serialize};
use tokio::fs;

pub struct Cache<K, V>
where
    K: Serialize + DeserializeOwned + std::cmp::Eq + std::hash::Hash,
    V: Serialize + DeserializeOwned,
{
    path: PathBuf,
    map: HashMap<K, V>,
    modified: bool,
}

impl<K, V> Cache<K, V>
where
    K: Serialize + DeserializeOwned + std::cmp::Eq + std::hash::Hash,
    V: Serialize + DeserializeOwned,
{
    pub async fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_owned();
        let map: HashMap<K, V> = if path.is_file() {
            let file = fs::File::open(&path).await?.into_std().await;
            bincode::deserialize_from(file)?
        } else {
            HashMap::new()
        };
        let modified = false;

        Ok(Self {
            path,
            map,
            modified,
        })
    }

    pub fn new_blocking(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_owned();
        let map: HashMap<K, V> = if path.is_file() {
            let file = std::fs::File::open(&path)?;
            bincode::deserialize_from(file)?
        } else {
            HashMap::new()
        };
        let modified = false;

        Ok(Self {
            path,
            map,
            modified,
        })
    }

    fn save_blocking(&self) -> Result<()> {
        let file = std::fs::File::create(&self.path)?;
        bincode::serialize_into(file, &self.map)?;
        Ok(())
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.modified = true;
        self.map.insert(key, value)
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key)
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.map.remove(key).map(|v| {
            self.modified = true;
            v
        })
    }

    pub fn raw(&self) -> &HashMap<K, V> {
        &self.map
    }
}

impl<K, V> Drop for Cache<K, V>
where
    K: Serialize + DeserializeOwned + std::cmp::Eq + std::hash::Hash,
    V: Serialize + DeserializeOwned,
{
    fn drop(&mut self) {
        if self.modified {
            self.save_blocking().unwrap();
            if let Some(name) = self.path.file_stem() {
                log::trace!("saved {}", name.to_string_lossy());
            }
        }
    }
}

pub trait LogCacher {
    fn insert(&self, log: &UploadedLog);
    fn get(&self, log: &logg::Log) -> Option<String>;
}

#[derive(Clone)]
pub struct Nop {}

impl LogCacher for Nop {
    fn insert(&self, _: &UploadedLog) {}
    fn get(&self, _: &logg::Log) -> Option<String> {
        None
    }
}

pub struct Log {
    cache: Arc<Mutex<Cache<Target, String>>>,
}

impl Clone for Log {
    fn clone(&self) -> Self {
        Self {
            cache: Arc::clone(&self.cache),
        }
    }
}

impl Log {
    pub async fn new(name: impl AsRef<Path>) -> Result<Self> {
        let backing_cache = Arc::new(Mutex::new(Cache::new(name).await?));
        Ok(Self {
            cache: backing_cache,
        })
    }
}

impl LogCacher for Log {
    fn insert(&self, log: &UploadedLog) {
        self.cache
            .lock()
            .unwrap()
            .insert(log.log.target(), log.link.clone());
    }

    fn get(&self, log: &logg::Log) -> Option<String> {
        if let Some(link) = self.cache.lock().unwrap().get(&log.target()) {
            if log.same_as(link) {
                return Some(link.clone());
            }
        }
        None
    }
}
