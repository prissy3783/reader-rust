# source_display_report.md — 换源弹窗显示修复报告

## 问题根因

**数据流追踪结果**：

1. `getAvailableBookSourceSSE` 从 `sources: Vec<BookSource>` 取出每个源
2. 调用 `svc.search_book()` 返回 `Vec<SearchBook>`
3. `SearchBook` 结构体**无 `bookSourceName` 字段**，只有 `origin`（URL）
4. SSE payload 发送 `SearchBook` 对象
5. 前端 `ReaderSource.vue` 显示 `item.book.origin`（URL）

**根因**：`SearchBook` 缺少 `bookSourceName`，搜索结果不携带书源名称。

## 修改内容

| 文件 | 变更 |
|---|---|
| `src/model/search.rs` | `SearchBook` 加 `book_source_name: Option<String>` |
| `src/api/handlers/book.rs` | SSE 搜索结果从 `sources[cur_idx].book_source_name` 注入 |
| `frontend/src/components/reader/ReaderSource.vue` | 主标题显示 `bookSourceName`，URL 降为副标题 |

## 修改前后效果

### 修改前
```
主标题: http://wap.wangshugu.org
副标题: (无)
```

### 修改后
```
主标题: 望书阁
副标题: http://wap.wangshugu.org
```

## 兼容性影响

- **后端**: `SearchBook` 新增 `book_source_name` 字段，`skip_serializing_if = "None"` 保证旧客户端不受影响
- **前端**: 优先显示 `bookSourceName`，无则回退到 `getSourceDisplayName(origin)`
- **API**: 新增字段在 JSON 中为可选，不影响现有调用方

## 验证结果

| 步骤 | 状态 |
|---|---|
| 98/98 测试通过 | ✅ |
| cargo fmt + clippy 干净 | ✅ |
| ARM64 编译 | ✅ |
| NAS 部署 (June 18 20:21) | ✅ |
| GitHub push (`11f7754`) | ✅ |
