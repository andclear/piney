/**
 * World Info Converter Utilities
 * 
 * Implements conversion logic between Global World Info (SillyTavern/Pygmalion format, flattened)
 * and Inline Character Book (Piney/V2 Character Card format, nested in extensions).
 * 
 * Rules from docs/global2char.md
 */

// --- Types ---

export interface GlobalWorldInfoEntry {
    uid: number | string;
    key: string[]; // keys
    keysecondary: string[]; // secondary_keys
    comment: string;
    content: string;
    constant: boolean;
    selective: boolean;
    order: number; // insertion_order
    disable: boolean; // INVERSE of enabled
    excludeRecursion: boolean;
    displayIndex: number;
    probability: number;
    useProbability: boolean;
    depth: number;
    selectiveLogic: number;
    outletName: string;
    group: string;
    groupOverride: boolean;
    groupWeight: number;
    preventRecursion: boolean;
    delayUntilRecursion: number | boolean;
    scanDepth?: number | null;
    matchWholeWords?: boolean | null;
    useGroupScoring?: boolean | null;
    caseSensitive?: boolean | null;
    automationId: string;
    role: number | null;
    vectorized: boolean;
    sticky?: number | null;
    cooldown?: number | null;
    delay?: number | null;
    matchPersonaDescription: boolean;
    matchCharacterDescription: boolean;
    matchCharacterPersonality: boolean;
    matchCharacterDepthPrompt: boolean;
    matchScenario: boolean;
    matchCreatorNotes: boolean;
    triggers: string[];
    ignoreBudget: boolean;
    // ... possibly others
    [key: string]: any;
}

export interface CharacterBookEntry {
    id: number | string;
    keys: string[];
    secondary_keys: string[];
    comment: string;
    content: string;
    constant: boolean;
    selective: boolean;
    insertion_order: number;
    enabled: boolean;
    position: "before_char" | "after_char";
    use_regex: boolean; // Always true for ST compatibility
    extensions: {
        position: number;
        exclude_recursion: boolean;
        display_index: number;
        probability: number;
        useProbability: boolean;
        depth: number;
        selectiveLogic: number;
        outlet_name: string;
        group: string;
        group_override: boolean;
        group_weight: number;
        prevent_recursion: boolean;
        delay_until_recursion: number | boolean;
        scan_depth?: number | null;
        match_whole_words?: boolean | null;
        use_group_scoring?: boolean | null;
        case_sensitive?: boolean | null;
        automation_id: string;
        role: number | null;
        vectorized: boolean;
        sticky?: number | null;
        cooldown?: number | null;
        delay?: number | null;
        match_persona_description?: boolean;
        match_character_description?: boolean;
        match_character_personality?: boolean;
        match_character_depth_prompt?: boolean;
        match_scenario?: boolean;
        match_creator_notes?: boolean;
        triggers?: string[];
        ignore_budget?: boolean;
        [key: string]: any;
    };
}

export interface GlobalWorldInfo {
    entries: Record<string, GlobalWorldInfoEntry>;
    name: string;
}

export interface CharacterBook {
    name?: string;
    description?: string;
    scan_depth?: number;
    token_budget?: number;
    recursive_scanning?: boolean;
    extensions?: Record<string, any>;
    entries: CharacterBookEntry[];
}

// --- Conversion Logic ---

/**
 * Converts a specific Global World Info Entry to Character Book Entry format.
 */
export function convertGlobalEntryToInline(entry: GlobalWorldInfoEntry): CharacterBookEntry {
    // 1. Basic Fields Mapping
    const inline: CharacterBookEntry = {
        id: entry.uid,
        keys: entry.key || [],
        secondary_keys: entry.keysecondary || [],
        comment: entry.comment || "",
        content: entry.content || "",
        constant: entry.constant ?? false,
        selective: entry.selective ?? true,
        insertion_order: entry.order ?? 100,
        enabled: !entry.disable, // Inverted
        position: (entry.position === 0) ? "before_char" : "after_char",
        use_regex: true, // Always true
        extensions: {
            position: typeof entry.position === 'number' ? entry.position : 1, // Store original number
            exclude_recursion: entry.excludeRecursion ?? false,
            display_index: entry.displayIndex ?? 0,
            probability: entry.probability ?? 100,
            useProbability: entry.useProbability ?? true,
            depth: entry.depth ?? 4,
            selectiveLogic: entry.selectiveLogic ?? 0,
            outlet_name: entry.outletName || "",
            group: entry.group || "",
            group_override: entry.groupOverride ?? false,
            group_weight: entry.groupWeight ?? 100,
            prevent_recursion: entry.preventRecursion ?? false,
            delay_until_recursion: entry.delayUntilRecursion ?? 0,
            scan_depth: entry.scanDepth,
            match_whole_words: entry.matchWholeWords,
            use_group_scoring: entry.useGroupScoring,
            case_sensitive: entry.caseSensitive,
            automation_id: entry.automationId || "",
            role: entry.role ?? 0,
            vectorized: entry.vectorized ?? false,
            sticky: entry.sticky,
            cooldown: entry.cooldown,
            delay: entry.delay,
            match_persona_description: entry.matchPersonaDescription ?? false,
            match_character_description: entry.matchCharacterDescription ?? false,
            match_character_personality: entry.matchCharacterPersonality ?? false,
            match_character_depth_prompt: entry.matchCharacterDepthPrompt ?? false,
            match_scenario: entry.matchScenario ?? false,
            match_creator_notes: entry.matchCreatorNotes ?? false,
            triggers: entry.triggers || [],
            ignore_budget: entry.ignoreBudget ?? false,
        }
    };

    // Preserve any extra fields in extensions
    const handledKeys = new Set([
        'uid', 'key', 'keysecondary', 'comment', 'content', 'constant', 'selective',
        'order', 'disable', 'position', 'excludeRecursion', 'displayIndex', 'probability',
        'useProbability', 'depth', 'selectiveLogic', 'outletName', 'group', 'groupOverride',
        'groupWeight', 'preventRecursion', 'delayUntilRecursion', 'scanDepth', 'matchWholeWords',
        'useGroupScoring', 'caseSensitive', 'automationId', 'role', 'vectorized', 'sticky',
        'cooldown', 'delay', 'matchPersonaDescription', 'matchCharacterDescription',
        'matchCharacterPersonality', 'matchCharacterDepthPrompt', 'matchScenario',
        'matchCreatorNotes', 'triggers', 'ignoreBudget'
    ]);

    for (const k in entry) {
        if (!handledKeys.has(k)) {
            inline.extensions[k] = entry[k];
        }
    }

    return inline;
}

