<script lang="ts">
    import { onMount, onDestroy, untrack } from "svelte";
    import { EditorView, highlightSpecialChars, drawSelection, dropCursor, rectangularSelection, crosshairCursor, highlightActiveLine, keymap, placeholder as cmPlaceholder } from "@codemirror/view";
    import { EditorState, Compartment, StateEffect } from "@codemirror/state";
    import { history, defaultKeymap, historyKeymap } from "@codemirror/commands";
    import { syntaxHighlighting, defaultHighlightStyle, bracketMatching, foldKeymap } from "@codemirror/language";
    import { lineNumbers, highlightActiveLineGutter } from "@codemirror/view";
    import { indentWithTab } from "@codemirror/commands";
    import { searchKeymap, SearchQuery, search, setSearchQuery, findNext, findPrevious } from "@codemirror/search";
    import { html } from "@codemirror/lang-html";
    import { css } from "@codemirror/lang-css";
    import { javascript } from "@codemirror/lang-javascript";
    import { oneDark } from "@codemirror/theme-one-dark";
    import { indentationMarkers } from "@replit/codemirror-indentation-markers";
    import { Button } from "$lib/components/ui/button";
    import { Input } from "$lib/components/ui/input";
    import { cn } from "$lib/utils";
    import { 
        Search, 
        FileCode, 
        AlignLeft, 
        ArrowUp,
        ArrowDown,
        X
    } from "lucide-svelte";
    // @ts-ignore
    import js_beautify from "js-beautify";

    let { 
        value = $bindable(""), 
        language = $bindable("html"),
        placeholder: placeholderText = "",
        class: className = "",
        toolbarActions
    } = $props<{
        value: string;
        language?: "html" | "css" | "javascript";
        placeholder?: string;
        lastSaved?: number;
        toolbarActions?: any;
    }>();

    let editorElement: HTMLDivElement;
    let view: EditorView;
    let langCompartment = new Compartment();
    
    // Search State
    let isSearchOpen = $state(false);
    let searchQuery = $state("");
    let searchInput: HTMLInputElement;

    const languages = {
        html: html(),
        css: css(),
        javascript: javascript()
    };

    const beautifiers = {
        html: js_beautify.html,
        css: js_beautify.css,
        javascript: js_beautify.js
    };

    function formatCode() {
        if (!view) return;
        const currentCode = view.state.doc.toString();
        const formatter = beautifiers[language] || beautifiers.html;
        
        try {
            const formatted = formatter(currentCode, {
                indent_size: 2,
                preserve_newlines: true,
                max_preserve_newlines: 2,
                wrap_line_length: 120, // Set a reasonable line length
                wrap_attributes: 'auto', // Don't force expand attributes to new lines
                wrap_attributes_indent_size: 2,
                unformatted: [] // Ensure even inline tags are formatted if needed
            });
            
            view.dispatch({
                changes: {
                    from: 0,
                    to: view.state.doc.length,
                    insert: formatted
                }
            });
        } catch (e) {
            console.error("Formatting failed:", e);
        }
    }

    function toggleSearch() {
        isSearchOpen = !isSearchOpen;
        if (isSearchOpen) {
            setTimeout(() => {
                searchInput?.focus();
                updateSearchQuery();
            }, 0);
        } else {
            if (view) {
                view.dispatch({
                    effects: setSearchQuery.of(new SearchQuery({ search: "" }))
                });
                view.focus();
            }
        }
    }

    function updateSearchQuery() {
        if (!view || searchQuery === undefined) return;
        view.dispatch({
            effects: setSearchQuery.of(new SearchQuery({ 
                search: searchQuery,
                caseSensitive: false,
                regexp: false
            }))
        });
    }

    function findMatch(forward: boolean) {
        if (!view || !searchQuery) return;
        
        try {
            // Ensure query is synced
            updateSearchQuery();
            
            // Use standard CM commands
            if (forward) {
                findNext(view);
            } else {
                findPrevious(view);
            }
        } catch (e) {
            console.error("Search error:", e);
        }
    }

    function handleSearchKeydown(e: KeyboardEvent) {
        if (e.key === "Enter") {
            e.preventDefault();
            findMatch(!e.shiftKey);
        } else if (e.key === "Escape") {
            e.preventDefault();
            toggleSearch();
        }
    }
    
    // Sync search query when typing
    $effect(() => {
        if (isSearchOpen && searchQuery !== "") {
           untrack(() => updateSearchQuery());
        }
    });

    function updateLanguage(newLang: string) {
        language = newLang as any;
        view.dispatch({
            effects: langCompartment.reconfigure(languages[language])
        });
    }

    $effect(() => {
        if (view && value !== view.state.doc.toString()) {
            const current = view.state.doc.toString();
            if (value !== current) {
                untrack(() => {
                    view.dispatch({
                        changes: { from: 0, to: current.length, insert: value }
                    });
                });
            }
        }
    });

    onMount(() => {
        const state = EditorState.create({
            doc: value,
            extensions: [
                // Manual Basic Setup without foldGutter
                lineNumbers(),
                highlightActiveLineGutter(),
                highlightSpecialChars(),
                history(),
                drawSelection(),
                dropCursor(),
                EditorState.allowMultipleSelections.of(true),
                indentationMarkers(),
                EditorView.lineWrapping, // Soft wrapping
                syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
                bracketMatching(),
                rectangularSelection(),
                crosshairCursor(),
                highlightActiveLine(),
                search({ top: true }), // Enable search extension state
                keymap.of([
                    ...defaultKeymap,
                    ...historyKeymap,
                    ...foldKeymap,
                    indentWithTab
                    // Removed default searchKeymap to avoid conflicts with our UI, or keep it?
                    // Keeping it allows Ctrl+F/G to work naturally if focus is in editor.
                    // But we want to redirect Ctrl+F to OUR UI?
                    // For now, let's keep searchKeymap but maybe we should override Ctrl+F
                ]),
                langCompartment.of(languages[language]),
                oneDark, 
                cmPlaceholder(placeholderText),
                EditorView.updateListener.of((update) => {
                    if (update.docChanged) {
                        value = update.state.doc.toString();
                    }
                }),
                EditorView.theme({
                    "&": { height: "100%", fontSize: "14px" },
                    ".cm-scroller": { fontFamily: "monospace" },
                    ".cm-content": { minWidth: "100%" }
                })
            ]
        });

        view = new EditorView({
            state,
            parent: editorElement
        });

        // Add custom keymap to intercept Ctrl+F in editor to open our search bar
        view.dispatch({
            effects: StateEffect.appendConfig.of(
                keymap.of([
                    {
                        key: "Mod-f",
                        run: () => {
                            if (!isSearchOpen) toggleSearch();
                            searchInput?.focus();
                            searchInput?.select();
                            return true;
                        }
                    }
                ])
            )
        });

        return () => {
            view?.destroy();
        };
    });
