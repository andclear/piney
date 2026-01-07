<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog/index.js";
    import * as Tabs from "$lib/components/ui/tabs/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Label } from "$lib/components/ui/label/index.js";
    import { toast } from "svelte-sonner";
    import { auth } from "$lib/stores/auth.svelte"; // Assuming auth store is here

    let { open = $bindable(false), user = { name: "", avatar: "" } } = $props();

    let avatarUrl = $state(user.avatar);

    // Account form
    let currentPassword = $state("");
    let newUsername = $state(user.name);
    let newPassword = $state("");
    let confirmPassword = $state("");
    let loading = $state(false);
    let fileInput: HTMLInputElement;

    $effect(() => {
        if (open) {
            // Reset form on open
            avatarUrl = user.avatar;
            newUsername = user.name;
            currentPassword = "";
            newPassword = "";
            confirmPassword = "";
        }
    });

    async function handleUpdateAvatar() {
        if (!avatarUrl) return;
        loading = true;
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch("/api/settings", {
                method: "PATCH",
                headers: {
                    "Content-Type": "application/json",
                    Authorization: `Bearer ${token}`,
                },
                body: JSON.stringify({ avatar: avatarUrl }),
            });
            if (!res.ok) throw new Error("Failed to update avatar");

            toast.success("头像已更新");
            // Ideally trigger a reload of user info in parent or auth store
            // For now, parent might update via binding? No, need to reload page or store.
            // Let's assume auth store or page reload.
            location.reload();
        } catch (e) {
            toast.error("更新头像失败");
        } finally {
            loading = false;
        }
    }

    async function handleFileUpload(e: Event) {
        const target = e.target as HTMLInputElement;
        const file = target.files?.[0];
        if (!file) return;

        // Reset input immediately so same file can be selected again
        target.value = "";

        loading = true;
        const formData = new FormData();
        formData.append("avatar", file);

        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch("/api/upload", {
                method: "POST",
                headers: {
                    Authorization: `Bearer ${token}`,
                },
                body: formData,
            });
            if (!res.ok) throw new Error("Upload failed");

            const data = await res.json();
            avatarUrl = data.url;

            // Auto-save setting after upload
            await handleUpdateAvatar();
        } catch (e) {
            toast.error("上传图片失败");
            console.error(e);
            loading = false;
        }
    }

    async function handleUpdateProfile() {
        if (!currentPassword) {
            toast.error("请输入当前密码");
            return;
        }
        if (newPassword && newPassword !== confirmPassword) {
            toast.error("两次输入的密码不一致");
            return;
        }

        loading = true;
        try {
            const token = localStorage.getItem("auth_token");
            const res = await fetch("/api/auth/profile", {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                    Authorization: `Bearer ${token}`,
                },
                body: JSON.stringify({
                    current_password: currentPassword,
                    new_username:
                        newUsername !== user.name ? newUsername : null,
                    new_password: newPassword || null,
                }),
            });

            if (!res.ok) {
                const err = await res.text();
                throw new Error(err || "Failed to update profile");
            }

            toast.success("个人信息已更新");

            const usernameChanged = newUsername !== user.name;
            if (newPassword || usernameChanged) {
                toast.info("账户信息已修改，请重新登录");
                auth.logout();
            } else {
                location.reload();
            }
            open = false;
        } catch (e: any) {
            // Handle error e.g. "Invalid current password"
            // Usually e.message is a JSON string if calling res.text() above failed to parse generic error?
            // Actually backend returns text for error.
            toast.error(e.message);
        } finally {
            loading = false;
        }
    }
</script>

<Dialog.Root bind:open>
    <Dialog.Content class="sm:max-w-[425px]">
        <Dialog.Header>
            <Dialog.Title>用户设置</Dialog.Title>
            <Dialog.Description>
                修改您的个人信息和账户安全设置。
            </Dialog.Description>
        </Dialog.Header>

        <Tabs.Root value="profile" class="w-full">
            <Tabs.List class="grid w-full grid-cols-2">
                <Tabs.Trigger value="profile">个人资料</Tabs.Trigger>
                <Tabs.Trigger value="account">账户安全</Tabs.Trigger>
            </Tabs.List>

            <!-- Profile Tab (Avatar) -->
            <Tabs.Content value="profile" class="space-y-4 py-4">
                <div class="flex flex-col justify-center items-center gap-4">
                    <img
                        src={avatarUrl}
                        alt="Avatar"
                        class="h-24 w-24 rounded-full object-cover border"
                    />
                    <div
                        class="space-y-2 w-full flex flex-col justify-center items-center text-center"
                    >
                        <Label
                            for="avatar-upload"
                            class="cursor-pointer font-normal"
                        >
                            <div class="flex flex-col items-center gap-2">
                                <Button
                                    variant="outline"
                                    class="w-auto px-8"
                                    onclick={() => fileInput?.click()}
                                >
                                    上传图片
                                </Button>
                                <span class="text-xs text-muted-foreground"
                                    >点击上传本地图片作为头像</span
                                >
                            </div>
                        </Label>
                        <input
                            bind:this={fileInput}
                            id="avatar-upload"
                            type="file"
                            accept="image/*"
                            class="hidden"
                            onchange={handleFileUpload}
                        />
                    </div>
                </div>
                <!-- Remove manual save button since we auto-save, or keep it for manual URL entry? 
                     User said "don't use link way", so remove URL input and manual save. 
                     Wait, if upload updates `avatarUrl`, we still need to save to settings. 
                     I put auto-save in handleFileUpload. So we can remove this button.
                -->
            </Tabs.Content>

            <!-- Account Tab (Username/Password) -->
            <Tabs.Content value="account" class="space-y-4 py-4">
                <div class="space-y-4">
                    <div class="space-y-2">
                        <Label for="username">用户名</Label>
                        <Input id="username" bind:value={newUsername} />
                    </div>

                    <div class="space-y-2">
                        <Label for="current_password">当前密码 (必填)</Label>
                        <Input
                            id="current_password"
                            type="password"
                            bind:value={currentPassword}
                        />
                    </div>

                    <div class="space-y-2">
                        <Label for="new_password">新密码</Label>
                        <Input
                            id="new_password"
                            type="password"
                            placeholder="如果不修改请留空"
                            bind:value={newPassword}
                        />
                    </div>

                    <div class="space-y-2">
                        <Label for="confirm_password">确认新密码</Label>
                        <Input
                            id="confirm_password"
                            type="password"
                            placeholder="再次输入新密码"
                            bind:value={confirmPassword}
                        />
                        {#if newPassword && confirmPassword && newPassword !== confirmPassword}
                            <p class="text-sm text-destructive font-medium">
                                两次输入的密码不一致
                            </p>
                        {/if}
                    </div>
                </div>
                <div class="flex justify-end">
                    <Button onclick={handleUpdateProfile} disabled={loading}>
                        {loading ? "保存中..." : "保存更改"}
                    </Button>
                </div>
            </Tabs.Content>
        </Tabs.Root>
    </Dialog.Content>
</Dialog.Root>
