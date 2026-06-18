# compat_report.md — Legado 兼容性评估

## 一、正文提取兼容性

### 已对齐的 Legado 行为

| 行为 | Legado | reader-rust | 状态 |
|---|---|---|---|
| sourceRegex 预处理 | ✅ | ✅ | 已对齐 |
| webJs 预处理 | ✅ | ✅ | 已对齐 |
| CSS/JSONPath/XPath/Regex/JS 解析模式 | ✅ | ✅ | 已对齐 |
| `@` 链式选择器 | ✅ | ✅ | 已修复(P0) |
| replaceRegex 后处理 | ✅ | ✅ | 已对齐 |
| 多页正文分页 (nextContentUrl) | ✅ 串行+循环检测 | ✅ 串行+循环检测 | 已对齐 |
| 卷章跳过 (isVolume) | ✅ 跳过正文提取 | ⚠️ 字段存在但未检查 | 见下方分析 |
| 空内容异常 | ✅ ContentEmptyException | ✅ AppError | 已对齐(P1) |
| HTML 保留图片 | ✅ formatKeepImg | ✅ format_keep_img | 已对齐(P2) |

### 卷章跳过分析

| 方面 | Legado | reader-rust |
|---|---|---|
| `is_volume` 字段 | ✅ BookChapter.isVolume | ✅ BookChapter.is_volume |
| 识别方式 | TocRule.isVolume 规则 | TocRule.is_volume 规则 |
| content() 中检查 | ✅ `if(bookChapter.isVolume) return ""` | ❌ 未检查 |
| 合成 URL | ✅ title+index 生成占位 URL | ✅ finalize_chapter_url() |
| **结论** | 字段和识别逻辑已对齐，仅 content() 中缺少跳过检查 | **风险低，建议后续实现** |

### 多页正文分析

| 方面 | Legado | reader-rust |
|---|---|---|
| 串行分页 | ✅ while 循环 | ✅ loop + visited_urls |
| 并发分页 | ✅ 多 URL 时 async | ❌ 仅串行 |
| 循环检测 | ✅ nextUrlList.contains | ✅ visited_urls HashSet |
| 同章判断 | ✅ URL path 比较 | ✅ should_follow_content_page() |
| **结论** | 大部分书源是链式分页（第1页→第2页），并发不适用 | **当前串行实现正确，无需并发** |

## 二、JS 引擎兼容性

### 已实现的 JS API

| API | Legado | reader-rust | 说明 |
|---|---|---|---|
| `java.ajax(url)` | ✅ | ✅ | HTTP GET |
| `java.get(url)` | ✅ | ✅ | HTTP GET |
| `java.post(url, body)` | ✅ | ✅ | HTTP POST |
| `java.md5Encode(str)` | ✅ | ✅ | MD5 哈希 |
| `java.base64Encode/Decode` | ✅ | ✅ | Base64 |
| `java.timeFormat(ts)` | ✅ | ✅ | 时间格式化 |
| `java.now()` | ✅ | ✅ | 当前时间戳 |
| `java.uuid()` | ✅ | ✅ | UUID 生成 |
| `java.ajaxAll(urls)` | ✅ | ✅ | 并发 HTTP |
| `java.connect(url, header)` | ✅ | ✅ | 自定义 header |
| `java.importScript(path)` | ✅ | ✅ | 导入脚本 |
| `java.readTxtFile(path)` | ✅ | ✅ | 读取文件 |
| `java.writeFile(path, content)` | ✅ | ✅ | 写入文件 |
| `java.deleteFile(path)` | ✅ | ✅ | 删除文件 |
| `java.aesBase64DecodeToString` | ✅ | ✅ | AES 解密 |
| `java.desDecodeToString` | ✅ | ✅ | DES 解密 |
| `java.tripleDESDecodeStr` | ✅ | ✅ | 3DES 解密 |
| `java.getCookie(tag, key)` | ✅ | ✅ | Cookie 读取 |
| `java.setCookie(tag, key, val)` | ✅ | ✅ | Cookie 写入 |
| `java.htmlFormat(str)` | ✅ | ⚠️ stub | 仅返回原值 |
| `java.utf8ToGbk(str)` | ✅ | ⚠️ stub | 仅返回原值 |

### 未实现的 Legado JS API

| API | 影响 | 优先级 |
|---|---|---|
| `java.queryTTF(font)` | 字体反爬 | P3 |
| `java.replaceFont(text, font1, font2)` | 字体替换 | P3 |
| `java.toast(msg)` | UI 提示 | P4 |
| `java.logType(any)` | 类型检测 | P4 |

## 三、规则解析兼容性

| 规则类型 | Legado | reader-rust | 状态 |
|---|---|---|---|
| CSS 选择器 | ✅ | ✅ | 已对齐 |
| JSONPath | ✅ | ✅ | 已对齐 |
| XPath | ✅ | ✅ | 已对齐 |
| Regex 列表 | ✅ | ✅ | 已对齐 |
| JS 规则 | ✅ | ✅ | 已对齐 |
| `@` 链式选择 | ✅ | ✅ | 已修复 |
| `@@` 二次解析 | ✅ | ✅ | 已对齐 |
| `&&` `||` `%%` 组合 | ✅ | ✅ | 已对齐 |
| `@put/@get` 变量 | ✅ | ✅ | 已对齐 |
| `{{...}}` 内联 JS | ✅ | ✅ | 已对齐 |
| `-` 反转 / `+` 保留 | ✅ | ✅ | 已对齐 |
| `#`→`##` 规则语法 | ✅ | ✅ | 已增强 |
- `|`→`||` 规则语法 | ✅ | ✅ | 已增强 |

## 四、WebDAV 进度同步兼容性

| 功能 | Legado | reader-rust | 状态 |
|---|---|---|---|
| 进度保存路径 | `bookProgress/` | `legado/backgroundd/bookProgress/` | ✅ 兼容 |
| 进度文件格式 | `{name, author, durChapterIndex, durChapterPos:0, durChapterTime, durChapterTitle}` | 相同格式 | ✅ 已对齐 |
| 每次阅读保存 | ✅ getBookContent 也保存 | ✅ getBookContent 也保存 | ✅ 已对齐 |
| WebDAV PUT 同步 | ✅ 检测 /bookProgress/ | ✅ 检测 /bookProgress/ | ✅ 已对齐 |
| 冲突解决 | ❌ 直接覆盖 | ✅ last_read_time 优先 | reader-rust 更强 |
| 原子写入 | ❌ | ✅ | reader-rust 更强 |
| SHA256 BookID | ❌ | ✅ | reader-rust 更强 |

## 五、已知限制

| 限制 | 原因 | 影响 |
|---|---|---|
| QuickJS vs GraalJS | 架构差异，无法消除 | 少量 Java 高级 API 不可用 |
| 字体反爬 | 需 TTF 解析库 | 极少书源使用 |
| `htmlFormat` stub | 未实现 HTML 格式化 | 少量书源使用 |
| `utf8ToGbk` stub | 未实现编码转换 | 少量 GBK 书源 |

## 六、兼容性评级

| 领域 | 评级 |
|---|---|
| 正文提取 | A (已修复核心问题) |
| JS API | A- (主流 API 已覆盖) |
| 规则解析 | A (完全对齐) |
| WebDAV 进度同步 | A+ (reader-rust 更强) |
| **总体** | **A** |
