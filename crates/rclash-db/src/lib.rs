#![allow(dead_code)]
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;

pub use rusqlite::Connection as DbConnection;

#[derive(Debug, Clone)]
pub struct Config {
    pub id: i64,
    pub name: String,
    pub url: Option<String>,
    pub content: String,
    pub hash: String,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

fn db_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("RClash").join("rclash.db"))
}

#[derive(Debug, Clone)]
pub struct RawKey {
    pub id: i64,
    pub raw: String,
    pub scheme: Option<String>,
    pub name: String,
    pub parsed_yaml: String,
    pub added_at: i64,
}

pub fn ensure_dir() -> Result<PathBuf> {
    let p = db_path().ok_or_else(|| anyhow::anyhow!("no config dir"))?;
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)?;
    }
    Ok(p)
}

pub fn open() -> Result<Connection> {
    let path = ensure_dir()?;
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    ensure_schema(&conn)?;
    Ok(conn)
}

pub fn open_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    ensure_schema(&conn)?;
    Ok(conn)
}

fn ensure_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS configs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT UNIQUE NOT NULL,
            url TEXT,
            content TEXT NOT NULL,
            hash TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS favorites (
            proxy_name TEXT PRIMARY KEY,
            group_name TEXT,
            added_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS raw_keys (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            raw TEXT UNIQUE NOT NULL,
            scheme TEXT,
            name TEXT NOT NULL,
            parsed_yaml TEXT NOT NULL,
            added_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_configs_active ON configs(is_active);
        "#,
    )?;
    Ok(())
}

fn hash_content(s: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    format!("{:x}", h.finalize())
}

pub fn list_configs(conn: &Connection) -> Result<Vec<Config>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, url, content, hash, is_active, created_at, updated_at FROM configs ORDER BY updated_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Config {
            id: row.get(0)?,
            name: row.get(1)?,
            url: row.get(2)?,
            content: row.get(3)?,
            hash: row.get(4)?,
            is_active: row.get::<_, i64>(5)? != 0,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn get_active(conn: &Connection) -> Result<Option<Config>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, url, content, hash, is_active, created_at, updated_at FROM configs WHERE is_active=1 LIMIT 1",
    )?;
    let res = stmt
        .query_row([], |row| {
            Ok(Config {
                id: row.get(0)?,
                name: row.get(1)?,
                url: row.get(2)?,
                content: row.get(3)?,
                hash: row.get(4)?,
                is_active: row.get::<_, i64>(5)? != 0,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .optional()?;
    Ok(res)
}

pub fn save_config(conn: &Connection, name: &str, url: Option<&str>, content: &str) -> Result<()> {
    let hash = hash_content(content);
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        r#"
        INSERT INTO configs (name, url, content, hash, is_active, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, 0, ?5, ?5)
        ON CONFLICT(name) DO UPDATE SET url=excluded.url, content=excluded.content, hash=excluded.hash, updated_at=excluded.updated_at
        "#,
        params![name, url, content, hash, now],
    )?;
    Ok(())
}

pub fn update_content(conn: &Connection, name: &str, new_content: &str) -> Result<()> {
    let hash = hash_content(new_content);
    let now = chrono::Utc::now().timestamp();
    let n = conn.execute(
        "UPDATE configs SET content=?1, hash=?2, updated_at=?3 WHERE name=?4",
        params![new_content, hash, now, name],
    )?;
    if n == 0 {
        anyhow::bail!("config not found: {name}");
    }
    Ok(())
}

pub fn set_active(conn: &Connection, name: &str) -> Result<()> {
    conn.execute("UPDATE configs SET is_active=0", [])?;
    let n = conn.execute(
        "UPDATE configs SET is_active=1 WHERE name=?1",
        params![name],
    )?;
    if n == 0 {
        anyhow::bail!("config not found: {name}");
    }
    Ok(())
}

pub fn delete_config(conn: &Connection, name: &str) -> Result<()> {
    conn.execute("DELETE FROM configs WHERE name=?1", params![name])?;
    Ok(())
}

pub fn list_favorites(conn: &Connection) -> Result<std::collections::HashSet<String>> {
    let mut stmt = conn.prepare("SELECT proxy_name FROM favorites")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut set = std::collections::HashSet::new();
    for r in rows {
        set.insert(r?);
    }
    Ok(set)
}

pub fn add_favorite(conn: &Connection, proxy_name: &str, group_name: Option<&str>) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT OR REPLACE INTO favorites (proxy_name, group_name, added_at) VALUES (?1, ?2, ?3)",
        params![proxy_name, group_name, now],
    )?;
    Ok(())
}

pub fn remove_favorite(conn: &Connection, proxy_name: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM favorites WHERE proxy_name=?1",
        params![proxy_name],
    )?;
    Ok(())
}

pub fn is_favorite(conn: &Connection, proxy_name: &str) -> Result<bool> {
    let mut stmt = conn.prepare("SELECT 1 FROM favorites WHERE proxy_name=?1")?;
    let exists = stmt.exists(params![proxy_name])?;
    Ok(exists)
}

