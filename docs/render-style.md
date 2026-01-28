# 通用文本渲染规范 (General Text Rendering Standard)

本文档定义了系统通用的富文本渲染逻辑，旨在提供一致的阅读体验。该规范复用了聊天记录的渲染核心，但移除了特定的过滤逻辑。

## 渲染流程管道

文本内容的渲染遵循以下处理顺序：

1.  **标准化换行**：统一将 CRLF (`\r\n`) 转换为 LF (`\n`)。
2.  **正则替换 (Regex Scripts)**：
    *   应用 **角色卡正则 (Card Regex)**。
    *   *注：不应用特定的聊天记录正则。*
3.  **格式化预处理**：
    *   **代码块**：识别 \`\`\`包裹的内容，转换为 `<pre><code>` 结构。
    *   **空白折叠**：将连续 3 个以上的换行符折叠为 2 个。
4.  **DOM 净化 (Sanitization)**：
    *   **白名单机制**：移除非法标签（`script`, `iframe`, `object`, `form` 等）。
    *   **属性清洗**：移除所有 `on*` 事件处理器和 `javascript:` 伪协议链接。
5.  **文本节点解析 (Markdown-like Parsing)**：
    *   遍历 DOM 文本节点，进行以下语法转换：
        *   **行内代码**：`` `text` `` → `<code>text</code>`
        *   **删除线**：`~~text~~` → `<del>text</del>`
        *   **粗斜体**：`***text***` → `<strong><em>text</em></strong>`
        *   **粗体**：`**text**` → `<strong>text</strong>`
        *   **斜体**：`*text*` → `<em>text</em>`
        *   **引用/对话**：`"text"`, `「text」`, `『text』` → `<q>text</q>`
        *   **换行**：`\n` → `<br>`

## 样式定义 (CSS)

建议在渲染容器（如 Shadow DOM 或 Scoped CSS）中应用以下样式：

```css
/* 基础排版 */
.render-content {
    line-height: 1.8;
    color: inherit;
    font-family: sans-serif;
}

/* 引用 (变色处理) */
q { color: #2e7d32; } /* Light: Green */
q::before { content: '"'; }
q::after { content: '"'; }

/* 强调 */
strong { color: #c62828; font-weight: bold; } /* Light: Red */
em { color: #b8860b; font-style: italic; }    /* Light: Gold */
del { color: #888; text-decoration: line-through; }

/* 代码 */
code {
    background: rgba(128,128,128,0.2);
    padding: 2px 6px;
    border-radius: 4px;
    font-family: monospace;
    font-size: 0.9em;
}
pre {
    background: #f5f5f5;
    border: 1px solid #e5e5e5;
    border-radius: 8px;
    padding: 1rem;
    overflow-x: auto;
}

/* 深色模式适配 (.dark 类) */
.dark q { color: #99cc99; }
.dark strong { color: #ff9966; }
.dark em { color: #ffcc00; }
.dark pre { background: #1e1e1e; border-color: #333; }
```

## 实现代码示例 (TypeScript/Svelte)

以下是核心渲染逻辑的参考实现：

```typescript
import { processContentWithScripts, type RegexScript } from "$lib/utils/regexProcessor";

/**
 * 通用文本渲染函数
 * @param text 原始文本
 * @param cardRegex 角色卡正则脚本列表
 * @returns 处理后的 HTML 字符串
 */
export function renderContent(text: string, cardRegex: RegexScript[] = []): string {
    // 1. 标准化换行
    let res = text.replace(/\r\n/g, '\n').replace(/\r/g, '\n');

    // 2. 正则替换 (仅角色卡正则)
    res = processContentWithScripts(res, cardRegex);
    
    // 3. 折叠多余空行
    res = res.replace(/(\n\s*){3,}/g, '\n\n');
    
    // 4. 处理代码块 (避免被后续的 DOM Parser 破坏)
    res = res.replace(/```(\w*)\n?([\s\S]*?)```/g, (match, lang, content) => {
        const langClass = lang ? ` class="language-${lang}"` : '';
        // 简单转义，防止内部 HTML 被解析
        const escaped = content
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;');
        return `<pre><code${langClass}>${escaped}</code></pre>`;
    });

    // 5. DOM 操作与净化
    const container = document.createElement('div');
    container.innerHTML = res; // 解析 HTML 结构
    
    // 移除危险元素
    container.querySelectorAll('script, iframe, object, embed, form').forEach(el => el.remove());
    container.querySelectorAll('*').forEach(el => {
        Array.from(el.attributes).forEach(attr => {
            if (attr.name.startsWith('on') || 
                (attr.name === 'href' && attr.value.trim().toLowerCase().startsWith('javascript:'))) {
                el.removeAttribute(attr.name);
            }
        });
    });
    
    // 6. 文本节点解析 (Markdown 语法)
    processTextNodes(container);
    
    // 7. 清理多余 BR
    let html = container.innerHTML;
    return html.replace(/(<br\s*\/?>\s*){2,}/gi, '<br>'); // 将连续BR合并
}

/**
 * 递归处理文本节点
 */
function processTextNodes(element: HTMLElement): void {
    const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT, null);
    const textNodes: Text[] = [];
    while (walker.nextNode()) textNodes.push(walker.currentNode as Text);
    
    textNodes.forEach(node => {
        const parent = node.parentNode as HTMLElement;
        if (!parent) return;
        
        // 跳过代码块和已处理元素
        if (parent.closest('pre, code, script, style')) return;
        
        let text = node.textContent || '';
        let hasChanges = false;
        
        // 行内代码 `code`
        if (text.includes('`')) {
            text = text.replace(/(?<!`)`([^`]+)`(?!`)/g, '<code>$1</code>');
            hasChanges = true;
        }
        
        // 删除线 ~~text~~
        if (text.includes('~~')) {
            text = text.replace(/~~(.+?)~~/g, '<del>$1</del>');
            hasChanges = true;
        }
        
        // 粗斜体 ***text***
        if (text.includes('*')) {
            text = text.replace(/\*\*\*(.+?)\*\*\*/g, '<strong><em>$1</em></strong>');
            text = text.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
            text = text.replace(/(?<![<\\])\*([^*\n]+)\*(?![>])/g, '<em>$1</em>');
            hasChanges = true;
        }
        
        // 引用 (自动变色)
        if (text.includes('"') || text.includes('「') || text.includes('『')) {
            text = text.replace(/"([^"]+)"/g, '<q>$1</q>');
            text = text.replace(/「([^」]+)」/g, '<q>$1</q>');
            text = text.replace(/『([^』]+)』/g, '<q>$1</q>');
            hasChanges = true;
        }
        
        // 换行转 <br>
        if (text.includes('\n')) {
            text = text.replace(/\n/g, '<br>');
            hasChanges = true;
        }
        
        if (hasChanges) {
            const span = document.createElement('span');
            span.innerHTML = text;
            parent.replaceChild(span, node);
        }
    });
}
```