</script>

<div class={cn("flex flex-col h-full w-full border rounded-md overflow-hidden bg-[#282c34]", className)}>
    <!-- Toolbar -->
    <div class="flex items-center gap-2 p-2 bg-muted/20 border-b border-white/10 text-white/80 shrink-0 min-h-[44px]">
        {#if isSearchOpen}
            <!-- Simple Search Bar -->
            <div class="flex items-center gap-2 w-full animate-in fade-in slide-in-from-top-1 duration-200">
                <Search class="h-4 w-4 opacity-50 ml-1" />
                <input
                    bind:this={searchInput}
                    bind:value={searchQuery}
                    onkeydown={handleSearchKeydown}
                    placeholder="查找..."
                    class="flex-1 bg-transparent border-none outline-none text-sm text-white placeholder:text-white/30 h-7"
                />
                
                <div class="flex items-center gap-1 border-l border-white/10 pl-2">
                    <Button 
                        variant="ghost" 
                        size="icon" 
                        class="h-6 w-6 hover:bg-white/10 hover:text-white rounded-sm"
                        onclick={() => findMatch(false)}
                        title="上一个 (Shift+Enter)"
                    >
                        <ArrowUp class="h-3.5 w-3.5" />
                    </Button>
                    <Button 
                        variant="ghost" 
                        size="icon" 
                        class="h-6 w-6 hover:bg-white/10 hover:text-white rounded-sm"
                        onclick={() => findMatch(true)}
                        title="下一个 (Enter)"
                    >
                        <ArrowDown class="h-3.5 w-3.5" />
                    </Button>
                    <Button 
                        variant="ghost" 
                        size="icon" 
                        class="h-6 w-6 hover:bg-red-500/20 hover:text-red-400 rounded-sm ml-1"
                        onclick={toggleSearch}
                        title="关闭搜索 (Esc)"
                    >
                        <X class="h-3.5 w-3.5" />
                    </Button>
                </div>
            </div>
        {:else}
            <!-- Standard Toolbar -->
            <div class="flex items-center gap-2 mr-auto animate-in fade-in duration-200">
                 <FileCode class="h-4 w-4 opacity-70" />
                 <select 
                    class="bg-transparent border border-white/20 rounded px-2 py-1 text-xs outline-none focus:border-primary/50 cursor-pointer hover:bg-white/5 transition-colors"
                    value={language}
                    onchange={(e) => updateLanguage(e.currentTarget.value)}
                >
                    <option value="html">HTML</option>
                    <option value="javascript">JavaScript</option>
                    <option value="css">CSS</option>
                </select>
            </div>
    
            <!-- Actions -->
            <Button 
                variant="ghost" 
                size="sm" 
                class="h-7 text-xs gap-1.5 hover:bg-white/10 hover:text-white"
                onclick={toggleSearch}
                title="搜索 (Ctrl+F)"
            >
                <Search class="h-3.5 w-3.5" />
                <span class="hidden sm:inline">搜索</span>
            </Button>
    
            <Button 
                variant="ghost" 
                size="sm" 
                class="h-7 text-xs gap-1.5 hover:bg-white/10 hover:text-white"
                onclick={formatCode}
                title="格式化代码"
            >
                <AlignLeft class="h-3.5 w-3.5" />
                <span class="hidden sm:inline">格式化</span>
            </Button>

            {#if toolbarActions}
                {@render toolbarActions()}
            {/if}
        {/if}
    </div>

    <!-- Editor Container -->
    <div class="flex-1 min-h-0 w-full relative" bind:this={editorElement}></div>
</div>


