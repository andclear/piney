# 未保存更改提示与样式反馈实现指南

本文档详细介绍了如何在 Svelte 5 (Runes) 项目中实现“输入框内容编辑但未保存时更改样式”以及“页面跳转时拦截并提示”的功能。

## 1. 核心原理

1.  **快照对比 (Snapshot Comparison)**: 在页面加载或者是保存成功后，保存一份数据的副本 (`originalFormState`)。
2.  **细粒度脏检查 (Granular Dirty Checking)**: 使用 Svelte 5 的 `$derived` 或 `$effect` 实时比较当前表单值与快照值，计算出每个字段的 `isDirty` 状态。
3.  **视觉反馈 (Visual Feedback)**: 根据 `isDirty` 状态，动态应用 CSS 类（如琥珀色边框、背景）到输入框组件。
4.  **导航拦截 (Navigation Guard)**:
    *   使用 SvelteKit 的 `beforeNavigate` 拦截应用内路由跳转。
    *   使用 `window.onbeforeunload` 拦截浏览器刷新或关闭标签页。

## 2. 代码实现示例

### 2.1 状态管理与脏检查 (`+page.svelte`)

首先定义表单状态、原始快照以及脏检查逻辑。

```svelte
<script lang="ts">
    import { beforeNavigate, goto } from "$app/navigation";
    
    // 1. 表单状态
    let formName = $state("");
    let formDescription = $state("");
    
    // 2. 原始快照 (用于对比)
    let originalFormState = $state({
        name: "",
        description: "",
    });

    // 更新快照的函数 (在加载数据或保存成功后调用)
    function updateFormSnapshot() {
        originalFormState = {
            name: formName,
            description: formDescription,
        };
    }

    // 3. 细粒度脏检查 (Reactive)
    let isNameDirty = $derived(formName !== originalFormState.name);
    let isDescDirty = $derived(formDescription !== originalFormState.description);
    
    // 全局脏状态
    let isDirty = $derived(isNameDirty || isDescDirty);
</script>
```

### 2.2 输入框样式反馈

使用 `cn` (clsx + tailwind-merge) 工具函数，根据 `isDirty` 状态添加特定样式。

**Input 组件示例:**

```svelte
<Label>角色名称</Label>
<Input
    bind:value={formName}
    class={cn(
        "transition-all duration-300", 
        // 当未保存时，应用琥珀色背景和边框
        isNameDirty && "border-amber-500/50 focus:border-amber-500 bg-amber-500/5"
    )}
/>
```

**RichTextarea 组件封装示例:**

如果你封装了组件，可以将 `isDirty` 作为 prop 传入，并在组件内部处理样式和提示红点。

```svelte
<!-- RichTextarea.svelte -->
<script lang="ts">
    let { value = $bindable(), isDirty = false, class: className } = $props();
</script>

<div class={cn("relative group/dirty", className)}>
    <!-- 右上角小红点提示 -->
    {#if isDirty}
        <span class="absolute -right-1 -top-1 w-2 h-2 rounded-full bg-amber-500 z-10 shadow-sm"></span>
    {/if}
    
    <Textarea 
        bind:value 
        class={cn(
            "resize-none transition-all duration-300",
            // 脏状态样式覆盖
            isDirty && "border-amber-500/50 focus-visible:ring-amber-500/50 bg-amber-500/5"
        )} 
    />
</div>
```

### 2.3 导航拦截与弹窗提示

**拦截逻辑:**

```svelte
<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog";

    let showUnsavedDialog = $state(false);
    let pendingTarget: string | null = null; // 暂存目标 URL
    let bypassCheck = false; // 用于在确认丢弃后绕过检查

    // 拦截应用内跳转 (SvelteKit)
    beforeNavigate(({ cancel, to }) => {
        if (bypassCheck) return; // 如果用户已确认丢弃，放行
        
        if (isDirty) {
            cancel(); // 取消本次跳转
            pendingTarget = to?.url?.href || null; // 记录用户想去哪
            showUnsavedDialog = true; // 显示弹窗
        }
    });

    // 拦截浏览器刷新/关闭
    $effect(() => {
        const handleBeforeUnload = (e: BeforeUnloadEvent) => {
            if (isDirty) {
                e.preventDefault();
                e.returnValue = ""; // 现代浏览器很多只显示通用提示
            }
        };
        window.addEventListener("beforeunload", handleBeforeUnload);
        return () => window.removeEventListener("beforeunload", handleBeforeUnload);
    });

    // 确认丢弃更改
    function confirmDiscard() {
        bypassCheck = true; // 设置绕过标志
        showUnsavedDialog = false;
        if (pendingTarget) {
            goto(pendingTarget); // 重新执行跳转
        }
    }
</script>

<!-- 确认弹窗 UI (使用 Shadcn Dialog) -->
<Dialog.Root bind:open={showUnsavedDialog}>
    <Dialog.Content>
        <Dialog.Header>
            <Dialog.Title class="flex items-center gap-2 text-amber-600">
                <AlertTriangle class="h-5 w-5" /> 未保存的更改
            </Dialog.Title>
            <Dialog.Description>
                您有未保存的编辑内容。离开页面将丢失这些更改。确定要离开吗？
            </Dialog.Description>
        </Dialog.Header>
        <Dialog.Footer>
            <Button variant="outline" onclick={() => (showUnsavedDialog = false)}>
                取消 (留在页面)
            </Button>
            <Button variant="destructive" onclick={confirmDiscard}>
                丢弃更改并离开
            </Button>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>
```

## 3. 完整流程总结

1.  **加载**: 页面加载数据 `updateFormSnapshot()`。
2.  **编辑**: 用户修改输入框 -> `formName` 变化 -> `isNameDirty` 变为 `true` -> 输入框变黄。
3.  **跳转**: 用户点击返回 -> `beforeNavigate` 触发 -> 检测到 `isDirty` -> `cancel()` -> `showUnsavedDialog = true`。
4.  **决策**:
    *   用户点“取消” -> 弹窗关闭，停留在当前页面。
    *   用户点“离开” -> `bypassCheck = true` -> `goto(pendingTarget)` -> 跳转成功。

这种模式既保证了数据的安全性（防止意外丢失），又提供了清晰的视觉反馈，提升了用户体验。