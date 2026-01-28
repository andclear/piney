# 前端通用组件库文档

本文档介绍了项目中封装的通用业务组件，旨在统一 UI 交互体验并提高开发效率。

## 目录

- [未保存更改拦截模块 (UnsavedGuard)](#未保存更改拦截模块)

---

## 脏数据状态输入框

包含 `DirtyInput`、`DirtyTextarea` 和 `DirtyLabel` 三个组件。


**功能**：在标准 Shadcn UI 组件的基础上，增加 `isDirty` 属性。当该属性为 `true` 时，组件会自动应用项目统一的“未保存”通过视觉样式（琥珀色边框、背景、文字，以及右上角小圆点）。

### 引入方式

```svelte
import DirtyInput from "$lib/components/common/DirtyInput.svelte";
import DirtyTextarea from "$lib/components/common/DirtyTextarea.svelte";
import DirtyLabel from "$lib/components/common/DirtyLabel.svelte";
```

### Props

这两个组件继承了原 `Input` 和 `Textarea` 的所有 Props，并增加了一个核心属性：

| 属性名 | 类型 | 默认值 | 说明 |
| :--- | :--- | :--- | :--- |
| `isDirty` | `boolean` | `false` | 是否处于“脏数据”（未保存）状态。为 `true` 时应用高亮样式。 |
| `value` | `string` | - | **(bindable)** 输入框的值 |
| `class` | `string` | - | 自定义样式类（会与默认样式和 dirty 样式合并） |

### 使用示例

#### 基础用法

```svelte
<script lang="ts">
    import DirtyInput from "$lib/components/common/DirtyInput.svelte";

    let name = $state("初始名字");
    let originalName = "初始名字";
    let isNameDirty = $derived(name !== originalName);
</script>

<!-- 当输入内容改变时，输入框会自动变色 -->
<DirtyInput 
    bind:value={name} 
    isDirty={isNameDirty} 
    placeholder="请输入名称..."
/>
```

#### 结合 Textarea 使用

```svelte
<script lang="ts">
    import DirtyTextarea from "$lib/components/common/DirtyTextarea.svelte";
    // ... 状态逻辑
</script>

<DirtyTextarea 
    bind:value={description} 
    isDirty={isDescDirty} 
    placeholder="请输入描述..."
    rows={5}
/>
```

### 样式说明

当 `isDirty={true}` 时，组件会应用以下 Tailwind 类：

- `border-amber-500/50`: 琥珀色半透明边框
- `bg-amber-500/10`: 浅琥珀色背景
- `text-amber-600`: 深琥珀色文字
- `focus-visible:ring-amber-500`: 聚焦时琥珀色光圈
- 右上角会出现一个 `h-2 w-2` 的琥珀色圆点 (`bg-amber-500`)

#### 结合 Label 使用

`DirtyLabel` 组件会在 `isDirty` 为真时将文字变为琥珀色并加粗，同时带有脉冲动画效果。

```svelte
<script lang="ts">
    import DirtyLabel from "$lib/components/common/DirtyLabel.svelte";
</script>

<DirtyLabel isDirty={isNameDirty}>世界书名称</DirtyLabel>
```

---

## 未保存更改拦截模块

包含 `useUnsavedChanges` Hook 和 `UnsavedGuard` 组件。

**功能**：提供统一的“未保存更改”拦截机制。
1.  **路由拦截**：当用户尝试通过应用内导航（如点击返回按钮）离开当前页面时拦截。
2.  **浏览器拦截**：当用户尝试刷新页面或关闭标签页时拦截。
3.  **统一弹窗**：显示标准化的警告对话框，允许用户“留存页面”或“丢弃更改”。

### 引入方式

```typescript
import { useUnsavedChanges } from "$lib/hooks/use-unsaved-changes.svelte";
import UnsavedGuard from "$lib/components/common/UnsavedGuard.svelte";
```

### 使用步骤

1.  **定义脏数据信号**：你需要有一个 `isDirty` 的响应式状态或计算属性。
2.  **初始化 Hook**：调用 `useUnsavedChanges`，传入一个返回 boolean 的函数。
3.  **放置组件**：将 `UnsavedGuard` 放置在页面模板中（通常在底部），并将 hook 返回的控制器传给它。

### 代码示例

```svelte
<script lang="ts">
    import { useUnsavedChanges } from "$lib/hooks/use-unsaved-changes.svelte";
    import UnsavedGuard from "$lib/components/common/UnsavedGuard.svelte";

    let formName = $state("Initial");
    let originalName = "Initial";
    
    // 1. 定义脏数据逻辑
    let isDirty = $derived(formName !== originalName);

    // 2. 初始化 Hook
    // 注意：这里传入的是函数 () => isDirty，以保持响应性
    const unsaved = useUnsavedChanges(() => isDirty);
</script>

<!-- 页面内容... -->
<input bind:value={formName} />

<!-- 3. 放置守护组件 -->
<UnsavedGuard controller={unsaved} />
```

### API 说明

#### `useUnsavedChanges(isDirtyFn: () => boolean)`

- **参数**:
    - `isDirtyFn`: 一个无参函数，返回 `true` 表示有未保存更改，需要拦截；返回 `false` 表示安全，允许通行。
- **返回**:
    - `controller`: 一个对象，包含内部状态和回调方法。通常直接作为 props 传给 `<UnsavedGuard>`。

#### `<UnsavedGuard />`

- **Props**:
    - `controller`: 接收 `useUnsavedChanges` 的返回值。

