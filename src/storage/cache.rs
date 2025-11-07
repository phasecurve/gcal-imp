use rusqlite::{Connection, Result as SqliteResult};
use thiserror::Error;

use crate::calendar::Event;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

pub struct Cache {
    conn: Connection,
}

impl Cache {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    pub fn initialize(&self) -> Result<(), CacheError> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                calendar_id TEXT NOT NULL,
                data TEXT NOT NULL,
                start_date TEXT NOT NULL,
                end_date TEXT NOT NULL,
                last_modified TEXT NOT NULL
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS calendars (
                id TEXT PRIMARY KEY,
                data TEXT NOT NULL
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS sync_queue (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                operation TEXT NOT NULL,
                event_id TEXT NOT NULL,
                data TEXT NOT NULL,
                created_at TEXT NOT NULL
            )",
            [],
        )?;

        Ok(())
    }

    pub fn store_event(&self, event: &Event) -> Result<(), CacheError> {
        let data = serde_json::to_string(event)?;
        self.conn.execute(
            "INSERT OR REPLACE INTO events (id, calendar_id, data, start_date, end_date, last_modified)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                &event.id,
                &event.calendar_id,
                &data,
                event.start.to_rfc3339(),
                event.end.to_rfc3339(),
                event.last_modified.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn load_event(&self, id: &str) -> Result<Option<Event>, CacheError> {
        let mut stmt = self.conn.prepare("SELECT data FROM events WHERE id = ?1")?;
        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            let data: String = row.get(0)?;
            let event: Event = serde_json::from_str(&data)?;
            Ok(Some(event))
        } else {
            Ok(None)
        }
    }

    pub fn delete_event(&self, id: &str) -> Result<(), CacheError> {
        self.conn.execute("DELETE FROM events WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn table_exists(&self, table_name: &str) -> bool {
        let result: SqliteResult<i32> = self.conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            [table_name],
            |row| row.get(0),
        );
        result.unwrap_or(0) > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::calendar::EventStatus;

    fn create_test_cache() -> Cache {
        let conn = Connection::open_in_memory().unwrap();
        let cache = Cache::new(conn);
        cache.initialize().unwrap();
        cache
    }

    fn create_test_event(id: &str, title: &str) -> Event {
        let start = Utc::now();
        Event {
            id: id.to_string(),
            calendar_id: "primary".to_string(),
            title: title.to_string(),
            description: None,
            location: None,
            start,
            end: start + chrono::Duration::hours(1),
            all_day: false,
            attendees: vec![],
            reminders: vec![],
            status: EventStatus::Confirmed,
            last_modified: Utc::now(),
            html_link: None,
        }
    }

    #[test]
    fn creates_database_schema() {
        let conn = Connection::open_in_memory().unwrap();
        let cache = Cache::new(conn);

        cache.initialize().unwrap();

        assert!(cache.table_exists("events"));
        assert!(cache.table_exists("calendars"));
        assert!(cache.table_exists("sync_queue"));
    }

    #[test]
    fn stores_event_in_cache() {
        let cache = create_test_cache();
        let event = create_test_event("event1", "Meeting");

        cache.store_event(&event).unwrap();

        let loaded = cache.load_event(&event.id).unwrap();
        assert_eq!(loaded, Some(event));
    }

    #[test]
    fn loads_nonexistent_event_returns_none() {
        let cache = create_test_cache();

        let loaded = cache.load_event("nonexistent").unwrap();

        assert_eq!(loaded, None);
    }

    #[test]
    fn updates_existing_event() {
        let cache = create_test_cache();
        let mut event = create_test_event("event1", "Original");
        cache.store_event(&event).unwrap();

        event.title = "Updated".to_string();
        cache.store_event(&event).unwrap();

        let loaded = cache.load_event(&event.id).unwrap().unwrap();
        assert_eq!(loaded.title, "Updated");
    }

    #[test]
    fn deletes_event_from_cache() {
        let cache = create_test_cache();
        let event = create_test_event("event1", "To Delete");
        cache.store_event(&event).unwrap();

        cache.delete_event(&event.id).unwrap();

        let loaded = cache.load_event(&event.id).unwrap();
        assert!(loaded.is_none());
    }
}
