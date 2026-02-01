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

        // 1. 准备变量
        const variables = this.prepareVariables(card, systemTags);

        // 2. 构建提示词
        const userPrompt = PromptBuilder.buildUserPrompt(AiFeature.OVERVIEW, variables);
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
     * 获取调试信息
     */
    static async getPromptDebugInfo(card: any, feature: AiFeature = AiFeature.OVERVIEW) {
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
     * 通用文本处理
     */
    static async processText(feature: AiFeature, text: string) {
        if (this.activeRequests >= this.MAX_CONCURRENT) {
            throw new Error(`AI 请求队列已满 (${this.activeRequests}/${this.MAX_CONCURRENT})，请稍后再试`);
        }

        this.activeRequests++;
        try {
            const globalPrompt = await this.getGlobalPrompt();

            const variables = {
                text,
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
        let cardData: any = {};
        try {
            cardData = typeof card.data === 'string' ? JSON.parse(card.data) : card.data;
        } catch (e) {
            console.error("无法解析角色卡数据 (card.data)", e);
        }
        cardData = cardData || {};

        const getField = (key: string) => cardData[key] || cardData.data?.[key] || "";

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
            let content = result.choices?.[0]?.message?.content || "";
            if (!content && result.response) content = result.response;



            const jsonMatch = content.match(/\[[\s\S]*\]/);
            if (jsonMatch) {
                content = jsonMatch[0];
            } else {
                content = content.replace(/^[\s\S]*?```json/i, "").replace(/^[\s\S]*?```/i, "").replace(/```[\s\S]*$/, "").trim();
            }

            try {
                return JSON.parse(content);
            } catch (e) {
                throw new Error("生成内容无法解析为JSON");
            }
        } catch (e: any) {
            console.error("Generate World Info Error:", e);
            throw e;
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
            let content = response.choices?.[0]?.message?.content || "";

            // 预处理：移除 <think>/<thinking>/<cot> 标签及其内容
            content = content.replace(/<think>[\s\S]*?<\/think>/gi, "");
            content = content.replace(/<thinking>[\s\S]*?<\/thinking>/gi, "");
            content = content.replace(/<cot>[\s\S]*?<\/cot>/gi, "");
            content = content.trim();

            return content;
        } finally {
            this.activeRequests--;
        }
    }

    /**
     * 生成开场白
     */
    static async generateOpening(
        card: any,
        userInput: string,
        wordCount: string,
        worldInfoContent: string,
        personType: string
    ) {
        if (this.activeRequests >= this.MAX_CONCURRENT) {
            throw new Error(`AI 请求队列已满 (${this.activeRequests}/${this.MAX_CONCURRENT})，请稍后再试`);
        }

        this.activeRequests++;
        try {
            const { GENERATE_OPENING_TEMPLATE } = await import('./templates');
            const globalPrompt = await this.getGlobalPrompt();

            // 准备变量 (兼容 V1/V2/V3)
            let cardData: any = {};
            try {
                cardData = typeof card.data === 'string' ? JSON.parse(card.data) : card.data;
            } catch (e) { }
            cardData = cardData || {};
            const getField = (key: string) => cardData[key] || cardData.data?.[key] || "";

            const description = card.description || "";
            const personality = getField('personality');

            // 替换变量
            const userPrompt = GENERATE_OPENING_TEMPLATE
                .replace(/{{description}}/g, description)
                .replace(/{{personality}}/g, personality)
                .replace(/{{world_info}}/g, worldInfoContent)
                .replace(/{{user_request}}/g, userInput)
                .replace(/{{word_count}}/g, wordCount)
                .replace(/{{person_type}}/g, personType);

            // 注意：{{char}} 和 {{user}} 已在 Prompt 中硬编码为字面量要求，无需替换

            const feature = AiFeature.GENERATE_OPENING;
            const systemPrompt = globalPrompt || ""; // 开场白通常不需要特定 System Prompt，使用全局即可，或者可以为空

            const messages = [
                { role: "system", content: systemPrompt },
                { role: "user", content: userPrompt }
            ];

            const token = localStorage.getItem("auth_token");
            const response = await this.execute(feature, messages, token);
            let content = response.choices?.[0]?.message?.content || "";

            // 预处理：移除 <think>/<thinking>/<cot> 标签及其内容
            content = content.replace(/<think>[\s\S]*?<\/think>/gi, "");
            content = content.replace(/<thinking>[\s\S]*?<\/thinking>/gi, "");
            content = content.replace(/<cot>[\s\S]*?<\/cot>/gi, "");
            content = content.trim();

            return content;

        } finally {
            this.activeRequests--;
        }
    }

    /**
     * 生成前端样式（皮皮美化生成器）
     * @param params 生成参数
     * @param params.originalText 原始文本
     * @param params.userRequest 用户需求描述
     * @param params.currentHtml 当前 HTML（后续轮次）
     * @param params.currentRegex 当前正则（后续轮次）
     * @param params.currentWorldinfoKey 当前世界书 Key（后续轮次）
     * @param params.currentWorldinfoContent 当前世界书内容（后续轮次）
     * @param params.selectedElement 用户选中的元素描述（交互式编辑）
     * @param params.isFirstRound 是否首轮生成
     */
    static async generateFrontendStyle(params: {
        originalText: string;
        userRequest: string;
        currentHtml?: string;
        currentRegex?: string;
        currentWorldinfoKey?: string;
        currentWorldinfoContent?: string;
        selectedElement?: string;
        isFirstRound: boolean;
        isFixMode?: boolean;
    }): Promise<{
        worldinfo: { key: string; content: string };
        regex: string;
        html: string;
        original_text?: string;
        formatted_original_text?: string;
    }> {
        if (this.activeRequests >= this.MAX_CONCURRENT) {
            throw new Error(`AI 请求队列已满 (${this.activeRequests}/${this.MAX_CONCURRENT})，请稍后再试`);
        }

        this.activeRequests++;
        try {
            const feature = AiFeature.GENERATE_FRONTEND_STYLE;

            let userPrompt: string;

            // 判断使用哪个模板：
            // 0. 修复模式（用户点击“修复正则”）
            // 1. 首轮 + 无选中元素 → 首轮生成模板
            // 2. 首轮 + 有选中元素 → 仅修改代码模板（用户修改已有样式）
            // 3. 非首轮 → 后续修改模板
            const isFirstRoundCodeOnly = params.isFirstRound && params.selectedElement;
            const useFirstRoundTemplate = params.isFirstRound && !params.selectedElement;

            if (params.isFixMode) {
                // 修复模式 - 使用 FRONTEND_STYLE_FIX_REGEX 模板
                const { FRONTEND_STYLE_FIX_REGEX } = await import('./templates');
                userPrompt = FRONTEND_STYLE_FIX_REGEX
                    .replace(/{{current_regex}}/g, params.currentRegex || '')
                    .replace(/{{current_worldinfo_key}}/g, params.currentWorldinfoKey || '')
                    .replace(/{{current_worldinfo_content}}/g, params.currentWorldinfoContent || '')
                    .replace(/{{original_text}}/g, params.originalText || '')
                    .replace(/{{current_html}}/g, params.currentHtml || '');
            } else if (isFirstRoundCodeOnly) {
                // 首轮但有选中元素 - 使用仅修改代码模板
                const { FRONTEND_STYLE_CODE_ONLY } = await import('./templates');
                userPrompt = FRONTEND_STYLE_CODE_ONLY
                    .replace(/{{current_html}}/g, params.currentHtml || '')
                    .replace(/{{selected_element}}/g, params.selectedElement || '')
                    .replace(/{{original_text}}/g, params.originalText || '')
                    .replace(/{{user_request_value}}/g, params.userRequest);
            } else if (useFirstRoundTemplate) {
                // 首轮对话 - 使用 FRONTEND_STYLE_FIRST_ROUND 模板
                const { FRONTEND_STYLE_FIRST_ROUND } = await import('./templates');
                userPrompt = FRONTEND_STYLE_FIRST_ROUND
                    .replace(/{{original_text_value}}/g, params.originalText || '（未提供）')
                    .replace(/{{user_request_value}}/g, params.userRequest);
            } else {
                // 后续对话 - 使用 FRONTEND_STYLE_FOLLOWUP 模板
                const { FRONTEND_STYLE_FOLLOWUP } = await import('./templates');

                const selectedInstruction = params.selectedElement
                    ? `### SELECTED ELEMENT (User wants to modify this specific part)\n\`\`\`\n${params.selectedElement}\n\`\`\``
                    : '';

                userPrompt = FRONTEND_STYLE_FOLLOWUP
                    .replace(/{{current_html}}/g, params.currentHtml || '')
                    .replace(/{{current_regex}}/g, params.currentRegex || '')
                    .replace(/{{current_worldinfo_key}}/g, params.currentWorldinfoKey || '')
                    .replace(/{{current_worldinfo_content}}/g, params.currentWorldinfoContent || '')
                    .replace(/{{original_text}}/g, params.originalText || '')
                    .replace(/{{user_request_value}}/g, params.userRequest)
                    .replace(/{{selected_element_instruction}}/g, selectedInstruction);
            }

            const { SYSTEM_PROMPTS } = await import('./templates');
            // 前端样式生成不附加全局提示词
            const systemPrompt = SYSTEM_PROMPTS[feature] || '';

            const messages = [
                { role: "system", content: systemPrompt },
                { role: "user", content: userPrompt }
            ];

            const token = localStorage.getItem("auth_token");
            const response = await this.execute(feature, messages, token);

            let content = response.choices?.[0]?.message?.content || "";
            if (!content) {
                throw new Error("AI 返回空内容");
            }

            // 清理 markdown 代码块
            content = content.trim();
            if (content.startsWith('```json')) {
                content = content.slice(7);
            } else if (content.startsWith('```')) {
                content = content.slice(3);
            }
            if (content.endsWith('```')) {
                content = content.slice(0, -3);
            }
            content = content.trim();

            // 解析 JSON
            try {
                const result = JSON.parse(content);
                return {
                    worldinfo: result.worldinfo || { key: '', content: '' },
                    regex: result.regex || '',
                    html: result.html || '',
                    original_text: result.original_text,
                    formatted_original_text: result.formatted_original_text
                };
            } catch (e) {
                console.error("Failed to parse AI response:", content);
                throw new Error("AI 返回格式错误，无法解析 JSON");
            }
        } finally {
            this.activeRequests--;
        }
    }
}

