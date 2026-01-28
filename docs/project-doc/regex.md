# 角色卡正则字段说明

> 基于 `character.data.extensions.regex_scripts` 数据结构

---

## 字段列表

| JSON字段 | 类型 | 中文名称 | 功能说明 |
|----------|------|----------|----------|
| `id` | string | - | 正则脚本唯一标识符 (UUID) |
| `scriptName` | string | 脚本名称 | 正则脚本的名称 |
| `findRegex` | string | **查找正则表达式** | 用于匹配的正则表达式字符串 |
| `replaceString` | string | **替换为** | 替换匹配内容的字符串，支持 `$1`、`$2` 等捕获组引用 |
| `trimStrings` | string[] | **修剪掉** | 从匹配结果中移除的字符串数组（替换前修剪），回车分隔 |
| `placement` | number[] | **作用范围** | 应用位置的数组 |
| `disabled` | boolean | 禁用/启用 | 是否禁用该脚本 |
| `markdownOnly` | boolean | **仅格式显示** | - |
| `promptOnly` | boolean | **仅格式提示词** | - |
| `runOnEdit` | boolean | **在编辑时运行** | 当指定角色的消息被编辑时运行正则脚本 |
| `substituteRegex` | number | **正则表达式查找时的宏** | 参数替换模式 |
| `minDepth` | number | **最小深度** | 仅影响深度至少为 N 级的消息，`0` 为最后一条消息 |
| `maxDepth` | number | **最大深度** | 仅影响深度不超过 N 级的消息 |

---

## 作用范围（placement）值对照

| 值 | 英文 | 中文 |
|----|------|------|
| `1` | User Input | 用户输入 |
| `2` | AI Output | AI输出 |
| `3` | Slash Commands | 快捷命令 |
| `5` | World Info | 世界信息 |
| `6` | Reasoning | 推理 |

**注意**：`minDepth` 和 `maxDepth` 仅计算 WI 条目 `@Depth` 和可用消息（不含隐藏或系统消息）。

---

## substituteRegex 值对照

| 值 | 行为 |
|----|------|
| `0` | 不替换 - 不进行宏替换 |
| `1` | 原始 - 直接替换 |
| `2` | 转义 - 对宏进行转义处理 |

---