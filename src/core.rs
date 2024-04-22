use crate::{
    cache::LogCacher,
    error::{Error, Result},
    filter::{self, Filter},
    log::Log,
    message,
    parse::{self, Encounter},
    sender::{self, Sender, Webhook},
    target::Target,
    upload,
    watcher::{Event, Watcher},
};

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

use tokio::sync::Mutex;

pub async fn links<P, W, C>(log_dir: P, url: &str, writer: W, cache: C) -> Result<()>
where
    P: AsRef<Path>,
    W: Write + Send + Sync,
    C: LogCacher + Sync,
{
    let msg_gen = message::TextGenerator {};
    let sender = sender::Write::new(writer);
    let filter = filter::Length {};
    let mut snek = Snek::new(&log_dir, url, sender, cache, msg_gen, filter);
    snek.upload_dailies_and_send().await
}

pub async fn daily<P, C>(log_dir: P, url: &str, sender: Webhook, cache: C) -> Result<()>
where
    P: AsRef<Path>,
    C: LogCacher + Sync,
{
    let msg_gen = message::WebhookGenerator {};
    let filter = filter::Length {};
    let mut snek = Snek::new(&log_dir, url, sender, cache, msg_gen, filter);
    snek.upload_dailies_and_send().await
}

pub async fn watch<P, C>(log_dir: P, url: &str, sender: Webhook, cache: C) -> Result<()>
where
    P: AsRef<Path>,
    C: LogCacher + Clone + Sync + Send + 'static,
{
    let msg_gen = message::WebhookGenerator {};
    let filter = filter::Length {};
    let snek = Snek::new(&log_dir, url, sender, cache, msg_gen, filter);
    snek.watch_dir().await
}

struct Snek<'a, C, S, M, F>
where
    C: LogCacher,
    S: Sender,
    F: Filter,
    M: message::Generator,
{
    log_dir: &'a Path,
    url: &'a str,
    sender: S,
    cache: C,
    msg_gen: M,
    filter: F,
}

impl<'a, C, S, M, F> Snek<'a, C, S, M, F>
where
    C: LogCacher + Sync,
    S: Sender,
    F: Filter,
    M: message::Generator,
    <M as message::Generator>::Message: Sync,
{
    fn new(
        log_dir: &'a impl AsRef<Path>,
        url: &'a str,
        sender: S,
        cache: C,
        msg_gen: M,
        filter: F,
    ) -> Self {
        let log_dir = log_dir.as_ref();
        Self {
            log_dir,
            url,
            sender,
            cache,
            msg_gen,
            filter,
        }
    }

    async fn upload_dailies_and_send(&mut self) -> Result<()> {
        use Target::*;

        let targets = vec![Skor, Arts, Arkk, Mama, Siax, Enso];
        let uploaded_logs = self.upload_recent_logs(&targets).await;
        let mut log_infos: Vec<LogInfo> = uploaded_logs
            .iter()
            .map(|uploaded_log| {
                let (encounters, _) = parse::parse(&uploaded_log.log).unwrap();
                encounters
                    .into_iter()
                    .map(|e| LogInfo::new(uploaded_log, e))
                    .collect::<Vec<LogInfo>>()
            })
            .flatten()
            .collect();
        log_infos.sort_by(|a, b| a.encounter.target.cmp(&b.encounter.target));
        let msg = self.msg_gen.generate(&log_infos);
        self.sender.send(&msg).await
    }

    async fn upload_recent_logs(&self, targets: &[Target]) -> Vec<UploadedLog> {
        use futures::{future, stream, StreamExt as _, TryStreamExt as _};

        stream::iter(targets.iter())
            .map(|&target| self.upload_recent_log(target))
            .buffer_unordered(targets.len())
            .map_err(|e| log::warn!("failed to upload log: {}", e))
            .filter_map(|res| future::ready(res.ok()))
            .collect()
            .await
    }

    async fn upload_recent_log(&self, target: Target) -> Result<UploadedLog> {
        let log = find_recent_log(self.log_dir, target).await?;
        self.upload_log(log).await
    }

    async fn upload_log(&self, log: Log) -> Result<UploadedLog> {
        if let Some(link) = self.cache.get(&log) {
            log::info!("`{}` found in cache", log);
            return Ok(UploadedLog::new(log, link));
        }

        log::info!("uploading log: {}", log);
        let res = upload::push(self.url, log.path()).await?;
        let uploaded_log = UploadedLog::new(log, res.permalink);
        self.cache.insert(&uploaded_log);
        Ok(uploaded_log)
    }
}

