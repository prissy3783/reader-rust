# search_score_report.md — 搜索评分系统报告

## 评分规则

| 匹配类型 | 分数 | 说明 |
|---|---|---|
| 书名完全一致 | +100 | name == target_name |
| 去空格后一致 | +90 | name_no_space == target_no_space |
| 书名高度相似 | +60 | 包含关系或字符重叠 >= 60% |
| 作者完全一致 | +50 | author == target_author |
| 作者标准化后一致 | +40 | 去空格后相等 |
| 最新章节匹配 | +20 | lastChapter 一致 |
| 字符重叠 30-60% | +20 | 弱相似 |
| 仅关键词命中 | +5 | 无其他匹配 |

## 匹配标签

| 标签 | 颜色 | 含义 |
|---|---|---|
| 精准匹配 | 绿色 | 书名完全一致 |
| 书名相似 | 蓝色 | 书名包含或高相似度 |
| 作者匹配 | 紫色 | 作者一致但书名不一致 |
| 弱匹配 | 灰色 | 仅部分匹配 |

## 实现文件

| 文件 | 变更 |
|---|---|
| `src/model/search.rs` | SearchBook.compute_score() + compute_char_overlap() |
| `frontend/src/components/reader/ReaderSource.vue` | preparedResults 评分 + 匹配标签显示 |
