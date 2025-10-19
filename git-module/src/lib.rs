use std::sync::Mutex;

use files_module::app::feed_file_with_base;
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
            let status = std::process::Command::new("git")
                .current_dir(entry.path())
                .args(["pull"])
                .status()
                .unwrap();
            if !status.success() {
                error!("git pull failed: {status}");
                continue;
            }
            let origin = std::process::Command::new("git")
                .current_dir(entry.path())
                .args(["remote", "get-url", "origin"])
                .output()
                .unwrap()
                .stdout;
            let origin = String::from_utf8(origin).unwrap();
            let main_branch = std::process::Command::new("git")
                .current_dir(entry.path())
                .args(["branch", "--show-current"])
                .output()
                .unwrap()
                .stdout;
            let main_branch = String::from_utf8(main_branch).unwrap();

            for result in walkdir::WalkDir::new(entry.path()) {
                let entry = result.unwrap();
                let path = entry.path();
                if path.is_file() {
                    feed_file_with_base(&path, RUYI_GITS);
                }
            }
        }
    });
}
