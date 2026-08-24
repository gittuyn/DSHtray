# DSHtray

DSHtray 是 DeepSeek Harness 的 Windows 托盘管理器。它只管理一个 DSH 目标，并支持：

- 源码模式：`pnpm dsh web`
- 打包模式：`DSH.exe`
- 启动、停止、重启和本机健康检查
- 代理 `http://127.0.0.1:7897` 的持久化开关
- 运行中切换代理前明确确认并重启 DSH
- 外部 DSH 进程默认只观察，确认后才接管
- 关闭窗口隐藏到托盘；只有“退出管理器”会退出托盘进程
- 管理器开机启动与 DSH 登录自动启动分离；默认不自动启动 DSH
- 配置损坏备份、脱敏日志、诊断自检

## 安全边界

- 管理器不保存 API key、token、密码或其他凭据。
- 代理关闭时，管理器不主动注入或清理已有环境变量。
- 停止流程先请求正常退出并等待 5 秒；只有已确认归属的 Job Object 进程树才会强制终止。
- 服务地址第一版只允许 `127.0.0.1` 或 `localhost`。

## 托盘图标和进程窗口

- 代理关闭时使用 DeepSeek 蓝色托盘图标。
- 代理开启时使用 DeepSeek 黑色托盘图标。
- 图标由 `scripts/fetch-tray-icons.mjs` 从 Simple Icons 下载并转换为内置 PNG；运行时不依赖网络。
- DSH 源码/打包目标使用 Windows `CREATE_NO_WINDOW` 启动，不显示控制台黑框。

刷新图标资源：

```text
pnpm fetch:tray-icons
```

## 构建

要求：Windows 11、Rust MSVC、Visual Studio C++ Build Tools、Node.js 和 pnpm。

```text
pnpm install
pnpm typecheck
pnpm test
pnpm build
pnpm tauri build --no-bundle --ci
pnpm tauri build --ci --bundles nsis --no-sign
```

Rust 全量测试（包含进程 fixture）：

```text
cargo test --manifest-path src-tauri/Cargo.toml --features test-fixture --all-targets
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

## 交付物

- 绿色版：`src-tauri/target/release/dshtray.exe`
- NSIS 安装器：`src-tauri/target/release/bundle/nsis/DSHtray_0.1.0_x64-setup.exe`
- 绿色版复制脚本：`scripts/build-portable.ps1`
- 产物校验脚本：`scripts/verify-release.ps1`

执行校验：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-release.ps1
```

未配置代码签名证书时，构建产物会显示 `NotSigned`；正式发布前应通过组织的代码签名流程签名并重新校验。
