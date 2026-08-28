# DSHtray

DSHtray 是 DeepSeek Harness 的 Windows 托盘管理器。它只管理一个 DSH 目标，并支持：

- 源码模式：`pnpm dsh web`
- 打包模式：`DSH.exe`
- 启动、停止、重启和本机健康检查
- 代理 `http://127.0.0.1:7897` 的持久化开关
- 运行中切换代理前明确确认并重启 DSH
- 外部 DSH 进程默认只观察，确认后才接管
- 接管前重新验证 listener/父链和 Job Object 归属；已属于其他 Job Object 时使用已确认 PID 的直接控制
- 管理器使用固定 `Local\\DeepSeekHarnessManager` Job 名称，重启后可重新打开自己的旧 Job，不会误报为外部冲突
- 关闭窗口隐藏到托盘；只有“退出管理器”会退出托盘进程
- 管理器开机启动与 DSH 登录自动启动分离；默认不自动启动 DSH
- 配置损坏备份、脱敏日志、诊断自检

## 安全边界

- 不保存任何 API 密钥、令牌、密码或其他凭据值。
- 代理关闭时，管理器不主动注入或清理已有环境变量。
- 停止流程先请求正常退出并等待 5 秒；管理器 Job 或用户确认的精确 PID 树才允许强制终止。
- 外部进程若已属于其他 Job Object，不强行跨 Job 加入；确认后保存 PID/身份快照，强制阶段逐 PID 复核后调用 `TerminateProcess`，不使用递归 `taskkill /T`，PID 被复用或身份变化则拒绝终止。
- 服务地址第一版只允许 `127.0.0.1` 或 `localhost`。

## 托盘图标和进程窗口

托盘图标按运行状态优先、代理状态其次选择：

- DSH 未启动（`Stopped`，以及没有可运行 DSH 的 `PortConflict`/启动失败状态）使用红色鲸鱼图标 `#DC2626`。
- 发现由非 DSHtray 启动、等待用户确认接管的外部 DSH（`External` + `External`）使用黄色鲸鱼图标 `#EAB308`。
- DSH 正常由管理器负责或已确认接管时，代理关闭使用蓝色鲸鱼图标，代理开启使用黑色鲸鱼图标。
- 托盘初始化会读取当前运行快照；启动、停止、重启、外部接管、代理变更和配置变更后都会重新同步图标与 tooltip。
- 鼠标交互：左键单击在双击判定窗口结束后，直接使用系统默认浏览器打开当前配置的 DSH 页面；Windows 左键双击取消待执行的单击动作并显示 DSHtray 管理器；右键继续显示托盘菜单。
- 四种图标均使用 64×64 源图，并带约 6 像素的白色描边（`2.25` SVG `viewBox` 单位）；`viewBox` 额外扩展 3 个单位，确保头尾和上下轮廓不接触画布边缘。
- 图标由 `scripts/fetch-tray-icons.mjs` 从 Simple Icons 下载、添加描边、扩展透明留白并转换为内置 64×64 PNG；运行时不依赖网络。
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

Release 使用 `opt-level = "z"`、Thin LTO、单 codegen unit、`panic = "abort"` 和符号剥离，以减少 EXE 体积；这些设置不删除生命周期、安全检查或 UI 功能。

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
