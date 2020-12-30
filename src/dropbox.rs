use anyhow::Result;
use dropbox_sdk::default_client::{NoauthDefaultClient, UserAuthDefaultClient};
use dropbox_sdk::oauth2::{
    oauth2_token_from_authorization_code, Oauth2AuthorizeUrlBuilder, Oauth2Type,
};
use dropbox_sdk::{files, UserAuthClient};
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::sync::atomic::{AtomicU64, Ordering::SeqCst};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;
use {
    crate::sqlite::Message,
    dropbox_sdk::files::{FileMetadata, ListFolderResult, Metadata},
};

/// How many blocks to upload in parallel.
const PARALLELISM: usize = 20;

/// The size of a block. This is a Dropbox constant, not adjustable.
const BLOCK_SIZE: usize = 4 * 1024 * 1024;

pub fn list_directory2(path: &str, tx: Sender<Message>) {
    let client = UserAuthDefaultClient::new(get_oauth2_token());
    let requested_path = if path == "/" {
        String::new()
    } else {
        path.to_owned()
    };
    let mut count = 0;
    match files::list_folder(
        &client,
        &files::ListFolderArg::new(requested_path).with_recursive(false),
    ) {
        Ok(Ok(ListFolderResult {
            entries,
            mut cursor,
            has_more,
            ..
        })) => {
            let tx2 = tx.clone();
            tokio::spawn(async move {
                for meta in entries {
                    match meta {
                        Metadata::File(FileMetadata {
                            name, content_hash, ..
                        }) => match content_hash {
                            Some(hash) => {
                                tx2.send(Message::Progress(name, hash)).await;
                            }
                            None => {
                                tx2.send(Message::Abort(format!(
                                    "content hash was empty: {}",
                                    name
                                )))
                                .await;
                            }
                        },
                        _ => {}
                    }
                }
                if !has_more {
                    tx2.send(Message::Finish).await;
                }
            });

            if !has_more {
                return;
            }
            let mut new_cursor = cursor;

            loop {
                println!("fetch count: {}", count);
                match files::list_folder_continue(
                    &client,
                    &files::ListFolderContinueArg::new(new_cursor),
                ) {
                    Ok(Ok(ListFolderResult {
                        entries,
                        mut cursor,
                        has_more,
                        ..
                    })) => {
                        new_cursor = cursor;
                        let tx2 = tx.clone();
                        tokio::spawn(async move {
                            for meta in entries {
                                match meta {
                                    Metadata::File(FileMetadata {
                                        name, content_hash, ..
                                    }) => match content_hash {
                                        Some(hash) => {
                                            tx2.send(Message::Progress(name, hash)).await;
                                        }
                                        None => {
                                            tx2.send(Message::Abort(format!(
                                                "content hash was empty: {}",
                                                name
                                            )))
                                            .await;
                                            return;
                                        }
                                    },
                                    _ => {}
                                }
                            }
                            if !has_more {
                                tx2.send(Message::Finish).await;
                                return;
                            }
                        });
                        if !has_more {
                            return;
                        }
                    }
                    Ok(Err(e)) => {
                        println!("{}", e);
                        tx.send(Message::Abort(format!("request failure: {}", e)));
                        return;
                    }
                    Err(e) => {
                        println!("{}", e);
                        tx.send(Message::Abort(format!("request failure: {}", e)));
                        return;
                    }
                }
                // cursor = "1".to_string();
                count = count + 1;
            }
            // println!("{:?}", result);
        }
        Ok(Err(e)) => {
            println!("{}", e);
            tx.send(Message::Abort(format!("request failure: {}", e)));
        }
        Err(e) => {
            println!("{}", e);
            tx.send(Message::Abort(format!("request failure: {}", e)));
        }
    };
}
pub fn list_directory(path: &str) {
    let client = UserAuthDefaultClient::new(get_oauth2_token());
    let requested_path = if path == "/" {
        String::new()
    } else {
        path.to_owned()
    };
    match files::list_folder(
        &client,
        &files::ListFolderArg::new(requested_path).with_recursive(false),
    ) {
        Ok(Ok(result)) => {
            println!("{:?}", result)
        }
        Ok(Err(e)) => {
            println!("{}", e)
        }
        Err(e) => {
            println!("{}", e)
        }
    };
}

pub fn get_file_metadata(path: &str) {
    let client = UserAuthDefaultClient::new(get_oauth2_token());
    match files::get_metadata(
        &client,
        &files::GetMetadataArg::new(path.to_string()).with_include_media_info(true),
    ) {
        Ok(Ok(result)) => {
            println!("{:?}", result)
        }
        Ok(Err(e)) => {
            println!("{}", e)
        }
        Err(e) => {
            println!("{}", e)
        }
    };
}

pub fn get_oauth2_token() -> String {
    env::var("DBX_OAUTH_TOKEN").unwrap()
}

#[derive(Debug)]
pub struct Resume {
    start_offset: u64,
    session_id: String,
}

struct UploadSession {
    session_id: String,
    start_offset: u64,
    file_size: u64,
    bytes_transferred: AtomicU64,
    completion: Mutex<CompletionTracker>,
}

#[derive(Default)]
struct CompletionTracker {
    complete_up_to: u64,
    uploaded_blocks: HashMap<u64, u64>,
}

impl UploadSession {
    fn new(client: &UserAuthDefaultClient, file_size: u64) -> Result<Self> {
        let session_id = match files::upload_session_start(
            client,
            &files::UploadSessionStartArg::default()
                .with_session_type(files::UploadSessionType::Concurrent),
            &[],
        ) {
            Ok(result) => match result {
                Ok(result) => result.session_id,
                Err(e) => return Err(anyhow::anyhow!(format!("{}", e))),
            },
            Err(e) => return Err(anyhow::anyhow!(format!("{}", e))),
        };
        Ok(Self {
            session_id,
            start_offset: 0,
            file_size,
            bytes_transferred: AtomicU64::new(0),
            completion: Mutex::new(CompletionTracker::default()),
        })
    }
}

pub fn upload_file(mut source_file: File, dest_path: String) -> Result<()> {
    let client = Arc::new(UserAuthDefaultClient::new(get_oauth2_token()));
    let source_len = source_file.metadata()?.len();
    let session = UploadSession::new(&client, source_len)?;
    let session_id = session.session_id.clone();
    let start_offset = session.start_offset;
    let cloned_client = client.clone();
    println!("upload session ID is {}", session.session_id);
    let result = parallel_reader::read_stream_and_process_chunks_in_parallel(
        &mut source_file,
        BLOCK_SIZE,
        PARALLELISM,
        Arc::new(move |block_offset, data: &[u8]| -> Result<()> {
            let mut append = files::UploadSessionAppendArg::new(files::UploadSessionCursor::new(
                session_id.clone(),
                start_offset + block_offset,
            ));
            if data.len() != BLOCK_SIZE {
                append.close = true;
            }
            files::upload_session_append_v2(cloned_client.as_ref(), &append, data);
            Ok(())
        }),
    );
    let finish = files::UploadSessionFinishArg::new(
        files::UploadSessionCursor::new(session.session_id.clone(), source_len),
        files::CommitInfo::new(dest_path),
    );
    files::upload_session_finish(client.as_ref(), &finish, &[]);
    Ok(())
}

// pub fn upload_file2(path: &Path, client, rx) {
//
// }
