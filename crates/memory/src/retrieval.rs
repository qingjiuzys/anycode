use anycode_core::{Memory, MemoryType};

pub trait MemoryRetrieval {
    fn rank(&self, query: &str, memories: Vec<Memory>) -> Vec<Memory>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct KeywordRetrieval;

impl MemoryRetrieval for KeywordRetrieval {
    fn rank(&self, query: &str, mut memories: Vec<Memory>) -> Vec<Memory> {
        let q = query.to_ascii_lowercase();
        memories.sort_by_key(|memory| {
            let mut score = 0i32;
            if memory.title.to_ascii_lowercase().contains(&q) {
                score -= 3;
            }
            if memory.content.to_ascii_lowercase().contains(&q) {
                score -= 2;
            }
            if memory
                .tags
                .iter()
                .any(|tag| tag.to_ascii_lowercase().contains(&q))
            {
                score -= 1;
            }
            score
        });
        memories
    }
}

pub fn filter_memories_by_type(memories: Vec<Memory>, mem_type: MemoryType) -> Vec<Memory> {
    memories
        .into_iter()
        .filter(|memory| memory.mem_type == mem_type)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anycode_core::{MemoryScope, MemoryType};

    fn memory(title: &str, content: &str, tags: &[&str], mem_type: MemoryType) -> Memory {
        Memory {
            id: title.to_string(),
            mem_type,
            title: title.to_string(),
            content: content.to_string(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            scope: MemoryScope::Project,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn keyword_retrieval_prefers_title_then_content_then_tags() {
        let memories = vec![
            memory(
                "misc",
                "contains alpha in content",
                &[],
                MemoryType::Project,
            ),
            memory("alpha title", "other", &[], MemoryType::Project),
            memory("misc2", "other", &["alpha"], MemoryType::Project),
        ];
        let ranked = KeywordRetrieval.rank("alpha", memories);
        assert_eq!(ranked[0].title, "alpha title");
        assert_eq!(ranked[1].title, "misc");
        assert_eq!(ranked[2].title, "misc2");
    }

    #[test]
    fn filter_memories_by_type_keeps_only_requested_type() {
        let memories = vec![
            memory("a", "x", &[], MemoryType::Project),
            memory("b", "x", &[], MemoryType::User),
        ];
        let filtered = filter_memories_by_type(memories, MemoryType::Project);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].title, "a");
    }
}
