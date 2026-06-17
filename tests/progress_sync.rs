use reader_rust::model::book::Book;
use reader_rust::model::reading_progress::{ConflictResolution, ReadingProgress};

fn make_book(name: &str, author: &str, chapter: i32, pos: i32, time: i64) -> Book {
    Book {
        name: name.to_string(),
        author: author.to_string(),
        book_url: format!("https://example.com/{}", name),
        origin: "https://example.com".to_string(),
        dur_chapter_index: Some(chapter),
        dur_chapter_pos: Some(pos),
        dur_chapter_time: Some(time),
        dur_chapter_title: Some(format!("第{}章", chapter)),
        ..Default::default()
    }
}

fn make_progress(name: &str, author: &str, chapter: i32, pos: i32, time: i64) -> ReadingProgress {
    let url = format!("https://example.com/{}", name);
    ReadingProgress {
        book_id: reader_rust::util::hash::book_id_from_url(&url),
        book_name: name.to_string(),
        author: author.to_string(),
        chapter_index: chapter,
        chapter_title: format!("第{}章", chapter),
        page_index: 0,
        scroll_offset: pos,
        position: pos,
        progress_percent: 0.0,
        last_read_time: time,
    }
}

#[test]
fn test_single_device_sync_roundtrip() {
    let book = make_book("测试书", "作者A", 10, 500, 1718640000000);
    let progress = ReadingProgress::from_book(&book).unwrap();
    let json = progress.to_legado_json();
    let restored = ReadingProgress::from_legado_json(json.to_string().as_bytes()).unwrap();

    assert_eq!(restored.chapter_index, 10);
    assert_eq!(restored.scroll_offset, 500);
    assert_eq!(restored.last_read_time, 1718640000000);
    assert_eq!(restored.book_name, "测试书");
    assert_eq!(restored.author, "作者A");
}

#[test]
fn test_dual_device_sync_remote_wins() {
    let local = make_progress("学霸的军工科研系统", "十月廿二", 100, 200, 1718640000000);
    let remote = make_progress("学霸的军工科研系统", "十月廿二", 120, 300, 1718643600000);

    match ReadingProgress::resolve_conflict(&local, &remote) {
        ConflictResolution::UseRemote => {}
        _ => panic!("remote should win (newer timestamp)"),
    }
}

#[test]
fn test_dual_device_sync_local_wins() {
    let local = make_progress("学霸的军工科研系统", "十月廿二", 120, 300, 1718643600000);
    let remote = make_progress("学霸的军工科研系统", "十月廿二", 100, 200, 1718640000000);

    match ReadingProgress::resolve_conflict(&local, &remote) {
        ConflictResolution::UseLocal => {}
        _ => panic!("local should win (newer timestamp)"),
    }
}

#[test]
fn test_conflict_same_time_higher_chapter_wins() {
    let local = make_progress("书A", "作者A", 150, 0, 1718640000000);
    let remote = make_progress("书A", "作者A", 120, 0, 1718640000000);

    match ReadingProgress::resolve_conflict(&local, &remote) {
        ConflictResolution::UseLocal => {}
        _ => panic!("local should win (same time, higher chapter)"),
    }
}

#[test]
fn test_missing_directory_handled() {
    let progress = make_progress("测试书", "作者", 5, 100, 1718640000000);
    let filename = progress.legado_filename();
    assert!(filename.ends_with(".json"));
    assert!(filename.contains("测试书"));
    assert!(filename.contains("作者"));
}

#[test]
fn test_empty_book_progress_json() {
    let json = r#"{}"#;
    let progress = ReadingProgress::from_legado_json(json.as_bytes());
    // Empty JSON should still parse (all defaults)
    assert!(progress.is_some());
    let p = progress.unwrap();
    assert_eq!(p.chapter_index, 0);
    assert_eq!(p.book_name, "");
}

#[test]
fn test_corrupted_json_returns_none() {
    let corrupted = b"this is not json at all {{{";
    let result = ReadingProgress::from_legado_json(corrupted);
    assert!(result.is_none());
}

#[test]
fn test_old_zip_without_book_progress() {
    let legacy_json = r#"{
        "durChapterIndex": 5,
        "durChapterTitle": "第五章",
        "durChapterPos": 200,
        "durChapterTime": 1718640000000,
        "name": "旧书",
        "author": "旧作者"
    }"#;
    let progress = ReadingProgress::from_legado_json(legacy_json.as_bytes()).unwrap();
    assert_eq!(progress.chapter_index, 5);
    assert_eq!(progress.book_name, "旧书");
}

