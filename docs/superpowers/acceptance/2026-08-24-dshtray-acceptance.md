# DSHtray 验收记录

- 日期：2026-08-25 00:40（中国标准时间）
- 平台：Windows 11 主机（`tauri info` 报告 Windows 10.0.26200 build）
- 提交：包含 `5a76bef`、`2278360`、`0429a8b`、`0c7ab7a` 及本次无黑框/托盘图标修复

## 自动化结果

| 检查 | 结果 | 实际命令/证据 |
|---|---|---|
| Rust 格式 | 通过 | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` |
| Rust Clippy | 通过 | `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` |
| Rust 测试 | 通过 | `--features test-fixture --all-targets`：13 个 lib 单元测试 + 33 个集成/平台测试，全部通过；新增无黑框和蓝/黑图标测试 |
| 前端类型 | 通过 | `pnpm typecheck` |
| 前端 lint | 通过 | `pnpm lint` |
| 前端测试 | 通过 | `pnpm test`：3 个 Vitest 测试通过 |
| 前端构建 | 通过 | `pnpm build`，Vite 38 modules transformed |
| Tauri 环境 | 通过 | WebView2 151.0.4129.101、Visual Studio Build Tools 2022、Rust 1.98.0、Node 24.19.0、pnpm 11.21.0 |

Rust 测试覆盖：配置损坏恢复/camelCase、代理继承策略、源码/打包 argv、无代理健康检查、listener PID 查询、Job Object 归属、正常退出/5 秒强制、外部观察/接管、状态同步、首次启动 flag、日志脱敏、诊断和发现。

## 交付物

| 产物 | 路径 | 大小 | SHA-256 |
|---|---|---:|---|
| 绿色版 | `artifacts/portable/DSHtray/DSHtray.exe` | 14,014,464 bytes | `0743c141c2106bc1fd81c95f963195be42462e579f3c0967ddee68a296a0b5dc` |
| NSIS 安装器 | `src-tauri/target/release/bundle/nsis/DSHtray_0.1.0_x64-setup.exe` | 3,377,228 bytes | `ec0428000eadca976d86ea7deeafba10ce6e8c0835f6d6eb7569a4bd15b62e58` |

`pnpm package:portable` 和 `pnpm package:verify` 均实际执行成功。当前产物签名状态为 `NotSigned`，因为没有配置代码签名证书；正式发布前需要组织签名流程。

## 只读外部回归

现有 DSH Web 服务未被本项目操作：

```text
GET http://127.0.0.1:3080/ -> HTTP 200
LISTENING PID -> 21420
```

## 未在本轮执行的手工项

为避免影响用户当前正在运行的 DSH 服务，本轮没有通过新管理器 GUI 启动/停止真实源码目标或 `DSH.exe`，也没有切换当前真实代理。真实目标的首次向导、托盘视觉操作、安装后注销/登录和升级/卸载仍需在用户允许中断现有 PID 21420 的维护窗口中执行。

这不影响已交付的自动化安全边界和构建验证，但不能把 fake fixture 测试描述成真实 DSH smoke test。