/**
 * Converts a specific Character Book Entry to Global World Info Entry format.
 */
export function convertInlineEntryToGlobal(entry: CharacterBookEntry): GlobalWorldInfoEntry {
    const ext = entry.extensions || {};

    const global: GlobalWorldInfoEntry = {
        uid: entry.id,
        key: entry.keys || [],
        keysecondary: entry.secondary_keys || [],
        comment: entry.comment || "",
        content: entry.content || "",
        constant: entry.constant ?? false,
        selective: entry.selective ?? true,
        order: entry.insertion_order ?? 100,
        disable: !entry.enabled, // Inverted
        position: typeof ext.position === 'number' ? ext.position : (entry.position === 'before_char' ? 0 : 1),

        // Extensions flattened
        excludeRecursion: ext.exclude_recursion ?? false,
        displayIndex: ext.display_index ?? 0,
        probability: ext.probability ?? 100,
        useProbability: ext.useProbability ?? true,
        depth: ext.depth ?? 4,
        selectiveLogic: ext.selectiveLogic ?? 0,
        outletName: ext.outlet_name || "",
        group: ext.group || "",
        groupOverride: ext.group_override ?? false,
        groupWeight: ext.group_weight ?? 100,
        preventRecursion: ext.prevent_recursion ?? false,
        delayUntilRecursion: ext.delay_until_recursion ?? 0,
        scanDepth: ext.scan_depth,
        matchWholeWords: ext.match_whole_words,
        useGroupScoring: ext.use_group_scoring,
        caseSensitive: ext.case_sensitive,
        automationId: ext.automation_id || "",
        role: ext.role ?? 0,
        vectorized: ext.vectorized ?? false,
        sticky: ext.sticky,
        cooldown: ext.cooldown,
        delay: ext.delay,
        matchPersonaDescription: ext.match_persona_description ?? false,
        matchCharacterDescription: ext.match_character_description ?? false,
        matchCharacterPersonality: ext.match_character_personality ?? false,
        matchCharacterDepthPrompt: ext.match_character_depth_prompt ?? false,
        matchScenario: ext.match_scenario ?? false,
        matchCreatorNotes: ext.match_creator_notes ?? false,
        triggers: ext.triggers || [],
        ignoreBudget: ext.ignore_budget ?? false,
    };

    // Preserve extras from extensions
    const handledExtKeys = new Set([
        'position', 'exclude_recursion', 'display_index', 'probability', 'useProbability',
        'depth', 'selectiveLogic', 'outlet_name', 'group', 'group_override', 'group_weight',
        'prevent_recursion', 'delay_until_recursion', 'scan_depth', 'match_whole_words',
        'use_group_scoring', 'case_sensitive', 'automation_id', 'role', 'vectorized',
        'sticky', 'cooldown', 'delay', 'match_persona_description', 'match_character_description',
        'match_character_personality', 'match_character_depth_prompt', 'match_scenario',
        'match_creator_notes', 'triggers', 'ignore_budget'
    ]);

    // Also preserve extras from entry root if any (less likely but good for unknown fields)
    // ...

    for (const k in ext) {
        if (!handledExtKeys.has(k)) {
            global[k] = ext[k];
        }
    }

    return global;
}

/**
 * Converts a Global World Info object to a Character Book object.
 */
export function convertGlobalToCharacterBook(global: GlobalWorldInfo): CharacterBook {
    const book: CharacterBook = {
        name: global.name,
        entries: []
    };

    if (global.entries) {
        // Global entries are Object map (uid string -> entry) or Array (rarely in some formats but here we assume map per docs)
        const entriesList = Array.isArray(global.entries)
            ? global.entries
            : Object.values(global.entries);

        book.entries = entriesList.map(convertGlobalEntryToInline);
    }

    return book;
}

/**
 * Converts a Character Book object to a Global World Info object.
 */
export function convertCharacterBookToGlobal(book: CharacterBook): GlobalWorldInfo {
    const global: GlobalWorldInfo = {
        name: book.name || "Exported World Info",
        entries: {}
    };

    if (book.entries) {
        let index = 0;
        for (const entry of book.entries) {
            // Determine UID: prefer existing ID, else index
            const uid = (entry.id !== undefined && entry.id !== null) ? entry.id : index;
            global.entries[uid] = convertInlineEntryToGlobal(entry);
            index++;
        }
    }

    return global;
}
