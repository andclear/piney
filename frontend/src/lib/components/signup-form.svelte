<script lang="ts">
    import { Button } from "$lib/components/ui/button/index.js";
    import * as Card from "$lib/components/ui/card/index.js";
    import * as Field from "$lib/components/ui/field/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import type { ComponentProps } from "svelte";
    import { auth } from "$lib/stores/auth.svelte";

    let { ...restProps }: ComponentProps<typeof Card.Root> = $props();

    let username = $state("");
    let password = $state("");
    let confirmPassword = $state("");
    let loading = $state(false);
    let error = $state("");

    async function handleSubmit(e: Event) {
        e.preventDefault();
        error = "";

        if (password !== confirmPassword) {
            error = "两次输入的密码不一致";
            return;
        }

        if (password.length < 6) {
            error = "密码长度至少为 6 位";
            return;
        }

        loading = true;
        try {
            const res = await fetch("http://localhost:9696/api/auth/setup", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ username, password }),
            });

            if (!res.ok) {
                const text = await res.text();
                throw new Error(text || "创建失败");
            }

            const data = await res.json();
            auth.setup(data.token);
        } catch (e: any) {
            console.error(e);
            error = e.message;
        } finally {
            loading = false;
        }
    }
</script>

<Card.Root {...restProps}>
    <Card.Header>
        <Card.Title>创建您的管理员账户</Card.Title>
        <Card.Description
            >创建完成后您将通过此账号和密码登录 Piney</Card.Description
        >
    </Card.Header>
    <Card.Content>
        <form onsubmit={handleSubmit}>
            <Field.Group>
                <Field.Field>
                    <Field.Label for="username">用户名</Field.Label>
                    <Input
                        id="username"
                        type="text"
                        placeholder="请输入用户名"
                        bind:value={username}
                        required
                    />
                </Field.Field>
                <Field.Field>
                    <Field.Label for="password">密码</Field.Label>
                    <Input
                        id="password"
                        type="password"
                        placeholder="请输入密码"
                        bind:value={password}
                        required
                    />
                    <Field.Description>密码长度至少为6位</Field.Description>
                </Field.Field>
                <Field.Field>
                    <Field.Label for="confirm-password">确认密码</Field.Label>
                    <Input
                        id="confirm-password"
                        type="password"
                        bind:value={confirmPassword}
                        required
                    />
                    <Field.Description
                        >如果忘记密码，可以查看config.yml配置文件</Field.Description
                    >
                </Field.Field>
                {#if error}
                    <div class="text-sm font-medium text-destructive">
                        {error}
                    </div>
                {/if}
                <Field.Group>
                    <Field.Field>
                        <Button type="submit" disabled={loading}>
                            {loading ? "创建中..." : "创建账户"}
                        </Button>
                        <Field.Description class="px-6 text-center">
                            已有账号？ <a href="/login">登录</a>
                        </Field.Description>
                    </Field.Field>
                </Field.Group>
            </Field.Group>
        </form>
    </Card.Content>
</Card.Root>
