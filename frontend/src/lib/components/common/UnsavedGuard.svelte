<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog";
    import { Button } from "$lib/components/ui/button";
    import { AlertTriangle } from "lucide-svelte";

    let { controller } = $props<{
        controller: ReturnType<
            typeof import("$lib/hooks/use-unsaved-changes.svelte").useUnsavedChanges
        >;
    }>();
</script>

<svelte:window onbeforeunload={controller.handleBeforeUnload} />

<Dialog.Root bind:open={controller.showDialog}>
    <Dialog.Content class="sm:max-w-[425px]">
        <Dialog.Header>
            <Dialog.Title class="flex items-center gap-2 text-destructive">
                <AlertTriangle class="h-5 w-5" />
                未保存的更改
            </Dialog.Title>
            <Dialog.Description class="pt-2">
                当前页面有未保存的编辑内容。如果离开，您的更改将会丢失。
            </Dialog.Description>
        </Dialog.Header>
        <Dialog.Footer class="mt-4 gap-2 sm:gap-0">
            <Button variant="outline" onclick={controller.cancelLeave}>
                取消（留在页面）
            </Button>
            <Button variant="destructive" onclick={controller.confirmLeave}>
                丢弃更改并离开
            </Button>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>
