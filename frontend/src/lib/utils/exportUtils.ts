export interface ChatMessage {
    name?: string;
    is_user?: boolean;
    mes?: string;
    message?: string;
    message_id?: number;
    [key: string]: any;
}

const STANDARD_HTML_TAGS = new Set([
    'html', 'head', 'body', 'div', 'span', 'p', 'a', 'img', 'br', 'hr',
    'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'ul', 'ol', 'li',
    'table', 'tr', 'td', 'th', 'thead', 'tbody',
    'strong', 'b', 'em', 'i', 'u', 's', 'del', 'ins', 'sub', 'sup',
    'pre', 'code', 'blockquote', 'q', 'cite',
    'details', 'summary', 'header', 'footer', 'nav', 'main', 'section', 'article', 'aside',
    'figure', 'figcaption', 'style', 'script', 'form', 'input', 'button', 'select', 'option', 'textarea', 'label'
]);

/**
 * Scans the file content for all unique XML-like tag names.
 * Uses loose token detection (start or end tags) to support nested tags.
 * Ignores tags inside code blocks and removes comments.
 */
export function scanTags(fileContent: string): string[] {
    const tagCounts = new Map<string, number>();
    // Loose regex to match <tag> or </tag>
    const regex = /<\/?([\p{L}0-9_\-\.]+)(?:\s[^>]*)?>/gu;

    const lines = fileContent.trim().split('\n');
    for (let i = 1; i < lines.length; i++) {
        try {
            const rawLine = lines[i];
            if (!rawLine.includes('<')) continue;

            const msg = JSON.parse(rawLine);
            let content = msg.mes || msg.message || "";
            if (!content) continue;

            content = content.replace(/```[\s\S]*?```/g, '');
            content = content.replace(/`[^`]*`/g, '');
            content = content.replace(/<!--[\s\S]*?-->/g, '');

            // Special handling for <think> and <thinking>:
            // Scan them first and remove them so nested tags are NOT scanned.
            const thinkRegex = /<(think|thinking)(?:\s[^>]*)?>([\s\S]*?)<\/\1>/gi;
            content = content.replace(thinkRegex, (match: string, tag: string) => {
                // We count the think tag itself
                tagCounts.set(tag, (tagCounts.get(tag) || 0) + 1);
                return ""; // Remove entire block
            });

            const matches = content.matchAll(regex);
            for (const m of matches) {
                const tag = m[1]; // original case
                const tagLower = tag.toLowerCase();

                // Filter out standard HTML tags
                if (!STANDARD_HTML_TAGS.has(tagLower)) {
                    tagCounts.set(tag, (tagCounts.get(tag) || 0) + 1);
                }
            }
        } catch { }
    }

    // Sort by count descending, with priority overrides
    const priority = ['content', 'status', 'small theater', 'small-theater', 'small_theater'];

    return Array.from(tagCounts.entries())
        .sort((a, b) => {
            const [tagA, countA] = a;
            const [tagB, countB] = b;

            const idxA = priority.indexOf(tagA);
            const idxB = priority.indexOf(tagB);

            if (idxA !== -1 && idxB !== -1) return idxA - idxB;
            if (idxA !== -1) return -1;
            if (idxB !== -1) return 1;

            return countB - countA;
        })
        .map(entry => entry[0]);
}

export function convertJsonlToTxt(fileContent: string, selectedTags: string[] | null = null): string {
    const lines = fileContent.trim().split('\n');
    let output = "";
    let floorCounter = 0;

    // 1. Skip Header (index 0) if it looks like metadata, or just process it safely if logic requires.
    // The doc says first line is header.
    // Iterating from 1

    // Tag replacement regex (global)
    // Matches any paired tag: <tag ...>content</tag>
    const tagRegex = /<([a-zA-Z0-9_\-\.]+)(?:\s[^>]*)?>([\s\S]*?)<\/\1>/g;

    for (let i = 1; i < lines.length; i++) {
        if (!lines[i].trim()) continue;
        try {
            const msg: ChatMessage = JSON.parse(lines[i]);

            // Get name
            const name = msg.name || (msg.is_user ? "User" : "Character");

            // Get floor ID (mandatory per doc)
            // Use message_id if present, otherwise fallback to id or simple counter (Floor logic)
            // Since some JSONL exports (like SillyTavern) lack ID in the object, we use the line sequence.
            const floor = msg.message_id !== undefined ? msg.message_id : (msg.id !== undefined ? msg.id : floorCounter++);

            // Get content
            let content = msg.mes || msg.message || "";

            // 0. MASK CODE BLOCKS FIRST (Priority 1)
            // This prevents comments/tags INSIDE code from being stripped/scanned
            const codeBlocks: string[] = [];
            content = content.replace(/```[\s\S]*?```/g, (match) => {
                codeBlocks.push(match);
                return `__CODE_BLOCK_${codeBlocks.length - 1}__`;
            });
            const inlineCodeBlocks: string[] = [];
            content = content.replace(/`[^`]*`/g, (match) => {
                inlineCodeBlocks.push(match);
                return `__INLINE_CODE_${inlineCodeBlocks.length - 1}__`;
            });

            // 1. Remove comments <!-- ... --> (Priority 2)
            // Now safe to do, because code blocks are masked.
            content = content.replace(/<!--[\s\S]*?-->/g, '');

            // 2. HTML cleanup (br, div) -> newline (Priority 3)
            content = content.replace(/<br\s*\/?>/gi, '\n');
            content = content.replace(/<div[^>]*>/gi, '\n');

            // 3. Tag Filtering Logic (Priority 4)
            if (selectedTags !== null) {
                content = content.replace(tagRegex, (match, tag, innerText) => {
                    if (selectedTags.includes(tag)) {
                        return innerText; // Keep content
                    } else {
                        return ""; // Discard
                    }
                });
            } else {
                content = content.replace(/<\/?[^>]+(>|$)/g, "");
            }

            // 4. Final cleanup of any remaining HTML-ish tags (Priority 5)
            // Only if we are in filtering mode, clean up remaining tags. 
            // If in legacy mode, they were already stripped above.
            if (selectedTags !== null) {
                content = content.replace(/<\/?[^>]+(>|$)/g, "");
            }

            // 5. Restore Code Blocks (Priority 6)
            content = content.replace(/__INLINE_CODE_(\d+)__/g, (_, idx) => inlineCodeBlocks[parseInt(idx)]);
            content = content.replace(/__CODE_BLOCK_(\d+)__/g, (_, idx) => codeBlocks[parseInt(idx)]);

            // Constraint: No more than one empty line between content lines (i.e. max 2 newlines consecutive)
            // Replace 3 or more newlines with 2
            content = content.replace(/\n{3,}/g, '\n\n');

            // Format: [#Floor] 【Name】
            output += `[#${floor}] 【${name}】\n${content.trim()}\n\n--------------------\n\n`;

        } catch (e) {
            console.warn("Skipping invalid line", i, e);
        }
    }

    return output;
}
