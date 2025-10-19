use std::{
    fmt::Display,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

static NEW_SEARCHABLE_DOCUMENT: OnceLock<UnboundedSender<SearchableDocument>> = OnceLock::new();
static NEW_SEARCHABLE_MESSAGE: OnceLock<UnboundedSender<SearchableMessage>> = OnceLock::new();
static INVALIDATED_MESSAGE_AUTHOR_ID: OnceLock<UnboundedSender<u64>> = OnceLock::new();
static INVALIDATED_SEARCHABLE_DOCUMENT: OnceLock<UnboundedSender<InvalidatedSearchableDocument>> =
    OnceLock::new();

pub struct InvalidatedSearchableDocument(pub SearchableDocumentId);

pub fn invalidate_searchable_document(id: SearchableDocumentId) {
    let _ = INVALIDATED_SEARCHABLE_DOCUMENT
        .get()
        .unwrap()
        .send(InvalidatedSearchableDocument(id));
}

pub fn init_documents_senders() -> (
    UnboundedReceiver<SearchableDocument>,
    UnboundedReceiver<InvalidatedSearchableDocument>,
    UnboundedReceiver<SearchableMessage>,
    UnboundedReceiver<u64>,
) {
    let (tx, rx1) = tokio::sync::mpsc::unbounded_channel();
    NEW_SEARCHABLE_DOCUMENT.set(tx).unwrap();
    let (tx, rx2) = tokio::sync::mpsc::unbounded_channel();
    INVALIDATED_SEARCHABLE_DOCUMENT.set(tx).unwrap();
    let (tx, rx3) = tokio::sync::mpsc::unbounded_channel();
    NEW_SEARCHABLE_MESSAGE.set(tx).unwrap();
    let (tx, rx4) = tokio::sync::mpsc::unbounded_channel();
    INVALIDATED_MESSAGE_AUTHOR_ID.set(tx).unwrap();
    (rx1, rx2, rx3, rx4)
}

pub fn send_new_searchable_document(doc: SearchableDocument) {
    let _ = NEW_SEARCHABLE_DOCUMENT.get().unwrap().send(doc);
}

pub fn send_new_searchable_message(msg: SearchableMessage) {
    let _ = NEW_SEARCHABLE_MESSAGE.get().unwrap().send(msg);
}
pub fn invalidate_message_author_id(id: u64) {
    let _ = INVALIDATED_MESSAGE_AUTHOR_ID.get().unwrap().send(id);
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
    root: String,
}

impl SearchableDocument {
    pub fn new(filename: String, root: SearchableRoot, contents: String) -> Self {
        Self {
            id: SearchableDocumentId::new(&filename, &root),
            filename,
            contents,
            root: root.to_string(),
        }
    }

    pub fn filename(&self) -> &str {
        &self.filename
    }

    pub fn root(&self) -> SearchableRoot {
        if let Some(path) = self.root.strip_prefix("root://") {
            SearchableRoot::Files {
                folder_path: path.into(),
            }
        } else {
            let root = self.root.strip_prefix("gdrive://").unwrap();
            let (drive_name, path) = root.split_at(root.find('/').unwrap());
            SearchableRoot::GoogleDrive {
                drive_name: drive_name.into(),
                folder_path: path.into(),
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchableRoot {
    Files {
        folder_path: PathBuf,
    },
    GitHub {
        origin: String,
        branch: String,
        folder_path: PathBuf,
    },
    GoogleDrive {
        drive_name: String,
        folder_path: PathBuf,
    },
}

impl SearchableRoot {
    pub fn new_file(path: &Path, base: &Path) -> Self {
        Self::Files {
            folder_path: path
                .canonicalize()
                .unwrap()
                .parent()
                .unwrap()
                .strip_prefix(base.canonicalize().unwrap())
                .unwrap()
                .into(),
        }
    }
}

impl Display for SearchableRoot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        macro_rules! folder_path {
            ($folder_path: ident) => {
                $folder_path.to_string_lossy().trim_end_matches('/')
            };
        }
        match self {
            SearchableRoot::GitHub {
                origin,
                folder_path,
            } => write!(
                f,
                "{}",
                format!("file://{}/", folder_path!(folder_path)).replace("///", "//")
            ),
            SearchableRoot::Files { folder_path } => write!(
                f,
                "{}",
                format!("file://{}/", folder_path!(folder_path)).replace("///", "//")
            ),
            SearchableRoot::GoogleDrive {
                drive_name,
                folder_path,
            } => write!(
                f,
                "{}",
                format!("gdrive://{drive_name}/{}/", folder_path!(folder_path))
                    .replace("///", "//")
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchableMessage {
    id: String,
    author_id: u64,
    message: String,
}

impl SearchableMessage {
    pub fn new(author_id: u64, message: String) -> Self {
        let mut digest = Sha256::new();
        digest.update(author_id.to_ne_bytes());
        digest.update(message.as_bytes());

        Self {
            id: hex::encode(digest.finalize()),
            author_id,
            message,
        }
    }

    pub fn author_id(&self) -> u64 {
        self.author_id
    }
}
