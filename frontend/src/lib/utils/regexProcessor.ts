export interface RegexScript {
    id: string;
    scriptName: string;
    findRegex: string;
    replaceString: string;
    trimStrings?: string[];
    placement?: number[];
    disabled?: boolean;
    markdownOnly?: boolean;
    promptOnly?: boolean;
    runOnEdit?: boolean;
    substituteRegex?: number;
    minDepth?: number | null;
    maxDepth?: number | null;
}

interface ProcessOptions {
    isMarkdown?: boolean; // Display context
    isPrompt?: boolean;   // Generation context
}

export function processContentWithScripts(content: string, scripts: RegexScript[], options: ProcessOptions = {}): string {
    if (!content) return "";
    let processed = content;

    for (const script of scripts) {
        if (script.disabled) continue;

        // Filter based on context (SillyTavern Logic)
        // 1. If script is markdownOnly (Display), run ONLY if isMarkdown is true.
        // 2. If script is promptOnly (Generation), run ONLY if isPrompt is true.
        // 3. If neither, run in both.
        if (script.markdownOnly && !options.isMarkdown) continue;
        if (script.promptOnly && !options.isPrompt) continue;

        try {

            // ST regex format usually implies flags. 
            // Often input as "/regex/flags". If just string, assume global? 
            let pattern = script.findRegex;
            let flags = "g"; // default global

            const trimmed = pattern.trim();
            // Robust parsing: Find the LAST slash that separates pattern and flags
            // Standard JS Regex literal syntax: /pattern/flags
            if (trimmed.startsWith("/") && trimmed.lastIndexOf("/") > 0) {
                const lastSlashIndex = trimmed.lastIndexOf("/");
                const extractedPattern = trimmed.substring(1, lastSlashIndex);
                const extractedFlags = trimmed.substring(lastSlashIndex + 1);

                // Validate flags: Keep only g, i, m, s, u, y
                // AND ensure 'g' is present if implied by context (though ST usually explicit)
                const validFlags = extractedFlags.split('').filter(c => "gimsuy".includes(c)).join('');

                pattern = extractedPattern;
                flags = validFlags || "g";
            }

            // Should fail if pattern is invalid
            const regex = new RegExp(pattern, flags);

            // Handle replace string
            let replacement = script.replaceString || "";

            // Unescape common sequences that might be stored as literals in JSON
            // But be careful NOT to break $1, $2, $& etc.
            // If the user literally typed "\n" in their replacement box, it comes as "\\n" in JSON -> "\n" in string
            // Wait, if JSON is "replacement": "\\n", JS string is "\n" (length 2). We want actual newline.
            // If JSON is "replacement": "\n", JS string is newline (length 1). Already good.
            // ST usually stores them as escaped strings.
            replacement = replacement
                .replace(/\\n/g, '\n')
                .replace(/\\r/g, '\r')
                .replace(/\\t/g, '\t')
                .replace(/\\"/g, '"');
            // If the string came from JSON.parse, \" is already "
            // If the user typed \" (literal backslash, quote), it is \\" in JSON.
            // We typically only unescape standard C-style escapes that users typed literally.
            ;

            // 自动为非空的替换内容添加 <piney_render> 标签
            // 这样渲染管道可以识别并正确处理替换后的内容
            if (replacement.trim()) {
                replacement = `<piney_render>\n${replacement}\n</piney_render>`;
            }

            // Note: JS replace(regex, string) automatically handles $1, $2, $&, $', $`
            // We do NOT need to implement them manually.

            processed = processed.replace(regex, replacement);
        } catch (e) {
            console.warn(`Failed to apply regex script ${script.scriptName}:`, e);
        }
    }

    return processed;
}
