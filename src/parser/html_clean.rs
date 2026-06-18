/// Safe HTML cleanup for content extraction.
/// Only removes script, style, and known ad elements.
/// Preserves img tags and normal content structure.
pub fn format_keep_img(html: &str, _base_url: &str) -> String {
    if html.trim().is_empty() {
        return String::new();
    }

    let mut result = html.to_string();

    // Remove <script>...</script> tags (including content)
    let re_script = regex::Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap();
    result = re_script.replace_all(&result, "").to_string();

    // Remove <style>...</style> tags (including content)
    let re_style = regex::Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap();
    result = re_style.replace_all(&result, "").to_string();

    // Convert <br> and <br/> to newlines
    let re_br = regex::Regex::new(r"(?i)<br\s*/?\s*>").unwrap();
    result = re_br.replace_all(&result, "\n").to_string();

    // Convert </p> to newlines
    let re_p = regex::Regex::new(r"(?is)</p>").unwrap();
    result = re_p.replace_all(&result, "\n").to_string();

    // Convert </div> to newlines
    let re_div = regex::Regex::new(r"(?is)</div>").unwrap();
    result = re_div.replace_all(&result, "\n").to_string();

    // Protect <img> tags with placeholders before stripping other tags
    let mut img_map = std::collections::HashMap::new();
    let re_img = regex::Regex::new(r"(?i)<img[^>]*>").unwrap();
    let mut img_idx = 0;
    result = re_img
        .replace_all(&result, |caps: &regex::Captures| {
            let placeholder = format!("__IMG_{}__", img_idx);
            img_map.insert(placeholder.clone(), caps[0].to_string());
            img_idx += 1;
            placeholder
        })
        .to_string();

    // Remove remaining HTML tags
    let re_tags = regex::Regex::new(r"<[^>]+>").unwrap();
    result = re_tags.replace_all(&result, "").to_string();

    // Restore img tags
    for (placeholder, img_tag) in &img_map {
        result = result.replace(placeholder, img_tag);
    }

    // Decode common HTML entities
    result = result.replace("&amp;", "&");
    result = result.replace("&lt;", "<");
    result = result.replace("&gt;", ">");
    result = result.replace("&quot;", "\"");
    result = result.replace("&#39;", "'");
    result = result.replace("&nbsp;", " ");

    // Collapse multiple newlines
    let re_newlines = regex::Regex::new(r"\n{3,}").unwrap();
    result = re_newlines.replace_all(&result, "\n\n").to_string();

    result.trim().to_string()
}
