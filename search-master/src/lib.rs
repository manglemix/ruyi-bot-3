use search_master_interface::init_documents_senders;
use tokio::select;
use tracing::{debug, error};


pub fn initialize() {
    let (mut new_searchable, mut invalidated_searchable) = init_documents_senders();
    let client = meilisearch_sdk::client::Client::new("http://localhost:7700", Option::<String>::None).unwrap();
    let index = client.index("docs");

    tokio::spawn(async move {
        loop {
            select! {
                opt = new_searchable.recv() => {
                    let Some(doc) = opt else { break; };
                    if let Err(e) = index.add_or_replace(&[&doc], Some("id")).await {
                        error!("Failed to add {} to meilisearch: {e}", doc.filename());
                    } else {
                        debug!("Added {} to meilisearch", doc.filename());
                    }
                }
                opt = invalidated_searchable.recv() => {
                    let Some(doc) = opt else { break; };
                    if let Err(e) = index.delete_document(&doc.0).await {
                        error!("Failed to delete {} from meilisearch: {e}", doc.0);
                    } else {
                        debug!("Deleted {} from meilisearch", doc.0);
                    }
                }
            }
        }
    });

    files_module::app::start_watcher().expect("Expected files watcher to initialize");
}