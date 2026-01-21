import { API_BASE } from '$lib/api';
import { PromptBuilder } from './promptBuilder';
import { CHAR_GEN_NO_YAML, CHAR_GEN_YAML } from './templates';
import { AiFeature, type PromptVariables } from './types';

export class AiService {
    private static activeRequests = 0;
    private static readonly MAX_CONCURRENT = 3;

    private static async execute(feature: AiFeature, messages: any[], token: string | null) {
        let lastError;
        const RETRIES = 1;

        for (let attempt = 0; attempt <= RETRIES; attempt++) {
            try {
                const res = await fetch(`${API_BASE}/api/ai/execute`, {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                        ...(token ? { 'Authorization': `Bearer ${token}` } : {})
                    },
                    body: JSON.stringify({
                        feature_id: feature,
                        messages
                    })
                });

                if (!res.ok) {
                    const data = await res.json().catch(() => ({}));
                    throw new Error(data.error || `AI request failed: ${res.status}`);
                }

                return await res.json();
            } catch (e) {
                lastError = e;
                if (attempt < RETRIES) {
                    console.warn(`[AiService] ${feature} attempt ${attempt + 1} failed, retrying...`, e);
                }
            }
        }
        throw lastError;
    }

    /**
     * 生成角色概览
     * @param card 角色卡数据
     */
    private static cache: { tags: string[] | null, globalPrompt: string | null } = { tags: null, globalPrompt: null };

    private static async getGlobalPrompt(): Promise<string> {
        // Simple caching or fresh fetch? Let's fresh fetch for now to be safe, or lightweight cache.
        // settings are usually stable.
        const token = localStorage.getItem("auth_token");
        try {
            const res = await fetch(`${API_BASE}/api/settings`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {}
            });
            if (res.ok) {
                const data = await res.json();
                return data.global_prompt || "";
            }
        } catch (e) {
            console.error("Failed to fetch global prompt", e);
        }
        return "";
    }

    private static async getSystemTags(): Promise<string[]> {
        // Fetch all cards to get unique tags
        // This is heavy, but matches backend logic.
        // Optimization: In a real app we might want a dedicated /api/tags endpoint.
        const token = localStorage.getItem("auth_token");
        try {
            const res = await fetch(`${API_BASE}/api/cards`, {
                headers: token ? { Authorization: `Bearer ${token}` } : {}
            });
            if (res.ok) {
                const data = await res.json();
                const cards = Array.isArray(data) ? data : (data.items || []);
                const tags = new Set<string>();
                cards.forEach((c: any) => {
                    let t: string[] = [];
                    try {
                        t = typeof c.tags === 'string' ? JSON.parse(c.tags) : c.tags;
                    } catch { }
                    if (Array.isArray(t)) t.forEach(tag => tags.add(tag));
                });
                return Array.from(tags);
            }
        } catch (e) {
            console.error("Failed to fetch system tags", e);
        }
        return [];
    }

    /**
     * 生成角色概览
     * @param card 角色卡数据
     */
    static async generateOverview(card: any) {
        // Prepare context parallelly
        const [globalPrompt, systemTags] = await Promise.all([
            this.getGlobalPrompt(),
            this.getSystemTags()
        ]);

        // 1. 准备变量 (Determine task instruction)
        const variables = this.prepareVariables(card, systemTags);

        // 2. 构建提示词
        const userPrompt = PromptBuilder.buildUserPrompt(AiFeature.OVERVIEW, variables);
        // Prepend global prompt (Break Limitations)
        const systemPrompt = PromptBuilder.getSystemPrompt(AiFeature.OVERVIEW, globalPrompt);

        // 3. 构造消息
        const messages = [
            { role: "system", content: systemPrompt },
            { role: "user", content: userPrompt }
        ];

        // 4. 调用后端
        const token = localStorage.getItem("auth_token");
        const response = await this.execute(AiFeature.OVERVIEW, messages, token);

        // 5. 解析响应内容
        const content = response.choices?.[0]?.message?.content;
        if (!content) {
            throw new Error("AI returned empty content");
        }

        // 清理 markdown
        const cleaned = content.replace(/```json/g, '').replace(/```/g, '').trim();

        try {
            return JSON.parse(cleaned);
        } catch (e) {
            console.error("Failed to parse AI JSON", cleaned);
            throw new Error("AI response format error");
        }
    }

    /**
     * 获取调试信息（不实际调用 AI）
     */
    static async getPromptDebugInfo(card: any, feature: AiFeature = AiFeature.OVERVIEW) {
        // Need async fetch for context
        const [globalPrompt, systemTags] = await Promise.all([
            this.getGlobalPrompt(),
            this.getSystemTags()
        ]);

        const variables = this.prepareVariables(card, systemTags);
        return {
            systemPrompt: PromptBuilder.getSystemPrompt(feature, globalPrompt),
            userPrompt: PromptBuilder.buildUserPrompt(feature, variables),
            variables
        };
    }

    /**
     * 通用文本处理（优化/翻译）
     */
    static async processText(feature: AiFeature, text: string) {
        if (this.activeRequests >= this.MAX_CONCURRENT) {
            throw new Error(`AI 请求队列已满 (${this.activeRequests}/${this.MAX_CONCURRENT})，请稍后再试`);
        }

        this.activeRequests++;
        try {
            // 加载全局配置（如破限）
            const globalPrompt = await this.getGlobalPrompt();

            const variables = {
                text,
                // 补全 PromptVariables 要求的必需字段为空字符串，避免 TS 报错
                // 因为 PromptBuilder 可能只需要 {{text}}，但也可能校验类型
                name: "",
                description: "",
                personality: "",
                first_mes: "",
                creator_notes: ""
            };

            const userPrompt = PromptBuilder.buildUserPrompt(feature, variables);
            const systemPrompt = PromptBuilder.getSystemPrompt(feature, globalPrompt);

            const messages = [
                { role: "system", content: systemPrompt },
                { role: "user", content: userPrompt }
            ];

            const token = localStorage.getItem("auth_token");
            const response = await this.execute(feature, messages, token);

            const content = response.choices?.[0]?.message?.content;
            if (!content) {
                throw new Error("AI returned empty content");
            }

            // 清理可能存在的 Markdown 代码块包裹
            let cleaned = content.trim();
            if (cleaned.startsWith('```') && cleaned.endsWith('```')) {
                cleaned = cleaned.replace(/^```[a-z]*\n?/i, '').replace(/\n?```$/, '');
            }
            return cleaned;
        } finally {
            this.activeRequests--;
        }
    }

    private static prepareVariables(card: any, allSystemTags: string[]): PromptVariables {
        // card.data 在数据库中是字符串字段，但在前端可能已经是解析后的对象或字符串
        // 调用此方法的组件通常拥有完整的 card 对象。
        // 我们期望 `card` 是包含字段的映射对象。
        // 但是，原始数据库 card 的 `data` 是字符串。
        // 我们将稳健地处理这两种情况。

        let cardData: any = {};
        try {
            cardData = typeof card.data === 'string' ? JSON.parse(card.data) : card.data;
        } catch (e) {
            console.error("无法解析角色卡数据 (card.data)", e);
        }
        cardData = cardData || {};

        // Helper to get field from root or nested data
        const getField = (key: string) => cardData[key] || cardData.data?.[key] || "";

        // Tag Generation Logic
        let currentTags: string[] = [];
        try {
            currentTags = typeof card.tags === 'string' ? JSON.parse(card.tags) : card.tags;
        } catch { }

        let taskInstruction = "";
        let responseFormat = "";

        if (!currentTags || currentTags.length === 0) {
            const tagsStr = JSON.stringify(allSystemTags);
            taskInstruction = `1. 概览总结：250字以内，精炼概括角色核心特征。\n2. 标签生成:生成最多5个标签，必须优先从以下[系统现有标签]中选择；仅当匹配度较低或无匹配时才生成新标签。\n   *特别注意*："系统"标签仅代表【网络文学中一种将现实世界规则“游戏化”或“数据化”的叙事装置与外挂设定】。仅在完全符合定义时才使用此标签，严禁滥用。\n   [系统现有标签]: ${tagsStr}`;
            responseFormat = `{"summary": "...", "tags": ["tag1", "tag2"]}`;
        } else {
            taskInstruction = `1. 概览总结：250字以内，精炼概括角色核心特征。`;
            responseFormat = `{"summary": "..."}`;
        }

        return {
            name: card.name || "",
            description: card.description || "",
            personality: getField('personality'),
            first_mes: getField('first_mes'),
            creator_notes: getField('creator_notes') || getField('creatorcomment'),
            task_instruction: taskInstruction,
            response_format: responseFormat
        };
    }

    /**
     * 生成世界书条目
     */
    static async generateWorldInfo(
        userInput: string,
        currentWorldInfo: string
    ): Promise<any[]> {
        const globalPrompt = await this.getGlobalPrompt();
        const feature = AiFeature.GENERATE_WORLD_INFO;

        const variables: any = {
            user_request: userInput,
            current_world_info: currentWorldInfo,
            // Required placeholders
            name: "", description: "", personality: "", first_mes: "", creator_notes: ""
        };

        const userPrompt = PromptBuilder.buildUserPrompt(feature, variables);
        const systemPrompt = globalPrompt || "";

        const messages = [
            { role: "system", content: systemPrompt },
            { role: "user", content: userPrompt }
        ];

        try {
            const token = localStorage.getItem("auth_token");
            const result = await this.execute(feature, messages, token);
            // Parse OpenAI-compatible response
            let content = result.choices?.[0]?.message?.content || "";
            if (!content && result.response) content = result.response; // Fallback



            // 1. Try to extract JSON array via Regex (Most robust)
            const jsonMatch = content.match(/\[[\s\S]*\]/);
            if (jsonMatch) {
                content = jsonMatch[0];
            } else {
                // 2. Fallback cleanup
                content = content.replace(/^[\s\S]*?```json/i, "").replace(/^[\s\S]*?```/i, "").replace(/```[\s\S]*$/, "").trim();
            }

            try {
                return JSON.parse(content);
            } catch (e) {
                throw new Error("生成内容无法解析为JSON");
            }
        } catch (e: any) {
            console.error("Generate World Info Error:", e);
            throw e; // Re-throw to be caught by UI
        }
    }

    /**
     * 生成角色详情
     */
    static async generateCharacter(
        userInput: string,
        useYaml: boolean,
        worldInfoContent: string
    ) {
        if (this.activeRequests >= this.MAX_CONCURRENT) {
            throw new Error(`AI 请求队列已满 (${this.activeRequests}/${this.MAX_CONCURRENT})，请稍后再试`);
        }

        this.activeRequests++;
        try {
            const template = useYaml ? CHAR_GEN_YAML : CHAR_GEN_NO_YAML;
            const globalPrompt = await this.getGlobalPrompt();

            let templateContent = template
                .replace(/{{user_request}}/g, userInput)
                .replace(/{{world_info}}/g, worldInfoContent);

            const variables = {
                task_instruction: templateContent,
                name: "",
                description: "",
                personality: "",
                first_mes: "",
                creator_notes: ""
            };

            const feature = AiFeature.GENERATE_CHARACTER;

            const userPrompt = PromptBuilder.buildUserPrompt(feature, variables);
            const systemPrompt = globalPrompt || "";

            const messages = [
                { role: "system", content: systemPrompt },
                { role: "user", content: userPrompt }
            ];

            const token = localStorage.getItem("auth_token");
            const response = await this.execute(feature, messages, token);
            return response.choices?.[0]?.message?.content || "";
        } finally {
            this.activeRequests--;
        }
    }
}
