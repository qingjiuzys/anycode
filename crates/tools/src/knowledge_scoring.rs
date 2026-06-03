//! Shared keyword/BM25-lite scoring for project knowledge chunks (no embedding API required).

/// Score a document chunk against a query (higher is better).
pub fn score_knowledge_chunk(query: &str, content: &str) -> f32 {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return 0.0;
    }
    let tokens: Vec<String> = q
        .split_whitespace()
        .filter(|t| t.len() >= 2)
        .map(str::to_string)
        .collect();
    if tokens.is_empty() {
        return 0.0;
    }
    let lower = content.to_lowercase();
    let doc_len = lower.split_whitespace().count().max(1) as f32;
    let avg_dl = 800.0_f32;
    let k1 = 1.2_f32;
    let b = 0.75_f32;
    let mut score = 0.0_f32;
    for term in &tokens {
        let tf = lower.matches(term.as_str()).count() as f32;
        if tf <= 0.0 {
            continue;
        }
        let idf = 1.0 + (1.0 + (100.0 / (1.0 + tf))).ln();
        let tf_norm = (tf * (k1 + 1.0)) / (tf + k1 * (1.0 - b + b * (doc_len / avg_dl)));
        score += idf * tf_norm;
        if lower.contains(term.as_str()) {
            score += 0.25;
        }
    }
    // Phrase boost when full query appears as substring.
    if lower.contains(&q) {
        score += 2.0;
    }
    score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phrase_beats_single_token() {
        let doc = "weekly report for Q1 revenue summary";
        let phrase = score_knowledge_chunk("weekly report", doc);
        let loose = score_knowledge_chunk("report", doc);
        assert!(phrase >= loose);
    }
}
