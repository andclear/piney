# Windows 环境下 Piney 项目 APK 打包指南

本指南将帮助你在 Windows 云电脑上配置完整的 Android 打包环境。

## 目录

1. [系统要求](#系统要求)
2. [安装必要软件](#安装必要软件)
3. [环境变量配置](#环境变量配置)
4. [克隆项目](#克隆项目)
5. [签名配置](#签名配置)
6. [打包 APK](#打包-apk)
7. [常见问题](#常见问题)

---

## 系统要求

- Windows 10/11 64位
- 至少 16GB RAM（推荐 32GB）
- 至少 50GB 可用磁盘空间
- 稳定的网络连接

---

## 安装必要软件

### 1. Git

下载安装：https://git-scm.com/download/win

安装时保持默认选项即可。

### 2. Node.js (LTS 版本)

下载安装：https://nodejs.org/

推荐使用 LTS 版本（如 20.x）。安装时勾选 "Add to PATH"。

验证安装：
```powershell
node --version
npm --version
```

### 3. Rust 工具链

打开 PowerShell（管理员），运行：
```powershell
winget install Rustlang.Rustup
```

或者访问 https://rustup.rs/ 下载安装程序。

安装完成后，**重新打开 PowerShell**，添加 Android 目标：
```powershell
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

验证安装：
```powershell
rustc --version
cargo --version
```

### 4. Android Studio

下载安装：https://developer.android.com/studio

安装时选择 "Standard" 安装类型。

**安装完成后，打开 Android Studio：**

1. 点击 `More Actions` → `SDK Manager`
2. 在 `SDK Platforms` 标签页，勾选：
   - Android 14.0 (API 34) 或更高版本
3. 在 `SDK Tools` 标签页，勾选：
   - Android SDK Build-Tools
   - Android SDK Command-line Tools
   - Android SDK Platform-Tools
   - NDK (Side by side) - **必须安装！**
4. 点击 Apply 并等待下载完成

### 5. Java JDK 17

Android Studio 自带 JDK，后续会用它。如果需要单独安装：
```powershell
winget install Microsoft.OpenJDK.17
```

---

## 环境变量配置

### 方法 1：通过系统设置

1. 按 `Win + R`，输入 `sysdm.cpl`，回车
2. 点击 `高级` → `环境变量`
3. 在 `系统变量` 中添加/修改：

| 变量名 | 值 |
|--------|-----|
| `JAVA_HOME` | `C:\Program Files\Android\Android Studio\jbr` |
| `ANDROID_HOME` | `C:\Users\<你的用户名>\AppData\Local\Android\Sdk` |

4. 编辑 `Path` 变量，添加：
   - `%ANDROID_HOME%\platform-tools`
   - `%ANDROID_HOME%\tools`
   - `%ANDROID_HOME%\tools\bin`

### 方法 2：使用 PowerShell 脚本

创建一个 `setup-env.ps1` 文件：
```powershell
# Android SDK 路径
$env:ANDROID_HOME = "$env:LOCALAPPDATA\Android\Sdk"
$env:NDK_HOME = "$env:ANDROID_HOME\ndk\$(Get-ChildItem $env:ANDROID_HOME\ndk | Select-Object -First 1)"

# Java JDK (使用 Android Studio 自带)
$env:JAVA_HOME = "C:\Program Files\Android\Android Studio\jbr"

# 添加到 PATH
$env:PATH = "$env:ANDROID_HOME\platform-tools;$env:JAVA_HOME\bin;$env:PATH"

Write-Host "环境变量已设置："
Write-Host "  JAVA_HOME = $env:JAVA_HOME"
Write-Host "  ANDROID_HOME = $env:ANDROID_HOME"
Write-Host "  NDK_HOME = $env:NDK_HOME"
```

每次打开 PowerShell 后运行：
```powershell
.\setup-env.ps1
```

验证配置：
```powershell
java -version
adb --version
```

---

## 克隆项目

```powershell
# 克隆仓库
git clone https://github.com/你的用户名/piney.git
cd piney

# 安装前端依赖
cd frontend
npm install
cd ..

# 安装 Tauri CLI (如果尚未安装)
npm install
```

---

## 签名配置

### 1. 复制 Keystore 文件

将你的 `piney-key.jks` 文件复制到项目根目录：
```
piney/
├── piney-key.jks    <-- 放在这里
├── frontend/
├── src-tauri/
└── ...
```

### 2. 创建签名配置文件

在项目根目录创建 `android-signing.properties` 文件：
```properties
key.store=C:/path/to/piney/piney-key.jks
key.store.password=你的密码
key.alias=piney
key.alias.password=你的密码
```

> ⚠️ **注意**：Windows 路径使用正斜杠 `/` 或双反斜杠 `\\`

### 3. 确保 Gradle 读取配置

检查 `src-tauri/gen/android/app/build.gradle.kts` 是否包含读取 `android-signing.properties` 的代码。如果没有，需要手动添加。

---

## 打包 APK

### 1. 初始化 Android 项目（仅首次）

```powershell
npm run tauri -- android init
```

### 2. 打包 Release 版本

```powershell
npm run tauri -- android build --apk true
```

打包完成后，APK 文件位于：
```
src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release.apk
```

### 3. 快速打包脚本

创建 `build-android.ps1`：
```powershell
# 设置环境变量
$env:JAVA_HOME = "C:\Program Files\Android\Android Studio\jbr"
$env:ANDROID_HOME = "$env:LOCALAPPDATA\Android\Sdk"
$env:PATH = "$env:JAVA_HOME\bin;$env:PATH"

# 显示版本信息
Write-Host "Java: $(java -version 2>&1 | Select-Object -First 1)"
Write-Host "Rust: $(rustc --version)"
Write-Host "Node: $(node --version)"

# 开始打包
Write-Host "`n开始打包 Android APK...`n"
npm run tauri -- android build --apk true

# 复制输出文件
$apkPath = "src-tauri\gen\android\app\build\outputs\apk\universal\release\app-universal-release.apk"
if (Test-Path $apkPath) {
    Copy-Item $apkPath ".\piney-release.apk"
    Write-Host "`n✅ 打包成功！APK 已复制到: piney-release.apk"
} else {
    Write-Host "`n❌ 打包失败，请检查错误信息"
}
```

运行：
```powershell
.\build-android.ps1
```

---

## 常见问题

### Q: 提示找不到 NDK

确保 NDK 已安装。在 Android Studio 的 SDK Manager 中检查 NDK 是否勾选。

或手动设置：
```powershell
$env:NDK_HOME = "C:\Users\<用户名>\AppData\Local\Android\Sdk\ndk\<版本号>"
```

### Q: Gradle 下载缓慢

配置国内镜像。编辑 `src-tauri/gen/android/build.gradle.kts`，在 `repositories` 中添加阿里云镜像：
```kotlin
maven { url = uri("https://maven.aliyun.com/repository/google") }
maven { url = uri("https://maven.aliyun.com/repository/central") }
maven { url = uri("https://maven.aliyun.com/repository/gradle-plugin") }
```

### Q: 内存不足导致编译失败

增加 Gradle 内存。编辑或创建 `src-tauri/gen/android/gradle.properties`：
```properties
org.gradle.jvmargs=-Xmx4096m -XX:MaxMetaspaceSize=1024m
```

### Q: Rust 编译缓慢

使用 sccache 缓存编译结果：
```powershell
cargo install sccache
$env:RUSTC_WRAPPER = "sccache"
```

### Q: 签名配置不生效

检查 `android-signing.properties` 路径是否正确，确保使用正斜杠。

---

## 快速检查清单

打包前确认以下内容：

- [ ] Git 已安装
- [ ] Node.js 已安装 (node --version)
- [ ] Rust 已安装 (rustc --version)
- [ ] Android 目标已添加 (rustup target list | grep android)
- [ ] Android Studio 已安装
- [ ] Android SDK 已安装
- [ ] NDK 已安装
- [ ] JAVA_HOME 已设置
- [ ] ANDROID_HOME 已设置
- [ ] 签名文件已配置

---

## 预计时间

在配置合理的 Windows 云电脑上：
- 首次完整编译：约 10-20 分钟
- 增量编译：约 2-5 分钟
