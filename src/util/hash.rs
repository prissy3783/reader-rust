use md5::{Digest, Md5};
use sha2::{Sha256};

pub fn md5_hex(input: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

pub fn book_id_from_url(url: &str) -> String {
    let normalized = normalize_book_url(url);
    sha256_hex(&normalized)
}

pub fn book_id_fallback(book_source: &str, book_name: &str) -> String {
    let combined = format!("{}:{}", book_source, book_name);
    sha256_hex(&combined)
}

fn normalize_book_url(url: &str) -> String {
    let mut s = url.trim().to_string();
    s = s.trim_end_matches('/').to_string();
    if let Some(idx) = s.find('#') {
        s.truncate(idx);
    }
    s = s.trim_end_matches('/').to_string();
    s.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_hex() {
        let hash = sha256_hex("hello");
        assert_eq!(hash.len(), 64);
        assert_eq!(hash, "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
    }

    #[test]
    fn test_book_id_from_url() {
        let id1 = book_id_from_url("https://example.com/book/1");
        let id2 = book_id_from_url("https://example.com/book/1");
        assert_eq!(id1, id2);

        // Different URLs produce different IDs
        let id3 = book_id_from_url("https://example.com/book/2");
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_book_id_url_normalization() {
        // Trailing slash should be normalized
        let id1 = book_id_from_url("https://example.com/book/1");
        let id2 = book_id_from_url("https://example.com/book/1/");
        assert_eq!(id1, id2);

        // Hash fragment should be stripped
        let id3 = book_id_from_url("https://example.com/book/1#section");
        assert_eq!(id1, id3);

        // Case normalization
        let id4 = book_id_from_url("HTTPS://EXAMPLE.COM/book/1");
        assert_eq!(id1, id4);
    }

    #[test]
    fn test_book_id_fallback() {
        let id = book_id_fallback("https://source.com", "测试书");
        assert_eq!(id.len(), 64);
    }

    #[test]
    fn test_normalize_book_url() {
        assert_eq!(normalize_book_url("https://example.com/book/1/"), "https://example.com/book/1");
        assert_eq!(normalize_book_url("HTTPS://Example.COM/Book/1"), "https://example.com/book/1");
        assert_eq!(normalize_book_url("https://example.com/book/1#section"), "https://example.com/book/1");
    }
}
