# compat_report.md — Legado 兼容性评估（更新版）

## 一、搜索评分兼容性

| 功能 | Legado | reader-rust | 状态 |
|---|---|---|---|
| 搜索结果排序 | 按返回顺序 | ✅ 按评分降序 | 已增强 |
| 书名匹配检测 | 无 | ✅ 精准/相似/弱匹配 | 已新增 |
| 作者匹配检测 | 无 | ✅ 精准/标准化匹配 | 已新增 |
| 匹配标签显示 | 无 | ✅ 精准匹配/书名相似/作者匹配/弱匹配 | 已新增 |
| 书源名称显示 | URL | ✅ 从域名提取名称 | 已新增 |

## 二、内容质量检测

| 功能 | Legado | reader-rust | 状态 |
|---|---|---|---|
| 目录检测 | 无 | ✅ is_catalog_like() | 已实现 |
| JS 规则检测 | 无 | ✅ contains_js_rule_text() | 已新增 |
| 正文长度检测 | 无 | ✅ SuspectTooShort | 已新增 |
| 空内容检测 | ContentEmptyException | ✅ AppError | 已对齐 |

## 三、已有功能（无需修改）

| 功能 | 状态 |
|---|---|
| WebDAV 同步 | ✅ 已超越（冲突解决 + 原子写入） |
| 阅读进度同步 | ✅ 已对齐（durChapterPos=0 + getBookContent 保存） |
| 内容提取管线 | ✅ 已对齐（sourceRegex → webJs → content → replaceRegex） |
| HTML 清理 | ✅ 已对齐（format_keep_img） |
| 书源测速 | ✅ 已对齐（SSE + 延迟显示） |
| 书源解析增强 | ✅ 已增强（bookSourceType + 规则语法） |

## 四、总体评级

| 领域 | 评级 |
|---|---|
| 搜索评分 | A+（新增智能评分） |
| 内容质量检测 | A（新增检测器） |
| WebDAV 同步 | A+（已超越） |
| 内容提取 | A（已对齐） |
| 书源测速 | A（已对齐） |
| **总体** | **A** |
