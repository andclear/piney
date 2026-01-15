import { processContentWithScripts, type RegexScript } from "$lib/utils/regexProcessor";

/**
 * 通用文本渲染函数
 * @param text 原始文本
 * @param cardRegex 角色卡正则脚本列表
 * @returns 处理后的 HTML 字符串
 */
export function renderContent(text: string, cardRegex: RegexScript[] = []): string {
    if (!text) return "";

    // 1. 标准化换行 (Unescape literal \n too, as users often expect this in raw text processing)
    let res = text.replace(/\r\n/g, '\n')
        .replace(/\r/g, '\n')
        .replace(/\\n/g, '\n'); // Handle literal \n

    // 2. 正则替换 (仅角色卡正则)
    res = processContentWithScripts(res, cardRegex);

    // 3. 清理标签间的换行 (不要在标签之间加换行，例如 </div><content>)
    res = res.replace(/>\s*\n\s*</g, '><');

    // 4. 折叠多余空行 (保留段落间距)
    res = res.replace(/(\n\s*){3,}/g, '\n\n');

    // 5. 处理代码块
    // 如果代码块内容包含 HTML/JS 标签，则直接渲染 (Raw Embed)，否则标准 Markdown 代码块
    res = res.replace(/```(\w*)\n?([\s\S]*?)```/g, (match, lang, content) => {
        // 启发式检测：如果包含常见 HTML 结构标签，视为想要渲染的 HTML
        if (content.match(/<(script|style|body|html|div|iframe|table|form|link|meta)/i)) {
            return content;
        }

        const langClass = lang ? ` class="language-${lang}"` : '';
        const escaped = content
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;');
        return `<pre><code${langClass}>${escaped}</code></pre>`;
    });

    // 6. DOM 操作与净化
    const container = document.createElement('div');
    container.innerHTML = res;

    // 移除危险元素 (仅保留基本沙箱安全，允许 script 以支持动态效果)
    // container.querySelectorAll('iframe, object, embed, form').forEach(el => el.remove());
    // 移除 on* (已被用户要求开启 JS，所以不再移除)

    // 7. 文本节点解析 (Markdown 语法 & 全局换行)
    processTextNodes(container);

    // 8. 清理多余 BR
    let html = container.innerHTML;
    // 保留段落间距.
    html = html.replace(/(<br\s*\/?>\s*){2,}/gi, '<br><br>');

    // 9. 注入基础样式 (通过内联 style 标签)
    const styleBlock = `
        <style>
            html, body { 
                background: transparent !important;
                margin: 0;
                padding: 0;
                line-height: 1.5; 
                color: #000000;
                font-family: ui-sans-serif, system-ui, sans-serif;
            }
            html.dark body {
                color: #e0e0e0;
            }
            
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
                padding: 0.75rem;
                overflow-x: auto;
                margin: 1em 0;
            }
            html.dark pre {
                background: #1e1e1e;
                border-color: #333;
            }
            
            q { color: #2e7d32; }
            html.dark q { color: #99cc99; }
            q::before { content: '"'; }
            q::after { content: '"'; }
            
            em { color: #b8860b; font-style: italic; }
            html.dark em { color: #ffcc00; }
            
            strong { color: #c62828; font-weight: bold; }
            html.dark strong { color: #ff9966; }
            
            del { color: #888; text-decoration: line-through; }
        </style>
    `;

    return styleBlock + `<div style="padding: 1rem;">${html}</div>`;
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

        // --- Markdown 处理 ---

        // 行内代码 `code`
        if (text.includes('`') && !text.includes('```')) {
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

        // 图片 ![alt](url) (放在引用处理之后，防止引号被替换)
        if (text.includes('![') && text.includes('](')) {
            text = text.replace(/!\[(.*?)\]\((.*?)\)/g, (match, alt, src) => {
                return `<img src="${src}" alt="${alt}" style="max-width: 100%; border-radius: 8px; margin: 8px 0;" />`;
            });
            hasChanges = true;
        }


        // --- 换行处理 ---

        // 段落间距 (Double Newline) -> 1.5x Paragraph Spacing
        // 1.5 lines gap. Assuming line-height 1.5, 1 line = 1.5em. Gap = 2.25em.
        // We use a block span to create specific spacing.
        if (/\n\s*\n/.test(text)) {
            text = text.replace(/\n\s*\n/g, '<span style="display:block; height: 2.25em; width: 100%;"></span>');
            hasChanges = true;
        }

        // 移除白名单限制：所有标签内的文本 \n 都转换为 <br>
        // 结构性的换行已在 regex 步骤 (><) 中处理，此处仅处理内容换行
        if (text.includes('\n')) {
            text = text.replace(/\n/g, '<br>');
            hasChanges = true;
        }

        if (hasChanges) {
            const span = document.createElement('span');
            // Only apply Z-index fix if text is a direct child of the container (likely the main greeting text)
            // This prevents breaking layout inside injected HTML templates (which might rely on specific stacking)
            if (parent === element) {
                span.style.cssText = "position: relative; z-index: 1;";
            }
            span.innerHTML = text;
            parent.replaceChild(span, node);
        } else {
            // Even if no changes, wrap in relative span to fix Z-Index stacking issues
            // BUT ONLY for root level text
            if (parent === element && text.trim().length > 0) {
                const span = document.createElement('span');
                span.style.cssText = "position: relative; z-index: 1;";
                span.textContent = text;
                parent.replaceChild(span, node);
            }
        }
    });
}
