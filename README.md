# 技术交流

加密图文聊天桌面软件（端到端加密），跨 Windows / macOS。

## 技术栈

- 桌面端：Tauri 2.x + React + TypeScript + TipTap
- 后端：Rust (axum) + SQLite
- 加密：X25519 + AES-256-GCM + Ed25519 + argon2

## 目录结构

```
crates/
  crypto/      E2EE 加密原语（前后端共享）
  protocol/    serde 类型与 API DTO
  server/      axum 后端（账号 / 公钥分发 / 密文中继 / 附件存储）
desktop/       Tauri 桌面端（src-tauri: Rust 核心；src: React 前端）
website/       官网与 Windows / macOS 下载入口
```

## 构建

```bash
# 后端
cargo build -p sealed-server --release

# 桌面端（需要 Tauri CLI 与系统 WebView2 / MSVC 工具链）
cd desktop && pnpm install && pnpm tauri dev
```

## 发布与自动更新

- `.github/workflows/release.yml` 在 GitHub Actions 的 Windows 与 macOS 环境构建安装包。
- 客户端每次启动会检查 GitHub Release 的 `latest.json`，发现更高版本后自动下载、安装并重启。
- 发布产物必须使用同一把 Tauri 私钥签名；私钥只存放在 GitHub Secret `TAURI_SIGNING_PRIVATE_KEY` 与离线备份中，不能提交到仓库。
- 发布新版本前同步更新 `desktop/package.json`、`desktop/src-tauri/Cargo.toml` 与 `desktop/src-tauri/tauri.conf.json` 的版本号，然后手动运行 Release workflow。

## 注册模式

- 注册必须填写一次性邀请码，邀请码生成后 30 天内有效。
- 仅用户名严格等于 `wangxin` 的已登录账号可在客户端“设置”中生成邀请码，服务端会再次校验权限。
- 所有注册用户默认互相可见，可直接搜索并发起会话；当前不维护额外好友关系表。

## 安全模型

- 服务端只存密文，无法解密任何消息内容。
- 客户端本地历史消息用本地主密钥（LMK）加密后落盘，防第三方扫描。
