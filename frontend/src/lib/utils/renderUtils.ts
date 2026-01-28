import { processContentWithScripts, type RegexScript } from "$lib/utils/regexProcessor";
import { smartFilterTags, processTagNewlines } from "$lib/utils/tagFilter";
import { marked } from "marked";

// ============================================================================
// 完整渲染管道 v2 - 支持 piney_render、代码块保护、script 标签
// ============================================================================

// --- 1. 代码块类型检测 ---
export function isHtmlCodeBlock(language: string): boolean {
    const htmlLangs = ['html', 'xml', 'svg', 'htm', 'xhtml'];
    return htmlLangs.includes(language.toLowerCase());
}

// --- 2. 完整 HTML 文档检测 ---
export function isFrontend(content: string): boolean {
    const lower = content.toLowerCase();
    return ['<!doctype', '<html', '<head>', '<body'].some(tag => lower.includes(tag));
}

// --- 3. Iframe Content Generator ---
export function createIframeContent(content: string): string {
    const style = `
        <style>
            *,*::before,*::after{box-sizing:border-box;}
            html,body{margin:0!important;padding:0;overflow:hidden!important;max-width:100%!important;background-color:transparent!important;}
            ::-webkit-scrollbar { width: 0px; background: transparent; }
        </style>
    `;

    const resizeScript = `
        <script>
            function postHeight() {
                const height = document.body ? document.body.scrollHeight : 0;
                if (!height) return;
                window.parent.postMessage({ type: 'TH_ADJUST_IFRAME_HEIGHT', height: height, name: window.name }, '*');
            }
            window.addEventListener('load', postHeight);
            window.addEventListener('resize', postHeight);
            if (document.readyState === 'loading') {
                document.addEventListener('DOMContentLoaded', () => {
                   new MutationObserver(postHeight).observe(document.body, {childList: true, subtree: true, attributes: true});
                   postHeight();
                });
            } else {
                if(document.body) {
                    new MutationObserver(postHeight).observe(document.body, {childList: true, subtree: true, attributes: true});
                    postHeight();
                }
            }
        </script>
    `;

    const trimmed = content.trim().toLowerCase();
    const isFullDoc = trimmed.startsWith('<!doctype') || trimmed.startsWith('<html') || trimmed.startsWith('<head');

    if (isFullDoc) {
        let injected = content;
        const payload = style + resizeScript;

        const injectBefore = (tag: string, payload: string) => {
            const idx = injected.toLowerCase().indexOf(tag);
            if (idx !== -1) {
                injected = injected.slice(0, idx) + payload + injected.slice(idx);
                return true;
            }
            return false;
        };

        if (!injectBefore('</head>', payload)) {
            if (!injectBefore('<body', payload)) {
                if (!injectBefore('</body>', payload)) {
                    injected += payload;
                }
            }
        }
        return injected;
    } else {
        return `<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    ${style}
    ${resizeScript}
</head>
<body>
    ${content}
</body>
</html>`;
    }
}

// --- 4. 代码块保护和提取 ---
interface CodeBlock {
    language: string;
    content: string;
    raw: string;
}

function extractCodeBlocks(text: string): { text: string; blocks: CodeBlock[] } {
    const blocks: CodeBlock[] = [];
    // 匹配 ```language\n...\n``` 格式的代码块
    const codeBlockRegex = /```(\w*)\n([\s\S]*?)```/g;

    const result = text.replace(codeBlockRegex, (match, lang, content) => {
        const idx = blocks.length;
        blocks.push({
            language: lang || '',
            content: content,
            raw: match
        });
        return `\n<!--CODE_BLOCK_${idx}-->\n`;
    });

    return { text: result, blocks };
}

// --- 5. 渲染块保护和提取（支持多种标签：piney_render, orange 等）---
// 需要整体渲染的标签列表
const RENDER_BLOCK_TAGS = ['piney_render', 'orange'];

function extractRenderBlocks(text: string): { text: string; blocks: string[] } {
    const blocks: string[] = [];
    let result = text;

    // 对每种标签类型进行提取
    for (const tagName of RENDER_BLOCK_TAGS) {
        const startTag = `<${tagName}>`;
        const endTag = `</${tagName}>`;
        let newResult = '';
        let i = 0;
        const lowerText = result.toLowerCase();

        while (i < result.length) {
            const startIdx = lowerText.indexOf(startTag, i);

            if (startIdx === -1) {
                newResult += result.slice(i);
                break;
            }

            // 添加标签前的内容
            newResult += result.slice(i, startIdx);

            // 使用深度计数找到匹配的闭合标签
            let depth = 1;
            let j = startIdx + startTag.length;

            while (j < result.length && depth > 0) {
                const nextStart = lowerText.indexOf(startTag, j);
                const nextEnd = lowerText.indexOf(endTag, j);

                if (nextEnd === -1) {
                    j = result.length;
                    break;
                }

                if (nextStart !== -1 && nextStart < nextEnd) {
                    depth++;
                    j = nextStart + startTag.length;
                } else {
                    depth--;
                    if (depth === 0) {
                        j = nextEnd + endTag.length;
                    } else {
                        j = nextEnd + endTag.length;
                    }
                }
            }

            // 提取整个块（不包含外层标签）
            const blockContent = result.slice(startIdx + startTag.length, j - endTag.length);
            const idx = blocks.length;
            blocks.push(blockContent);
            newResult += `\n<!--RENDER_BLOCK_${idx}-->\n`;
            i = j;
        }

        result = newResult;
    }

    return { text: result, blocks };
}

