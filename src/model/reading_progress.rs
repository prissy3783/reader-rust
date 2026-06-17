use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 阅读进度抽象模型
/// 与 UI 无关，支持导出、导入、合并和冲突解决
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ReadingProgress {
    /// 书籍唯一标识 (book_url 或 name+author)
    pub book_id: String,
    /// 书籍名称
    pub book_name: String,
    /// 作者
    pub author: String,
    /// 当前章节索引
    pub chapter_index: i32,
    /// 当前章节标题
    pub chapter_title: String,
    /// 章节内页码
    pub page_index: i32,
    /// 章节内滚动偏移
    pub scroll_offset: i32,
    /// 阅读位置 (综合定位)
    pub position: i32,
    /// 阅读进度百分比
    pub progress_percent: f64,
    /// 最后阅读时间 (Unix timestamp ms)
    pub last_read_time: i64,
}

/// Legado bookProgress JSON 格式 (兼容层)
/// 字段名使用 camelCase，与 Android Legado 客户端一致
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct LegadoBookProgress {
    #[serde(default)]
    pub dur_chapter_index: Option<i32>,
    #[serde(default)]
    pub dur_chapter_title: Option<String>,
    #[serde(default)]
    pub dur_chapter_pos: Option<i32>,
    #[serde(default)]
    pub dur_chapter_time: Option<i64>,
    #[serde(default)]
    pub book_url: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub total_chapter_num: Option<i32>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub chapter: Option<String>,
}

/// 同步冲突解决结果
#[derive(Debug, Clone)]
pub enum ConflictResolution {
    /// 使用本地进度
    UseLocal,
    /// 使用远程进度
    UseRemote,
    /// 合并结果 (取更新的时间戳)
    Merged(ReadingProgress),
}

impl ReadingProgress {
    /// 从 Book 结构体导出进度
    pub fn from_book(book: &crate::model::book::Book) -> Option<Self> {
        let chapter_index = book.dur_chapter_index?;
        let last_read_time = book.dur_chapter_time.unwrap_or(0);
        Some(Self {
            book_id: if book.book_url.is_empty() {
                format!("{}:{}", book.name, book.author)
            } else {
                book.book_url.clone()
            },
            book_name: book.name.clone(),
            author: book.author.clone(),
            chapter_index,
            chapter_title: book.dur_chapter_title.clone().unwrap_or_default(),
            page_index: 0,
            scroll_offset: book.dur_chapter_pos.unwrap_or(0),
            position: book.dur_chapter_pos.unwrap_or(0),
            progress_percent: 0.0,
            last_read_time,
        })
    }

    /// 导入为 Book 结构体 (更新进度字段)
    pub fn apply_to_book(&self, book: &mut crate::model::book::Book) {
        book.dur_chapter_index = Some(self.chapter_index);
        book.dur_chapter_pos = Some(self.scroll_offset);
        book.dur_chapter_time = Some(self.last_read_time);
        if !self.chapter_title.is_empty() {
            book.dur_chapter_title = Some(self.chapter_title.clone());
        }
    }

    /// 从 Legado bookProgress JSON 解析
    /// 兼容字段名差异，缺失字段使用默认值
    pub fn from_legado_json(json: &[u8]) -> Option<Self> {
        // 优先用 HashMap 方式解析 (最灵活，支持任意字段名映射)
        if let Ok(map) = serde_json::from_slice::<HashMap<String, serde_json::Value>>(json) {
            let progress = Self::from_legado_map(&map);
            if !progress.book_name.is_empty() || progress.chapter_index > 0 {
                return Some(progress);
            }
        }
        // 回退到结构体解析
        if let Ok(legado) = serde_json::from_slice::<LegadoBookProgress>(json) {
            return Some(Self::from_legado(legado));
        }
        None
    }

    /// 从 LegadoBookProgress 转换
    pub fn from_legado(legado: LegadoBookProgress) -> Self {
        let book_id = legado
            .book_url
            .or_else(|| {
                legado
                    .name
                    .as_ref()
                    .zip(legado.author.as_ref())
                    .map(|(n, a)| format!("{}:{}", n, a))
            })
            .unwrap_or_default();

        let book_name = legado
            .name
            .or(legado.title)
            .or(legado.dur_chapter_title.clone())
            .unwrap_or_default();

        let chapter_title = legado
            .chapter
            .or(legado.dur_chapter_title.clone())
            .unwrap_or_default();

        Self {
            book_id,
            book_name,
            author: legado.author.unwrap_or_default(),
            chapter_index: legado.dur_chapter_index.unwrap_or(0),
            chapter_title,
            page_index: 0,
            scroll_offset: legado.dur_chapter_pos.unwrap_or(0),
            position: legado.dur_chapter_pos.unwrap_or(0),
            progress_percent: 0.0,
            last_read_time: legado.dur_chapter_time.unwrap_or(0),
        }
    }

