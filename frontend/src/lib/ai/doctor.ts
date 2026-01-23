import { writable, get } from 'svelte/store';
import { API_BASE } from '$lib/api';

// --- Types ---

export interface DoctorReport {
    core_assessment: string;
    dimensions: Array<{
        name: string;
        status: string;
        issues: string;
        suggestions: string;
    }>;
    prescriptions: string[];
    conclusion: string;
}

export interface DoctorHistoryItem {
    id: string;
    status: string;
    final_report: string | null;
    created_at: string;
    // user_id?
}

export interface SseProgress {
    status: 'progress' | 'complete' | 'error';
    message: string;
    report?: DoctorReport;
    debug?: string; // JSON string
}

export interface DoctorTaskState {
    status: 'idle' | 'analyzing' | 'complete' | 'error';
    message: string;
    report: DoctorReport | null;
    debugInfo?: any;
}

// --- Store ---

// Map<cardId, TaskState>
export const doctorTasks = writable<Record<string, DoctorTaskState>>({});

// Keep controllers in a side-map to avoid store clutter/serializability issues (though stores can hold them)
const controllers = new Map<string, AbortController>();

// --- Actions ---

export function startDiagnosis(cardId: string) {
    // If already analyzing, do nothing (ui should handle picking up state)
    const current = get(doctorTasks)[cardId];
    if (current?.status === 'analyzing') {
        return;
    }

    // Reset State
    doctorTasks.update(s => ({
        ...s,
        [cardId]: {
            status: 'analyzing',
            message: '初始化诊断连接...',
            report: null
        }
    }));

    // Cleanup old controller
    if (controllers.has(cardId)) {
        controllers.get(cardId)?.abort();
        controllers.delete(cardId);
    }

    const controller = new AbortController();
    controllers.set(cardId, controller);
    const token = localStorage.getItem('auth_token');

    // Start Process using Fetch + ReadableStream
    fetch(`${API_BASE}/api/ai/doctor/analyze`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            ...(token ? { 'Authorization': `Bearer ${token}` } : {})
        },
        body: JSON.stringify({ card_id: cardId }),
        signal: controller.signal
    })
        .then(async (response) => {
            if (!response.ok) {
                const data = await response.json();
                throw new Error(data.error || '诊断请求失败');
            }

            const reader = response.body?.getReader();
            if (!reader) throw new Error('无法获取响应流');

            const decoder = new TextDecoder();
            let buffer = '';

            while (true) {
                const { done, value } = await reader.read();
                if (done) break;

                buffer += decoder.decode(value, { stream: true });
                const lines = buffer.split('\n');
                buffer = lines.pop() || '';

                for (const line of lines) {
                    if (line.startsWith('data: ')) {
                        const data = line.slice(6);
                        if (data === 'keep-alive') continue;

                        try {
                            const progress: SseProgress = JSON.parse(data);

                            // Parse Debug Info
                            let parsedDebug: any = undefined;
                            if (progress.debug) {
                                try {
                                    parsedDebug = JSON.parse(progress.debug);
                                } catch (e) {
                                    // ignore
                                }
                            }


                            // Update Store
                            doctorTasks.update(s => {
                                const state = s[cardId] || { status: 'analyzing', message: '', report: null };

                                // Debug Info persistence (optional, if UI wants to show log)
                                // We merge it? Or replace? 
                                // Since it's streaming, we might want to append logs if we had a log array.
                                // For now, let's just keep 'last' debug info or better:
                                // We can use console for detailed debug, and store just the messages for UI?
                                // User requirement is console log (DONE).
                                // Store status update:

                                if (progress.status === 'progress') {
                                    return {
                                        ...s,
                                        [cardId]: {
                                            ...state,
                                            status: 'analyzing',
                                            message: progress.message,
                                            debugInfo: parsedDebug || state.debugInfo
                                        }
                                    };
                                } else if (progress.status === 'complete' && progress.report) {
                                    return {
                                        ...s,
                                        [cardId]: {
                                            ...state,
                                            status: 'complete',
                                            message: '诊断完成',
                                            report: progress.report,
                                            debugInfo: parsedDebug || state.debugInfo
                                        }
                                    };
                                } else if (progress.status === 'error') {
                                    return {
                                        ...s,
                                        [cardId]: {
                                            ...state,
                                            status: 'error',
                                            message: progress.message
                                        }
                                    };
                                }
                                return s;
                            });

                        } catch (e) {
                            console.error('SSE Parse Error', e);
                        }
                    }
                }
            }
        })
        .catch((error) => {
            if (error.name !== 'AbortError') {
                doctorTasks.update(s => ({
                    ...s,
                    [cardId]: {
                        status: 'error',
                        message: error.message || '网络连接中断',
                        report: null
                    }
                }));
            }
        })
        .finally(() => {
            controllers.delete(cardId);
        });
}

export function stopDiagnosis(cardId: string) {
    const controller = controllers.get(cardId);
    if (controller) {
        try {
            controller.abort();
        } catch {
            // AbortError 是预期行为，静默处理
        }
        controllers.delete(cardId);
        doctorTasks.update(s => ({
            ...s,
            [cardId]: { ...s[cardId], status: 'idle', message: '已停止' }
        }));
    }
}

// --- History API ---

export async function getDoctorHistory(cardId: string): Promise<DoctorHistoryItem[]> {
    const token = localStorage.getItem('auth_token');
    const res = await fetch(`${API_BASE}/api/ai/doctor/history/${cardId}`, {
        headers: token ? { 'Authorization': `Bearer ${token}` } : {}
    });

    if (!res.ok) {
        const data = await res.json();
        throw new Error(data.error || '获取历史失败');
    }
    return res.json();
}

export async function deleteDoctorHistory(id: string): Promise<void> {
    const token = localStorage.getItem('auth_token');
    const res = await fetch(`${API_BASE}/api/ai/doctor/history/item/${id}`, {
        method: 'DELETE',
        headers: {
            'Content-Type': 'application/json',
            ...(token ? { 'Authorization': `Bearer ${token}` } : {})
        }
    });

    if (!res.ok) {
        const data = await res.json();
        throw new Error(data.error || '删除历史记录失败');
    }
}