// --- 6. 简化的 HTML 清理（允许 script 和 style）---
function lightSanitize(html: string): string {
    // 只移除危险的事件属性和 javascript: 链接
    return html.replace(/<[^>]+>/g, (tag) => {
        return tag
            .replace(/\s+on[a-z]+\s*=\s*(["']).*?\1/gi, " ")
            .replace(/\s+href\s*=\s*(["'])javascript:.*?\1/gi, ' href="#"');
    });
}

// --- 7. 主渲染管道 ---
interface RenderOptions {
    chatRegex?: RegexScript[];
    cardRegex?: RegexScript[];
    hiddenTags?: string[];
    newlineTags?: string[];
}

export interface ProcessedResult {
    html: string;
    codeBlocks: CodeBlock[];
    renderBlocks: string[];
}

export function processTextWithPipeline(text: string, options: RenderOptions): ProcessedResult {
    let res = text.replace(/\r\n/g, '\n').replace(/\r/g, '\n');

    // ========== Step 0: 预处理 - 规范化带空格的标签 ==========
    // 将 <tag name> 转换为 <tag-name>，将 </tag name> 转换为 </tag-name>
    // 例如：<phase 1> → <phase-1>，</phase 1> → </phase-1>
    res = res.replace(/<(\/?)([\w]+)\s+([\w]+)>/g, '<$1$2-$3>');

    // ========== Step 1: 保护代码块 ==========
    // 在任何处理之前先提取代码块，避免被正则或 Markdown 破坏
    const { text: afterCodeExtract, blocks: codeBlocks } = extractCodeBlocks(res);
    res = afterCodeExtract;

    // ========== Step 2: 正则脚本处理 ==========
    // 正则替换会自动添加 <piney_render> 标签包裹替换内容
    if (options.chatRegex) {
        res = processContentWithScripts(res, options.chatRegex, { isMarkdown: true });
    }
    if (options.cardRegex) {
        res = processContentWithScripts(res, options.cardRegex, { isMarkdown: true });
    }

    // ========== Step 3: 提取渲染块 ==========
    // 必须在正则处理之后，因为正则会添加 <piney_render> 标签
    // 支持 piney_render, orange 等标签
    const { text: afterRenderExtract, blocks: renderBlocks } = extractRenderBlocks(res);
    res = afterRenderExtract;

    // ========== Step 4: 标签过滤（可选）==========
    if (options.hiddenTags && options.hiddenTags.length > 0) {
        res = smartFilterTags(res, options.hiddenTags);
    }

    // ========== Step 5: 标签换行处理 ==========
    if (options.newlineTags) {
        const allTags = [...new Set([...(options.hiddenTags || []), ...options.newlineTags])];
        res = processTagNewlines(res, allTags, options.newlineTags);
    }

    // ========== Step 6: 压缩过多空行 ==========
    res = res.replace(/(\n\s*){3,}/g, '\n\n');

    // ========== Step 7: Markdown 渲染 ==========
    let html = marked.parse(res, { async: false, breaks: true }) as string;

    // ========== Step 8: 恢复渲染块占位符 ==========
    renderBlocks.forEach((block: string, idx: number) => {
        const placeholder = `<!--RENDER_BLOCK_${idx}-->`;
        html = html.replace(`<p>${placeholder}</p>`, `<div class="piney-render-block" data-render-idx="${idx}">${block}</div>`);
        html = html.replace(placeholder, `<div class="piney-render-block" data-render-idx="${idx}">${block}</div>`);
    });

    // ========== Step 9: 恢复代码块占位符 ==========
    codeBlocks.forEach((block, idx) => {
        const placeholder = `<!--CODE_BLOCK_${idx}-->`;
        const langClass = block.language ? `language-${block.language}` : '';

        if (isHtmlCodeBlock(block.language)) {
            // 标记为需要 iframe 渲染的代码块
            const escaped = block.content
                .replace(/&/g, '&amp;')
                .replace(/</g, '&lt;')
                .replace(/>/g, '&gt;');
            const codeHtml = `<pre class="piney-iframe-code" data-code-idx="${idx}" data-lang="${block.language}"><code class="${langClass}">${escaped}</code></pre>`;
            html = html.replace(`<p>${placeholder}</p>`, codeHtml);
            html = html.replace(placeholder, codeHtml);
        } else {
            // 普通代码块，恢复为高亮显示
            const escaped = block.content
                .replace(/&/g, '&amp;')
                .replace(/</g, '&lt;')
                .replace(/>/g, '&gt;');
            const codeHtml = `<pre><code class="${langClass}">${escaped}</code></pre>`;
            html = html.replace(`<p>${placeholder}</p>`, codeHtml);
            html = html.replace(placeholder, codeHtml);
        }
    });

    // ========== Step 10: 轻量级清理 ==========
    html = lightSanitize(html);

    return {
        html,
        codeBlocks,
        renderBlocks
    };
}

// --- 兼容旧 API（返回纯 HTML 字符串）---
export function processTextWithPipelineSimple(text: string, options: RenderOptions): string {
    return processTextWithPipeline(text, options).html;
}
