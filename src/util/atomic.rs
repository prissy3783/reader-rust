use std::path::Path;
use tokio::fs;

pub async fn write_atomic(path: &Path, data: &[u8]) -> Result<(), std::io::Error> {
    let parent = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(parent).await?;

    let mut tmp_path = path.to_path_buf();
    tmp_path.set_extension("tmp");

    fs::write(&tmp_path, data).await?;

    #[cfg(windows)]
    {
        let _ = fs::remove_file(path).await;
    }

    fs::rename(&tmp_path, path).await?;

    Ok(())
}

pub async fn write_atomic_string(path: &Path, data: &str) -> Result<(), std::io::Error> {
    write_atomic(path, data.as_bytes()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[tokio::test]
    async fn test_write_atomic_creates_file() {
        let dir = temp_dir().join("reader_test_atomic");
        let _ = fs::create_dir_all(&dir).await;
        let path = dir.join("test.txt");

        write_atomic(&path, b"hello world").await.unwrap();
        let content = fs::read_to_string(&path).await.unwrap();
        assert_eq!(content, "hello world");

        let _ = fs::remove_file(&path).await;
        let _ = fs::remove_dir(&dir).await;
    }

    #[tokio::test]
    async fn test_write_atomic_overwrites_existing() {
        let dir = temp_dir().join("reader_test_atomic2");
        let _ = fs::create_dir_all(&dir).await;
        let path = dir.join("test.txt");

        write_atomic(&path, b"first").await.unwrap();
        write_atomic(&path, b"second").await.unwrap();
        let content = fs::read_to_string(&path).await.unwrap();
        assert_eq!(content, "second");

        let _ = fs::remove_file(&path).await;
        let _ = fs::remove_dir(&dir).await;
    }

    #[tokio::test]
    async fn test_write_atomic_creates_parent_dirs() {
        let dir = temp_dir().join("reader_test_atomic3").join("nested");
        let path = dir.join("test.txt");

        write_atomic(&path, b"nested").await.unwrap();
        let content = fs::read_to_string(&path).await.unwrap();
        assert_eq!(content, "nested");

        let _ = fs::remove_file(&path).await;
        let _ = fs::remove_dir_all(temp_dir().join("reader_test_atomic3")).await;
    }

    #[tokio::test]
    async fn test_write_atomic_string() {
        let dir = temp_dir().join("reader_test_atomic4");
        let _ = fs::create_dir_all(&dir).await;
        let path = dir.join("test.json");

        write_atomic_string(&path, r#"{"key":"value"}"#).await.unwrap();
        let content = fs::read_to_string(&path).await.unwrap();
        assert_eq!(content, r#"{"key":"value"}"#);

        let _ = fs::remove_file(&path).await;
        let _ = fs::remove_dir(&dir).await;
    }
}
