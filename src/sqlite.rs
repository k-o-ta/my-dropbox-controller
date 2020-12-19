use rusqlite::types::ToSqlOutput;
use rusqlite::{params, Connection, Result, ToSql, NO_PARAMS};
use std::fs;

pub fn reset_db(path: &str) -> Result<()> {
    let _ = fs::remove_file(path);
    let conn = Connection::open(path)?;
    conn.execute(
        "CREATE TABLE files (
            name TEXT UNIQUE,
            file_type TEXT,
            hash TEXT UNIQUE
            );",
        params![],
    );
    let data = vec![
        FileData {
            name: "1.jpg".to_string(),
            file_type: FileType::Picture,
            hash: "1".to_string(),
        },
        FileData {
            name: "2.mp4".to_string(),
            file_type: FileType::Movie,
            hash: "2".to_string(),
        },
    ];
    for d in data {
        insert(&conn, &d)?;
    }
    let result: Result<i32> =
        conn.query_row("SELECT COUNT(*) FROM files;", NO_PARAMS, |row| row.get(0));
    println!("{:?}", result);

    Ok(())
}

pub enum FileType {
    Picture,
    Movie,
}
impl ToSql for FileType {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        match self {
            Picture => Ok(ToSqlOutput::from("picture")),
            Movie => Ok(ToSqlOutput::from("movie")),
        }
    }
}
pub struct FileData {
    name: String,
    file_type: FileType,
    hash: String,
}
fn insert(conn: &Connection, data: &FileData) -> Result<()> {
    conn.execute(
        "INSERT INTO files (name, file_type, hash) VALUES (?1, ?2, ?3);",
        params![data.name, data.file_type, data.hash],
    )?;
    Ok(())
}