impl<'a, C, S, M, F> Snek<'a, C, S, M, F>
where
    C: LogCacher + Clone + Sync + Send + 'static,
    S: Sender + Sync + Send + 'static,
    F: Filter + Sync + Send + 'static,
    M: message::Generator + Sync + Send + 'static,
    <M as message::Generator>::Message: Sync + Send,
{
    async fn watch_dir(self) -> Result<()> {
        let watcher = Watcher::watch(self.log_dir);
        let sender = Arc::new(Mutex::new(self.sender));
        let msg_gen = Arc::new(self.msg_gen);
        let filter = Arc::new(self.filter);

        log::info!("started watching log folder");
        while let Event::File(path) = watcher.recv() {
            let sender = Arc::clone(&sender);
            let msg_gen = Arc::clone(&msg_gen);
            let filter = Arc::clone(&filter);
            let url = self.url.to_owned();
            let cache = self.cache.clone();

            if let Some(log) = Log::from_file_checked(&path) {
                tokio::spawn(async move {
                    Self::handle_incoming_log(sender, &url, &cache, log, msg_gen, filter)
                        .await
                        .unwrap()
                });
            }
        }

        log::info!("stopped watching");
        Ok(())
    }

    async fn handle_incoming_log(
        sender: Arc<Mutex<S>>,
        url: &str,
        cache: &C,
        log: Log,
        msg_gen: Arc<M>,
        filter: Arc<F>,
    ) -> Result<()> {
        let (mut encounters, _) = match parse::parse(&log) {
            Some(e) => e,
            None => {
                log::trace!("incoming log is from an unsupported encounter");
                return Ok(());
            }
        };

        if !filter.filter(&encounters.first().unwrap()) {
            log::trace!("incoming log filtered out");
            return Ok(());
        }

        log::info!("uploading log: {}", &log);
        let response = upload::push(url, log.path()).await?;
        let uploaded_log = UploadedLog::new(log, response.permalink);

        cache.insert(&uploaded_log);

        if encounters.len() == 1 {
            let log_info = LogInfo::new(&uploaded_log, encounters.remove(0));
            let msg = msg_gen.generate(&[log_info]);
            sender.lock().await.send(msg).await?;
        } else {
            use futures::{future, stream, StreamExt as _, TryStreamExt as _};
            stream::iter(encounters.iter().cloned())
                .map(|encounter| {
                    let msg_gen = msg_gen.clone();
                    let sender = sender.clone();
                    let uploaded_log = uploaded_log.clone();

                    async move {
                        let log_info = LogInfo::new(&uploaded_log, encounter);
                        let msg = msg_gen.generate(&[log_info]);
                        sender.lock().await.send(msg).await?;
                        Ok::<(), Error>(())
                    }
                })
                .buffer_unordered(encounters.len())
                .map_err(|e| log::warn!("failed to send webhook msg: {}", e))
                .filter_map(|res| future::ready(res.ok()))
                .collect::<Vec<_>>()
                .await;
        }

        Ok(())
    }
}

async fn find_recent_log(log_dir: impl AsRef<Path>, target: Target) -> Result<Log> {
    log::trace!("finding most recent `{}` log", target);
    let dir_path: PathBuf = log_dir.as_ref().join(target.dir_name());
    let mut newest_log = PathBuf::new();
    visit_dir(dir_path, &mut newest_log)?;
    Log::from_file_checked(&newest_log).ok_or(Error::NoRecentLog)
}

fn visit_dir(path: PathBuf, current: &mut PathBuf) -> Result<()> {
    let dir = fs::read_dir(path)?;

    for f in dir {
        let f = f?;
        let path = f.path();
        let current_stem = current
            .file_stem()
            .unwrap_or_else(|| std::ffi::OsStr::new(""));
        if f.file_type()?.is_file() {
            let stem = path.file_stem().expect("found file with no name");
            if current_stem.cmp(stem) == std::cmp::Ordering::Less {
                *current = path;
            }
        } else {
            visit_dir(path, current)?;
        }
    }

    Ok(())
}

#[derive(Clone)]
pub struct UploadedLog {
    pub log: Log,
    pub link: String,
}

impl UploadedLog {
    pub const fn new(log: Log, link: String) -> Self {
        Self { log, link }
    }
}

pub struct LogInfo<'a> {
    pub log: &'a UploadedLog,
    pub encounter: Encounter,
}

impl<'a> LogInfo<'a> {
    pub const fn new(log: &'a UploadedLog, encounter: Encounter) -> Self {
        Self { log, encounter }
    }
}

pub fn get_log_dir() -> Result<std::path::PathBuf> {
    // try default location
    if let Some(documents) = dirs::document_dir() {
        let to_logs = "Guild Wars 2/addons/arcdps/arcdps.cbtlogs";
        let log_path = documents.join(to_logs);
        if log_path.is_dir() {
            return Ok(log_path);
        }
    }

    log::info!("no log directory in default location: checking `logdir.txt` instead");
    fs::read_to_string("logdir.txt")
        .map(|s| s.trim().into())
        .map_err(|_| Error::LogDirectory)
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "golem")]
    #[tokio::test]
    async fn watch_golem() {
        use crate::get_log_dir;
        use crate::log::Log;
        use crate::watcher::Event;
        use crate::watcher::Watcher;

        let log_dir = get_log_dir().unwrap();
        let watcher = Watcher::watch(log_dir);
        println!("watching..");

        while let Event::File(path) = watcher.recv() {
            if let Some(log) = Log::from_file_checked(&path) {
                println!("got a log..");
                crate::golem::owp_testing(&log);
            }
        }
    }

    #[tokio::test]
    async fn find_recent_log() {
        use crate::target::Target;

        let data = vec![
            (Target::Arts, "20200407-174541"),
            (Target::Enso, "20200407-133033"),
            (Target::Arkk, "20200408-233716"),
        ];

        for (target, expected) in data.into_iter() {
            let recent_log = super::find_recent_log("tests/example_logs", target)
                .await
                .unwrap();
            assert_eq!(expected, recent_log.path().file_stem().unwrap());
        }
    }
}
