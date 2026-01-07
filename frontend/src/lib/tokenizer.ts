/**
 * Token 计数器
 * 
 * 简易估算 Token 数量
 */

/**
 * 估算文本的 Token 数量
 * 约 4 个字符 = 1 个 Token（英文）
 * 约 2 个字符 = 1 个 Token（中文）
 */
export function countTokens(text: string): number {
    if (!text) return 0;

    // 分离中文和非中文字符
    const chineseChars = text.match(/[\u4e00-\u9fff]/g) || [];
    const nonChineseText = text.replace(/[\u4e00-\u9fff]/g, '');

    // 中文约 2 字符/token，其他约 4 字符/token
    const chineseTokens = chineseChars.length / 2;
    const otherTokens = nonChineseText.length / 4;

    return Math.ceil(chineseTokens + otherTokens);
}

/**
 * 估算费用（以 GPT-4 为基准）
 */
export function estimateCost(tokens: number): number {
    // GPT-4 约 $0.03 / 1K tokens
    return (tokens / 1000) * 0.03;
}

/**
 * 格式化 Token 数量
 */
export function formatTokenCount(count: number): string {
    if (count >= 1000) {
        return `${(count / 1000).toFixed(1)}K`;
    }
    return count.toString();
}

/**
 * 检查是否超出上下文窗口
 */
export function checkContextLimit(
    tokens: number,
    contextSize: number = 8192
): { exceeded: boolean; usage: number } {
    const usage = (tokens / contextSize) * 100;
    return {
        exceeded: tokens > contextSize,
        usage: Math.min(usage, 100)
    };
}
