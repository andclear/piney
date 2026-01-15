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

export function processContentWithScripts(content: string, scripts: RegexScript[]): string {
    if (!content) return "";
    let processed = content;

    for (const script of scripts) {
        if (script.disabled) continue;
        // Skip promptOnly scripts for display
        if (script.promptOnly) continue;

        try {
            // ST regex format usually implies flags. 
            // Often input as "/regex/flags". If just string, assume global? 
            // ST Regex scripts are usually just the pattern part or fully specified?
            // "findRegex": "/\\*.*\\*/|/\\/.*|/\\(.*\\)/g" example.

            let pattern = script.findRegex;
            let flags = "g"; // default global

            // Check if pattern is enclosed in slashes with flags
            // Use greedy match (.*) to find the LAST slash
            const match = pattern.trim().match(/^\/(.*)\/(.*)$/);
            if (match) {
                pattern = match[1];
                // Filter only valid flags, ignore garbage like illegal backslashes
                const validFlags = match[2].split('').filter(c => "gimsuy".includes(c)).join('');
                flags = validFlags || "g";
            }

            const regex = new RegExp(pattern, flags);

            // Handle replace string
            let replacement = script.replaceString || "";
            // Unescape common sequences that might be stored as literals in JSON
            replacement = replacement
                .replace(/\\n/g, '\n')
                .replace(/\\r/g, '\r')
                .replace(/\\"/g, '"') // Fix for users pasting JSON-escaped strings
                .replace(/\\t/g, '\t');

            // Special handling for {{saved:x}} macro? ST supports it, maybe skip for now or keep simple.

            processed = processed.replace(regex, replacement);
        } catch (e) {
            console.warn(`Failed to apply regex script ${script.scriptName}:`, e);
        }
    }

    return processed;
}
