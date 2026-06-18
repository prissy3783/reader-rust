use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct SearchBook {
    pub name: String,
    pub author: String,
    pub book_url: String,
    pub origin: String,
    pub cover_url: Option<String>,
    pub intro: Option<String>,
    pub kind: Option<String>,
    pub last_chapter: Option<String>,
    pub update_time: Option<String>,
    pub word_count: Option<String>,
    /// Book source URLs for the same book from different sources (for merged results)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub book_source_urls: Option<Vec<String>>,
    /// Search relevance score (computed by frontend)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    /// Match type label
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_type: Option<String>,
}

impl SearchBook {
    /// Generate a key for merging books with same name and author
    pub fn merge_key(&self) -> String {
        let name = self.name.trim().to_lowercase();
        let author = self.author.trim().to_lowercase();
        format!("{}|{}", name, author)
    }

    /// Compute relevance score against target book
    pub fn compute_score(&mut self, target_name: &str, target_author: &str) {
        let name = self.name.trim().to_lowercase();
        let author = self.author.trim().to_lowercase();
        let target_name_lower = target_name.trim().to_lowercase();
        let target_author_lower = target_author.trim().to_lowercase();

        let name_no_space = name.replace(char::is_whitespace, "");
        let target_name_no_space = target_name_lower.replace(char::is_whitespace, "");

        let mut score = 0.0f64;
        let mut match_type = "弱匹配".to_string();

        // Book name scoring
        if name == target_name_lower {
            score += 100.0;
            match_type = "精准匹配".to_string();
        } else if name_no_space == target_name_no_space {
            score += 90.0;
            match_type = "精准匹配".to_string();
        } else if name.contains(&target_name_lower) || target_name_lower.contains(&name) {
            score += 60.0;
            match_type = "书名相似".to_string();
        } else {
            // Check for high similarity (>= 60% character overlap)
            let overlap = compute_char_overlap(&name, &target_name_lower);
            if overlap >= 0.6 {
                score += 60.0;
                match_type = "书名相似".to_string();
            } else if overlap >= 0.3 {
                score += 20.0;
                match_type = "弱匹配".to_string();
            }
        }

        // Author scoring
        if !author.is_empty() && !target_author_lower.is_empty() {
            if author == target_author_lower {
                score += 50.0;
                if match_type == "弱匹配" {
                    match_type = "作者匹配".to_string();
                }
            } else {
                let author_no_space = author.replace(char::is_whitespace, "");
                let target_author_no_space = target_author_lower.replace(char::is_whitespace, "");
                if author_no_space == target_author_no_space {
                    score += 40.0;
                    if match_type == "弱匹配" {
                        match_type = "作者匹配".to_string();
                    }
                }
            }
        }

        self.score = Some(score);
        self.match_type = Some(match_type);
    }
}

/// Compute character overlap ratio between two strings
fn compute_char_overlap(a: &str, b: &str) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let a_chars: std::collections::HashSet<char> = a.chars().collect();
    let b_chars: std::collections::HashSet<char> = b.chars().collect();
    let intersection = a_chars.intersection(&b_chars).count();
    let union = a_chars.union(&b_chars).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}
