# 看板页面 (Dashboard) 实现计划

## 目标
实现一个美观、高效的首页看板，包含统计数据、最近编辑记录及每日幸运角色功能。

## 1. 后端实现 (Rust)

### 1.1 新增模块 `src/api/dashboard.rs`
-   **结构体定义的响应数据**:
    ```rust
    #[derive(Serialize)]
    pub struct DashboardStats {
        pub total_characters: u64,
        pub total_world_info: u64,
        pub total_tokens_k: f64, // 以 K 为单位
        pub db_size_mb: f64,     // 以 MB 为单位
        pub recent_cards: Vec<SimpleCard>,
        pub lucky_card: LuckyCardInfo,
    }
    
    #[derive(Serialize)]
    pub struct LuckyCardInfo {
        pub id: Uuid,
        pub name: String,
        pub avatar: String, // 相对路径
        pub quote: String,  // 随机骚话或者固定文案？暂时用 "今天的幸运色是..."
    }
    ```
-   **每日推荐逻辑**:
    -   读取 `settings` 表中的 `daily_lucky_date` 和 `daily_lucky_id`。
    -   如果不为今天或 ID 无效:
        -   从数据库随机获取一个 ID (使用 `ORDER BY RANDOM() LIMIT 1`)。
        -   更新 `settings` 表。
    -   返回该角色信息。
-   **统计逻辑**:
    -   `daily_lucky` 与 统计查询 并行处理或顺序处理 (速度很快)。
    -   `total_tokens` 使用 SQL `SUM` 聚合。
    -   数据库大小通过 `std::fs` 获取文件大小。

### 1.2 注册路由
-   在 `src/api/mod.rs` 中注册 `GET /api/dashboard`。

## 2. 前端实现 (Svelte)

### 2.1 修改 `src/routes/(app)/+page.svelte`
-   **布局**:
    -   顶部: 欢迎语 "欢迎回来, {User}".
    -   第一行: 统计卡片 (4列). 
        -   样式参考: 左侧对应颜色的粗边框。
        -   适配: PC 4列, 手机 2列.
    -   第二行: 双栏布局 (PC: 左 2/3, 右 1/3; 手机: 堆叠).
        -   左侧: "最近编辑" 列表.
        -   右侧: "每日幸运角色" 卡片 + 抽卡按钮.

### 2.2 UI 细节
-   **组件**: 使用 `shadcn-svelte` 的 `Card`, `Button`。
-   **图标**: `lucide-svelte` (Users, Book, Zap, Database 等)。
-   **每日角色**:
    -   显示大图/背景图? 参考设计图是竖向卡片。
    -   显示名字。
    -   按钮 "我命由我不由天，抽卡！"。

## 3. 执行顺序
1.  创建后端 API。
2.  测试 API 数据。
3.  实现前端 UI。
