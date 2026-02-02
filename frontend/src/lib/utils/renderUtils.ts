import { processContentWithScripts, type RegexScript } from "$lib/utils/regexProcessor";
import { smartFilterTags, processTagNewlines, detectTags } from "$lib/utils/tagFilter";
import { marked } from "marked";
// @ts-ignore
import pkg from 'js-beautify';
const { html: html_beautify } = pkg;

export function formatHtml(html: string): string {
    if (!html) return '';
    try {
        return html_beautify(html, {
            indent_size: 2,
            preserve_newlines: true,
            max_preserve_newlines: 2,
            indent_inner_html: true,
            extra_liners: ['head', 'body', '/html', 'script', 'style']
        });
    } catch (e) {
        console.warn('Formatting failed:', e);
        return html;
    }
}

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
export function createIframeContent(content: string, useBlobUrl: boolean = false): string {
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

// 需要整体渲染的标签列表
// 注意：`details` 必须排在最前面！
// 这样 `details` 会最先被提取，内部的 `piney_render` 会保持原样在 `details` 块内。
// 如果 `piney_render` 先提取，则 `details` 内部只剩占位符，后续处理会丢失内容。
// 注意：不要添加 `summary`，它是 `details` 的子元素，不应单独提取！
const RENDER_BLOCK_TAGS = ['details', 'piney_render', 'orange'];

function extractRenderBlocks(text: string): { text: string; blocks: string[] } {
    const blocks: string[] = [];
    let result = text;

    // 对每种标签类型进行提取
    for (const tagName of RENDER_BLOCK_TAGS) {
        // Construct Regex for finding this tag (Case Insensitive, support attributes)
        // Matches <tagName followed by space or >
        const startTagRegex = new RegExp(`<${tagName}([\\s>])`, 'i');
        const endTag = `</${tagName}>`;


        let newResult = '';
        let i = 0;

        while (i < result.length) {
            // Find start tag using regex on the substring
            const substr = result.slice(i);
            const match = substr.match(startTagRegex);

            if (!match) {
                newResult += substr;
                break;
            }


            const startIdx = i + match.index!;
            const startTagLen = match[0].length;

            // 添加标签前的内容
            newResult += result.slice(i, startIdx);

            // 使用深度计数找到匹配的闭合标签
            // 首先找到开始标签的闭合 >
            let j = result.indexOf('>', startIdx);
            if (j === -1) {
                // 没有找到闭合 >，跳过这个字符
                newResult += result[startIdx];
                i = startIdx + 1;
                continue;
            }
            j++; // 跳过 >

            let depth = 1;

            while (j < result.length && depth > 0) {
                const sub = result.slice(j);

                // Find next Start or End
                const nextStartMatch = sub.match(startTagRegex);
                const nextEndIdx = sub.toLowerCase().indexOf(endTag.toLowerCase());

                const nextStartRel = nextStartMatch ? nextStartMatch.index! : -1;

                if (nextEndIdx === -1) {
                    // Unclosed tag - treat rest of string as content? 
                    // Or just abort block? Standard browser behavior closes at end.
                    // Let's assume end of string closes it to avoid infinite loop
                    j = result.length;
                    break;
                }

                // If found start tag CLOSER than end tag
                if (nextStartRel !== -1 && nextStartRel < nextEndIdx) {
                    depth++;
                    // Advance past the full nested start tag (find the closing >)
                    const nestedStartAbsIdx = j + nextStartRel;
                    const nestedTagEnd = result.indexOf('>', nestedStartAbsIdx);
                    if (nestedTagEnd !== -1) {
                        j = nestedTagEnd + 1;
                    } else {
                        j += nextStartRel + nextStartMatch![0].length;
                    }
                } else {
                    depth--;
                    // Advance past this end tag
                    j += nextEndIdx + endTag.length;
                }
            }

            // 提取整个块
            const blockContent = result.slice(startIdx, j);
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

function processNestedDetails(content: string): string {
    // Use depth-counting to correctly handle nested <details> tags
    // Cannot use simple regex because [\s\S]*? is non-greedy and breaks nesting

    const startTagRegex = /<details([^>]*)>/gi;
    const endTag = '</details>';

    let result = '';
    let i = 0;
    const lowerContent = content.toLowerCase();

    while (i < content.length) {
        // Find next <details> tag
        startTagRegex.lastIndex = i;
        const startMatch = startTagRegex.exec(content);

        if (!startMatch) {
            result += content.slice(i);
            break;
        }

        // Add content before this <details>
        result += content.slice(i, startMatch.index);

        const attrs = startMatch[1] || '';
        const startIdx = startMatch.index;
        let j = startMatch.index + startMatch[0].length;

        // Use depth counting to find matching </details>
        let depth = 1;
        while (j < content.length && depth > 0) {
            const remainingLower = lowerContent.slice(j);
            const nextStartIdx = remainingLower.search(/<details[\s>]/i);
            const nextEndIdx = remainingLower.indexOf(endTag.toLowerCase());

            if (nextEndIdx === -1) {
                // No closing tag found, take rest of string
                j = content.length;
                break;
            }

            if (nextStartIdx !== -1 && nextStartIdx < nextEndIdx) {
                // Found nested <details> before </details>
                depth++;
                j += nextStartIdx + 1; // Move past '<'
                // Find the end of this start tag
                const tagEnd = content.indexOf('>', j);
                if (tagEnd !== -1) j = tagEnd + 1;
            } else {
                // Found </details>
                depth--;
                if (depth === 0) {
                    j += nextEndIdx + endTag.length;
                } else {
                    j += nextEndIdx + endTag.length;
                }
            }
        }

        // Extract the full <details>...</details> block
        const fullBlock = content.slice(startIdx, j);
        const innerContent = content.slice(startMatch.index + startMatch[0].length, j - endTag.length);

        // Process this details block
        result += processSingleDetails(attrs, innerContent);
        i = j;
    }

    return result;
}

// Helper function to process a single <details> block (may recursively call processNestedDetails)
function processSingleDetails(attrs: string, innerContent: string): string {
    let summaryHTML = '';
    let bodyContent = innerContent;

    // Extract summary if present
    const sumMatch = innerContent.match(/^\s*<summary([^>]*)>([\s\S]*?)<\/summary>/i);
    if (sumMatch) {
        const sumAttrs = sumMatch[1];
        const sumContent = sumMatch[2];
        summaryHTML = `<summary${sumAttrs}>${marked.parseInline(sumContent, { breaks: true })}</summary>`;
        bodyContent = innerContent.slice(sumMatch[0].length);
    }

    // FAST PATH: If content contains rich HTML (piney_render, style, div with class/style, etc.),
    // skip all complex processing and just return the HTML as-is to preserve the embedded content
    const hasRichHtml = /<piney_render[\s>]/i.test(bodyContent) ||
        /<style[\s>]/i.test(bodyContent) ||
        /<div\s+[^>]*(class|style)\s*=/i.test(bodyContent) ||
        /<span\s+[^>]*style\s*=/i.test(bodyContent);

    if (hasRichHtml) {
        return `<details${attrs}>${summaryHTML}${bodyContent}</details>`;
    }

    // Step 1: Recursively process any nested <details> in body FIRST
    bodyContent = processNestedDetails(bodyContent);

    // Step 2: Protect code blocks before Markdown processing
    const codeBlocks: { language: string; content: string }[] = [];
    let protectedBody = bodyContent.replace(/```(\w*)\n?([\s\S]*?)```/g, (match: string, lang: string, code: string) => {
        const idx = codeBlocks.length;
        codeBlocks.push({ language: lang || '', content: code });
        return `<!--DETAILS_CODE_${idx}-->`;
    });

    // Step 3: Protect already-processed <details> blocks from Markdown
    // Must use depth-counting because [\s\S]*? is non-greedy and breaks nesting
    const detailsBlocks: string[] = [];
    let detailsTempBody = protectedBody;
    let detailsProtectedBody = '';
    let detailsScanIdx = 0;
    const detailsStartRegex = /<details([\s>])/i;
    const detailsEndTag = '</details>';

    while (detailsScanIdx < detailsTempBody.length) {
        const substr = detailsTempBody.slice(detailsScanIdx);
        const startMatch = substr.match(detailsStartRegex);

        if (!startMatch) {
            detailsProtectedBody += substr;
            break;
        }

        const startIdx = detailsScanIdx + startMatch.index!;
        detailsProtectedBody += detailsTempBody.slice(detailsScanIdx, startIdx);

        // Find matching end tag with depth counting
        // First, find the end of the opening tag (the closing >)
        let j = detailsTempBody.indexOf('>', startIdx);
        if (j === -1) {
            // No closing > for start tag, skip this
            detailsProtectedBody += detailsTempBody[startIdx];
            detailsScanIdx = startIdx + 1;
            continue;
        }
        j++; // Move past the >

        let depth = 1;
        while (j < detailsTempBody.length && depth > 0) {
            const sub = detailsTempBody.slice(j);
            const nextStartMatch = sub.match(detailsStartRegex);
            const nextEndIdx = sub.toLowerCase().indexOf(detailsEndTag.toLowerCase());
            const nextStartRel = nextStartMatch ? nextStartMatch.index! : -1;

            if (nextEndIdx === -1) {
                j = detailsTempBody.length;
                break;
            }

            if (nextStartRel !== -1 && nextStartRel < nextEndIdx) {
                depth++;
                // Skip past the full opening tag (find the closing >)
                const nestedStartAbsIdx = j + nextStartRel;
                const nestedTagEnd = detailsTempBody.indexOf('>', nestedStartAbsIdx);
                if (nestedTagEnd !== -1) {
                    j = nestedTagEnd + 1;
                } else {
                    j += nextStartRel + nextStartMatch![0].length;
                }
            } else {
                depth--;
                j += nextEndIdx + detailsEndTag.length;
            }
        }

        const blockContent = detailsTempBody.slice(startIdx, j);
        const idx = detailsBlocks.length;
        detailsBlocks.push(blockContent);
        detailsProtectedBody += `<!--NESTED_DETAILS_${idx}-->`;
        detailsScanIdx = j;
    }
    protectedBody = detailsProtectedBody;

    // Step 3.5: Protect <piney_render> blocks from Markdown (they should be preserved as-is)
    const pineyRenderBlocks: string[] = [];
    // Use depth-counting to find matching closing tags
    let tempBody = protectedBody;
    let protectedBodyNew = '';
    let scanIdx = 0;
    const pineyStartRegex = /<piney_render([\s>])/i;
    const pineyEndTag = '</piney_render>';

    while (scanIdx < tempBody.length) {
        const substr = tempBody.slice(scanIdx);
        const startMatch = substr.match(pineyStartRegex);

        if (!startMatch) {
            protectedBodyNew += substr;
            break;
        }

        const startIdx = scanIdx + startMatch.index!;
        protectedBodyNew += tempBody.slice(scanIdx, startIdx);

        // Find matching end tag with depth counting
        // First, find the end of the opening tag (the closing >)
        let j = tempBody.indexOf('>', startIdx);
        if (j === -1) {
            // No closing > for start tag, skip this
            protectedBodyNew += tempBody[startIdx];
            scanIdx = startIdx + 1;
            continue;
        }
        j++; // Move past the >

        let depth = 1;
        while (j < tempBody.length && depth > 0) {
            const sub = tempBody.slice(j);
            const nextStartMatch = sub.match(pineyStartRegex);
            const nextEndIdx = sub.toLowerCase().indexOf(pineyEndTag.toLowerCase());
            const nextStartRel = nextStartMatch ? nextStartMatch.index! : -1;

            if (nextEndIdx === -1) {
                j = tempBody.length;
                break;
            }

            if (nextStartRel !== -1 && nextStartRel < nextEndIdx) {
                depth++;
                // Skip past the full opening tag (find the closing >)
                const nestedStartAbsIdx = j + nextStartRel;
                const nestedTagEnd = tempBody.indexOf('>', nestedStartAbsIdx);
                if (nestedTagEnd !== -1) {
                    j = nestedTagEnd + 1;
                } else {
                    j += nextStartRel + nextStartMatch![0].length;
                }
            } else {
                depth--;
                j += nextEndIdx + pineyEndTag.length;
            }
        }

        const blockContent = tempBody.slice(startIdx, j);
        const idx = pineyRenderBlocks.length;
        pineyRenderBlocks.push(blockContent);
        protectedBodyNew += `<!--PINEY_RENDER_${idx}-->`;
        scanIdx = j;
    }
    protectedBody = protectedBodyNew;

    // Step 4: Check if content is already HTML (contains styled block elements)
    // If so, skip Markdown parsing to avoid escaping the HTML
    // Also skip if we have piney_render placeholders (content was already HTML)
    const hasPineyRenderPlaceholders = pineyRenderBlocks.length > 0;
    const isAlreadyHtml = hasPineyRenderPlaceholders ||
        /<(div|p|span|strong|em|table|ul|ol|li)\s+[^>]*style\s*=/i.test(protectedBody) ||
        /<(div|table|ul|ol)\s*>/i.test(protectedBody.trim().slice(0, 100));

    let bodyHTML: string;
    if (isAlreadyHtml) {
        // Content is already HTML, just preserve it (only apply minimal newline normalization)
        bodyHTML = protectedBody;
    } else {
        // Content is Markdown, parse it
        const safeBody = protectedBody.replace(/\n/g, '  \n');
        bodyHTML = marked.parse(safeBody, { async: false, breaks: true }) as string;
    }

    // Step 5: Restore nested details blocks
    detailsBlocks.forEach((block, idx) => {
        const placeholder = `<!--NESTED_DETAILS_${idx}-->`;
        bodyHTML = bodyHTML.replace(`<p>${placeholder}</p>`, block);
        bodyHTML = bodyHTML.replace(placeholder, block);
    });

    // Step 5.5: Restore piney_render blocks (preserved as-is, wrapped in div for styling)
    pineyRenderBlocks.forEach((block, idx) => {
        const placeholder = `<!--PINEY_RENDER_${idx}-->`;
        // Wrap in a div with class for potential styling, but keep content as-is
        const wrappedBlock = `<div class="piney-render-block">${block}</div>`;
        bodyHTML = bodyHTML.replace(`<p>${placeholder}</p>`, wrappedBlock);
        bodyHTML = bodyHTML.replace(placeholder, wrappedBlock);
    });

    // Step 6: Restore code blocks with proper styling
    codeBlocks.forEach((block, idx) => {
        const placeholder = `<!--DETAILS_CODE_${idx}-->`;
        const langClass = block.language ? `language-${block.language}` : '';
        const escaped = block.content
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;');

        // Check if it's an HTML code block (should be rendered in iframe by history page)
        const isHtml = isHtmlCodeBlock(block.language);
        const codeHtml = isHtml
            ? `<pre class="piney-iframe-code" data-lang="${block.language}"><code class="${langClass}">${escaped}</code></pre>`
            : `<pre><code class="${langClass}">${escaped}</code></pre>`;

        bodyHTML = bodyHTML.replace(`<p>${placeholder}</p>`, codeHtml);
        bodyHTML = bodyHTML.replace(placeholder, codeHtml);
    });

    return `<details${attrs}>${summaryHTML}${bodyHTML}</details>`;
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
    detectedTags: Set<string>; // New field
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

    // ========== Step 2.5: 标签检测 ==========
    // 在这里检测标签（正则处理后，渲染块提取前）
    const detectedTags = detectTags(res);


    // ========== Step 3: 提取渲染块 ==========
    // 必须在正则处理之后，因为正则会添加 <piney_render> 标签
    // 支持 piney_render, orange 等标签
    let { text: afterRenderExtract, blocks: renderBlocks } = extractRenderBlocks(res);
    res = afterRenderExtract;


    // ========== Step 3.5: 对提取的渲染块应用设置 ==========
    // 因为渲染块被提取保护了，所以需要单独对其应用隐藏/换行设置
    if (renderBlocks.length > 0) {
        renderBlocks = renderBlocks.map(block => {
            const match = block.match(/^<([\w-]+)/);
            if (!match) return block;
            const tagName = match[1].toLowerCase();


            // 1. 过滤（隐藏）- Case Insensitive Check
            if (options.hiddenTags && options.hiddenTags.some(t => t.toLowerCase() === tagName)) {
                return ''; // Replace with empty string (removes block)
            }

            // 2. Details 特殊处理 (使用 processSingleDetails 函数，它已经包含 piney_render 保护逻辑)
            if (tagName === 'details') {
                // Capture Attributes before stripping
                const startTagMatch = block.match(/^<details([^>]*)>/i);
                const attrs = startTagMatch ? startTagMatch[1] : '';

                // Remove outer tags
                const content = block.replace(/^<details[^>]*>/i, '').replace(/<\/details>$/i, '');

                // 使用 processSingleDetails 处理，它会保护嵌套的 piney_render
                return processSingleDetails(attrs, content);
            }

            // 3. 换行处理 - Case Insensitive Check
            // 如果标签启用了换行，我们对块内容应用 processTagNewlines
            if (options.newlineTags && options.newlineTags.some(t => t.toLowerCase() === tagName)) {
                return processTagNewlines(block, [tagName], [tagName], true);
            }
            return block;
        });
    }

    // ========== Step 3.6: 处理嵌套的 <details> ==========
    // 因为 piney_render 可能包含 <details>，而 <details> 需要特殊 Markdown 处理
    // 这里对所有渲染块递归处理其中的 <details>
    // 注意：跳过 details 块本身，因为它们已经在 Step 3.5 中处理过了
    if (renderBlocks.length > 0) {
        renderBlocks = renderBlocks.map(block => {
            // 如果块本身就是 <details>，跳过（已在 Step 3.5 处理）
            if (/^<details[\s>]/i.test(block)) {
                return block;
            }
            return processNestedDetails(block);
        });
    }

    // ========== Step 4: 标签过滤（剩余文本）==========
    if (options.hiddenTags && options.hiddenTags.length > 0) {
        res = smartFilterTags(res, options.hiddenTags);
    }

    // ========== Step 5: 标签换行处理（剩余文本）==========
    // 不强制分行，交给 Markdown 处理
    // 所以这里不再对剩余文本进行 processTagNewlines 处理
    /*
    if (options.newlineTags) {
        const allTags = [...new Set([...(options.hiddenTags || []), ...options.newlineTags])];
        res = processTagNewlines(res, allTags, options.newlineTags);
    }
    */

    // ========== Step 6: 压缩过多空行 ==========
    res = res.replace(/(\n\s*){3,}/g, '\n\n');

    // ========== Step 7: Markdown 渲染 ==========
    let html = marked.parse(res, { async: false, breaks: true }) as string;

    // ========== Step 8: 恢复渲染块占位符 ==========
    // 首先，递归解析渲染块内部的占位符（处理嵌套情况，如 piney_render > details）
    // 需要多次遍历，因为可能有多层嵌套
    let resolved = true;
    while (resolved) {
        resolved = false;
        for (let i = 0; i < renderBlocks.length; i++) {
            for (let j = 0; j < renderBlocks.length; j++) {
                if (i !== j) {
                    const placeholder = `<!--RENDER_BLOCK_${j}-->`;
                    if (renderBlocks[i].includes(placeholder)) {
                        renderBlocks[i] = renderBlocks[i].replace(placeholder, renderBlocks[j]);
                        resolved = true;
                    }
                }
            }
        }
    }

    // 然后恢复到主 HTML
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
        html: html,
        codeBlocks,
        renderBlocks,
        detectedTags
    };
}

// --- 兼容旧 API（返回纯 HTML 字符串）---
export function processTextWithPipelineSimple(text: string, options: RenderOptions): string {
    return processTextWithPipeline(text, options).html;
}
