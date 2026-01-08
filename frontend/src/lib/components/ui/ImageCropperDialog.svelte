<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog";
    import { Button } from "$lib/components/ui/button";
    import Cropper from "svelte-easy-crop";
    import getCroppedImg from "$lib/utils/canvasUtils";
    import { createEventDispatcher } from "svelte";
    import { Loader2 } from "lucide-svelte";

    export let open = false;
    export let imageSrc: string | null = null;
    export let aspect = 2 / 3; // Default to 2:3 for character cards (512x768)

    let crop = { x: 0, y: 0 };
    let zoom = 1;
    let pixelCrop: any = null;
    let loading = false;

    const dispatch = createEventDispatcher();

    function onCropComplete(e: any) {
        pixelCrop = e.pixels;
    }

    async function handleConfirm() {
        if (!imageSrc || !pixelCrop) return;
        loading = true;
        try {
            const blob = await getCroppedImg(imageSrc, pixelCrop);
            if (blob) {
                dispatch("confirm", blob);
                open = false;
            }
        } catch (e) {
            console.error(e);
        } finally {
            loading = false;
        }
    }

    function handleCancel() {
        open = false;
        dispatch("cancel");
    }
</script>

<Dialog.Root bind:open>
    <Dialog.Content class="sm:max-w-[600px]">
        <Dialog.Header>
            <Dialog.Title>裁剪图片</Dialog.Title>
            <Dialog.Description>
                拖动以调整图片位置和缩放，确保关键内容在框内。
            </Dialog.Description>
        </Dialog.Header>

        <!-- Use theme background instead of black -->
        <div class="relative w-full h-[400px] bg-secondary/20 rounded-md overflow-hidden flex items-center justify-center">
            {#if imageSrc}
                <Cropper
                    image={imageSrc}
                    {crop}
                    {zoom}
                    {aspect}
                    oncropcomplete={onCropComplete}
                    oncropchange={(e) => (crop = e)}
                    onzoomchange={(e) => (zoom = e)}
                />
            {/if}
        </div>

        <div class="flex flex-col gap-2">
            <div class="flex items-center justify-between text-xs text-muted-foreground">
                <span>缩放</span>
                <span>{zoom.toFixed(1)}x</span>
            </div>
            <input
                type="range"
                min="1"
                max="3"
                step="0.1"
                bind:value={zoom}
                class="w-full h-2 bg-secondary rounded-lg appearance-none cursor-pointer accent-primary"
            />
        </div>

        <Dialog.Footer>
            <Button variant="outline" onclick={handleCancel} disabled={loading}>取消</Button>
            <Button onclick={handleConfirm} disabled={loading}>
                {#if loading}
                    <Loader2 class="mr-2 h-4 w-4 animate-spin" />
                {/if}
                确认裁剪
            </Button>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>
