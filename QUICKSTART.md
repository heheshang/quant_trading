# 快速入门指南

## 第一次运行项目

### 1. 安装依赖

```bash
# 安装 Rust 依赖（首次运行会自动下载编译）
cd ea_test
cargo check

# 安装前端依赖
npm install
```

### 2. 配置数据库（可选）

如果不配置数据库，系统将以内存模式运行（适合测试）。

#### PostgreSQL
```bash
# 安装 PostgreSQL 14+
# Windows: https://www.postgresql.org/download/windows/

# 创建数据库和用户
psql -U postgres
CREATE DATABASE quant_trading;
CREATE USER quant WITH PASSWORD 'quant_password';
GRANT ALL PRIVILEGES ON DATABASE quant_trading TO quant;
\q
```

#### Redis
```bash
# Windows: 使用 WSL 或 Docker
docker run -d -p 6379:6379 redis:latest
```

#### InfluxDB（可选）
```bash
# Docker 安装
docker run -d -p 8086:8086 influxdb:2.7
# 访问 http://localhost:8086 初始化
```

### 3. 配置环境变量

```bash
# 复制示例配置
cp .env.example .env

# 编辑 .env 文件，填入真实配置
notepad .env
```

### 4. 运行项目

```bash
# 开发模式（推荐）
npm run tauri dev

# 或者分步运行
npm run dev           # 终端1：启动 Vite
cargo tauri dev       # 终端2：启动 Tauri
```

首次运行会编译 Rust 代码，需要 5-10 分钟，请耐心等待。

### 5. 访问系统

应用会自动打开桌面窗口，或访问 http://localhost:5173

默认进入仪表盘页面。

## 常见问题

### Q: Cargo 编译失败？
**A**: 检查 Rust 版本是否 >= 1.77，运行 `rustup update`

### Q: 前端依赖安装失败？
**A**: 尝试使用淘宝镜像：
```bash
npm config set registry https://registry.npmmirror.com
npm install
```

### Q: Tauri 构建报错？
**A**: Windows 需要安装 WebView2，通常会自动安装。手动下载：
https://developer.microsoft.com/en-us/microsoft-edge/webview2/

### Q: 数据库连接失败？
**A**: 检查配置文件中的连接信息，确保数据库服务已启动。系统可以在无数据库情况下运行。

## 生产部署

```bash
# 构建优化版本
npm run build
npm run tauri build

# 生成的可执行文件在：
# src-tauri/target/release/
```

## 下一步

- 查看 [README.md](README.md) 了解系统架构
- 阅读各模块文档学习 API 使用
- 开发自己的交易策略
- 运行回测验证策略

## 技术支持

遇到问题？
1. 查看日志文件：`logs/quant-trading.log`
2. 提交 Issue 描述问题
3. 查阅在线文档
