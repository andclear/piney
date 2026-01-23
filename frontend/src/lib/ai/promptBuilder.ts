import { PROMPT_TEMPLATES, SYSTEM_PROMPTS } from './templates';
import { AiFeature, type PromptVariables } from './types';

export class PromptBuilder {
    /**
     * 构建用户提示词
     * @param feature 功能ID
     * @param variables 变量集合
     * @param customTemplate 可选的用户自定义模版
     */
    static buildUserPrompt(feature: AiFeature, variables: PromptVariables, customTemplate?: string): string {
        let template = customTemplate || PROMPT_TEMPLATES[feature];

        if (!template) {
            console.warn(`No template found for feature ${feature}, returning empty string`);
            return "";
        }

        // Replace {{key}} with value
        return template.replace(/\{\{(\w+)\}\}/g, (match, key) => {
            return variables[key] || "";
        });
    }

    /**
     * 获取系统提示词
     * @param feature 功能ID
     * @param globalPrompt 全局提示词 (可选)
     */
    static getSystemPrompt(feature: AiFeature, globalPrompt?: string): string {
        const base = SYSTEM_PROMPTS[feature] || "You are a helpful assistant.";
        if (globalPrompt && globalPrompt.trim()) {
            return `${globalPrompt}\n\n${base}`;
        }
        return base;
    }
}
