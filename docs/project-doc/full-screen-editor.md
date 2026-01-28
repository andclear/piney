# 全屏编辑器 (Zen Mode) 实现指南

本文档介绍如何将一个普通的文本输入框扩展为支持“全屏 Zen 模式”的编辑器。该模式允许用户在一个大弹窗中专注于写作，并且解决了常见的“打开即滚动到底部”的体验问题。

## 1. 核心思路

1.  **组件封装**: 创建一个 `RichTextarea` 组件，包含默认的 `Textarea` 和一个控制全屏状态的按钮。
2.  **全屏容器**: 使用 `Dialog` (模态框) 作为全屏容器，设置 `max-w-full h-[95vh]` 等样式使其几乎占满屏幕。
3.  **状态同步**: 内部和外部使用同一个 `value` 变量（双向绑定）。
4.  **滚动重置 (关键点)**: 当全屏弹窗打开时，浏览器或组件的自动聚焦行为往往会导致 Textarea 滚动到光标所在的最底部（特别是长文本）。我们需要在弹窗打开的瞬间强制将滚动条置顶。

## 2. 代码实现

### 2.1 RichTextarea 组件结构

```svelte
<!-- src/lib/components/character/RichTextarea.svelte -->
<script lang="ts">
    import { Textarea } from "$lib/components/ui/textarea";
    import * as Dialog from "$lib/components/ui/dialog";
    import { Maximize2, Minimize2 } from "lucide-svelte";
    import { tick } from "svelte";
    
    // Props
    let { 
        value = $bindable(), 
        placeholder = "",
        label = "内容"
    } = $props();

    // 状态
    let isZenMode = $state(false);
    let zenTextarea: HTMLTextAreaElement | undefined = $state(); // 绑定 DOM 引用

    // 滚动重置逻辑 (使用 $effect 监听模式切换)
    $effect(() => {
        if (isZenMode) {
            // tick() 等待 DOM 更新（Dialog 渲染出来）
            tick().then(() => {
                if (zenTextarea) {
                    // 1. 强制滚动到顶部
                    zenTextarea.scrollTop = 0;
                    // 2. 将光标重置到开头 (可选，防止自动聚焦到底部)
                    zenTextarea.setSelectionRange(0, 0);
                    // 3. 聚焦
                    zenTextarea.focus();
                }
            });
        }
    });
</script>

<!-- 默认视图 (小输入框) -->
<div class="relative group">
    <div class="flex items-center justify-between mb-2">
        <label class="text-sm font-medium">{label}</label>
        <button 
            class="opacity-0 group-hover:opacity-100 transition-opacity p-1 text-muted-foreground hover:text-primary"
            onclick={() => isZenMode = true}
            title="全屏模式"
        >
            <Maximize2 class="h-4 w-4" />
        </button>
    </div>
    
    <Textarea bind:value {placeholder} class="min-h-[100px] resize-y" />
</div>

<!-- 全屏视图 (Zen Mode Dialog) -->
<Dialog.Root bind:open={isZenMode}>
    <Dialog.Content class="max-w-[95vw] w-full h-[90vh] flex flex-col p-6">
        <Dialog.Header>
            <Dialog.Title class="flex items-center gap-2 text-xl">
                <Maximize2 class="h-5 w-5 text-primary" />
                {label} - 全屏编辑
            </Dialog.Title>
            <Dialog.Description>
                按 ESC 退出，内容会自动保存。
            </Dialog.Description>
        </Dialog.Header>

        <!-- 
           大输入框区域 
           flex-1 让它占据剩余所有高度
        -->
        <div class="flex-1 mt-4 relative">
            <textarea
                bind:this={zenTextarea} 
                bind:value
                class="w-full h-full p-6 rounded-md border bg-muted/30 focus:bg-background transition-colors resize-none focus:outline-none focus:ring-2 focus:ring-primary/20 text-lg leading-relaxed font-mono"
            ></textarea>
        </div>

        <Dialog.Footer>
            <Button onclick={() => isZenMode = false}>
                <Minimize2 class="h-4 w-4 mr-2" /> 完成
            </Button>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>
```

## 3. 关键实现细节解析

### 3.1 `tick()` 的使用
在 `isZenMode` 变为 `true` 时，Svelte 刚刚开始渲染 Dialog 的 DOM 结构。如果不使用 `tick()` 或 `setTimeout`，直接访问 `zenTextarea` 可能是 `undefined`，或者此时浏览器还没有完成 Layout 计算，设置 `scrollTop` 无效。
`tick()` 确保了我们的代码在 DOM 更新完成后执行。

### 3.2 滚动位置控制
```javascript
zenTextarea.scrollTop = 0;
zenTextarea.setSelectionRange(0, 0);
```
这是**打开即显示顶部**的关键。标准 `<textarea>` 在 `focus()` 时，如果光标在末尾（通常是双向绑定的默认行为），浏览器会自动滚动到底部。
通过显式设置光标位置到 `0, 0` 和 `scrollTop = 0`，我们强制让用户从头开始阅读/编辑。

### 3.3 样式技巧
*   **Dialog 宽高度**: `max-w-[95vw] h-[90vh]` 保证了全屏感，但又保留了少许边距，显得不压抑。
*   **Flex 布局**: 使用 `flex-col` 和 `flex-1` 让 textarea 自动填满 Dialog 中 Header 和 Footer 之外的所有空间。
*   **字体设置**: 全屏模式通常用于长文写作，使用 `text-lg` (较大字号) 和 `leading-relaxed` (宽松行高) 可以显著提升阅读体验。

## 4. 扩展功能 (可选)

*   **字数统计**: 在 Footer 中实时显示 `value.length`。
*   **快捷键**: 监听 `Ctrl+Enter` 快速关闭/保存。
*   **双栏预览**: 如果内容是 Markdown，全屏模式可以改为左右分栏（左侧编辑，右侧预览）。
