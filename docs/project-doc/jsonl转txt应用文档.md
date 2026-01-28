# JSONL 转 TXT 应用文档

本文档基于 `酒馆助手脚本-聊天记录导出.json` 中的逻辑，说明如何将导出的 JSONL 聊天记录文件转换为易读的 TXT 文本格式。

## 一、文件格式说明

导出的 `.jsonl` 文件是一种 **行分隔 JSON (Line-delimited JSON)** 格式。每一行都是一个独立的、合法的 JSON 对象。

### 1. 文件结构
```text
{Header Object}  <-- 第一行：元数据头
{Message Object} <-- 第二行：第一条消息
{Message Object} <-- 第三行：第二条消息
...
```

### 2. 字段定义

#### header (第一行)
包含了对话的基础信息。
```json
{
  "user_name": "User",        // 用户名称
  "character_name": "Char",   // 角色名称
  "create_date": 1679000000,  // 导出时间戳
  "chat_metadata": { ... }    // 其他元数据
}
```

#### message (后续行)
每一行代表一条对话记录。
```json
{
  "name": "角色的名字",       // 发送者名称
  "is_user": false,          // 是否为用户发言 (true/false)
  "mes": "消息内容...",       // 消息正文 (可能包含HTML标签)
  "message_id": 10,          // 楼层号/ID
  "send_date": "...",        // 发送时间 (可选)
  ...
}
```
*注：早期版本或不同源可能使用 `message` 字段代替 `mes`，解析时建议优先取 `mes`，若不存在则取 `message`。*

---

## 二、转换逻辑

要将 JSONL 转换为 TXT，建议遵循以下标准流程：

### 1. 读取流程
1.  **按行读取**文件内容。
2.  **解析第一行**：获取 `user_name` 和 `character_name` 作为上下文参考（可选）。
3.  **遍历后续行**：
    *   将每一行解析为 JSON 对象。
    *   提取关键字段：`name` (发送者), `mes` (内容)。
    *   处理内容格式（如去除多余 HTML 标签）。
    *   按照指定格式写入目标 TXT 文件。

### 2. 建议的导出格式

为了保持阅读体验，建议采用以下格式块：

```text
【发送者名称】 
消息内容...

--------------------
```

### 3. 数据处理细节

根据脚本逻辑，在转换时需注意以下细节：

*   **HTML 清理**：
    *   源数据中的 `mes` 可能包含 `<br>`, `<em>` 等 HTML 标签。
    *   建议将 `<br>` 和 `<div>` 替换为换行符 `\n`。
    *   **标签处理逻辑**：
        *   脚本的原始逻辑会**移除所有 XML/HTML 标签**（包括 `<context>`, `<small_theater>`, `<status>` 等）。
        *   无论导出何种标签的内容，最终 TXT 中**只保留标签内的纯文本**，标签本身不会被导出。
        *   若处于“标签筛选模式”，只会导出被选中标签内部的内容（不含标签外壳）。

*   **楼层号 (必要)**：
    *   每条消息必须包含楼层号，例如：`[#10] 【Char】`。

---

## 三、代码实现示例 (伪代码)

```javascript
function convertJsonlToTxt(fileContent) {
    const lines = fileContent.trim().split('\n');
    let output = "";
    
    // 1. (可选) 读取 Header
    // const header = JSON.parse(lines[0]);

    // 2. 遍历消息
    // 从索引 1 开始，因为索引 0 是 Header
    for (let i = 1; i < lines.length; i++) {
        try {
            const msg = JSON.parse(lines[i]);
            
            // 获取名字
            const name = msg.name || (msg.is_user ? "User" : "Character");
            
            // 获取内容 (兼容字段)
            let content = msg.mes || msg.message || "";
            
            // 简单清洗 HTML/XML (将 <br> 转为换行，去除其他所有标签)
            // 符合脚本原始逻辑：无论是什么标签，最终只保留文本内容
            content = content
                .replace(/<br\s*\/?>/gi, '\n')
                .replace(/<\/?[^>]+(>|$)/g, ""); 
            
            // 拼接格式
            output += `【${name}】\n${content}\n\n--------------------\n\n`;
            
        } catch (e) {
            console.warn("Skipping invalid line", i);
        }
    }
    
    return output;
}
```
