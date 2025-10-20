use std::{sync::{Mutex, OnceLock}, time::{Duration, Instant}};

use meilisearch_sdk::documents::DocumentDeletionQuery;
use search_master_interface::{init_documents_senders, DocumentReceivers};
use tokio::{select, task::block_in_place};
use tracing::{debug, error};

pub fn initialize() {
    let DocumentReceivers { mut new_searchable_document, new_searchable_github_document: mut new_searchable_git_document, mut new_searchable_message, mut invalidated_message_author_id } = init_documents_senders();
    let client =
        meilisearch_sdk::client::Client::new("http://localhost:7700", Option::<String>::None)
            .unwrap();
    let doc_index = client.index("docs");
    let githubs_index = client.index("githubs");
    let msg_index = client.index("messages");

    tokio::spawn(async move {
        loop {
            select! {
                opt = new_searchable_document.recv() => {
                    let Some(doc) = opt else { break; };
                    if let Err(e) = doc_index.add_or_replace(&[&doc], Some("id")).await {
                        error!("Failed to add {} to meilisearch: {e}", doc.filename());
                    } else {
                        debug!("Added {} to meilisearch", doc.filename());
                    }
                }
                opt = new_searchable_git_document.recv() => {
                    let Some(doc) = opt else { break; };
                    if let Err(e) = githubs_index.add_or_replace(&[&doc], Some("id")).await {
                        error!("Failed to add {} to meilisearch: {e}", doc.filename());
                    } else {
                        debug!("Added {} to meilisearch", doc.filename());
                    }
                }
                opt = new_searchable_message.recv() => {
                    let Some(msg) = opt else { break; };
                    if let Err(e) = msg_index.add_or_replace(&[&msg], Some("id")).await {
                        error!("Failed to add message (author: {}) to meilisearch: {e}", msg.author_id());
                    } else {
                        debug!("Added message (author: {}) to meilisearch", msg.author_id());
                    }
                }
                opt = invalidated_message_author_id.recv() => {
                    let Some(author_id) = opt else { break; };
                    let mut query = DocumentDeletionQuery::new(&msg_index);
                    let clause = format!("author_id = {author_id}");
                    query.with_filter(&clause);
                    let task = match msg_index.delete_documents_with(&query).await {
                        Ok(x) => x,
                        Err(e) => {
                            error!("Failed to delete author: {author_id} from meilisearch: {e}");
                            return;
                        }
                    };
                    if let Err(e) = task.wait_for_completion(&client, None, None).await {
                        error!("Failed to delete author: {author_id} from meilisearch: {e}");
                    } else {
                        debug!("Deleted author: {author_id} from meilisearch");
                    }
                }
            }
        }
    });

    update_githubs();
}

// pub async fn delete_all_documents() {
//     let client =
//         meilisearch_sdk::client::Client::new("http://localhost:7700", Option::<String>::None)
//             .unwrap();
//     if let Err(e) = client.delete_index("docs").await {
//         tracing::error!("{e}");
//     }
// }

static LAST_GITHUB_UPDATE: OnceLock<Mutex<Instant>> = OnceLock::new();
const MINIMUM_GITHUB_UPDATE_DURATION: Duration = Duration::from_secs(1800);

pub fn update_githubs() {
    tokio::spawn(async {
        if let Some(mutex) = LAST_GITHUB_UPDATE.get() {
            let mut guard = mutex.lock().unwrap();
            if guard.elapsed() < MINIMUM_GITHUB_UPDATE_DURATION {
                return;
            }
            *guard = Instant::now();
        } else {
            let _ = LAST_GITHUB_UPDATE.set(Mutex::new(Instant::now()));
        }
        let client =
            meilisearch_sdk::client::Client::new("http://localhost:7700", Option::<String>::None)
                .unwrap();
        if let Err(e) = client.delete_index("githubs").await {
            tracing::error!("{e}");
        }
        block_in_place(|| {
            git_module::update_gits();
        });
    });
}