pub fn list_raw_keys(conn: &Connection) -> Result<Vec<RawKey>> {
    let mut stmt = conn.prepare(
        "SELECT id, raw, scheme, name, parsed_yaml, added_at FROM raw_keys ORDER BY added_at ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(RawKey {
            id: row.get(0)?,
            raw: row.get(1)?,
            scheme: row.get(2)?,
            name: row.get(3)?,
            parsed_yaml: row.get(4)?,
            added_at: row.get(5)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn add_raw_key(
    conn: &Connection,
    raw: &str,
    scheme: Option<&str>,
    name: &str,
    parsed_yaml: &str,
) -> Result<bool> {
    let now = chrono::Utc::now().timestamp();
    let n = conn.execute(
        "INSERT OR IGNORE INTO raw_keys (raw, scheme, name, parsed_yaml, added_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![raw, scheme, name, parsed_yaml, now],
    )?;
    Ok(n == 1)
}

pub fn remove_raw_key_by_name(conn: &Connection, name: &str) -> Result<()> {
    conn.execute("DELETE FROM raw_keys WHERE name=?1", params![name])?;
    Ok(())
}

pub fn clear_raw_keys(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM raw_keys", [])?;
    Ok(())
}

pub fn migration_needed(conn: &Connection) -> Result<bool> {
    let v: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    Ok(v < 1)
}

pub fn mark_migrated(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "user_version", 1)?;
    Ok(())
}

pub fn migrate_from_files(conn: &Connection) -> Result<usize> {
    let mut imported = 0;
    if let Some(cfg_dir) = rclash_config::config_dir() {
        let profiles_path = cfg_dir.join("profiles.json");
        if profiles_path.exists() {
            if let Ok(s) = std::fs::read_to_string(&profiles_path) {
                if let Ok(store) = serde_json::from_str::<rclash_config::profile::ProfileStore>(&s)
                {
                    for p in store.profiles {
                        if let Ok(content) = std::fs::read_to_string(&p.path) {
                            let _ = save_config(conn, &p.name, p.url.as_deref(), &content);
                            imported += 1;
                        }
                    }
                    if let Some(active) = store.active {
                        let _ = set_active(conn, &active);
                    }
                }
            }
        }
        let custom_path = cfg_dir.join("profiles").join("custom.yaml");
        if custom_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&custom_path) {
                if !content.trim().is_empty() {
                    if let Ok(v) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
                        if let Some(seq) = v.get("proxies").and_then(|p| p.as_sequence()) {
                            for item in seq {
                                let Some(m) = item.as_mapping() else {
                                    continue;
                                };
                                let name = m
                                    .get(serde_yaml::Value::String("name".into()))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_owned();
                                if name.is_empty() {
                                    continue;
                                }
                                let scheme = m
                                    .get(serde_yaml::Value::String("type".into()))
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_owned());
                                if let Ok(dump) = serde_yaml::to_string(item) {
                                    if add_raw_key(
                                        conn,
                                        dump.trim(),
                                        scheme.as_deref(),
                                        &name,
                                        dump.trim(),
                                    )
                                    .unwrap_or(false)
                                    {
                                        imported += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let _ = mark_migrated(conn);
    Ok(imported)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crud_memory() {
        let conn = open_memory().unwrap();
        save_config(&conn, "test", Some("https://example.com"), "proxies: []").unwrap();
        let list = list_configs(&conn).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "test");
        set_active(&conn, "test").unwrap();
        let active = get_active(&conn).unwrap().unwrap();
        assert_eq!(active.name, "test");
        update_content(&conn, "test", "proxies:\n  - name: a").unwrap();
        let fav = list_favorites(&conn).unwrap();
        assert!(fav.is_empty());
        add_favorite(&conn, "a", Some("PROXY")).unwrap();
        assert!(is_favorite(&conn, "a").unwrap());
        remove_favorite(&conn, "a").unwrap();
        assert!(!is_favorite(&conn, "a").unwrap());
        delete_config(&conn, "test").unwrap();
        assert!(list_configs(&conn).unwrap().is_empty());
    }

    #[test]
    fn raw_keys_crud_memory() {
        let conn = open_memory().unwrap();
        assert!(list_raw_keys(&conn).unwrap().is_empty());
        assert!(migration_needed(&conn).unwrap());
        let inserted =
            add_raw_key(&conn, "hysteria2://a", Some("hysteria2"), "a", "name: a").unwrap();
        assert!(inserted);
        let dup = add_raw_key(&conn, "hysteria2://a", Some("hysteria2"), "a", "name: a").unwrap();
        assert!(!dup);
        let keys = list_raw_keys(&conn).unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].name, "a");
        remove_raw_key_by_name(&conn, "a").unwrap();
        assert!(list_raw_keys(&conn).unwrap().is_empty());
        mark_migrated(&conn).unwrap();
        assert!(!migration_needed(&conn).unwrap());
    }
}
