use rusqlite::{Connection, Result};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Database {
    pub conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        
        // Create tables if they don't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS pastes (
                id INTEGER PRIMARY KEY,
                token TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        Ok(Database {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn insert_paste(&self, token: &str, title: &str, content: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO pastes (token, title, content) VALUES (?, ?, ?)",
            [token, title, content],
        )?;
        Ok(())
    }

    pub fn get_paste_by_token(&self, token: &str) -> Result<(String, String)> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT content, title FROM pastes WHERE token = ?",
            [token],
            |row| {
                let content: String = row.get(0)?;
                let title: String = row.get(1)?;
                Ok((content, title))
            },
        )
    }

    pub fn get_content_by_token(&self, token: &str) -> Result<String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT content FROM pastes WHERE token = ?",
            [token],
            |row| row.get::<_, String>(0),
        )
    }
}