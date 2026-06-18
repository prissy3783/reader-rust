# source_health_report.md — 书源健康状态报告

## 正文质量检测器

### 检测类型

| 类型 | 标签 | 检测逻辑 |
|---|---|---|
| Normal | ✓ 正文可读 | 内容正常，无异常特征 |
| SuspectCatalog | ⚠ 正文疑似目录 | is_catalog_like() 检测到目录关键词 |
| SuspectJsRuleNotExecuted | ⚠ JS规则未执行 | 内容以 @js: / eval( / function( 开头 |
| SuspectTooShort | ⚠ 正文长度异常 | 字符数 < 50 且行数 < 3 |
| Empty | ✗ 内容为空 | 内容为空或仅空白 |

### 检测逻辑

1. **目录检测**: `is_catalog_like()` — 标题密度 > 15% 或短行比例 > 70%
2. **JS 规则检测**: 逐行检查是否以 `@js:` / `eval(` / `function(` 等开头
3. **长度检测**: 字符数 < 50 且行数 < 3
4. **空内容检测**: 内容为空或仅空白

### 集成位置

- `src/parser/rule_engine.rs` 的 `content()` 方法中，HTML 清理后输出质量分析日志
- 启用 `LOG_LEVEL=debug` 时输出 `[ContentDebug] content_quality=XXX`

## 实现文件

| 文件 | 变更 |
|---|---|
| `src/service/content_quality.rs` | ContentQuality 枚举 + analyze_content_quality() |
| `src/parser/rule_engine.rs` | content pipeline 集成质量分析 |
