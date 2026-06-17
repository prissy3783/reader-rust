pub fn is_catalog_like(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    let lines: Vec<&str> = text.lines().collect();
    let total_lines = lines.len();
    if total_lines < 3 {
        return false;
    }

    let catalog_patterns = [
        "第一章", "第二章", "第三章", "第四章", "第五章",
        "最新章节", "章节目录", "返回目录", "目录",
        "上一章", "下一章", "章节列表",
    ];

    let short_lines = lines
        .iter()
        .filter(|l| l.trim().chars().count() < 15)
        .count();
    let pattern_matches = lines
        .iter()
        .filter(|l| catalog_patterns.iter().any(|p| l.contains(p)))
        .count();

    let title_density = pattern_matches as f64 / total_lines as f64;
    let short_ratio = short_lines as f64 / total_lines as f64;

    title_density > 0.15 || short_ratio > 0.7
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_text() {
        assert!(!is_catalog_like(""));
    }

    #[test]
    fn test_short_text() {
        assert!(!is_catalog_like("hello\nworld"));
    }

    #[test]
    fn test_normal_content() {
        let text = "这是一段正常的正文内容，包含了足够的文字来描述故事情节。\
            主角走上了修炼之路，开始了他的冒险旅程。\
            在这个世界里，强者为尊，弱者只能仰望。\
            他决心要成为最强的存在，不惜一切代价。\
            路途遥远，但他从未放弃过希望。";
        assert!(!is_catalog_like(text));
    }

    #[test]
    fn test_catalog_with_chapter_patterns() {
        let text = "第一章 初入江湖\n第二章 拜师学艺\n第三章 初露锋芒\n\
            第四章 江湖恩怨\n第五章 生死抉择\n第六章 绝处逢生\n\
            第七章 逆天改命\n第八章 一统天下\n第九章 功成身退\n\
            第十章 归隐山林\n第十一章 重返故里\n第十二章 最终决战";
        assert!(is_catalog_like(text));
    }

    #[test]
    fn test_catalog_with_navigation() {
        let text = "目录\n上一章\n下一章\n返回目录\n\
            第一章\n第二章\n第三章\n第四章\n第五章\n\
            最新章节\n章节列表\n下一章\n上一章";
        assert!(is_catalog_like(text));
    }

    #[test]
    fn test_many_short_lines() {
        let text = "你好\n世界\n测试\n数据\n类型\n\
            示例\n文本\n内容\n格式\n样式\n\
            字体\n颜色\n大小\n位置\n对齐\n\
            边距\n填充\n背景\n前景\n透明";
        assert!(is_catalog_like(text));
    }
}
