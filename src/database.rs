use rusqlite::{Connection, Result, params};
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Create or open a SQLite database
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Database {
            conn: Mutex::new(conn),
        };
        db.init()?;
        Ok(db)
    }

    /// Initialize the database schema
    fn init(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS emails (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                email TEXT NOT NULL,
                source_url TEXT NOT NULL,
                found_at TEXT DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(email, source_url)
            )",
            [],
        )?;
        
        // Create index for faster lookups
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_email ON emails(email)",
            [],
        )?;
        
        Ok(())
    }

    /// Insert an email with its source URL (ignores duplicates)
    pub fn insert_email(&self, email: &str, source_url: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let result = conn.execute(
            "INSERT OR IGNORE INTO emails (email, source_url) VALUES (?1, ?2)",
            params![email, source_url],
        )?;
        Ok(result > 0)
    }

    /// Get total count of unique emails
    pub fn get_email_count(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let count: u64 = conn.query_row(
            "SELECT COUNT(DISTINCT email) FROM emails",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Get total count of email entries (including duplicates from different sources)
    pub fn get_total_entries(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let count: u64 = conn.query_row(
            "SELECT COUNT(*) FROM emails",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Get statistics about the database
    pub fn get_stats(&self) -> Result<(u64, u64)> {
        let unique = self.get_email_count()?;
        let total = self.get_total_entries()?;
        Ok((unique, total))
    }
}
