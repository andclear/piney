# syntax=docker/dockerfile:1

# Stage 0: 引入 xx 跨平台工具
FROM --platform=$BUILDPLATFORM tonistiigi/xx:master AS xx

# Stage 1: Chef - 基础环境与工具安装
FROM --platform=$BUILDPLATFORM rust:slim-bookworm AS chef
# 复制 xx 工具
COPY --from=xx / /
WORKDIR /app
# 安装基础构建工具 (clang/lld 用于跨平台链接，cmake/perl/make 用于部分 crate 源码构建)
RUN apt-get update && apt-get install -y clang lld pkg-config git cmake perl make build-essential
RUN cargo install cargo-chef

# Stage 2: Planner - 计算依赖配方
FROM chef AS planner
COPY . .
# 移除 cargo-chef 无法处理的 workspace members (如 src-tauri，Docker 不需要构建它)
RUN sed -i 's/"src-tauri",//' Cargo.toml
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Builder - 交叉编译依赖与应用
FROM chef AS builder
# 引入目标平台参数 (如 linux/arm64)
ARG TARGETPLATFORM
# 允许 sys crate (如 openssl-sys) 调用 pkg-config 进行跨平台编译
# OPENSSL_NO_VENDOR=1 强制使用系统库
ENV PKG_CONFIG_ALLOW_CROSS=1 \
    OPENSSL_NO_VENDOR=1 \
    OPENSSL_STATIC=0

# 安装目标架构的系统依赖 (libwebp, openssl 等)
# xx-apt-get 会自动根据 TARGETPLATFORM 安装对应的架构包
# 注意：不要在此安装 pkg-config，应使用 Stage 1 基础镜像中的宿主机版本，否则会导致架构不匹配无法运行
RUN xx-apt-get install -y libssl-dev libwebp-dev gcc libc6-dev

COPY --from=planner /app/recipe.json recipe.json
# 构建依赖层 (如果 Cargo.toml/lock 未变，此层将使用缓存)
# 显式设置 PKG_CONFIG_PATH 以便找到交叉编译的库
# 引入 cache mount 加速构建
RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    export PKG_CONFIG_PATH="/usr/lib/$(xx-info triple)/pkgconfig" && \
    xx-cargo chef cook --release --recipe-path recipe.json

# 构建应用层
COPY . .
RUN sed -i 's/"src-tauri",//' Cargo.toml
# 执行交叉编译
RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/root/.cargo/git \
    export PKG_CONFIG_PATH="/usr/lib/$(xx-info triple)/pkgconfig" && \
    xx-cargo build --release && \
    cp target/$(xx-cargo --print-target-triple)/release/piney-server /app/piney-server && \
    xx-verify /app/piney-server

# Stage 4: Frontend - 构建前端 (始终在构建平台/amd64 运行，速度最快，避免 QEMU)
FROM --platform=$BUILDPLATFORM node:20-slim AS frontend-builder
WORKDIR /app/frontend
# 单独复制依赖定义文件以利用缓存
COPY frontend/package.json frontend/package-lock.json ./
# 使用 npm ci 进行纯净安装 (比 npm install 更快更可靠)
RUN npm ci
# 复制源码并构建
COPY frontend ./
RUN npx svelte-kit sync
RUN npm run build

# Stage 5: Runtime - 最终运行时环境
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# 安装运行时依赖 (原生架构)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libwebp-dev \
    curl \
    openssl \
    && rm -rf /var/lib/apt/lists/*

# 复制产物
COPY --from=builder /app/piney-server /app/piney
COPY --from=frontend-builder /app/frontend/build /app/static
RUN mkdir -p /app/data

ENV RUN_MODE=server \
    PORT=9696 \
    DATA_DIR=/app/data

EXPOSE 9696
VOLUME ["/app/data"]

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:9696/api/health || exit 1

CMD ["/app/piney"]
