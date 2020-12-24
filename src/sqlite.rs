use crate::dropbox::{get_oauth2_token, list_directory2};
use anyhow::Result;
use dropbox_sdk::default_client::UserAuthDefaultClient;
use rusqlite::types::ToSqlOutput;
use rusqlite::{params, Connection, Result as SqResult, ToSql, NO_PARAMS};
use std::fs;
use tokio::sync::mpsc;

pub async fn reset_db(path: &str) -> Result<()> {
    let _ = fs::remove_file(path);
    let conn = Connection::open(path)?;
    conn.execute(
        "CREATE TABLE files (
            name TEXT UNIQUE,
            hash TEXT UNIQUE
            );",
        params![],
    );
    let data = vec![
        FileData {
            name: "1.jpg".to_string(),
            hash: "1".to_string(),
        },
        FileData {
            name: "2.mp4".to_string(),
            hash: "2".to_string(),
        },
    ];
    // for d in data {
    //     insert(&conn, &d)?;
    // }
    // let result: SqResult<i32> =
    //     conn.query_row("SELECT COUNT(*) FROM files;", NO_PARAMS, |row| row.get(0));
    // println!("{:?}", result);
    let (mut tx, mut rx): (mpsc::Sender<Message>, mpsc::Receiver<Message>) = mpsc::channel(32);
    let client = UserAuthDefaultClient::new(get_oauth2_token());
    list_directory2("/カメラアップロード", tx);
    while let Some(message) = rx.recv().await {
        match message {
            Message::Finish => {}
            Message::Abort(e) => return Err(anyhow::anyhow!(format!("{}", e))),
            Message::Progress(name, hash) => {
                let data = FileData {
                    name: name,
                    hash: hash,
                };
                match insert(&conn, &data) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("insert error: file: {}, error: {}", data.name, e)
                    }
                }
            }
        }
    }

    Ok(())
}

pub enum Message {
    Finish,
    Abort(String),
    Progress(String, String),
}

pub enum FileType {
    Picture,
    Movie,
}
impl ToSql for FileType {
    fn to_sql(&self) -> SqResult<ToSqlOutput<'_>> {
        match self {
            Picture => Ok(ToSqlOutput::from("picture")),
            Movie => Ok(ToSqlOutput::from("movie")),
        }
    }
}
pub struct FileData {
    name: String,
    hash: String,
}
fn insert(conn: &Connection, data: &FileData) -> Result<()> {
    conn.execute(
        "INSERT INTO files (name,  hash) VALUES (?1, ?2);",
        params![data.name, data.hash],
    )?;
    Ok(())
}
