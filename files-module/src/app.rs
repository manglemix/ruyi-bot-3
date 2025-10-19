use std::{fs::File, path::Path};

use notify::{event::{CreateKind, ModifyKind, RemoveKind, RenameMode}, Event, EventKind, RecursiveMode, Watcher};
use search_master_interface::{invalidate_searchable_document, send_new_searchable_document, SearchableDocument, SearchableDocumentId, SearchableRoot};
use tracing::{debug, error, info};

use crate::{extract_text_docx, extract_text_pdf};

pub fn start_watcher() -> notify::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel::<notify::Result<Event>>();

    let mut watcher = notify::recommended_watcher(tx).expect("Expected watcher to initialize");

    watcher.watch(Path::new("ruyi-files"), RecursiveMode::Recursive).expect("Expected ruyi-files to be watchable");

    if let Ok(entries) = std::fs::read_dir("ruyi-files") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                feed_file(&path);
            }
        }
    }

    std::thread::spawn(|| {
        let _watcher = watcher;
        info!("Files Watcher started");
        for res in rx {
            let event = match res {
                Ok(x) => x,
                Err(e) => {
                    error!("File watch error: {:?}", e);
                    continue;
                }
            };

            match event.kind {
                EventKind::Modify(ModifyKind::Name(RenameMode::From)) | EventKind::Remove(RemoveKind::File) => {
                    for path in event.paths {
                        if path.is_dir() {
                            debug!("Folder {path:?} was renamed/deleted");
                            continue;
                        }

                        let Some(filename) = path.file_name().map(|x| x.to_str()).flatten() else {
                            continue;
                        };
                        invalidate_searchable_document(SearchableDocumentId::new(filename, &SearchableRoot::Files));
                    }
                    continue;
                }
                EventKind::Create(CreateKind::File) | EventKind::Modify(_) => {}
                _ => continue
            }

            for path in event.paths {
                if path.is_dir() {
                    debug!("Folder {path:?} was touched");
                    continue;
                }

                feed_file(&path);
            }
        }
    });

    Ok(())
}

pub fn feed_file(path: &Path) {
    let contents;
    match path.extension().map(|x| x.to_str()).flatten() {
        Some("txt" | "md") => {
            contents = match std::fs::read_to_string(path) {
                Ok(x) => x,
                Err(e) => {
                    error!("Failed to read: {path:?}: {e}");
                    return;
                }
            };
        }
        Some("pdf") => {
            let bytes = match std::fs::read(path) {
                Ok(x) => x,
                Err(e) => {
                    error!("Failed to read: {path:?}: {e}");
                    return;
                }
            };
            contents = match extract_text_pdf(&bytes) {
                Ok(x) => x,
                Err(e) => {
                    error!("Failed to read: {path:?}: {e}");
                    return;
                }
            };
        }
        Some("docx") => {
            let docx = match File::open(path) {
                Ok(x) => x,
                Err(e) => {
                    error!("Failed to read: {path:?}: {e}");
                    return;
                }
            };
            contents = match extract_text_docx(docx) {
                Ok(x) => x,
                Err(e) => {
                    error!("Failed to read: {path:?}: {e}");
                    return;
                }
            };
        }
        Some(ext) => {
            error!("Unknown file extension {ext} for {path:?}");
            return;
        }
        None => {
            debug!("File {path:?} (no extension) was touched");
            return;
        }
    }
    send_new_searchable_document(
        SearchableDocument::new(
            path.file_name()
                .expect("Expected filename to exist")
                .to_str()
                .expect("Expected ascii filename")
                .to_string(),
            SearchableRoot::Files,
            contents
        )
    );
}