#[test]
fn test_legado_field_mapping_dur_chapter_index() {
    let json = r#"{"durChapterIndex": 42}"#;
    let p = ReadingProgress::from_legado_json(json.as_bytes()).unwrap();
    assert_eq!(p.chapter_index, 42);
}

#[test]
fn test_legado_field_mapping_cur_chapter_index() {
    let json = r#"{"curChapterIndex": 7}"#;
    let p = ReadingProgress::from_legado_json(json.as_bytes()).unwrap();
    assert_eq!(p.chapter_index, 7);
}

#[test]
fn test_legado_field_mapping_chapter_index() {
    let json = r#"{"chapterIndex": 3}"#;
    let p = ReadingProgress::from_legado_json(json.as_bytes()).unwrap();
    assert_eq!(p.chapter_index, 3);
}

#[test]
fn test_legado_filename_roundtrip() {
    let progress = make_progress("学霸的军工科研系统", "十月廿二", 42, 1024, 1718640000000);
    let filename = progress.legado_filename();
    assert_eq!(filename, "学霸的军工科研系统_十月廿二.json");

    let (name, author) = ReadingProgress::parse_legado_filename(&filename);
    assert_eq!(name, "学霸的军工科研系统");
    assert_eq!(author, "十月廿二");
}

#[test]
fn test_merge_multiple_progresses() {
    let progresses = vec![
        make_progress("书A", "作者", 10, 0, 100),
        make_progress("书A", "作者", 20, 0, 300),
        make_progress("书A", "作者", 15, 0, 200),
    ];
    let merged = ReadingProgress::merge(progresses).unwrap();
    assert_eq!(merged.last_read_time, 300);
    assert_eq!(merged.chapter_index, 20);
}

#[test]
fn test_apply_to_book() {
    let mut book = make_book("测试书", "作者", 5, 100, 1000);
    let progress = make_progress("测试书", "作者", 10, 200, 2000);
    progress.apply_to_book(&mut book);

    assert_eq!(book.dur_chapter_index, Some(10));
    assert_eq!(book.dur_chapter_pos, Some(200));
    assert_eq!(book.dur_chapter_time, Some(2000));
    assert_eq!(book.dur_chapter_title, Some("第10章".to_string()));
}

#[test]
fn test_legado_full_json_parse() {
    let json = r#"{
        "durChapterIndex": 523,
        "durChapterTitle": "第五百二十三章",
        "durChapterPos": 0,
        "durChapterTime": 1718640000000,
        "bookUrl": "https://example.com/book/123",
        "name": "学霸的军工科研系统",
        "author": "十月廿二",
        "total_chapterNum": 1000
    }"#;
    let p = ReadingProgress::from_legado_json(json.as_bytes()).unwrap();
    assert_eq!(p.chapter_index, 523);
    assert_eq!(p.chapter_title, "第五百二十三章");
    assert_eq!(p.scroll_offset, 0);
    assert_eq!(p.last_read_time, 1718640000000);
    // book_id is SHA256(normalized_url)
    let expected_id = reader_rust::util::hash::book_id_from_url("https://example.com/book/123");
    assert_eq!(p.book_id, expected_id);
    assert_eq!(p.book_name, "学霸的军工科研系统");
    assert_eq!(p.author, "十月廿二");
}

#[test]
fn test_partial_legado_json_missing_fields() {
    let json = r#"{
        "durChapterIndex": 10,
        "name": "部分字段书"
    }"#;
    let p = ReadingProgress::from_legado_json(json.as_bytes()).unwrap();
    assert_eq!(p.chapter_index, 10);
    assert_eq!(p.book_name, "部分字段书");
    assert_eq!(p.author, "");
    assert_eq!(p.scroll_offset, 0);
    assert_eq!(p.last_read_time, 0);
}

#[test]
fn test_debug_log_output() {
    let progress = make_progress("调试书", "调试作者", 1, 0, 1000);
    let json = progress.to_legado_json();
    let json_str = serde_json::to_string_pretty(&json).unwrap();
    assert!(json_str.contains("durChapterIndex"));
    assert!(json_str.contains("调试书"));
}
