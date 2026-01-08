<script lang="ts">
    import HTMLRender from '$lib/components/render/HTMLRender.svelte';
    import { Textarea } from '$lib/components/ui/textarea';
    import { Label } from '$lib/components/ui/label';

    let customInput = $state(`<div class="p-4 bg-blue-100 rounded-lg shadow">
  <h2 class="text-xl font-bold text-blue-800">Tailwind Test</h2>
  <p class="text-blue-600">This content is styled with TailwindCSS.</p>
</div>`);

    const cases = [
        {
            title: "Simple HTML & CSS",
            content: `
<style>
    .demo-box { background: #fee; border: 2px solid #f00; padding: 20px; text-align: center; }
    h1 { color: #d00; margin: 0; }
</style>
<div class="demo-box">
    <h1>Hello Render!</h1>
    <p>This is a basic HTML/CSS test.</p>
</div>
`
        },
        {
            title: "Vue 3 Interaction",
            content: `
<div id="app" class="p-4 border rounded bg-gray-50">
  <h3 class="mb-2 font-bold">{{ message }}</h3>
  <button @click="count++" class="px-4 py-2 bg-green-500 text-white rounded hover:bg-green-600">
    Count is: {{ count }}
  </button>
</div>

<script>
  const { createApp, ref } = Vue;
  createApp({
    setup() {
      const count = ref(0);
      return { count, message: 'Vue 3 works!' }
    }
  }).mount('#app')
<\/script>
`
        },
        {
            title: "Auto Height Adjustment",
            content: `
<div style="background: #eef; padding: 10px; border: 1px dashed #00f;">
    <p>Click the button to expand content.</p>
    <button onclick="document.getElementById('more').style.display = 'block'" style="padding: 5px 10px;">Expand</button>
    <div id="more" style="display: none; margin-top: 10px; padding: 20px; background: #fff;">
        <p>Extra content appeared!</p>
        <p>The iframe should resize to fit this.</p>
        <p>One more line.</p>
    </div>
</div>
`
        },
        {
            title: "VH Unit Conversion",
            content: `
<div style="height: 50vh; background: linear-gradient(to bottom, #f0f, #00f); color: white; display: flex; items-center; justify-center; text-align: center;">
    <div style="padding: 20px;">
        <p>This div is 50vh high.</p>
        <p>It should be 50% of the BROWSER window height,</p>
        <p>NOT 50% of the iframe height.</p>
    </div>
</div>
`
        }
    ];
</script>

<div class="container mx-auto py-10 space-y-12">
    <h1 class="text-3xl font-bold">HTMLRender Debug Page</h1>

    <!-- Custom Input -->
    <div class="space-y-4 border p-6 rounded-xl bg-card">
        <h2 class="text-xl font-semibold">Custom Test</h2>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div class="space-y-2">
                <Label>Input HTML</Label>
                <Textarea bind:value={customInput} rows={10} class="font-mono text-xs" />
            </div>
            <div class="space-y-2">
                <Label>Result</Label>
                <div class="border rounded-lg min-h-[200px] bg-white/50">
                    <HTMLRender content={customInput} />
                </div>
            </div>
        </div>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-2 gap-x-8 gap-y-12">
        {#each cases as demo}
            <div class="space-y-2">
                <h3 class="font-medium text-lg border-b pb-2">{demo.title}</h3>
                <div class="border rounded-lg bg-background shadow-sm">
                     <HTMLRender content={demo.content} />
                </div>
                <details class="text-xs text-muted-foreground mt-2">
                    <summary>View Source</summary>
                    <pre class="p-2 bg-muted rounded mt-2 overflow-x-auto">{demo.content}</pre>
                </details>
            </div>
        {/each}
    </div>
</div>
