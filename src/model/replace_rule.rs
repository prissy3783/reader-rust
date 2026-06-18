use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ReplaceRule {
    pub id: i64,
    pub name: String,
    pub group: Option<String>,
    pub pattern: String,
    pub replacement: String,
    pub scope: Option<String>,
    #[serde(rename = "isEnabled")]
    pub is_enabled: bool,
    #[serde(rename = "isRegex")]
    pub is_regex: bool,
    pub order: i32,
    /// Whether this rule applies to content (正文)
    pub scope_content: Option<bool>,
    /// Whether this rule applies to chapter titles (标题)
    pub scope_title: Option<bool>,
    /// Exclude scope: book names or source URLs to skip
    pub exclude_scope: Option<String>,
}

impl ReplaceRule {
    /// Check if this rule should be applied to the given book/source
    pub fn should_apply(&self, book_name: &str, source_url: &str) -> bool {
        if !self.is_enabled {
            return false;
        }
        if let Some(exclude) = &self.exclude_scope {
            if exclude.contains(book_name) || exclude.contains(source_url) {
                return false;
            }
        }
        true
    }

    /// Check if this rule applies to content (vs title)
    pub fn applies_to_content(&self) -> bool {
        self.scope_content.unwrap_or(true)
    }

    /// Check if this rule applies to chapter titles
    pub fn applies_to_title(&self) -> bool {
        self.scope_title.unwrap_or(false)
    }
}
