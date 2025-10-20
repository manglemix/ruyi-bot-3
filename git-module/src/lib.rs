use std::{path::Path, sync::Mutex};

use files_module::extract_contents;
use search_master_interface::{send_new_searchable_github_document, SearchableDocument, SearchableRoot};
use tracing::error;

const RUYI_GITS: &str = "ruyi-gits";

static UPDATE_LOCK: Mutex<()> = Mutex::new(());

pub fn update_gits() {
    std::thread::spawn(|| {
        let _guard = UPDATE_LOCK.lock().unwrap();
        for result in std::fs::read_dir(RUYI_GITS).unwrap() {
            let entry = result.unwrap();
            if !entry.path().is_dir() {
                continue;
            }
            let output = std::process::Command::new("git")
                .current_dir(entry.path())
                .args(["pull"])
                .output()
                .unwrap();
            if !output.status.success() {
                error!("git pull failed: {}", output.status);
                continue;
            }
            let origin = std::process::Command::new("git")
                .current_dir(entry.path())
                .args(["remote", "get-url", "origin"])
                .output()
                .unwrap()
                .stdout;
            let origin = String::from_utf8(origin).unwrap().trim().to_string();
            let main_branch = std::process::Command::new("git")
                .current_dir(entry.path())
                .args(["branch", "--show-current"])
                .output()
                .unwrap()
                .stdout;
            let main_branch = String::from_utf8(main_branch).unwrap().trim().to_string();

            let files = std::process::Command::new("git")
                .current_dir(entry.path())
                .args(["ls-tree", "-r", "HEAD", "--name-only"])
                .output()
                .unwrap()
                .stdout;
            let files = String::from_utf8(files).unwrap();

            for path in files.lines() {
                let path = entry.path().join(path);
                if !path.is_file() {
                    continue;
                }
                let Some(contents) = extract_contents(&path) else {
                    continue;
                };
                send_new_searchable_github_document(SearchableDocument::new(
                    path.file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string(),
                    SearchableRoot::new_github_file(&path, &Path::new(RUYI_GITS).join(entry.path().file_name().unwrap()), origin.clone(), main_branch.clone()),
                    contents,
                ));
            }
        }
    });
}
