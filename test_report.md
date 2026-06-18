# test_report.md — 正文提取强化测试报告

## 测试时间
2026-06-18

## 测试结果

### 单元测试 (76/76 通过)
- `cargo test --lib` ✅

### 集成测试 (17/17 通过)
- `cargo test --test progress_sync` ✅

### 静态分析
- `cargo fmt --check` ✅
- `cargo clippy --lib` ✅

### ARM64 编译
- `cross build --release --target aarch64-unknown-linux-musl` ✅

### Docker 构建
- `docker buildx build --platform linux/arm64` ✅

### NAS 部署
- `docker compose pull && up -d` ✅
- Health check: `{"isSuccess":true}` ✅

## 修改内容

| 步骤 | 文件 | 变更 |
|---|---|---|
| P0 | `src/parser/rule_engine.rs` | content() 方法加 [ContentDebug] 调试日志 |
| P0 | `src/service/book_service.rs` | get_content() 加详细调试日志 |
| P1 | `src/api/handlers/book.rs` | 空内容返回 AppError 而非空字符串 |
| P2 | `src/parser/html_clean.rs` | 新增 HTML 清理模块（保留图片） |
| P2 | `src/parser/mod.rs` | 注册 html_clean 模块 |
| P2 | `src/parser/rule_engine.rs` | content pipeline 加 HTML 清理调用 |
| 分析 | book_chapter.rs | is_volume 字段已存在 |
| 分析 | book_service.rs | 多页正文已有串行分页+循环检测 |

## 调试日志输出示例

启用 `LOG_LEVEL=debug` 后，正文提取链路输出：
```
[ContentDebug] get_content called, chapter_url=..., book_key=..., source=...
[ContentDebug] cache MISS, fetching from network
[ContentDebug] fetching: https://...
[ContentDebug] fetch OK: status=200, redirect_url=..., html_len=45000, elapsed=320ms
[ContentDebug] mode=Css, selector=id.chaptercontent@textNodes
[ContentDebug] CSS selector='id.chaptercontent@textNodes', extracted_len=12000
[ContentDebug] extracted content len=12000, is_catalog=false
[ContentDebug] htmlClean: before_len=12000, after_len=11800
[ContentDebug] replaceRegex: before_len=11800, after_len=11500
[ContentDebug] final content len=11500, empty=false
```
