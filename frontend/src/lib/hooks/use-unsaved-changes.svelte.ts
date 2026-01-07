import { beforeNavigate, goto } from "$app/navigation";

export function useUnsavedChanges(isDirtyFn: () => boolean) {
    let showDialog = $state(false);
    let pendingTarget = $state<string | null>(null);
    let bypassCheck = false;

    // 1. SvelteKit Route Navigation Guard
    beforeNavigate(({ cancel, to }) => {
        if (bypassCheck) return;

        if (isDirtyFn()) {
            cancel();
            pendingTarget = to?.url?.href || null;
            showDialog = true;
        }
    });

    // 2. Browser Window Unload Guard (Native)
    function handleBeforeUnload(e: BeforeUnloadEvent) {
        if (isDirtyFn()) {
            e.preventDefault();
            e.returnValue = "";
            return "";
        }
    }

    // Operations
    function confirmLeave() {
        bypassCheck = true;
        showDialog = false;
        if (pendingTarget) {
            goto(pendingTarget);
        }
    }

    function cancelLeave() {
        showDialog = false;
        pendingTarget = null;
    }

    return {
        get showDialog() { return showDialog; },
        set showDialog(v) { showDialog = v; },
        confirmLeave,
        cancelLeave,
        handleBeforeUnload
    };
}
