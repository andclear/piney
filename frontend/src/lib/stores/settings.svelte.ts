/**
 * 设置状态管理
 */

import { writable } from 'svelte/store';
import { api } from '$lib/api';
import { setMode } from "mode-watcher";

export interface Settings {
    ai_provider: string | null;
    ai_endpoint: string | null;
    ai_api_key_set: boolean;
    ai_model: string | null;
    theme: 'light' | 'dark' | 'system';
    language: string;
    default_view: 'grid' | 'list';
    items_per_page: number;
}

interface SettingsState extends Settings {
    loading: boolean;
    error: string | null;
}

const initialState: SettingsState = {
    ai_provider: null,
    ai_endpoint: null,
    ai_api_key_set: false,
    ai_model: null,
    theme: 'system',
    language: 'zh-CN',
    default_view: 'grid',
    items_per_page: 20,
    loading: false,
    error: null
};

function createSettingsStore() {
    const { subscribe, set, update } = writable<SettingsState>(initialState);

    return {
        subscribe,

        // 加载设置
        loadSettings: async () => {
            update(state => ({ ...state, loading: true, error: null }));

            const response = await api.get<Settings>('/settings');

            if (response.success && response.data) {
                update(state => ({
                    ...state,
                    ...response.data,
                    loading: false
                }));
                // Sync theme to mode-watcher
                if (response.data.theme) {
                    setMode(response.data.theme);
                }
            } else {
                update(state => ({
                    ...state,
                    error: response.error || '加载失败',
                    loading: false
                }));
            }
        },

        // 更新设置
        updateSettings: async (newSettings: Partial<Settings>) => {
            update(state => ({ ...state, loading: true, error: null }));

            const response = await api.patch<Settings>('/settings', newSettings);

            if (response.success && response.data) {
                update(state => ({
                    ...state,
                    ...response.data,
                    loading: false
                }));
                return true;
            } else {
                update(state => ({
                    ...state,
                    error: response.error || '保存失败',
                    loading: false
                }));
                return false;
            }
        },

        // 设置主题 (Used by local override or manual call)
        setTheme: (theme: 'light' | 'dark' | 'system') => {
            update(state => ({ ...state, theme }));
            setMode(theme);
        },

        // 重置状态
        reset: () => set(initialState)
    };
}

export const settings = createSettingsStore();
