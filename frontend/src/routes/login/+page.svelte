<script lang="ts">
    import LoginForm from "$lib/components/login-form.svelte";
</script>

<svelte:head>
    <title>登录 - Piney</title>
</svelte:head>

<div
    class="flex min-h-svh flex-col items-center justify-center bg-muted p-6 md:p-10"
>
    <div class="w-full max-w-sm md:max-w-2xl">
        <LoginForm />
    </div>
</div>
