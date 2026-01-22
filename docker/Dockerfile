# 多阶段构建 Dockerfile
# Stage 1: 构建 Rust 后端
FROM rust:slim-bookworm AS rust-builder

WORKDIR /app

# 安装构建依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libwebp-dev \
    && rm -rf /var/lib/apt/lists/*

# 复制 Cargo 配置
COPY Cargo.toml Cargo.lock ./
COPY migration ./migration

# 修改 Cargo.toml 移除 src-tauri 工作区成员 (Docker 不需要构建桌面应用)
RUN sed -i 's/"src-tauri",//' Cargo.toml

# 创建虚拟的 src 目录以缓存依赖
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    echo "" > src/lib.rs

# 预编译依赖
RUN cargo build --release && rm -rf src

# 复制实际源码
COPY src ./src

# 构建应用
RUN touch src/main.rs src/lib.rs && \
    cargo build --release

# Stage 2: 构建 Svelte 前端
FROM node:20-slim AS frontend-builder

WORKDIR /app/frontend

# 只复制 package.json (不复制 package-lock.json，避免跨平台依赖问题)
COPY frontend/package.json ./

# 安装依赖 (在 Linux 上重新生成 lockfile)
RUN npm install

# 复制前端源码
COPY frontend ./

# 构建静态文件
RUN npm run build

# Stage 3: 运行时镜像
FROM debian:bookworm-slim AS runtime

WORKDIR /app

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libwebp-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# 复制后端二进制文件
COPY --from=rust-builder /app/target/release/piney-server /app/piney

# 复制前端静态文件
COPY --from=frontend-builder /app/frontend/build /app/static

# 创建数据目录
RUN mkdir -p /app/data

# 设置环境变量
ENV RUN_MODE=server
ENV PORT=9696
ENV DATA_DIR=/app/data

# 暴露端口
EXPOSE 9696

# 数据卷
VOLUME ["/app/data"]

# 健康检查
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:9696/api/health || exit 1

# 启动命令
CMD ["/app/piney"]
