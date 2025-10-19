use std::path::Path;

use notify::{event::{CreateKind, RemoveKind}, Event, EventKind, RecursiveMode, Watcher};
use tracing::{debug, error};

pub fn start_watcher() -> notify::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel::<notify::Result<Event>>();

    let mut watcher = notify::recommended_watcher(tx)?;

    watcher.watch(Path::new("ruyi-files"), RecursiveMode::NonRecursive)?;

    std::thread::spawn(|| {
        for res in rx {
            let event = match res {
                Ok(x) => x,
                Err(e) => {
                    error!("File watch error: {:?}", e);
                    continue;
                }
            };

            match event.kind {
                EventKind::Create(CreateKind::File) | EventKind::Modify(_) | EventKind::Remove(RemoveKind::File) => {}
                _ => continue
            }

            for path in event.paths {
                if path.is_dir() {
                    debug!("Folder {path:?} was touched");
                    continue;
                }

                match path.extension().map(|x| x.to_str()).flatten() {
                    Some("txt" | "md") => {

                    }
                    Some("pdf") => {

                    }
                    Some("docx") => {

                    }
                    Some(ext) => {
                        error!("Unknown file extension {ext} for {path:?}");
                        continue;
                    }
                    None => debug!("File {path:?} (no extension) was touched")
                }
            }
        }
    });

    Ok(())
}