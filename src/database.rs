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
        
        // Emails table
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
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_email ON emails(email)",
            [],
        )?;

        // URL queue table for persistence
        conn.execute(
            "CREATE TABLE IF NOT EXISTS url_queue (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL UNIQUE,
                depth INTEGER NOT NULL,
                status TEXT DEFAULT 'pending'
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_queue_status ON url_queue(status)",
            [],
        )?;

        // Visited URLs table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS visited (
                url TEXT PRIMARY KEY
            )",
            [],
        )?;

        // Phones table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS phones (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                phone TEXT NOT NULL,
                source_url TEXT NOT NULL,
                found_at TEXT DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(phone, source_url)
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_phone ON phones(phone)",
            [],
        )?;

        // Images table (for face detection)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS images (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                uuid TEXT NOT NULL UNIQUE,
                source_url TEXT NOT NULL,
                found_at TEXT DEFAULT CURRENT_TIMESTAMP
            )",
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

    /// Add URL to queue (ignores if already exists)
    pub fn queue_url(&self, url: &str, depth: u32) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let result = conn.execute(
            "INSERT OR IGNORE INTO url_queue (url, depth, status) VALUES (?1, ?2, 'pending')",
            params![url, depth],
        )?;
        Ok(result > 0)
    }

    /// Get next pending URL from queue
    pub fn pop_url(&self) -> Result<Option<(String, u32)>> {
        let conn = self.conn.lock().unwrap();
        
        let result: Option<(i64, String, u32)> = conn.query_row(
            "SELECT id, url, depth FROM url_queue WHERE status = 'pending' LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).ok();

        if let Some((id, url, depth)) = result {
            conn.execute(
                "UPDATE url_queue SET status = 'processing' WHERE id = ?1",
                params![id],
            )?;
            Ok(Some((url, depth)))
        } else {
            Ok(None)
        }
    }

    /// Mark URL as completed
    pub fn complete_url(&self, url: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE url_queue SET status = 'done' WHERE url = ?1",
            params![url],
        )?;
        Ok(())
    }

    /// Check if URL was already visited
    pub fn is_visited(&self, url: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let count: u32 = conn.query_row(
            "SELECT COUNT(*) FROM visited WHERE url = ?1",
            params![url],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Mark URL as visited
    pub fn mark_visited(&self, url: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO visited (url) VALUES (?1)",
            params![url],
        )?;
        Ok(())
    }

    /// Get count of pending URLs
    pub fn pending_count(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let count: u64 = conn.query_row(
            "SELECT COUNT(*) FROM url_queue WHERE status = 'pending'",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Get count of processing URLs
    pub fn processing_count(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let count: u64 = conn.query_row(
            "SELECT COUNT(*) FROM url_queue WHERE status = 'processing'",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Reset processing URLs to pending (for resume)
    pub fn reset_processing(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let count = conn.execute(
            "UPDATE url_queue SET status = 'pending' WHERE status = 'processing'",
            [],
        )?;
        Ok(count as u64)
    }

    /// Clear queue (for fresh start)
    pub fn clear_queue(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM url_queue", [])?;
        conn.execute("DELETE FROM visited", [])?;
        Ok(())
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

    /// Get total count of email entries
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

    /// Insert a phone number with its source URL (ignores duplicates)
    pub fn insert_phone(&self, phone: &str, source_url: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let result = conn.execute(
            "INSERT OR IGNORE INTO phones (phone, source_url) VALUES (?1, ?2)",
            params![phone, source_url],
        )?;
        Ok(result > 0)
    }

    /// Get total count of unique phones
    pub fn get_phone_count(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let count: u64 = conn.query_row(
            "SELECT COUNT(DISTINCT phone) FROM phones",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Insert an image with its UUID and source URL
    pub fn insert_image(&self, uuid: &str, source_url: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let result = conn.execute(
            "INSERT OR IGNORE INTO images (uuid, source_url) VALUES (?1, ?2)",
            params![uuid, source_url],
        )?;
        Ok(result > 0)
    }

    /// Get total count of images
    pub fn get_image_count(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let count: u64 = conn.query_row(
            "SELECT COUNT(*) FROM images",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }
}
