<script lang="ts">
    import { Button } from "$lib/components/ui/button";
    import { onMount } from "svelte";
    import * as Card from "$lib/components/ui/card";
    import { Badge } from "$lib/components/ui/badge";
    import { Separator } from "$lib/components/ui/separator";
    import { 
        BookOpen, 
        ShieldCheck, 
        AlertTriangle, 
        Copyright, 
        Bot, 
        Gavel 
    } from "lucide-svelte";
    import { settings } from "$lib/stores/settings.svelte"; 
    import { auth } from "$lib/stores/auth.svelte";
    import { toast } from "svelte-sonner";
    import { goto } from "$app/navigation";
    import { breadcrumbs } from "$lib/stores/breadcrumb";

    let timeLeft = $state(10);
    let hasScrolledToBottom = $state(false);
    let agreeing = $state(false);
    
    // Check if user has already agreed
    $effect(() => {
        // If already agreed, maybe show "Agreed" status or just let them read
        // If not agreed, show the button
    });

    onMount(() => {
        // 设置面包屑
        breadcrumbs.set([{ label: '用户协议' }]);
        
        const timer = setInterval(() => {
            if (timeLeft > 0) {
                timeLeft--;
            } else {
                clearInterval(timer);
            }
        }, 1000);

        return () => clearInterval(timer);
    });

    function handleScroll(e: Event) {
        const target = e.target as HTMLDivElement;
        // Allow a small buffer (e.g. 20px)
        if (target.scrollTop + target.clientHeight >= target.scrollHeight - 20) {
            hasScrolledToBottom = true;
        }
    }

    async function handleAgree() {
        if (agreeing) return;
        agreeing = true;
        try {
            const token = localStorage.getItem('auth_token');
            // console.log("Submitting agreement with token:", token ? "Present" : "Missing");
            
            const headers: HeadersInit = {
                "Content-Type": "application/json",
            };
            if (token) {
                headers['Authorization'] = `Bearer ${token}`;
            }

            const res = await fetch("/api/settings", {
                method: "PATCH",
                headers,
                body: JSON.stringify({
                    user_agreement_accepted: true,
                }),
            });

            if (res.ok) {
                await settings.loadSettings();
                toast.success("已同意用户协议");
                // Redirect to home or previous page
                goto("/");
            } else if (res.status === 401) {
                toast.error("登录已过期，请重新登录");
                auth.logout();
            } else {
                toast.error("保存失败，请重试");
            }
        } catch (error) {
            console.error("Agreement error:", error);
            toast.error("网络错误，请重试");
        } finally {
            agreeing = false;
        }
    }
</script>

