# SillyTavern 角色卡完整字段文档

本文档详细列出 SillyTavern 角色卡（Character Card）中可读取和设置的所有字段，包括内置的世界书（World Info / Lorebook）和正则脚本（Regex Scripts）。

> **版本说明**：当前主流角色卡使用 **V3 规范**（`spec: "chara_card_v3"`，`spec_version: "3.0"`）。V3 是 V2 的扩展版本，`data` 对象结构沿用 V2 定义并增加了更多扩展字段。

---

## 目录

1. [角色卡顶层字段](#1-角色卡顶层字段)
2. [角色卡数据字段（data）](#2-角色卡数据字段data)
3. [扩展字段（extensions）](#3-扩展字段extensions)
4. [世界书 / 角色书（character_book）](#4-世界书--角色书character_book)
5. [世界书条目（WorldInfoEntry）](#5-世界书条目worldinfoentry)
6. [世界书条目扩展字段（entry.extensions）](#6-世界书条目扩展字段entryextensions)
7. [正则脚本（regex_scripts）](#7-正则脚本regex_scripts)
8. [完整 JSON 结构示例](#8-完整-json-结构示例)
9. [PNG 元数据存储](#9-png-元数据存储)
10. [开发注意事项](#10-开发注意事项)

---

## 1. 角色卡顶层字段

这是角色卡 JSON 文件的最外层结构（V3 规范）：

| 字段名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| `name` | `string` | ✅ | 角色名称 |
| `description` | `string` | ✅ | 角色描述（角色定义、人设描述） |
| `personality` | `string` | ❌ | 角色性格简述 |
| `scenario` | `string` | ❌ | 场景/背景描述 |
| `first_mes` | `string` | ✅ | 第一条消息（开场白） |
| `mes_example` | `string` | ❌ | 对话示例 |
| `creatorcomment` | `string` | ❌ | 创作者备注（等同于 `data.creator_notes`） |
| `avatar` | `string` | ❌ | 头像文件名（作为唯一标识符） |
| `talkativeness` | `string` \| `number` | ❌ | 健谈度（0-1 之间的数值） |
| `fav` | `boolean` \| `string` | ❌ | 是否收藏 |
| `tags` | `string[]` | ❌ | 标签列表 |
| `spec` | `string` | ✅ | 规范版本标识（如 `"chara_card_v3"`） |
| `spec_version` | `string` | ✅ | 规范版本号（如 `"3.0"`） |
| `create_date` | `string` | ❌ | 创建日期 |
| `data` | `v2CharData` | ✅ | V2 版本的完整数据对象 |

---

## 2. 角色卡数据字段（data）

`data` 字段包含完整的角色数据（V3 规范沿用 V2 数据结构）：

| 字段名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| `name` | `string` | ✅ | 角色名称 |
| `description` | `string` | ✅ | 角色描述 |
| `personality` | `string` | ❌ | 角色性格 |
| `scenario` | `string` | ❌ | 场景描述 |
| `first_mes` | `string` | ✅ | 第一条消息 |
| `mes_example` | `string` | ❌ | 对话示例 |
| `creator_notes` | `string` | ❌ | 创作者备注 |
| `system_prompt` | `string` | ❌ | 系统提示词 |
| `post_history_instructions` | `string` | ❌ | 历史后指令（作者注释/Author's Note） |
| `tags` | `string[]` | ❌ | 标签列表 |
| `creator` | `string` | ❌ | 创作者名称 |
| `character_version` | `string` | ❌ | 角色版本号 |
| `alternate_greetings` | `string[]` | ❌ | 备选开场白列表 |
| `character_book` | `CharacterBook` | ❌ | 内置世界书/角色书 |
| `extensions` | `object` | ❌ | 扩展字段 |
| `group_only_greetings` | `string[]` | ❌ | 仅群组可用的开场白 |

---

## 3. 扩展字段（extensions）

位于 `data.extensions` 下的扩展配置：

| 字段名 | 类型 | 描述 |
|--------|------|------|
| `talkativeness` | `string` \| `number` | 健谈度（0-1） |
| `fav` | `boolean` | 是否收藏 |
| `world` | `string` | 绑定的外部世界书名称 |
| `depth_prompt` | `object` | 深度提示词配置 |
| `depth_prompt.prompt` | `string` | 深度提示词内容 |
| `depth_prompt.depth` | `number` | 插入深度（0 = 最新消息后） |
| `depth_prompt.role` | `"system"` \| `"user"` \| `"assistant"` | 消息角色 |
| `regex_scripts` | `RegexScriptData[]` | 角色局部正则脚本列表 |

### 非标准扩展字段（外部工具添加）

| 字段名 | 类型 | 描述 |
|--------|------|------|
| `pygmalion_id` | `string` | Pygmalion.chat 唯一标识符 |
| `github_repo` | `string` | 关联的 GitHub 仓库 |
| `source_url` | `string` | 来源 URL |
| `chub` | `{ full_path: string }` | Chub.ai 相关数据 |
| `risuai` | `{ source: string[] }` | RisuAI 相关数据 |
| `sd_character_prompt` | `{ positive: string, negative: string }` | Stable Diffusion 角色提示词 |
| `tavern_helper` | `array` | 酒馆助手脚本配置 |

---

## 4. 世界书 / 角色书（character_book）

位于 `data.character_book` 的角色内置世界书：

| 字段名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| `name` | `string` | ❌ | 世界书名称 |
| `description` | `string` | ❌ | 世界书描述 |
| `scan_depth` | `number` | ❌ | 扫描深度 |
| `token_budget` | `number` | ❌ | Token 预算上限 |
| `recursive_scanning` | `boolean` | ❌ | 是否启用递归扫描 |
| `extensions` | `object` | ❌ | 扩展字段 |
| `entries` | `WorldInfoEntry[]` | ✅ | 世界书条目列表 |

---

## 5. 世界书条目（WorldInfoEntry）

每个世界书条目的完整字段：

### 5.1 基础字段

| 字段名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| `id` | `number` | ✅ | 条目唯一标识符 |
| `keys` | `string[]` | ✅ | 主要关键词列表 |
| `secondary_keys` | `string[]` | ❌ | 次要关键词列表 |
| `comment` | `string` | ❌ | 条目标题/注释（仅供编辑时查看） |
| `content` | `string` | ✅ | 条目内容（插入到提示词中） |
| `enabled` | `boolean` | ✅ | 是否启用 |
| `constant` | `boolean` | ❌ | 是否为常量（🔵蓝灯，始终激活） |
| `selective` | `boolean` | ❌ | 是否为可选项（🟢绿灯，需关键词匹配） |
| `insertion_order` | `number` | ❌ | 插入顺序（数值越大越后插入） |
| `position` | `string` | ❌ | 插入位置 |
| `use_regex` | `boolean` | ❌ | 关键词是否使用正则表达式 |

### 5.2 位置（position）可选值

| 值 | 描述 |
|---|------|
| `"before_char"` | 角色定义之前 |
| `"after_char"` | 角色定义之后 |
| `"before_example_messages"` | 示例消息之前 |
| `"after_example_messages"` | 示例消息之后 |
| `"before_author_note"` | 作者注释之前 |
| `"after_author_note"` | 作者注释之后 |
| `"at_depth"` | 插入到指定深度（需配合 extensions.depth 使用） |

---

## 6. 世界书条目扩展字段（entry.extensions）

每个世界书条目的 `extensions` 对象包含以下字段：

### 6.1 位置与排序

| 字段名 | 类型 | 默认值 | 描述 |
|--------|------|--------|------|
| `position` | `number` | `4` | 位置类型编码（0=角色定义前, 1=角色定义后, 4=指定深度等） |
| `depth` | `number` | `4` | 插入深度（0 = 最新消息后，1 = 倒数第二条后，以此类推） |
| `role` | `number` | `0` | 消息角色（0=system, 1=user, 2=assistant） |
| `display_index` | `number` | - | 显示排序索引 |

### 6.2 激活逻辑

| 字段名 | 类型 | 默认值 | 描述 |
|--------|------|--------|------|
| `selectiveLogic` | `number` | `0` | 次要关键词逻辑（0=AND_ANY, 1=AND_ALL, 2=NOT_ALL, 3=NOT_ANY） |
| `probability` | `number` | `100` | 激活概率（0-100） |
| `useProbability` | `boolean` | `true` | 是否启用概率判断 |
| `scan_depth` | `number` \| `null` | `null` | 扫描深度（null=跟随全局设置） |
| `case_sensitive` | `boolean` \| `null` | `null` | 是否区分大小写（null=跟随全局设置） |
| `match_whole_words` | `boolean` \| `null` | `null` | 是否匹配整词（null=跟随全局设置） |

### 6.3 递归控制

| 字段名 | 类型 | 默认值 | 描述 |
|--------|------|--------|------|
| `exclude_recursion` | `boolean` | `false` | 禁止其他条目递归激活本条目 |
| `prevent_recursion` | `boolean` | `false` | 禁止本条目递归激活其他条目 |
| `delay_until_recursion` | `boolean` \| `number` | `false` | 延迟到第 n 级递归时才激活 |

### 6.4 分组与权重

| 字段名 | 类型 | 默认值 | 描述 |
|--------|------|--------|------|
| `group` | `string` | `""` | 分组名称 |
| `group_override` | `boolean` | `false` | 是否覆盖分组设置 |
| `group_weight` | `number` | `100` | 分组权重（用于优先级排序） |
| `use_group_scoring` | `boolean` | `false` | 是否使用分组评分 |

### 6.5 效果控制

| 字段名 | 类型 | 默认值 | 描述 |
|--------|------|--------|------|
| `sticky` | `number` | `0` | 黏性（激活后保持激活的消息数） |
| `cooldown` | `number` | `0` | 冷却（激活后不可再激活的消息数） |
| `delay` | `number` | `0` | 延迟（需要多少条消息后才能激活） |

### 6.6 匹配目标

| 字段名 | 类型 | 默认值 | 描述 |
|--------|------|--------|------|
| `match_persona_description` | `boolean` | `false` | 匹配用户人设描述 |
| `match_character_description` | `boolean` | `false` | 匹配角色描述 |
| `match_character_personality` | `boolean` | `false` | 匹配角色性格 |
| `match_character_depth_prompt` | `boolean` | `false` | 匹配角色深度提示词 |
| `match_scenario` | `boolean` | `false` | 匹配场景描述 |
| `match_creator_notes` | `boolean` | `false` | 匹配创作者备注 |

### 6.7 其他

| 字段名 | 类型 | 默认值 | 描述 |
|--------|------|--------|------|
| `automation_id` | `string` | `""` | 自动化标识符 |
| `vectorized` | `boolean` | `false` | 是否向量化（用于语义搜索） |
| `triggers` | `array` | `[]` | 触发器列表 |
| `ignore_budget` | `boolean` | `false` | 是否忽略 Token 预算限制 |
| `outlet_name` | `string` | `""` | 输出口名称 |

---

## 7. 正则脚本（regex_scripts）

位于 `data.extensions.regex_scripts` 的角色局部正则脚本：

### 7.1 基础字段

| 字段名 | 类型 | 必填 | 描述 |
|--------|------|------|------|
| `id` | `string` | ✅ | 脚本唯一标识符（UUID） |
| `scriptName` | `string` | ✅ | 脚本名称 |
| `findRegex` | `string` | ✅ | 查找正则表达式 |
| `replaceString` | `string` | ✅ | 替换字符串 |
| `disabled` | `boolean` | ❌ | 是否禁用 |

### 7.2 作用范围（placement）

`placement` 是一个数字数组，表示正则应用于哪些来源：

| 值 | 常量名 | 描述 |
|---|--------|------|
| `0` | `MD_ONLY` | 仅应用于 Markdown 渲染（已弃用） |
| `1` | `USER_INPUT` | 用户输入 |
| `2` | `AI_OUTPUT` | AI 输出 |
| `3` | `SLASH_COMMAND` | 斜杠命令 |
| `4` | `WORLD_INFO` | 世界书内容 |
| `5` | `REASONING` | 推理内容 |

示例：`[1, 2]` 表示同时应用于用户输入和 AI 输出。

### 7.3 格式控制

| 字段名 | 类型 | 默认值 | 描述 |
|--------|------|--------|------|
| `markdownOnly` | `boolean` | `false` | 仅在显示时应用（不影响提示词） |
| `promptOnly` | `boolean` | `false` | 仅对提示词应用（不影响显示） |
| `runOnEdit` | `boolean` | `true` | 编辑消息时是否重新运行 |

### 7.4 深度限制

| 字段名 | 类型 | 默认值 | 描述 |
|--------|------|--------|------|
| `minDepth` | `number` \| `null` | `null` | 最小深度（null=无限制） |
| `maxDepth` | `number` \| `null` | `null` | 最大深度（null=无限制） |

> **深度说明**：深度 0 表示最新消息，深度 1 表示倒数第二条消息，以此类推。

### 7.5 其他字段

| 字段名 | 类型 | 默认值 | 描述 |
|--------|------|--------|------|
| `trimStrings` | `string[]` | `[]` | 需要裁剪的字符串列表 |
| `substituteRegex` | `number` \| `boolean` | `0` | 是否替换正则表达式变量 |

---

## 8. 完整 JSON 结构示例

以下是一个包含所有主要字段的角色卡 JSON 结构示例：

```json
{
    "name": "角色名称",
    "description": "角色描述",
    "personality": "角色性格",
    "scenario": "场景描述",
    "first_mes": "第一条消息",
    "mes_example": "对话示例",
    "creatorcomment": "创作者备注",
    "avatar": "avatar.png",
    "talkativeness": "0.5",
    "fav": false,
    "tags": ["标签1", "标签2"],
    "spec": "chara_card_v3",
    "spec_version": "3.0",
    "create_date": "2025-01-01",
    "data": {
        "name": "角色名称",
        "description": "角色描述",
        "personality": "角色性格",
        "scenario": "场景描述",
        "first_mes": "第一条消息",
        "mes_example": "对话示例",
        "creator_notes": "创作者备注",
        "system_prompt": "系统提示词",
        "post_history_instructions": "作者注释",
        "tags": ["标签1", "标签2"],
        "creator": "创作者名称",
        "character_version": "1.0",
        "alternate_greetings": [
            "备选开场白1",
            "备选开场白2"
        ],
        "group_only_greetings": [],
        "extensions": {
            "talkativeness": "0.5",
            "fav": false,
            "world": "外部世界书名称",
            "depth_prompt": {
                "prompt": "深度提示词内容",
                "depth": 4,
                "role": "system"
            },
            "regex_scripts": [
                {
                    "id": "uuid-xxxx-xxxx",
                    "scriptName": "正则脚本名称",
                    "disabled": false,
                    "runOnEdit": true,
                    "findRegex": "/查找模式/gm",
                    "replaceString": "替换内容",
                    "trimStrings": [],
                    "placement": [1, 2],
                    "substituteRegex": 0,
                    "minDepth": null,
                    "maxDepth": 2,
                    "markdownOnly": true,
                    "promptOnly": false
                }
            ]
        },
        "character_book": {
            "name": "世界书名称",
            "description": "世界书描述",
            "scan_depth": 2,
            "token_budget": 2048,
            "recursive_scanning": true,
            "extensions": {},
            "entries": [
                {
                    "id": 0,
                    "keys": ["关键词1", "关键词2"],
                    "secondary_keys": ["次要关键词"],
                    "comment": "条目标题",
                    "content": "条目内容",
                    "constant": false,
                    "selective": true,
                    "insertion_order": 100,
                    "enabled": true,
                    "position": "after_char",
                    "use_regex": false,
                    "extensions": {
                        "position": 4,
                        "exclude_recursion": false,
                        "display_index": 0,
                        "probability": 100,
                        "useProbability": true,
                        "depth": 4,
                        "selectiveLogic": 0,
                        "group": "",
                        "group_override": false,
                        "group_weight": 100,
                        "prevent_recursion": false,
                        "delay_until_recursion": false,
                        "scan_depth": null,
                        "match_whole_words": null,
                        "use_group_scoring": false,
                        "case_sensitive": null,
                        "automation_id": "",
                        "role": 0,
                        "vectorized": false,
                        "sticky": 0,
                        "cooldown": 0,
                        "delay": 0,
                        "match_persona_description": false,
                        "match_character_description": false,
                        "match_character_personality": false,
                        "match_character_depth_prompt": false,
                        "match_scenario": false,
                        "match_creator_notes": false,
                        "triggers": [],
                        "ignore_budget": false,
                        "outlet_name": ""
                    }
                }
            ]
        }
    }
}
```

---

## 附录：字段值枚举参考

### A. selectiveLogic（次要关键词逻辑）

| 值 | 名称 | 描述 |
|---|------|------|
| `0` | `AND_ANY` | 主要关键词匹配且次要关键词任意一个匹配 |
| `1` | `AND_ALL` | 主要关键词匹配且次要关键词全部匹配 |
| `2` | `NOT_ALL` | 主要关键词匹配且次要关键词至少有一个不匹配 |
| `3` | `NOT_ANY` | 主要关键词匹配且次要关键词全部不匹配 |

### B. position（位置编码） - extensions.position

| 值 | 描述 |
|---|------|
| `0` | 角色定义之前（before_char） |
| `1` | 角色定义之后（after_char） |
| `2` | 示例消息之前 |
| `3` | 示例消息之后 |
| `4` | 指定深度（at_depth） |
| `5` | 作者注释之前 |
| `6` | 作者注释之后 |

### C. role（消息角色） - extensions.role

| 值 | 描述 |
|---|------|
| `0` | system（系统消息） |
| `1` | user（用户消息） |
| `2` | assistant（助手消息） |

---

## 9. PNG 元数据存储

角色卡数据存储在 PNG 图片的 **`tEXt` 区块**（文本元数据区块）中。

### 9.1 存储格式

| 关键词 | 版本 | 描述 |
|--------|------|------|
| `chara` | V2 | 角色卡 V2 格式数据（Base64 编码的 JSON） |
| `ccv3` | V3 | 角色卡 V3 格式数据（Base64 编码的 JSON） |

### 9.2 读取优先级

1. **优先读取 `ccv3`**（V3 格式）
2. 如果没有 `ccv3`，则读取 `chara`（V2 格式）

### 9.3 写入规则

SillyTavern 写入 PNG 时会同时写入两个 `tEXt` 区块：
- `chara` - 兼容 V2 格式
- `ccv3` - V3 新格式

两个区块都插入在 `IEND` 区块之前。

### 9.4 解析步骤

1. 使用 PNG 解析库提取所有 `tEXt` 区块
2. 查找关键词为 `ccv3` 或 `chara` 的区块
3. 对文本内容进行 **Base64 解码**
4. 解析得到的 **JSON 字符串**

### 9.5 zTXt 区块

SillyTavern **不使用 `zTXt` 区块**（压缩文本区块），仅使用 `tEXt` 区块。开发解析器时只需处理 `tEXt` 即可。

---

## 10. 开发注意事项

### 10.1 扩展字段保护原则（重要）

> ⚠️ **重要开发原则**：SillyTavern 生态中有大量第三方插件（如 `tavern_helper`、`colors`、`expressions` 等）。解析器在读取 `data.extensions` 时，**必须将所有未定义的字段原样保存在内存中，并在保存时写回**。
>
> **严禁丢弃未知的扩展字段**，否则用户用你的编辑器保存一次，他的插件配置就全丢了。

### 10.2 版本兼容性建议

1. **读取时**：优先解析 `ccv3`，回退到 `chara`
2. **写入时**：同时写入 `ccv3` 和 `chara` 以保持兼容
3. **字段处理**：对于未知字段一律保留，不要删除或忽略
4. **规范版本**：检查 `spec` 和 `spec_version` 字段判断数据版本

---