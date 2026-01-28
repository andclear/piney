export function filterTagContent(text: string, tag: string): string {
    const tagLower = tag.toLowerCase();

    // Escape special characters in tag for Regex
    const escapedTag = tagLower.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

    // Regex for Open and Close tags with Unicode support
    // Modified to be more robust
    const openTagRegex = new RegExp(`<(${escapedTag})(?:\\s[^>]*)?>`, 'gi');
    const closeTagRegex = new RegExp(`</${escapedTag}>`, 'gi');

    // Collect positions
    const openTags: { index: number, end: number, type: 'open' }[] = [];
    const closeTags: { index: number, end: number, type: 'close' }[] = [];

    for (const match of text.matchAll(openTagRegex)) {
        openTags.push({ index: match.index!, end: match.index! + match[0].length, type: 'open' });
    }

    for (const match of text.matchAll(closeTagRegex)) {
        closeTags.push({ index: match.index!, end: match.index! + match[0].length, type: 'close' });
    }

    if (openTags.length === 0 && closeTags.length === 0) {
        return text;
    }

    // Collect ALL tag start positions (to determine where an unclosed tag automatically ends)
    // Matches any XML-like tag start <TagName ...> including unicode
    const anyTagStartRegex = /<[\p{L}0-9_\-\.]+(?:\s[^>]*)?>/gu;
    const allTagStartPositions: number[] = [];
    for (const match of text.matchAll(anyTagStartRegex)) {
        allTagStartPositions.push(match.index!);
    }

    // Sort all target tags by position
    const allTargetTags = [...openTags, ...closeTags].sort((a, b) => a.index - b.index);

    const removeRanges: { start: number, end: number }[] = [];
    const openStack: typeof openTags[0][] = [];

    // Set of current target tag open positions for quick lookup
    const currentTagOpenPositions = new Set(openTags.map(t => t.index));

    for (const tagObj of allTargetTags) {
        if (tagObj.type === 'open') {
            openStack.push(tagObj);
        } else {
            // Close tag
            if (openStack.length > 0) {
                // Matched pair: <tag>...</tag>
                const openTag = openStack.pop()!;
                removeRanges.push({ start: openTag.index, end: tagObj.end });
            } else {
                // Orphan close tag: 0...</tag>
                // "Delete from start of string to this close tag"
                removeRanges.push({ start: 0, end: tagObj.end });
            }
        }
    }

    for (const openTag of openStack) {
        let nextTagPos = text.length; // Default to end of string

        for (const pos of allTagStartPositions) {
            if (pos > openTag.index) {
                if (!currentTagOpenPositions.has(pos)) {
                    nextTagPos = pos;
                    break;
                }
            }
        }
        removeRanges.push({ start: openTag.index, end: nextTagPos });
    }

    if (removeRanges.length === 0) return text;

    // Merge overlapping ranges
    removeRanges.sort((a, b) => a.start - b.start);
    const mergedRanges = [removeRanges[0]];

    for (let i = 1; i < removeRanges.length; i++) {
        const last = mergedRanges[mergedRanges.length - 1];
        const current = removeRanges[i];
        if (current.start <= last.end) {
            last.end = Math.max(last.end, current.end);
        } else {
            mergedRanges.push(current);
        }
    }

    // Reconstruct string
    let result = '';
    let lastCursor = 0;
    for (const range of mergedRanges) {
        // Ensure bounds
        const start = Math.max(0, range.start);
        const end = Math.min(text.length, range.end);

        if (start > lastCursor) {
            result += text.slice(lastCursor, start);
        }
        lastCursor = Math.max(lastCursor, end);
    }
    result += text.slice(lastCursor);

    return result;
}

export function smartFilterTags(text: string, tags: string[]): string {
    let res = text;
    for (const tag of tags) {
        res = filterTagContent(res, tag);
    }
    return res;
}

/**
 * Detect all XML-like tags in the text
 * LOGIC SYNCED WITH project's `frontend/src/lib/utils/exportUtils.ts` -> `scanTags`
 * 1. Matches VALID PAIRED tags only: <tag>...</tag>
 * 2. Removes Code Blocks (```...```), Inline Code (`...`), and Comments (<!-- ... -->) before scanning.
 * 3. Does NOT use a hardcoded HTML blocklist (consistent with export tool).
 * 4. Supports Unicode (Chinese) tag names.
 */
