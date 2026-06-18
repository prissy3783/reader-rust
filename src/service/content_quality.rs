use crate::service::catalog::is_catalog_like;

#[derive(Debug, Clone, PartialEq)]
pub enum ContentQuality {
    Normal,
    SuspectCatalog,
    SuspectJsRuleNotExecuted,
    SuspectTooShort,
    Empty,
}

impl ContentQuality {
    pub fn label(&self) -> &str {
        match self {
            ContentQuality::Normal => "✓ 正文可读",
            ContentQuality::SuspectCatalog => "⚠ 正文疑似目录",
            ContentQuality::SuspectJsRuleNotExecuted => "⚠ JS规则未执行",
            ContentQuality::SuspectTooShort => "⚠ 正文长度异常",
            ContentQuality::Empty => "✗ 内容为空",
        }
    }

    pub fn is_ok(&self) -> bool {
        *self == ContentQuality::Normal
    }
}

/// Analyze extracted content quality
pub fn analyze_content_quality(content: &str) -> ContentQuality {
    if content.trim().is_empty() {
        return ContentQuality::Empty;
    }

    let trimmed = content.trim();

    // Check for catalog-like content
    if is_catalog_like(trimmed) {
        return ContentQuality::SuspectCatalog;
    }

    // Check for JS rule not executed (rule text leaked into content)
    if contains_js_rule_text(trimmed) {
        return ContentQuality::SuspectJsRuleNotExecuted;
    }

    // Check for suspiciously short content (likely not real chapter content)
    let line_count = trimmed.lines().count();
    let char_count = trimmed.chars().count();
    if char_count < 50 && line_count < 3 {
        return ContentQuality::SuspectTooShort;
    }

    ContentQuality::Normal
}

/// Check if content contains JS rule text that shouldn't be in final output
fn contains_js_rule_text(content: &str) -> bool {
    // Only check lines that start with rule-like prefixes
    // This prevents false positives from normal book content containing JS keywords
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("@js:")
            || trimmed.starts_with("eval(")
            || trimmed.starts_with("function(")
            || trimmed.starts_with("var ")
            || trimmed.starts_with("let ")
            || trimmed.starts_with("const ")
            || trimmed.starts_with("document.")
            || trimmed.starts_with("window.")
            || trimmed.starts_with("XMLHttpRequest")
            || trimmed.starts_with("fetch(")
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_content() {
        assert_eq!(analyze_content_quality(""), ContentQuality::Empty);
        assert_eq!(analyze_content_quality("  "), ContentQuality::Empty);
    }

    #[test]
    fn test_normal_content() {
        let content = "这是一段正常的正文内容，包含了足够的文字来描述故事情节。主角走上了修炼之路，开始了他的冒险旅程，在这个世界里强者为尊弱者只能仰望。";
        assert_eq!(analyze_content_quality(content), ContentQuality::Normal);
    }

    #[test]
    fn test_catalog_detection() {
        let content =
            "第一章 初入江湖\n第二章 拜师学艺\n第三章 初露锋芒\n第四章 江湖恩怨\n第五章 生死抉择";
        assert_eq!(
            analyze_content_quality(content),
            ContentQuality::SuspectCatalog
        );
    }

    #[test]
    fn test_js_rule_detection() {
        let content = "@js:R=result";
        assert_eq!(
            analyze_content_quality(content),
            ContentQuality::SuspectJsRuleNotExecuted
        );
    }

    #[test]
    fn test_too_short() {
        assert_eq!(
            analyze_content_quality("短"),
            ContentQuality::SuspectTooShort
        );
    }
}
