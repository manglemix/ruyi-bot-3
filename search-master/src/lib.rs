use meilisearch_sdk::documents::DocumentDeletionQuery;
use search_master_interface::init_documents_senders;
use tokio::select;
use tracing::{debug, error};


pub fn initialize() {
    let (
        mut new_searchable_doc,
        mut invalidated_searchable_doc,
        mut new_searchable_message,
        mut invalidated_message_author_id
    ) = init_documents_senders();
    let client = meilisearch_sdk::client::Client::new("http://localhost:7700", Option::<String>::None).unwrap();
    let doc_index = client.index("docs");
    let msg_index = client.index("messages");

    tokio::spawn(async move {
        loop {
            select! {
                opt = new_searchable_doc.recv() => {
                    let Some(doc) = opt else { break; };
                    if let Err(e) = doc_index.add_or_replace(&[&doc], Some("id")).await {
                        error!("Failed to add {} to meilisearch: {e}", doc.filename());
                    } else {
                        debug!("Added {} to meilisearch", doc.filename());
                    }
                }
                opt = invalidated_searchable_doc.recv() => {
                    let Some(doc) = opt else { break; };
                    if let Err(e) = doc_index.delete_document(&doc.0).await {
                        error!("Failed to delete {} from meilisearch: {e}", doc.0);
                    } else {
                        debug!("Deleted {} from meilisearch", doc.0);
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

    files_module::app::start_watcher().expect("Expected files watcher to initialize");
    git_module::update_gits();
}