<div class="container max-w-4xl py-8 mx-auto h-[calc(100vh-4rem)] flex flex-col">
    <div class="flex-1 flex flex-col bg-background/95 backdrop-blur-sm rounded-xl border shadow-sm overflow-hidden">
        <div class="p-8 pb-4 border-b bg-muted/20 shrink-0">
            <div class="mx-auto bg-primary/10 p-3 rounded-full mb-4 w-16 h-16 flex items-center justify-center">
                <ShieldCheck class="w-8 h-8 text-primary" />
            </div>
            <h1 class="text-3xl font-bold text-center tracking-tight">用户使用协议</h1>
            <div class="text-center text-base mt-2 flex items-center justify-center gap-2">
                <Badge variant="outline" class="px-3 py-1">版本日期：2026年1月30日</Badge>
            </div>
        </div>

        <div 
            class="flex-1 overflow-y-auto px-8 py-6 space-y-8 bg-background/50 text-foreground"
            onscroll={handleScroll}
        >
            <!-- 欢迎语 -->
            <div class="space-y-4">
                <p class="leading-relaxed">
                    欢迎使用 <strong>Piney</strong> ，中文名称为 <strong>小兄许</strong>（以下简称“本工具”）。在您开始使用本工具之前，请务必仔细阅读并理解本协议的所有条款。
                </p>
                <div class="bg-destructive/10 border-l-4 border-destructive p-4 rounded-r-lg">
                    <p class="font-bold text-destructive">
                        一旦您开始下载、安装、复制或使用本工具，即表示您已同意受本协议所有条款的约束。如果您不同意本协议的任何条款，请立即停止使用并删除本工具。
                    </p>
                </div>
            </div>

            <Separator />

            <!-- 一、定义与服务内容 -->
            <section class="space-y-4">
                <h3 class="flex items-center gap-2 text-xl font-bold text-primary">
                    <BookOpen class="w-5 h-5" />
                    一、定义与服务内容
                </h3>
                <Card.Root class="bg-muted/30 border-none shadow-sm">
                    <Card.Content class="p-5 space-y-4 text-sm leading-relaxed">
                        <p>本工具是一个专为 SillyTavern 角色卡制作与编辑设计的辅助工作站。</p>
                        <p>主要功能包括但不限于：角色卡/小剧场/世界书/图片的导入、编辑、修改与生成；AI 辅助生成；前端美化；数据备份及检测等。</p>
                        <p class="font-bold text-amber-600 dark:text-amber-400 bg-amber-500/10 p-2 rounded">
                            本工具仅作为技术辅助手段，不提供任何内容素材。
                        </p>
                    </Card.Content>
                </Card.Root>
            </section>

            <!-- 二、使用许可与限制 -->
            <section class="space-y-4">
                <h3 class="flex items-center gap-2 text-xl font-bold text-primary">
                    <Copyright class="w-5 h-5" />
                    二、使用许可与限制
                </h3>
                <div class="space-y-4 text-sm">
                    <p>本工具遵循“共享、交流”的原则发布。</p>
                    
                    <div class="grid gap-4 md:grid-cols-1">
                        <!-- 1. 仅限学习交流 -->
                            <div class="flex gap-3 p-4 bg-green-500/5 border border-green-500/20 rounded-lg">
                            <span class="font-bold shrink-0">1.</span>
                            <div>
                                <strong class="text-green-700 dark:text-green-400">仅限学习交流：</strong> 
                                您仅可将本工具用于个人学习、研究或技术交流目的。
                            </div>
                        </div>

                        <!-- 2. 严禁商用 -->
                        <div class="flex gap-3 p-4 bg-red-500/5 border border-red-500/20 rounded-lg shadow-sm">
                            <span class="font-bold shrink-0">2.</span>
                            <div>
                                <strong class="text-red-700 dark:text-red-400">严禁商用：</strong> 
                                <strong class="text-red-700 dark:text-red-400">禁止</strong>将本工具或基于本工具修改后的衍生版本用于任何形式的商业用途（包括但不限于付费下载、付费会员制、广告盈利、推广引流、作为商业软件的一部分等）。
                            </div>
                        </div>

                        <!-- 3. 开源与许可协议 -->
                        <div class="flex gap-3 p-4 bg-blue-500/5 border border-blue-500/20 rounded-lg">
                            <span class="font-bold shrink-0">3.</span>
                            <div class="space-y-2">
                                <p>
                                    <strong class="text-blue-700 dark:text-blue-400">开源与许可协议</strong> 
                                    本项目采用 <strong>CC BY-NC-SA 4.0 (署名-非商业性使用-相同方式共享 4.0 国际)</strong> 协议进行授权。
                                </p>
                                <ul class="list-disc list-inside space-y-1 pl-2 text-muted-foreground">
                                    <li><strong>自由使用</strong>：您可以自由地复制、分发、修改本工具的源代码。</li>
                                    <li><strong>禁止商用</strong>：您<strong>不得</strong>将本工具或其任何衍生版本用于商业目的（包括但不限于付费销售、通过广告盈利、作为商业产品的一部分）。</li>
                                    <li><strong>相同方式共享</strong>：如果您修改了本工具或基于本工具开发了新项目，您的项目必须同样采用 <strong>CC BY-NC-SA 4.0</strong> 协议开源，且<strong>必须包含本条禁止商用的限制</strong>。</li>
                                </ul>
                                <p class="text-xs">查看协议全文:<a href="https://creativecommons.org/licenses/by-nc-sa/4.0/deed.en" target="_blank" class="text-primary hover:underline text-xs inline-flex items-center gap-1 mt-1">
                                    https://creativecommons.org/licenses/by-nc-sa/4.0/deed.en
                                </a></p>
                            </div>
                        </div>
                    </div>
                </div>
            </section>

            <!-- 三、用户行为规范 -->
            <section class="space-y-4">
                <h3 class="flex items-center gap-2 text-xl font-bold text-primary">
                    <Gavel class="w-5 h-5" />
                    三、用户行为规范与版权尊重
                </h3>
                
                <div class="bg-amber-500/10 border-l-4 border-amber-500 p-4">
                    <strong>本条款为本协议的核心，请务必遵守。</strong>
                </div>

                <div class="space-y-4 text-sm leading-relaxed">
                    <div class="flex gap-3">
                        <span class="font-bold shrink-0">1.</span>
                        <div>
                            <strong>工具属性声明：</strong> 本工具仅提供编辑和处理数据的技术功能。工具本身不拥有、不提供、也不存储任何角色卡、图片或文本内容的版权。
                        </div>
                    </div>

                    <div class="flex gap-3">
                        <span class="font-bold shrink-0">2.</span>
                        <div class="space-y-2">
                            <strong>尊重原作者权利：</strong>
                            <ul class="list-disc list-inside space-y-1 pl-2 text-muted-foreground bg-muted/30 p-3 rounded-lg">
                                <li><strong class="text-primary">严禁擅自二改：</strong> 在导入他人创作的角色卡、提示词（Prompt）、世界书或图片进行编辑、修改或“二改”之前，<strong class="text-red-700 dark:text-red-400">您必须确认原作者明确授权允许此类操作</strong>。</li>
                                <li><strong class="text-primary">遵守原始协议：</strong> 如果原作者在角色卡描述、发布页面或元数据中声明了“禁止二改”、“禁止转载”或“禁止用于AI训练”等限制，您必须严格遵守。</li>
                                <li><strong class="text-primary">后果自负：</strong> 因您违反原作者规定擅自修改、分发或公开他人作品而产生的任何版权纠纷，<strong class="text-red-700 dark:text-red-400">由您个人承担全部责任</strong>，与本工具及本工具开发者无关。</li>
                            </ul>
                        </div>
                    </div>

                    <div class="flex gap-3">
                        <span class="font-bold shrink-0">3.</span>
                        <div>
                            <strong>导入内容的合法性：</strong> 您保证您导入本工具的所有数据（包括但不限于图片、文本、配置），您均拥有合法的使用权或版权。
                        </div>
                    </div>
                </div>
            </section>

            <!-- 四、AI 免责 -->
            <section class="space-y-4">
                <h3 class="flex items-center gap-2 text-xl font-bold text-primary">
                    <Bot class="w-5 h-5" />
                    四、关于 AI 生成内容的免责
                </h3>
                <div class="space-y-3 text-sm leading-relaxed">
                    <div class="flex gap-3">
                        <span class="font-bold shrink-0">1.</span>
                        <p><strong>AI功能的来源：</strong> 本工具不提供任何AI能力，用户需要自行接入LLM（大语言模型）的API进行使用。</p>
                    </div>
                    <div class="flex gap-3">
                        <span class="font-bold shrink-0">2.</span>
                        <p><strong>生成结果的不确定性：</strong> 本工具接入的 AI 功能仅作为辅助工具。AI 生成的内容具有随机性和不可控性，开发者不对生成内容的准确性、逻辑性或价值做任何保证。</p>
                    </div>
                    <div class="flex gap-3">
                        <span class="font-bold shrink-0">3.</span>
                        <p><strong>合规性责任：</strong> 您在使用 AI 生成功能时，必须遵守相关法律法规。不得利用本工具生成违反法律法规、违背公序良俗、色情、暴力、仇恨言论或侵犯他人权益的内容。</p>
                    </div>
                    <div class="flex gap-3">
                        <span class="font-bold shrink-0">4.</span>
                        <p><strong>AI 版权归属：</strong> 关于 AI 生成内容的版权归属，请遵循您所使用的 AI 模型服务商（如 OpenAI, Claude, 本地模型等）的服务条款。本工具开发者不对生成的作品主张任何版权，也不对因使用生成作品产生的版权纠纷负责。</p>
                    </div>
                </div>
            </section>

            <!-- 五、免责声明 -->
            <section class="space-y-4">
                <h3 class="flex items-center gap-2 text-xl font-bold text-primary">
                    <AlertTriangle class="w-5 h-5" />
                    五、免责声明
                </h3>
                <div class="text-sm bg-muted/20 p-4 rounded-lg space-y-3">
                    <div class="flex gap-3">
                        <span class="font-bold shrink-0">1.</span>
                        <p><strong>“按原样”提供：</strong> 本工具按“现状”提供，不包含任何明示或暗示的保证（包括但不限于适销性、特定用途适用性或不侵权性）。</p>
                    </div>
                    <div class="flex gap-3">
                        <span class="font-bold shrink-0">2.</span>
                        <p><strong>数据安全：</strong> 虽然本工具包含数据备份功能，但开发者不保证数据的绝对安全。<strong>请您务必自行定期备份重要数据。</strong> 因软件故障、操作失误或不可抗力导致的数据丢失、损坏，开发者不承担赔偿责任。</p>
                    </div>
                    <div class="flex gap-3">
                        <span class="font-bold shrink-0">3.</span>
                        <p><strong>责任限制：</strong> 在法律允许的最大范围内，开发者不对因使用或无法使用本工具而引起的任何直接、间接、附带或惩罚性损害赔偿负责。</p>
                    </div>
                </div>
            </section>

            <!-- 6. 协议修改 -->
            <section class="space-y-4 pb-8">
                <h3 class="text-xl font-bold text-primary">
                    六、协议修改
                </h3>
                <p class="text-sm text-muted-foreground">
                    开发者保留在任何时候修改本协议的权利。修改后的协议将在项目发布页面更新。如果您在协议更新后继续使用本工具，即表示您接受修改后的协议。
                </p>
            </section>
        </div>

        <div class="p-6 pt-4 flex items-center justify-between gap-4 border-t bg-muted/20 shrink-0">
             <div class="text-xs text-muted-foreground flex-1">
                {#if $settings.user_agreement_accepted}
                     您已同意本协议
                {:else}
                    {#if !hasScrolledToBottom}
                        请滑动到底部阅读完整协议
                    {:else if timeLeft > 0}
                        请仔细阅读协议内容 ({timeLeft}s)
                    {:else}
                         感谢您的阅读与支持
                    {/if}
                {/if}
            </div>

            {#if !$settings.user_agreement_accepted}
                <Button 
                    onclick={handleAgree} 
                    disabled={!hasScrolledToBottom || timeLeft > 0 || agreeing} 
                    class="w-32 transition-all duration-300"
                    variant={(!hasScrolledToBottom || timeLeft > 0) ? "secondary" : "default"}
                >
                    {#if agreeing}
                        处理中...
                    {:else if timeLeft > 0}
                        请阅读 ({timeLeft}s)
                    {:else}
                        我已阅读并同意
                    {/if}
                </Button>
            {:else}
                 <Button variant="outline" disabled>已同意</Button>
            {/if}
        </div>
    </div>
</div>
