//! Resolve wxid / chatroom IDs to display names via contact + session DB snapshots.

use crate::backend::{
    cipher_pragmas_for_key, discover_sqlite_files, run_sqlcipher_json, SnapshotDb,
};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct NameResolver {
    contact_names: HashMap<String, String>,
    session_names: HashMap<String, String>,
}

impl NameResolver {
    pub fn from_data_dir(data_dir: &Path, key_map: &HashMap<String, String>) -> Self {
        let mut resolver = Self::default();
        for db in discover_sqlite_files(data_dir) {
            let file = db.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let Some(key) = resolve_key_for_db(key_map, &db) else {
                continue;
            };
            let Ok(snapshot) = SnapshotDb::create(&db) else {
                continue;
            };
            let path = snapshot.copy_path();
            if file == "contact.db" {
                resolver.load_contact_db(path, key);
            } else if file == "session.db" {
                resolver.load_session_db(path, key);
            }
        }
        resolver
    }

    pub fn conversation_name(&self, username: &str) -> Option<String> {
        self.session_names
            .get(username)
            .or_else(|| self.contact_names.get(username))
            .cloned()
    }

    pub fn sender_name(&self, sender_id: &str) -> Option<String> {
        self.contact_names.get(sender_id).cloned()
    }

    fn load_contact_db(&mut self, db_path: &Path, key: &str) {
        let sql = format!(
            "{}\n.mode json\nSELECT user_name, nick_name, remark FROM contact;",
            cipher_pragmas_for_key(key)
        );
        let Ok(rows) = run_sqlcipher_json(db_path, &sql) else {
            return;
        };
        for row in rows {
            let Some(obj) = row.as_object() else {
                continue;
            };
            let username = obj
                .get("user_name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if username.is_empty() {
                continue;
            }
            let display = obj
                .get("remark")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .or_else(|| obj.get("nick_name").and_then(|v| v.as_str()))
                .filter(|s| !s.is_empty())
                .unwrap_or(&username)
                .to_string();
            self.contact_names.insert(username, display);
        }
    }

    fn load_session_db(&mut self, db_path: &Path, key: &str) {
        let sql = format!(
            "{}\n.mode json\nSELECT username, summary FROM SessionTable;",
            cipher_pragmas_for_key(key)
        );
        let Ok(rows) = run_sqlcipher_json(db_path, &sql) else {
            return;
        };
        for row in rows {
            let Some(obj) = row.as_object() else {
                continue;
            };
            let username = obj
                .get("username")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if username.is_empty() {
                continue;
            }
            if let Some(summary) = obj.get("summary").and_then(|v| v.as_str()) {
                if !summary.is_empty() {
                    self.session_names.insert(username, summary.to_string());
                }
            }
        }
    }
}

fn resolve_key_for_db<'a>(key_map: &'a HashMap<String, String>, db_path: &Path) -> Option<&'a str> {
    let file_name = db_path.file_name()?.to_str()?;
    if let Some(k) = key_map.get(file_name) {
        return Some(k.as_str());
    }
    let suffix = format!("/{file_name}");
    key_map
        .iter()
        .find(|(k, _)| k.ends_with(&suffix))
        .map(|(_, v)| v.as_str())
}
