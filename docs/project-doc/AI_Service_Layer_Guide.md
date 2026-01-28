# Piney AI Service Layer 开发指南

本文档旨在为开发者提供 Piney 项目中 AI 服务层的详细架构说明、使用指南及扩展教程。

## 1. 架构概览 (Architecture Overview)

Piney 的 AI 服务层采用了 **前端主导，后端代理** 的架构设计。

*   **Frontend (`src/lib/ai/`)**: 负责核心业务逻辑，包括 Prompt 构建、变量提取、模版渲染以及调用后端代理接口。
*   **Backend (`src/api/ai.rs`)**: 仅作为安全代理（Proxy），负责读取服务器配置（如 API Key），将前端构建好的消息转发给 LLM 提供商，并将原始响应返回给前端。

这种设计的优势在于：修改 Prompt 和业务逻辑无需重启后端，且充分利用了前端的灵活性。

### 核心组件

1.  **`AiService` (`frontend/src/lib/ai/service.ts`)**: 
    *   AI 服务的统一入口。
    *   负责协调 PromptBuilder、Templates 和 API 调用。
    *   包含特定功能的业务逻辑（如 `generateOverview` 中的标签生成策略）。
2.  **`PromptBuilder` (`frontend/src/lib/ai/promptBuilder.ts`)**:
    *   负责处理模版变量替换（`{{variable}}`）。
    *   负责构建 System Prompt 和 User Prompt。
3.  **`Templates` (`frontend/src/lib/ai/templates.ts`)**:
    *   存储所有 AI 功能的 Prompt 模版。
    *   支持动态变量（如 `{{task_instruction}}`）。
4.  **`Types` (`frontend/src/lib/ai/types.ts`)**:
    *   定义了所有 AI 功能相关的类型接口，确保类型安全。

---

## 2. 快速开始 (Quick Start)

### 调用 AI 概览功能

```typescript
import { AiService } from "$lib/ai/service";

// 假设 card 是当前角色卡对象
async function handleAiAnalysis() {
    try {
        const result = await AiService.generateOverview(card);
        console.log("Summary:", result.summary);
        console.log("Tags:", result.tags); // 可能为空，取决于生成策略
    } catch (error) {
        console.error("AI Analysis failed:", error);
    }
}
```

### 调试 AI Prompt

访问 `/debug/ai`页面。
该页面允许你选择一个现有的角色卡，查看系统生成的完整 System Prompt 和 User Prompt，并直接测试 AI 响应。这对于调整提示词非常有用。

---

## 3. 如何添加新的 AI 功能 (How to Add New Features)

假设我们要添加一个 **"一键润色 (Polish)"** 的功能，用于优化角色的描述。

### 第一步：定义功能 ID 和类型

编辑 `frontend/src/lib/ai/types.ts`：

```typescript
export enum AiFeature {
    OVERVIEW = "overview",
    POLISH = "polish", // [NEW] 添加新功能 ID
}

// 定义润色功能的返回结构
export interface PolishResult {
    polished_text: string;
    suggestions: string[];
}
```

### 第二步：配置 Prompt 模版

编辑 `frontend/src/lib/ai/templates.ts`：

```typescript
export const SYSTEM_PROMPTS: Record<AiFeature, string> = {
    // ... existing
    [AiFeature.POLISH]: `你是一个专业的创意写作助手。你的任务是优化用户提供的文本，使其更具文学性和感染力。`
};

export const PROMPT_TEMPLATES: Record<AiFeature, string> = {
    // ... existing
    [AiFeature.POLISH]: `
请润色以下文本：
{{raw_text}}

[要求]
1. 保持原意不变。
2. 使用更优美的词汇。

[回复格式]
请严格仅返回 JSON：
{
    "polished_text": "润色后的文本...",
    "suggestions": ["修改建议1", "修改建议2"]
}
`
};
```

### 第三步：在 Service 中实现方法

编辑 `frontend/src/lib/ai/service.ts`：

```typescript
// 1. 确保 PromptVariables 接口支持新变量
// 在 types.ts 中 PromptVariables 已经支持 [key: string]: string | undefined，可以直接使用

// 2. 在 AiService 类中添加方法
public static async polishText(rawText: string): Promise<PolishResult> {
    // 构建变量
    const variables: PromptVariables = {
        name: "User", // 占位
        description: "",
        raw_text: rawText, // 传入模版需要的变量
        // ... 其他必须字段给空值即可
    } as any; 

    // 获取 System Prompt (支持全局 Prompt 注入)
    const systemPrompt = await this.getSystemPrompt(AiFeature.POLISH);
    
    // 构建 User Prompt
    const userPrompt = PromptBuilder.buildUserPrompt(AiFeature.POLISH, variables);

    // 调用后端执行
    // 注意：execute 方法返回的是 LLM 的原始响应 (ProviderResponse)
    const response = await this.execute(AiFeature.POLISH, [
        { role: "system", content: systemPrompt },
        { role: "user", content: userPrompt }
    ]);

    // 解析结果 (假设后端返回的是 JSON 字符串)
    const content = response.choices[0]?.message?.content || "{}";
    try {
        return JSON.parse(content);
    } catch (e) {
        console.error("Failed to parse AI response", e);
        throw new Error("AI 响应格式错误");
    }
}
```

### 第四步：后端 API (无需修改)

后端接口 `/api/ai/execute` 是通用的。只要前端传递了正确的 `feature` 和 `messages`，后端就会负责调用 LLM。

**注意**：目前后端仅做透传。如果需要针对特定 feature 做特殊的后端处理（如特殊的模型参数），才需要修改 Rust 代码。

---

## 4. 关键文件说明

### `frontend/src/lib/ai/service.ts`

核心逻辑层。重点关注 `prepareVariables` 方法。
该方法负责将 `CharacterCard` 对象转换为 Prompt 模版可用的扁平化变量（如 `{{personality}}`, `{{first_mes}}`）。
**注意**：如果角色卡数据结构发生变化（如 V2/V3 嵌套结构），请务必更新此处的提取逻辑。

### `frontend/src/lib/ai/templates.ts`

Prompt 仓库。
*   `SYSTEM_PROMPTS`: 定义 AI 的人设和核心指令。
*   `PROMPT_TEMPLATES`: 定义 User 消息的结构。使用 Handlerbars 风格的 `{{key}}` 占位符。

### `src/api/ai.rs` (Backend)

*   `execute_feature`: 通用执行入口。
*   它从 `AppSettings` 中读取 `ai_config` (API Key, Base URL, Model)。
*   使用 `reqwest` 向 LLM 提供商发起请求。

## 5. 常见问题 (FAQ)

**Q: 为什么 Debug 页面里的详细设定是空的？**
A: 确保 `AiService.prepareVariables` 正确映射了 JSON 数据。注意 V2 角色卡的数据在 `data` 字段下的 `data` 属性中（嵌套）。我们在 `service.ts` 中实现了自动降级查找逻辑。

**Q: 如何修改生成的标签数量？**
A: 修改 `frontend/src/lib/ai/service.ts` 中的 `taskInstruction` 变量。它是动态注入到模版中的。

**Q: 如何更换 AI 模型？**
A: 在应用的「设置 -> AI 设置」中配置。前端 Service 此时不关心具体的模型，它只负责构建 Prompt。
