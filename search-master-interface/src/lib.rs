use std::{fmt::Display, path::PathBuf, sync::{OnceLock}};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver};

static NEW_SEARCHABLE_DOCUMENT: OnceLock<UnboundedSender<SearchableDocument>> = OnceLock::new();
static INVALIDATED_SEARCHABLE_DOCUMENT: OnceLock<UnboundedSender<InvalidatedSearchableDocument>> = OnceLock::new();

pub struct InvalidatedSearchableDocument(pub SearchableDocumentId);

pub fn invalidate_searchable_document(id: SearchableDocumentId) {
    let _ = INVALIDATED_SEARCHABLE_DOCUMENT.get().unwrap().send(InvalidatedSearchableDocument(id));
}

pub fn init_documents_senders() -> (UnboundedReceiver<SearchableDocument>, UnboundedReceiver<InvalidatedSearchableDocument>) {
    let (tx, rx1) = tokio::sync::mpsc::unbounded_channel();
    NEW_SEARCHABLE_DOCUMENT.set(tx).unwrap();
    let (tx, rx2) = tokio::sync::mpsc::unbounded_channel();
    INVALIDATED_SEARCHABLE_DOCUMENT.set(tx).unwrap();
    (rx1, rx2)
}

pub fn send_new_searchable_document(doc: SearchableDocument) {
    let _ = NEW_SEARCHABLE_DOCUMENT.get().unwrap().send(doc);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SearchableDocumentId(String);

impl SearchableDocumentId {
    pub fn new(filename: &str, root: &SearchableRoot) -> Self {
        Self(hex::encode(Sha256::digest(format!("{root}{filename}"))))
    }
}

impl Display for SearchableDocumentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchableDocument {
    id: SearchableDocumentId,
    filename: String,
    contents: String,
    root: String
}


impl SearchableDocument {
    pub fn new(filename: String, root: SearchableRoot, contents: String) -> Self {
        Self {
            id: SearchableDocumentId::new(&filename, &root),
            filename,
            contents,
            root: root.to_string()
        }
    }
    
    pub fn filename(&self) -> &str {
        &self.filename
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchableRoot {
    Files,
    GoogleDrive {
        drive_name: String,
        folder_path: PathBuf
    }
}

impl Display for SearchableRoot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchableRoot::Files => write!(f, "file://"),
            SearchableRoot::GoogleDrive { drive_name, folder_path } => write!(f, "gdrive://{drive_name}/{}/", folder_path.to_string_lossy().trim_end_matches('/')),
        }
    }
}
