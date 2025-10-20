use std::{fs::File, path::Path};
use search_master_interface::{
    SearchableDocument, SearchableRoot, send_new_searchable_document,
};
use tracing::error;

use crate::{RUYI_FILES, extract_text_docx, extract_text_pdf};

pub fn initialize() {
    for result in walkdir::WalkDir::new(RUYI_FILES) {
        let entry = result.unwrap();
        let path = entry.path();
        if path.is_file() {
            feed_file(&path);
        }
    }
}

pub(crate) fn feed_file(path: &Path) {
    let Some(contents) = extract_contents(path) else {
        return;
    };
    send_new_searchable_document(SearchableDocument::new(
        path.file_name()
            .expect("Expected filename to exist")
            .to_str()
            .expect("Expected ascii filename")
            .to_string(),
        SearchableRoot::new_file(path, RUYI_FILES.as_ref()),
        contents,
    ));
}

pub fn extract_contents(path: &Path) -> Option<String> {
    let contents;
    match path.extension().map(|x| x.to_str()).flatten() {
        Some("txt" | "md" | "rs" | "py" | "toml" | "json" | "cpp" | "bazel" | "ron" | "xml" | "h" | "jsonc" | "hpp" | "sql" | "c") => {
            contents = match std::fs::read_to_string(path) {
                Ok(x) => x,
                Err(e) => {
                    error!("Failed to read: {path:?}: {e}");
                    return None;
                }
            };
        }
        Some("pdf") => {
            let bytes = match std::fs::read(path) {
                Ok(x) => x,
                Err(e) => {
                    error!("Failed to read: {path:?}: {e}");
                    return None;
                }
            };
            contents = match extract_text_pdf(&bytes) {
                Ok(x) => x,
                Err(e) => {
                    error!("Failed to read: {path:?}: {e}");
                    return None;
                }
            };
        }
        Some("docx") => {
            let docx = match File::open(path) {
                Ok(x) => x,
                Err(e) => {
                    error!("Failed to read: {path:?}: {e}");
                    return None;
                }
            };
            contents = match extract_text_docx(docx) {
                Ok(x) => x,
                Err(e) => {
                    error!("Failed to read: {path:?}: {e}");
                    return None;
                }
            };
        }
        Some("gitignore" | "lock" | "obj" | "mtl" | "png" | "a" | "stl") | None => {
            return None;
        }
        Some(ext) => {
            error!("Unknown file extension {ext} for {path:?}");
            return None;
        }
    }
    Some(contents)
}
