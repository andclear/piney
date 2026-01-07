<script lang="ts">
	import * as Card from "$lib/components/ui/card/index.js";
	import {
		FieldGroup,
		Field,
		FieldLabel,
		FieldDescription,
		FieldSeparator,
	} from "$lib/components/ui/field/index.js";
	import { Input } from "$lib/components/ui/input/index.js";
	import { Button } from "$lib/components/ui/button/index.js";
	import { cn } from "$lib/utils.js";
	import type { HTMLAttributes } from "svelte/elements";
	import { auth } from "$lib/stores/auth.svelte";

	let { class: className, ...restProps }: HTMLAttributes<HTMLDivElement> =
		$props();

	const id = $props.id();
	let username = $state("");
	let password = $state("");
	let loading = $state(false);
	let error = $state("");

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = "";
		loading = true;

		try {
			const res = await fetch("http://localhost:9696/api/auth/login", {
				method: "POST",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify({ username, password }),
			});

			if (!res.ok) {
				const text = await res.text();
				throw new Error(text || "登录失败");
			}

			const data = await res.json();
			auth.login(username, data.token);
		} catch (e: any) {
			console.error(e);
			error = e.message;
		} finally {
			loading = false;
		}
	}
</script>

<div class={cn("flex flex-col gap-6", className)} {...restProps}>
	<Card.Root class="overflow-hidden p-0">
		<Card.Content class="grid p-0 md:grid-cols-2">
			<form class="p-6 md:p-8" onsubmit={handleSubmit}>
				<FieldGroup>
					<div class="flex flex-col items-center gap-2 text-center">
						<h1 class="text-2xl font-bold">你好！小兄许</h1>
						<p class="text-muted-foreground text-balance">
							使用用户名和密码登录
						</p>
					</div>
					<Field>
						<FieldLabel for="username-{id}">用户名</FieldLabel>
						<Input
							id="username-{id}"
							type="text"
							placeholder="请输入用户名"
							bind:value={username}
							required
						/>
					</Field>
					<Field>
						<FieldLabel for="password-{id}">密码</FieldLabel>
						<Input
							id="password-{id}"
							type="password"
							bind:value={password}
							required
						/>
					</Field>
					{#if error}
						<div class="text-sm font-medium text-destructive">
							{error}
						</div>
					{/if}
					<Field>
						<Button type="submit" disabled={loading}>
							{loading ? "登录中..." : "登录"}
						</Button>
					</Field>
					<div>
						<p
							class="text-center text-muted-foreground text-balance text-xs"
						>
							如果忘了用户名和密码就到XX里找
						</p>
					</div>
				</FieldGroup>
			</form>
			<div class="bg-muted relative hidden md:block">
				<img
					src="/login-bg.webp"
					alt="Login Background"
					class="absolute inset-0 h-full w-full object-cover dark:brightness-[0.2] dark:grayscale"
				/>
			</div>
		</Card.Content>
	</Card.Root>
	<div class="pt-4">
		<FieldDescription class="px-6 text-center">
			本项目仅供个人使用，严禁用于商业用途 | Power By Laopobao
		</FieldDescription>
	</div>
</div>
