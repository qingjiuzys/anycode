use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub id: Uuid,
    pub parent: Option<Uuid>,
    pub message: Value,
}
/// An append-only conversation tree. Selecting a branch does not rewrite its siblings.
#[derive(Default)]
pub struct SessionTree {
    entries: BTreeMap<Uuid, Entry>,
}
impl SessionTree {
    pub fn insert(&mut self, entry: Entry) -> Result<()> {
        if entry.id.is_nil()
            || self.entries.contains_key(&entry.id)
            || entry
                .parent
                .is_some_and(|id| !self.entries.contains_key(&id))
        {
            return Err(Error::Conflict(
                "invalid session parent or duplicate entry".into(),
            ));
        }
        if self.entries.len() >= 100_000 {
            return Err(Error::Capacity);
        }
        self.entries.insert(entry.id, entry);
        Ok(())
    }
    pub fn append(&mut self, parent: Option<Uuid>, message: Value) -> Result<Uuid> {
        let id = Uuid::new_v4();
        self.insert(Entry {
            id,
            parent,
            message,
        })?;
        Ok(id)
    }
    pub fn history(&self, leaf: Uuid) -> Result<Vec<Value>> {
        let mut next = Some(leaf);
        let mut result = vec![];
        let mut seen = BTreeSet::new();
        while let Some(id) = next {
            if !seen.insert(id) {
                return Err(Error::Conflict("session cycle".into()));
            }
            let entry = self
                .entries
                .get(&id)
                .ok_or_else(|| Error::Invalid("unknown session leaf".into()))?;
            result.push(entry.message.clone());
            next = entry.parent;
        }
        result.reverse();
        Ok(result)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn branching_does_not_copy_or_overwrite_siblings() {
        let mut tree = SessionTree::default();
        let root = tree.append(None, Value::from("root")).unwrap();
        let a = tree.append(Some(root), Value::from("a")).unwrap();
        let b = tree.append(Some(root), Value::from("b")).unwrap();
        assert_eq!(
            tree.history(a).unwrap(),
            vec![Value::from("root"), Value::from("a")]
        );
        assert_eq!(
            tree.history(b).unwrap(),
            vec![Value::from("root"), Value::from("b")]
        );
        assert!(tree.append(Some(Uuid::new_v4()), Value::Null).is_err());
    }
}