export function detectTags(text: string): Set<string> {
    const tags = new Set<string>();

    // 1. Clean Content (Order matters: blocks -> inline -> comments)
    let content = text;
    content = content.replace(/```[\s\S]*?```/g, '');
    content = content.replace(/`[^`]*`/g, '');
    content = content.replace(/<!--[\s\S]*?-->/g, '');

    // 2. Strict Regex from exportUtils.ts (Updated for Unicode)
    // Matches <tag> ... </tag>
    // Group 1: Tag Name (Unicode letters, numbers, _, -, .)
    // Reference \1 ensures close tag matches open tag name
    const regex = /<([\p{L}0-9_\-\.]+)(?:\s[^>]*)?>[\s\S]*?<\/\1>/gu;

    for (const m of content.matchAll(regex)) {
        tags.add(m[1]);
    }
    return tags;
}

/**
 * Sorts tags based on priority and then alphabetically
 */
export function sortTags(tags: string[] | Set<string>): string[] {
    const tagList = Array.isArray(tags) ? tags : Array.from(tags);
    const priority = ['content', 'status', 'small theater', 'small-theater', 'small_theater'];

    return tagList.sort((a, b) => {
        const idxA = priority.indexOf(a.toLowerCase());
        const idxB = priority.indexOf(b.toLowerCase());

        if (idxA !== -1 && idxB !== -1) return idxA - idxB;
        if (idxA !== -1) return -1;
        if (idxB !== -1) return 1;

        return a.localeCompare(b);
    });
}

/**
 * Locate content ranges for a specific tag using robust parsing (same as filterTagContent)
 */
function locateTagContentRanges(text: string, tag: string): { start: number, end: number }[] {
    const tagLower = tag.toLowerCase();
    const escapedTag = tagLower.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

    const openTagRegex = new RegExp(`<(${escapedTag})(?:\\s[^>]*)?>`, 'gi');
    const closeTagRegex = new RegExp(`</${escapedTag}>`, 'gi');

    const openTags: { index: number, end: number, type: 'open' }[] = [];
    const closeTags: { index: number, end: number, type: 'close' }[] = [];

    for (const match of text.matchAll(openTagRegex)) {
        openTags.push({ index: match.index!, end: match.index! + match[0].length, type: 'open' });
    }
    for (const match of text.matchAll(closeTagRegex)) {
        closeTags.push({ index: match.index!, end: match.index! + match[0].length, type: 'close' });
    }

    if (openTags.length === 0 && closeTags.length === 0) return [];

    const anyTagStartRegex = /<[\p{L}0-9_\-\.]+(?:\s[^>]*)?>/gu;
    const allTagStartPositions: number[] = [];
    for (const match of text.matchAll(anyTagStartRegex)) {
        allTagStartPositions.push(match.index!);
    }

    const allTargetTags = [...openTags, ...closeTags].sort((a, b) => a.index - b.index);
    const contentRanges: { start: number, end: number }[] = [];
    const openStack: typeof openTags[0][] = [];
    const currentTagOpenPositions = new Set(openTags.map(t => t.index));

    for (const tagObj of allTargetTags) {
        if (tagObj.type === 'open') {
            openStack.push(tagObj);
        } else {
            if (openStack.length > 0) {
                const openTag = openStack.pop()!;
                if (tagObj.index > openTag.end) {
                    contentRanges.push({ start: openTag.end, end: tagObj.index });
                }
            }
        }
    }

    // Handle unclosed open tags
    for (const openTag of openStack) {
        let nextTagPos = text.length;
        for (const pos of allTagStartPositions) {
            if (pos > openTag.index && !currentTagOpenPositions.has(pos)) {
                nextTagPos = pos;
                break;
            }
        }
        if (nextTagPos > openTag.end) {
            contentRanges.push({ start: openTag.end, end: nextTagPos });
        }
    }

    return contentRanges;
}

/**
 * For specified tags, convert \n to <br> within their content
 * Preserves other HTML structure.
 * Uses Robust Parsing (Token-based) instead of simplified Regex.
 */
export function processTagNewlines(text: string, allTags: string[], enabledTags: string[]): string {
    if (!allTags.length) return text;

    // 0. Sort Tags: Enabled (Preserve Newline) First
    // This allows nested tags (e.g. Enabled Poem inside Disabled Thought) to process first and preserve their newlines,
    // before the outer tag collapses everything else.
    const sortedTags = [...allTags].sort((a, b) => {
        const aEnabled = enabledTags.includes(a);
        const bEnabled = enabledTags.includes(b);
        if (aEnabled && !bEnabled) return -1;
        if (!aEnabled && bEnabled) return 1;
        return 0;
    });

    // 1. Mask Code Blocks
    const codeBlocks: string[] = [];
    let maskedText = text.replace(/```[\s\S]*?```/g, (match) => {
        codeBlocks.push(match);
        return `__CODE_BLOCK_${codeBlocks.length - 1}__`;
    });

    // 2. Process Tags
    for (const tag of sortedTags) {
        const isEnabled = enabledTags.includes(tag);

        // Re-locate ranges in the CURRENT state (since we modify it)
        let ranges = locateTagContentRanges(maskedText, tag);

        if (ranges.length === 0) continue;

        // FILTER OVERLAPPING RANGES (Same-type nesting)
        // Keep only Outermost ranges (which contain the inner ones).
        // Since the setting applies to the whole tag type, processing the outer block is sufficient/correct.
        ranges = ranges.filter(r =>
            !ranges.some(other => other !== r && other.start <= r.start && other.end >= r.end)
        );

        // Sort ranges by start position
        ranges.sort((a, b) => a.start - b.start);

        let newText = "";
        let cursor = 0;

        for (const range of ranges) {
            // Append unaffected pre-content
            if (range.start > cursor) {
                newText += maskedText.slice(cursor, range.start);
            }

            // Process content
            const content = maskedText.slice(range.start, range.end);

            // Safety check for HTML block tags
            const hasCommonHtml = /<\/?(?:div|p|table|tr|td|th|ul|ol|li|script|style|blockquote|pre|form|input)\b/i.test(content);

            if (hasCommonHtml) {
                newText += content; // No change
            } else {
                // Apply transformation
                const processed = isEnabled
                    ? content.replace(/\n/g, '<br>')
                    : content.replace(/\n/g, ' ');
                newText += processed;
            }

            cursor = range.end;
        }

        // Append remaining text
        if (cursor < maskedText.length) {
            newText += maskedText.slice(cursor);
        }
        maskedText = newText;
    }

    // 3. Restore Code Blocks
    const res = maskedText.replace(/__CODE_BLOCK_(\d+)__/g, (match, index) => {
        return codeBlocks[parseInt(index)] || match;
    });

    return res;
}

