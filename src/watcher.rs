use std::path::{Path, PathBuf};

use crossbeam_channel::Receiver;
use notify::{RecommendedWatcher, RecursiveMode, Result};

pub enum Event {
    File(PathBuf),
    Stop,
}

pub struct Watcher {
    _raw: RecommendedWatcher,
    rx: Receiver<Event>,
}

impl Watcher {
    pub fn watch(dir: impl AsRef<Path>) -> Self {
        use notify::Watcher as _;

        let (tx, rx) = crossbeam_channel::unbounded();
        let handler_tx = tx.clone();
        let mut raw: RecommendedWatcher =
            notify::Watcher::new_immediate(move |res: Result<notify::Event>| {
                if let Some(path) = handle_event(res.unwrap()) {
                    tx.send(Event::File(path)).unwrap();
                }
            })
            .unwrap();

        ctrlc::set_handler(move || {
            handler_tx.send(Event::Stop).unwrap();
        })
        .unwrap();

        raw.watch(dir, RecursiveMode::Recursive).unwrap();
        Self { _raw: raw, rx }
    }

    pub fn recv(&self) -> Event {
        self.rx.recv().unwrap()
    }
}

fn handle_event(event: notify::Event) -> Option<PathBuf> {
    use notify::event::EventKind::*;
    use notify::event::ModifyKind::*;
    use notify::event::RenameMode::*;

    match event.kind {
        Create(_) | Modify(Name(To)) => {
            if event.paths.len() != 1 {
                log::warn!("received event with {} files", event.paths.len());
                return None;
            }

            Some(event.paths[0].clone())
        }
        _ => None,
    }
}
