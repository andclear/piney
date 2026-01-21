export enum AiFeature {
    OVERVIEW = 'overview', // 概览
    OPTIMIZE_DESCRIPTION = 'optimize_description', // 优化描述
    OPTIMIZE_FIRST_MES = 'optimize_first_mes', // 优化开场白
    OPTIMIZE_WORLDBOOK = 'optimize_worldbook', // 优化世界书
    OPTIMIZE_SCENARIO = 'optimize_scenario', // 优化情景
    TRANSLATE = 'translate', // 翻译
    GENERATE_CHARACTER = 'generate_character', // 生成角色
    GENERATE_WORLD_INFO = 'generate_world_info', // 生成世界书
    // DIAGNOSIS = 'diagnosis', 
    // TAG_GENERATION = 'tag_generation'
}

export interface PromptVariables {
    name: string;
    description: string;
    personality: string;
    first_mes: string;
    creator_notes: string;
    task_instruction?: string;
    response_format?: string;
    [key: string]: string | undefined; // Allow flexible keys
}