    /// 从 Legado 风格 HashMap 解析 (字段映射)
    fn from_legado_map(map: &HashMap<String, serde_json::Value>) -> Self {
        let get_i32 = |keys: &[&str]| -> i32 {
            for key in keys {
                if let Some(val) = map.get(*key) {
                    if let Some(n) = val.as_i64() {
                        return n as i32;
                    }
                    if let Some(s) = val.as_str() {
                        if let Ok(n) = s.parse::<i32>() {
                            return n;
                        }
                    }
                }
            }
            0
        };

        let get_string = |keys: &[&str]| -> String {
            for key in keys {
                if let Some(val) = map.get(*key) {
                    if let Some(s) = val.as_str() {
                        return s.to_string();
                    }
                }
            }
            String::new()
        };

        let get_i64 = |keys: &[&str]| -> i64 {
            for key in keys {
                if let Some(val) = map.get(*key) {
                    if let Some(n) = val.as_i64() {
                        return n;
                    }
                    if let Some(s) = val.as_str() {
                        if let Ok(n) = s.parse::<i64>() {
                            return n;
                        }
                    }
                }
            }
            0
        };

        let book_name = get_string(&["name", "bookName", "title"]);
        let author = get_string(&["author"]);
        let book_url = get_string(&["bookUrl", "book_url"]);

        Self {
            book_id: if book_url.is_empty() {
                format!("{}:{}", book_name, author)
            } else {
                book_url
            },
            book_name,
            author,
            chapter_index: get_i32(&[
                "durChapterIndex",
                "chapterIndex",
                "curChapterIndex",
                "chapter_index",
            ]),
            chapter_title: get_string(&[
                "durChapterTitle",
                "chapterTitle",
                "curChapterTitle",
                "chapter_title",
            ]),
            page_index: get_i32(&["pageIndex", "page_index", "curPage"]),
            scroll_offset: get_i32(&[
                "durChapterPos",
                "scrollOffset",
                "curChapterPos",
                "scroll_offset",
            ]),
            position: get_i32(&["position", "curPosition"]),
            progress_percent: map
                .get("progressPercent")
                .or_else(|| map.get("progress_percent"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            last_read_time: get_i64(&[
                "durChapterTime",
                "lastReadTime",
                "last_read_time",
                "updateTime",
            ]),
        }
    }

    /// 导出为 Legado 兼容 JSON
    pub fn to_legado_json(&self) -> serde_json::Value {
        serde_json::json!({
            "durChapterIndex": self.chapter_index,
            "durChapterTitle": self.chapter_title,
            "durChapterPos": self.scroll_offset,
            "durChapterTime": self.last_read_time,
            "bookUrl": self.book_id,
            "name": self.book_name,
            "author": self.author,
        })
    }

    /// 冲突解决: 比较两个进度，返回较新的那个
    /// 策略: last_read_time 优先
    pub fn resolve_conflict(local: &Self, remote: &Self) -> ConflictResolution {
        if local.last_read_time > remote.last_read_time {
            ConflictResolution::UseLocal
        } else if remote.last_read_time > local.last_read_time {
            ConflictResolution::UseRemote
        } else {
            // 时间相同，比较章节索引
            if local.chapter_index >= remote.chapter_index {
                ConflictResolution::UseLocal
            } else {
                ConflictResolution::UseRemote
            }
        }
    }

    /// 合并多个进度记录，返回最新的
    pub fn merge(progress_list: Vec<Self>) -> Option<Self> {
        progress_list
            .into_iter()
            .max_by_key(|p| (p.last_read_time, p.chapter_index))
    }

    /// 生成 Legado 兼容的文件名
    /// 格式: {书名}_{作者}.json
    pub fn legado_filename(&self) -> String {
        let safe_name = self.book_name.replace(['/', '\\'], "_");
        let safe_author = self.author.replace(['/', '\\'], "_");
        if safe_author.is_empty() {
            format!("{}.json", safe_name)
        } else {
            format!("{}_{}.json", safe_name, safe_author)
        }
    }

    /// 从 Legado 文件名解析书名和作者
    pub fn parse_legado_filename(filename: &str) -> (String, String) {
        let name = filename.strip_suffix(".json").unwrap_or(filename);
        if let Some(idx) = name.rfind('_') {
            let book_name = name[..idx].to_string();
            let author = name[idx + 1..].to_string();
            (book_name, author)
        } else {
            (name.to_string(), String::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_book() {
        let book = crate::model::book::Book {
            name: "测试书籍".to_string(),
            author: "测试作者".to_string(),
            book_url: "https://example.com/book/1".to_string(),
            origin: "https://example.com".to_string(),
            dur_chapter_index: Some(10),
            dur_chapter_pos: Some(500),
            dur_chapter_time: Some(1718640000000),
            dur_chapter_title: Some("第十章".to_string()),
            ..Default::default()
        };
        let progress = ReadingProgress::from_book(&book).unwrap();
        assert_eq!(progress.chapter_index, 10);
        assert_eq!(progress.scroll_offset, 500);
        assert_eq!(progress.last_read_time, 1718640000000);
        assert_eq!(progress.chapter_title, "第十章");
    }

    #[test]
    fn test_from_legado_json() {
        let json = r#"{
            "durChapterIndex": 42,
            "durChapterTitle": "第四十二章",
            "durChapterPos": 1024,
            "durChapterTime": 1718640000000,
            "name": "学霸的军工科研系统",
            "author": "十月廿二"
        }"#;
        let progress = ReadingProgress::from_legado_json(json.as_bytes()).unwrap();
        assert_eq!(progress.chapter_index, 42);
        assert_eq!(progress.chapter_title, "第四十二章");
        assert_eq!(progress.scroll_offset, 1024);
        assert_eq!(progress.book_name, "学霸的军工科研系统");
        assert_eq!(progress.author, "十月廿二");
    }

    #[test]
    fn test_from_legado_map_flexible() {
        let json = r#"{
            "curChapterIndex": 5,
            "curChapterTitle": "第五章",
            "curChapterPos": 200,
            "bookUrl": "https://example.com/book/1",
            "bookName": "测试书"
        }"#;
        let progress = ReadingProgress::from_legado_json(json.as_bytes()).unwrap();
        assert_eq!(progress.chapter_index, 5);
        assert_eq!(progress.scroll_offset, 200);
        assert_eq!(progress.book_id, "https://example.com/book/1");
    }

    #[test]
    fn test_conflict_resolution() {
        let local = ReadingProgress {
            last_read_time: 1718640000000,
            chapter_index: 100,
            ..Default::default()
        };
        let remote = ReadingProgress {
            last_read_time: 1718643600000,
            chapter_index: 120,
            ..Default::default()
        };
        match ReadingProgress::resolve_conflict(&local, &remote) {
            ConflictResolution::UseRemote => {}
            _ => panic!("remote should win (newer timestamp)"),
        }
    }

    #[test]
    fn test_merge() {
        let progresses = vec![
            ReadingProgress {
                last_read_time: 100,
                chapter_index: 5,
                ..Default::default()
            },
            ReadingProgress {
                last_read_time: 200,
                chapter_index: 10,
                ..Default::default()
            },
            ReadingProgress {
                last_read_time: 150,
                chapter_index: 15,
                ..Default::default()
            },
        ];
        let merged = ReadingProgress::merge(progresses).unwrap();
        // last_read_time=200 wins
        assert_eq!(merged.last_read_time, 200);
        assert_eq!(merged.chapter_index, 10);
    }

    #[test]
    fn test_legado_filename() {
        let p = ReadingProgress {
            book_name: "学霸的军工科研系统".to_string(),
            author: "十月廿二".to_string(),
            ..Default::default()
        };
        assert_eq!(p.legado_filename(), "学霸的军工科研系统_十月廿二.json");
    }

    #[test]
    fn test_parse_legado_filename() {
        let (name, author) =
            ReadingProgress::parse_legado_filename("学霸的军工科研系统_十月廿二.json");
        assert_eq!(name, "学霸的军工科研系统");
        assert_eq!(author, "十月廿二");
    }

    #[test]
    fn test_to_legado_json_roundtrip() {
        let original = ReadingProgress {
            book_id: "https://example.com/book/1".to_string(),
            book_name: "测试书".to_string(),
            author: "作者".to_string(),
            chapter_index: 42,
            chapter_title: "第四十二章".to_string(),
            scroll_offset: 512,
            last_read_time: 1718640000000,
            ..Default::default()
        };
        let json = original.to_legado_json();
        let restored = ReadingProgress::from_legado_json(json.to_string().as_bytes()).unwrap();
        assert_eq!(restored.chapter_index, 42);
        assert_eq!(restored.scroll_offset, 512);
        assert_eq!(restored.last_read_time, 1718640000000);
    }
}
