use anyhow::Result;
use chrono::{Date, DateTime, Local, Utc};
use chrono::{Duration, NaiveDateTime};
use dropbox_sdk::default_client::{NoauthDefaultClient, UserAuthDefaultClient};
use dropbox_sdk::oauth2::{
    oauth2_token_from_authorization_code, Oauth2AuthorizeUrlBuilder, Oauth2Type,
};
use dropbox_sdk::{files, UserAuthClient};
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering::SeqCst};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use {
    crate::{
        calc::{DatetimeExtnameDigests, SumNameDigests},
        sqlite::{connection, exist, Message},
    },
    dropbox_sdk::dbx_async,
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

pub async fn upload_files(files: DatetimeExtnameDigests) -> Result<()> {
    println!("upload start");
    let client = Arc::new(UserAuthDefaultClient::new(get_oauth2_token()));
    let max = 1000;
    let mut sum = 0;
    let mut threads = Vec::new();
    let mut path_names = Vec::new();
    let conn = connection("my-dropbox.db3")?;

    for (datetime, datetime_files) in files {
        println!("1, len: {}", path_names.len());
        sum = sum + datetime_files.sum;
        if sum <= max {
            println!("2");
            let mut count = 0;
            for pic in datetime_files.pic {
                if exist(&conn, pic.digest)? {
                    continue;
                }
                let name = if count != 0 {
                    format!("{}_{}.JPG", datetime, count)
                } else {
                    format!("{}.JPG", datetime)
                };
                path_names.push((pic.path, name));
                count = count + 1;
            }
            let mut count = 0;
            for mov in datetime_files.mov {
                if exist(&conn, mov.digest)? {
                    continue;
                }
                let name = if count != 0 {
                    format!("{}_{}.MP4", datetime, count)
                } else {
                    format!("{}.MP4", datetime)
                };
                path_names.push((mov.path, name));
                count = count + 1;
            }
        } else {
            println!("3");
            let cloned = path_names.clone();
            let cloned_client = client.clone();
            threads.push(tokio::spawn(async move {
                upload_files2(cloned, cloned_client).await
            }));
            path_names.clear();
            let mut count = 0;
            for pic in datetime_files.pic {
                if exist(&conn, pic.digest)? {
                    continue;
                }
                let name = if count != 0 {
                    format!("{}_{}.JPG", datetime, count)
                } else {
                    format!("{}.JPG", datetime)
                };
                path_names.push((pic.path, name));
                count = count + 1;
            }
            let mut count = 0;
            for mov in datetime_files.mov {
                if exist(&conn, mov.digest)? {
                    continue;
                }
                let name = if count != 0 {
                    format!("{}_{}.MP4", datetime, count)
                } else {
                    format!("{}.MP4", datetime)
                };
                path_names.push((mov.path, name));
                count = count + 1;
            }
            sum = datetime_files.sum;
        }
    }
    threads.push(tokio::spawn(async move {
        upload_files2(path_names, client).await
    }));
    println!("4");
    let finishes = futures::future::join_all(threads).await;
    println!("5");
    Ok(())
}
enum UploadMessage {
    Cont((String, SumNameDigests)),
    Finish(u32),
}

async fn upload_files2(
    path_names: Vec<(String, String)>,
    client: Arc<UserAuthDefaultClient>,
) -> Result<()> {
    let start_time: DateTime<Local> = Local::now();
    println!(
        "upload thread start: {}, len: {}",
        start_time,
        path_names.len()
    );
    let mut threads = Vec::new();
    for (path, name) in path_names {
        let cloned = client.clone();
        println!("thread spawn");
        threads.push(tokio::spawn(
            async move { upload_file2(&path, &name, cloned) },
        ));
    }
    let finishes = futures::future::join_all(threads).await;
    let mut v: Vec<files::UploadSessionFinishArg> = Vec::new();
    for finish in finishes {
        match finish {
            Ok(Ok(f)) => v.push(f),
            Ok(Err(e)) => {
                println!("OK Error: {}", e)
            }
            Err(e) => {
                println!("Error: {}", e)
            }
        }
    }
    let finish_batch_arg = files::UploadSessionFinishBatchArg::new(v);
    match files::upload_session_finish_batch(client.as_ref(), &finish_batch_arg) {
        Ok(Ok(res)) => match res {
            files::UploadSessionFinishBatchLaunch::AsyncJobId(async_job_id) => {
                let poll_arg = dbx_async::PollArg::new(async_job_id);
                loop {
                    match files::upload_session_finish_batch_check(client.as_ref(), &poll_arg) {
                        Ok(Ok(res)) => match res {
                            files::UploadSessionFinishBatchJobStatus::InProgress => {
                                println!("batch check inprogress");
                            }
                            files::UploadSessionFinishBatchJobStatus::Complete(_) => {
                                println!("batch check complete");
                                break;
                            }
                        },
                        Ok(Err(e)) => {
                            println!("batch check ok err: {}", e);
                            break;
                        }
                        Err(e) => {
                            println!("batch check err: {}", e);
                            break;
                        }
                    }
                }
            }
            files::UploadSessionFinishBatchLaunch::Complete(_) => {
                println!("upload batch finish");
            }
            _ => {}
        },
        Ok(Err(e)) => println!("Finish batch Ok Err : {}", e),
        Err(e) => println!("Finish batch Err : {}", e),
    }
    let end_time: DateTime<Local> = Local::now();
    println!(
        "upload thread finish: {}, duration: {}",
        start_time,
        (end_time - start_time).num_seconds()
    );
    Ok(())
}

pub fn upload_file2(
    path: &String,
    name: &String,
    client: Arc<UserAuthDefaultClient>,
) -> Result<files::UploadSessionFinishArg> {
    let mut source_file = File::open(Path::new(&path))?;
    let source_len = source_file.metadata()?.len();
    let mut session = UploadSession::new(&client, source_len)?;
    let session_id = session.session_id.clone();
    let start_offset = session.start_offset;
    // let cloned_client = client.clone();
    println!("-------upload session ID is {}", session.session_id);
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
            files::upload_session_append_v2(client.as_ref(), &append, data);
            Ok(())
        }),
    );
    let finish = files::UploadSessionFinishArg::new(
        files::UploadSessionCursor::new(session.session_id.clone(), source_len),
        files::CommitInfo::new(format!("/カメラアップロード/{}", name)),
    );
    Ok(finish)
}
