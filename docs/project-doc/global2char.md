# Global WorldInfo to Character Book Conversion

当将全局世界书转换为角色卡内联世界书（Character Book）时，字段映射关系如下：

## 字段映射表

| 全局世界书 | 角色卡内联 | 类型 | 说明 |
|-----------|-----------|------|------|
| `uid` | `id` | string/number | 条目唯一标识符 |
| `key` | `keys` | string[] | 主关键词数组 |
| `keysecondary` | `secondary_keys` | string[] | 次要关键词数组 |
| `comment` | `comment` | string | 备注/说明 |
| `content` | `content` | string | 条目内容 |
| `constant` | `constant` | boolean | 是否为常驻条目 |
| `selective` | `selective` | boolean | 是否启用选择性逻辑 |
| `order` | `insertion_order` | number | 插入顺序 |
| `disable` | `enabled` | boolean | **取反**：禁用→false，启用→true |
| `position` | `position` | string | 位置：`0` → `"before_char"`，其他 → `"after_char"` |
| - | `use_regex` | boolean | 固定为 `true`，SillyTavern 关键词始终使用正则 |

## 扩展字段映射（Extensions）

以下字段被放入 `extensions` 对象中：

| 全局世界书 | 角色卡内联.extensions | 类型 | 说明 |
|-----------|---------------------|------|------|
| - | `position` | number | 原始 position 值 |
| `excludeRecursion` | `exclude_recursion` | boolean | 排除递归 |
| `displayIndex` | `display_index` | number | 显示索引 |
| `probability` | `probability` | number/null | 触发概率 |
| `useProbability` | `useProbability` | boolean | 是否使用概率 |
| `depth` | `depth` | number | 深度，默认 4 |
| `selectiveLogic` | `selectiveLogic` | number | 选择性逻辑，默认 0 |
| `outletName` | `outlet_name` | string | 出口名称 |
| `group` | `group` | string | 分组名 |
| `groupOverride` | `group_override` | boolean | 分组覆盖 |
| `groupWeight` | `group_weight` | number/null | 分组权重 |
| `preventRecursion` | `prevent_recursion` | boolean | 防止递归 |
| `delayUntilRecursion` | `delay_until_recursion` boolean | 延迟递归 |
| `scanDepth` | `scan_depth` | number/null | 扫描深度 |
| `matchWholeWords` | `match_whole_words` | boolean/null | 全词匹配 |
| `useGroupScoring` | `use_group_scoring` | boolean | 使用分组评分 |
| `caseSensitive` | `case_sensitive` | boolean/null | 区分大小写 |
| `automationId` | `automation_id` | string | 自动化ID |
| `role` | `role` | number | 角色，默认 0 |
| `vectorized` | `vectorized` | boolean | 向量化 |
| `sticky` | `sticky` | number/null | 粘性 |
| `cooldown` | `cooldown` | number/null | 冷却时间 |
| `delay` | `delay` | number/null | 延迟 |
| `matchPersonaDescription` | `match_persona_description` | boolean | 匹配人设描述 |
| `matchCharacterDescription` | `match_character_description` | boolean | 匹配角色描述 |
| `matchCharacterPersonality` | `match_character_personality` | boolean | 匹配角色性格 |
| `matchCharacterDepthPrompt` | `match_character_depth_prompt` | boolean | 匹配深度提示 |
| `matchScenario` | `match_scenario` | boolean | 匹配场景 |
| `matchCreatorNotes` | `match_creator_notes` | boolean | 匹配作者备注 |
| `triggers` | `triggers` | string[] | 触发器列表 |
| `ignoreBudget` | `ignore_budget` | boolean | 忽略预算 |

## 转换函数位置

**文件**: `docs/characters.js`
**函数**: `convertWorldInfoToCharacterBook(name, entries)`

## 输出结构

```json
{
  "entries": [
    {
      "id": "<uid>",
      "keys": ["<key>"],
      "secondary_keys": ["<keysecondary>"],
      "comment": "<comment>",
      "content": "<content>",
      "constant": <boolean>,
      "selective": <boolean>,
      "insertion_order": <number>,
      "enabled": <boolean>,
      "position": "before_char" | "after_char",
      "use_regex": true,
      "extensions": {
        "position": <number>,
        "exclude_recursion": <boolean>,
        ...
      }
    }
  ],
  "name": "<world_info_name>"
}
```

## 注意事项

1. **`enabled` 与 `disable` 的关系**：全局世界书用 `disable` 字段（true=禁用），角色卡内联用 `enabled` 字段（true=启用），转换时取反。

2. **位置字段**：`position` 从数值转换为字符串描述：
   - `0` → `"before_char"`（角色之前）
   - 其他值 → `"after_char"`（角色之后）

3. **正则强制启用**：`use_regex` 固定为 `true`，SillyTavern 的关键词始终按正则处理。

4. **扩展字段保留**：所有未在上面列出的全局世界书字段会被保留在 `extensions` 对象中。
