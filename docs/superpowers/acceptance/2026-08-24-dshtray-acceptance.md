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

## 2026-08-26 09:44 回归验收

- 平台：Windows 11，`10.0.26200` build 26200；Rust `1.98.0`、Node `v24.19.0`、pnpm `11.21.0`、WebView2 `151.0.4129.101`。
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`：通过。
- `cargo test --manifest-path src-tauri/Cargo.toml --features test-fixture --all-targets`：通过，13 个 lib 单元测试、33 个集成/平台测试通过。
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`：通过。
- `pnpm typecheck`、`pnpm lint`、`pnpm test`、`pnpm build`：全部通过；Vitest 3 个测试通过，Vite 转换 38 个 modules。
- `pnpm tauri build --no-bundle --ci`、`pnpm tauri build --ci --bundles nsis --no-sign`、`pnpm package:portable`、`pnpm package:verify`：全部通过。
- 本轮最终产物：
  - 绿色版 `artifacts/portable/DSHtray/DSHtray.exe`：14,014,464 bytes，SHA-256 `97dddc1e3541e14bbfb6535f108dc347702d87380dacce0f4170a03448d6c024`。
  - NSIS `src-tauri/target/release/bundle/nsis/DSHtray_0.1.0_x64-setup.exe`：3,378,067 bytes，SHA-256 `e4d4f4f00eff86abbfe826fdf8ac66ecfc2e56d49e0eda7d667e0d88300d441c`。
  - 签名状态：`NotSigned`；发布校验通过。

### 源码目标真实启动

- 在 `C:\Users\Tony\Documents\Default Project\deepseek-harness` 执行 `pnpm dsh web`，真实服务返回 `HTTP 200`。
- listener：`127.0.0.1:3080`，PID `23840`，可执行文件 `C:\Program Files\nodejs\node.exe`。
- listener 命令行：`node --import tsx/esm apps/cli/src/bin.ts "web"`；父链为 `pnpm → node(pnpm.mjs) → cmd → node(listener)`。
- 已安装管理器 PID `15860` 对该服务显示“端口冲突”、PID/归属为 `—`，因此源码目标的**管理启动/接管/重启验收失败**。根因是 `src-tauri/src/lifecycle.rs:340-355` 的 `identity_matches()` 要求 listener 命令行同时包含完整工作目录、`dsh` 和 `web`；真实 listener 命令行不包含完整工作目录和 `dsh`。

### 打包目标真实启动

- `C:\Users\Tony\Documents\Default Project\deepseek-harness\DSH.exe`：15,533,804 bytes，SHA-256 `3009dcd10fcccccccb5223a1599a0a0f4df3a69c7f664919ac7059101c7fc437`。
- 直接启动后返回 `HTTP 200`；顶层 `DSH.exe` PID `12200`/`29712`，listener PID `15484` 为 `node.exe`。
- 父链为 `DSH.exe → cmd → pnpm/node → cmd → node(listener)`；因此打包目标直接启动通过，但管理器的 listener 身份接管验收同样被上述识别缺陷阻塞。

### 安全验收

- 独立 Python listener PID `29432` 占用 `127.0.0.1:3080` 时，管理器保持 `port-conflict`，未终止未知 PID；listener 在验收前后均存活，随后仅按精确 PID 清理。
- Rust 外部观察/未知 listener/Job Object/5 秒等待测试全部通过。
- 对 93 个 Git 跟踪文件扫描 `sk-*`、Bearer 值和常见凭据赋值，没有发现凭据值；README 保留安全约束：**“不保存任何 API 密钥、令牌、密码或其他凭据值。”**
- `git diff --check`：通过。

### 本轮最终状态

- 本轮启动的源码目标、打包目标和安全 listener 均已清理；`127.0.0.1:3080` 无 `LISTENING`，`DSH.exe` 无残留。
- 已安装管理器 PID `15860` 仍运行。
- 本轮验收结果：自动化、直接启动和安全边界通过；源码/打包目标的管理归属识别失败，**因此版本不能标记为全绿可交付**。

## 2026-08-27 listener 归属修复回归

- 修复 `src-tauri/src/process/inspect.rs`：读取进程当前工作目录并保留可读的部分父链；listener 自身读取失败仍 fail-closed。
- 修复 `src-tauri/src/lifecycle.rs`：源码目标按源码目录 + `pnpm dsh web`/`apps/cli/src/bin.ts web` 父链识别，打包目标沿父链匹配精确 `DSH.exe`；区分 listener PID 与可接管根 PID；外部接管以根→子顺序加入 Job Object；受管理停止后等待 3080 listener 真正释放。
- 修复 `src/App.tsx`：生命周期动作失败后重新读取后端快照，避免界面保留过期“运行中”。
- 回归测试：`cargo fmt`、`cargo test --features test-fixture --all-targets`（各 test target 合计 54 个 Rust 测试通过、0 失败）、Clippy `-D warnings`、`pnpm typecheck`、`pnpm lint`、`pnpm test`（4 个 Vitest 测试）、`pnpm build` 全部通过。
- 最终构建：`pnpm tauri build --no-bundle --ci`、NSIS、`pnpm package:portable`、`pnpm package:verify` 全部通过。

### 已安装程序真实 managed 回归

- 安装路径：`C:\Users\Tony\AppData\Local\DSHtray\dshtray.exe`。
- 源码模式：真实 `pnpm dsh web` 返回 HTTP 200；管理器显示 `管理器负责`，停止后 3080 无 `LISTENING`，重启产生新根 PID 且 HTTP 仍为 200。
- 打包模式：真实 `DSH.exe` 返回 HTTP 200；父链为 `DSH.exe → DSH.exe → cmd/pnpm/node → listener`；管理器显示 `DSH.exe / 管理器负责`，停止和重启均通过，重启产生新根 PID且 HTTP 仍为 200。
- 安全边界：未知 listener 未执行停止或终止；`PortConflict` 重启不再进入 `not_running`，会保持端口冲突语义。

### 已安装程序真实 external 观察回归

- 在管理器启动前独立启动真实源码 `pnpm dsh web`，listener PID `29984`；最新安装包启动后显示 `检测到外部 DSH`、`外部观察` 和 `确认接管`，不再显示 `not_running` 或把合法 DSH 判为未知端口。
- 点击 `确认接管` 时，本机 Hermes/终端测试环境返回 `job_assign_failed`：该环境会把所有测试启动的 Windows 进程预先放进另一个 Job Object，Windows 拒绝跨 Job 接管。进程未被管理器终止；随后仅按已知测试根 PID 清理。
- 因此“外部观察识别”有真实 UI 证据，“外部确认接管/停止”以父链、根 PID、Job 顺序的 fixture 回归覆盖，但不能把当前测试环境的 Job Object 拒绝伪报为真实 external 接管通过。普通用户从未被其他 Job 接管的外部 DSH 可按确认按钮进入接管路径。

### 最终清理

- 已恢复用户配置 `activeTarget=source`、`startDshOnLogin=false`；配置中未保存凭据值。
- 已删除临时外部启动脚本、日志、桌面快捷方式和计划任务；当前 3080 无 `LISTENING`、无 `DSH.exe` 残留；最终安装路径管理器保持运行。
- 最终产物哈希以本次构建实际值为准：
  - release/绿色版：`F614A8F311594EF5F0857C0AEC5A674391910E4262F4957EFC4C71023E9D8E00`（13,969,408 bytes）。
  - NSIS：`DC83EA49CDEAE62EFD7F1F42707FC36D22E2567E05E507E1E776BD61BD85197D`（3,373,718 bytes）。
- tracked 文件凭据扫描未发现未脱敏凭据值；日志脱敏测试值采用运行时拼接，未保留 token-like 字面量。
- `git diff --check`：通过。

## 2026-08-27 托盘图标极细白描边回归

- 构建脚本新增 `scripts/tray-icon-outline.mjs`：蓝鲸和黑鲸的 SVG 路径统一增加 `#FFFFFF` 描边，宽度由 `0.35` 调整为 `0.60` SVG `viewBox` 单位（32×32 栅格化后约 `0.8` 像素），并使用 `paint-order="stroke fill"` 尽量将描边留在主体外侧。
- `pnpm fetch:tray-icons` 成功重新生成 `src-tauri/icons/tray-deepseek-blue.png` 和 `src-tauri/icons/tray-deepseek-black.png`；运行时仍只读取内置 PNG，不联网。
- 描边转换测试先按 TDD 验证 RED（模块不存在时失败），实现后 `node --test scripts/tray-icon-outline.node-test.mjs`：1 passed、0 failed。
- 像素校验：两个图标均为合法 32×32 RGBA PNG；蓝鲸和黑鲸均检测到 69 个白色像素，其中 66 个位于透明背景边界，说明白边实际落在鲸鱼外缘而非仅存在于测试文本中。
- Rust gates：`cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`、`cargo test --manifest-path src-tauri/Cargo.toml --features test-fixture --all-targets`（54 passed、0 failed）、Clippy `-D warnings` 全部通过。
- 前端 gates：`pnpm typecheck`、`pnpm lint`、`pnpm test`、`pnpm build` 全部通过；Vitest 4 passed，图标 Node 测试 1 passed。
- 发布 gates：Tauri 无 bundle、NSIS、绿色版和 `pnpm package:verify` 全部通过；签名状态仍为 `NotSigned`。
- 本次产物：
  - release/绿色版：`artifacts/portable/DSHtray/DSHtray.exe`，13,969,408 bytes，SHA-256 `61db01bfb423645c0c079363095757834a63ef537f4261168422cae26d373bb1`。
  - NSIS：`src-tauri/target/release/bundle/nsis/DSHtray_0.1.0_x64-setup.exe`，3,376,157 bytes，SHA-256 `d1902dc8f701671292c740766583d0d3cd35dfc4c9919c1e79494b1ad1d0e830`。
- NSIS 静默升级退出码为 `0`；安装版文件已更新，大小 13,969,408 bytes，SHA-256 `b47187fbd2545ddb39d0f37c9a53ffb078127e1b3780ce34ccac73d02d2222de`；当前运行实例路径为 `C:\Users\Tony\AppData\Local\DSHtray\dshtray.exe`、PID `19848`。安装升级未停止当前外部 DSH；管理器继续显示外部观察状态。

## 2026-08-27 14:05 托盘图标描边清晰度微调

- 根据视觉反馈，将蓝鲸和黑鲸白色描边从 `0.35` 调整为 `0.60` SVG `viewBox` 单位（32×32 下约 `0.8` 像素）；颜色、轮廓位置和 `paint-order="stroke fill"` 保持不变。
- 同源渲染对比确认描边覆盖度实际增加：蓝鲸白边 alpha 总量 `2406 → 5348`，黑鲸 `2268 → 4209`；视觉检查确认白边更清晰，仍未形成粗框或光晕。
- `node --test scripts/tray-icon-outline.node-test.mjs`：1 passed、0 failed；Rust 54 个测试、Vitest 4 个测试、类型检查、lint、Clippy、前端构建和 Tauri/NSIS/绿色版 gates 全部通过。
- 新产物：绿色版 `artifacts/portable/DSHtray/DSHtray.exe`，13,969,408 bytes，SHA-256 `167d2191e8ac17823ae385053174e254cac64b4f1f5ab0e12cf5631b4fab8a95`；NSIS `src-tauri/target/release/bundle/nsis/DSHtray_0.1.0_x64-setup.exe`，3,376,573 bytes，SHA-256 `e22f7315775df18d94c758f79a3922e32b381caea39df1ec3b1c17d8902a8a03`。
- NSIS 静默升级退出码为 `0`；安装版 SHA-256 为 `d3a0209c17269605fd1dddcd63ce5bf2ddec97996a8b8384eca76ef408ffbbf7`，当前运行 manager PID `15276`。当前外部 DSH listener PID `4872` 保持运行，未被图标升级中断。

## 2026-08-27 64×64 源图与 6 像素描边样式

- 根据确认的样图，正式构建管线改为输出 64×64 PNG；SVG 描边为 `2.25` `viewBox` 单位，即源图中约 6 像素白边。蓝鲸和黑鲸使用相同参数。
- 本次仅在确认样图后应用正式资源；正式 PNG 重新生成前不执行 DSH 停止或重启。

## 2026-08-27 64×64/6px 正式应用回归

- `pnpm fetch:tray-icons` 已按确认样图重新生成正式内置资源；两个文件均为 64×64 RGBA PNG，并与确认过的 64×64/6px 样图字节级一致。
- Node 回归测试：`node --test scripts/tray-icon-outline.node-test.mjs` 为 2 passed、0 failed；`pnpm test` 干净通过（Vitest 4 passed，Node 测试 2 passed）。
- Rust 54 个测试、Clippy、类型检查、lint、前端构建、Tauri release、绿色版和 NSIS 构建均通过；发布校验通过，签名状态为 `NotSigned`。
- 新产物：绿色版 `artifacts/portable/DSHtray/DSHtray.exe`，13,973,504 bytes，SHA-256 `76b6566cf2ba4e5c8abe6f4e74c7f39c57ef74ec8867ec179fba672aa37a0e21`；NSIS `src-tauri/target/release/bundle/nsis/DSHtray_0.1.0_x64-setup.exe`，3,377,855 bytes，SHA-256 `f8e9141536127cf9b9edc346f3bba8f40d13e7bb9a5674a9511f23093be467a2`。
- NSIS 静默升级退出码为 `0`；安装版大小 13,973,504 bytes，SHA-256 `d8b4de886789ae02ab00c6b198f4ed6409d0ac6c6add33485926aadca41771ba`；当前 manager PID `16572`，路径为 `C:\Users\Tony\AppData\Local\DSHtray\dshtray.exe`。
- 当前 DSH listener PID `4872` 在升级前后保持运行，未因图标资源更新而停止或重启；`git diff --check` 通过。

## 2026-08-27 外部 DSH Job Object 接管保护

- 根因确认：旧版本点击“确认接管”会直接逐个调用 `AssignProcessToJobObject`；从 Hermes/终端等受 Job Object 管理的环境启动的外部 DSH 可能已有不兼容的 Job 归属，因此返回 `job_assign_failed`。该类进程不能安全地并入新的管理器 Job。
- 修复：接管前重新读取 listener 和父链，逐一预检已确认 PID 的 Job 归属；已有其他 Job 时返回 `external_job_conflict`，保持 `external` 状态和原进程运行，不执行递归强杀。
- 修复：管理器 Job 改用固定名称 `Local\\DeepSeekHarnessManager`。管理器重启后若进程仍在自己的 Job 中，会重新打开并复用该 Job；只有其他 Job 才拒绝接管。
- 修复：错误详情现在显示在界面中，包含冲突 PID 或 Windows 查询信息及处理建议。
- 回归覆盖：生命周期预检、listener 二次确认、管理器 Job 复用、其他 Job 拒绝、Windows Job 归属查询和前端错误详情显示。
- 本轮门禁：`cargo test --manifest-path src-tauri/Cargo.toml --features test-fixture --all-targets` 为 59 passed、0 failed；`cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`、`cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`、`pnpm typecheck`、`pnpm lint`、`pnpm test` 和 `pnpm build` 均通过。
- 本轮产物：release/绿色版 `dshtray.exe`，14,003,712 bytes，SHA-256 `72cb3804e2f62f34469576864f247d59614cb6b7a8cda70616201b2e1cb531dc`；NSIS `DSHtray_0.1.0_x64-setup.exe`，3,382,646 bytes，SHA-256 `5dd99621766c2573f8f2a98e4a6b7d6856872d606fc71b92a93ce80f7b5d7132`；签名状态 `NotSigned`。

## 2026-08-27 直接 PID 树控制与发布瘦身

- 用户明确批准外部 Job 冲突时的精确 PID 树控制：不尝试跨 Job 加入；接管时保存并复核 PID、可执行路径、命令行、父 PID 和工作目录；正常退出等待 5 秒后，仅按子到父顺序调用 `TerminateProcess`，不使用递归 `taskkill /T`。
- 管理器启动的 DSH 仍使用命名 Job Object；只有已确认外部 DSH 的冲突路径使用 direct PID control。PID 被复用、进程仍存在但无法重新确认、或存在性查询失败时，直接拒绝终止。
- 图标生成脚本已将 `padSvgViewBox` 接入正式资源。蓝鲸和黑鲸均为 64×64 RGBA PNG，alpha bbox 均为 `(4,10)-(59,53)`，四条画布边缘 `edge_alpha_max=0`。
- 本轮门禁：Rust 全目标测试 61 passed、0 failed；`cargo fmt --check`、Clippy、`pnpm typecheck`、`pnpm lint`、`pnpm test` 和 `pnpm build` 均通过；Windows fixture 验证了已有其他 Job 的 direct adoption、root descendant 刷新和精确终止。
- release 配置使用 `opt-level="z"`、`lto="thin"`、`codegen-units=1`、`panic="abort"`、`strip="symbols"`。release/绿色版 `dshtray.exe` 为 5,355,008 bytes，SHA-256 `6b9135d97e710f798b5ec432573dc444576dc113acdfc691bfac76c3177ff105`；NSIS 为 1,915,304 bytes，SHA-256 `1051b8314f155424ba57dc7e276129445003e856dce54763fa31080f7002c866`；release 与绿色版字节一致；签名状态 `NotSigned`。
- 用户报告的旧 PID `30528` 在本轮只读核查时已不存在，因此未执行真实用户进程接管或强制终止。

## 2026-08-27 外部接管安全加固回归

- direct PID 控制改为两阶段流程：强制终止前为当前 root descendant 树完整复核 PID、可执行路径、命令行、父 PID 和工作目录，并持有对应 `PROCESS_TERMINATE` handles；任一身份漂移、权限错误或 ToolHelp 查询失败都会在任何终止前返回错误。
- direct 控制只保留当前 root 的 descendant 树；已脱离 root 的旧 PID 不再追加或终止。root 消失时不再追杀孤儿进程。
- ToolHelp `Process32FirstW`/`Process32NextW` 仅将 `ERROR_NO_MORE_FILES` 视为正常结束，其余错误传播为 `process_snapshot_failed`，避免把查询故障误判为进程已退出。
- 停止阶段的强制终止失败和 listener 超时都会恢复为 `Failed`，保留控制句柄供用户重试，不再遗留不可恢复的 `Stopping` 状态。
- Windows 集成测试的临时 Job 名改为 PID + 原子序号，避免并行测试共享 Job Object；测试遗留的 fixture 进程已核验为零。
- 日志脱敏测试改为运行时拼接 token-like fixture，实际验证敏感值不会出现在输出中。
- 本轮门禁：Rust 全目标测试 **66 passed、0 failed**；`cargo fmt --check`、Clippy、`pnpm typecheck`、`pnpm lint`、`pnpm test`（Vitest 5 + Node 4）和 `pnpm build` 均通过；Tauri 无 bundle、NSIS、绿色版及 `package:verify` 均通过。
- 最终 release/绿色版 `dshtray.exe`：5,355,520 bytes，SHA-256 `ed410fd115c238f01caa1e205e9be081ba92eddac78c63e7f72c5f5fa0180391`；NSIS `DSHtray_0.1.0_x64-setup.exe`：1,914,992 bytes，SHA-256 `4000bb45b502ba4a9ee90e8619744ce71e8dcdf54d6cfa25cc4d5856391260c9`；绿色版与 release EXE 字节一致；签名状态 `NotSigned`。

## 2026-08-28 托盘运行状态颜色

- 托盘图标改为从完整 `RuntimeSnapshot` 选择：`Stopped`、无可运行 DSH 的 `PortConflict`/启动失败使用红色鲸鱼；`LifecycleState::External + Ownership::External` 的未确认外部 DSH 使用黄色鲸鱼；管理器负责或已确认接管的 DSH 继续按代理关闭/开启使用蓝色/黑色鲸鱼。
- 红色和黄色状态优先于代理开关；托盘初始化、启动、停止、重启、接管、代理变更和设置变更后均同步图标与 tooltip；生命周期失败路径也按最新快照同步，避免外部 DSH 重新识别后颜色滞留。
- 正式新增 `src-tauri/icons/tray-deepseek-red.png` 与 `tray-deepseek-yellow.png`，生成脚本固定使用 `#DC2626` 和 `#EAB308`；四张图均为 64×64 RGBA，非透明 bbox 均为 `(4,10)-(59,53)`，四边 `edge_alpha_max=0`。
- TDD 覆盖红色停止状态、黄色待接管状态、蓝/黑代理状态及四张内置 PNG；Node 图标测试为 **4 passed、0 failed**。
- 为避免测试争用用户正在使用的 `Local\\DeepSeekHarnessManager`，Windows “复用管理器 Job”集成测试改用独立的测试专属命名 Job；生产默认 Job 名未改变。
- 本轮门禁：Rust 全目标串行测试 **66 passed、0 failed**；`cargo fmt --check`、Clippy `-D warnings`、`pnpm typecheck`、`pnpm lint`、Vitest **5 passed**、`pnpm build`、Tauri 无 bundle、NSIS、绿色版和 `package:verify` 均通过。
- 最终 release/绿色版 `dshtray.exe`：5,360,640 bytes，SHA-256 `9a9ae393bc55ccb77bab4861c2f7f9c80df752eb2adae7d248ab20ff342f4a3a`；NSIS `DSHtray_0.1.0_x64-setup.exe`：1,920,513 bytes，SHA-256 `525c1660d28a09887efd2ff8cdcbab741bb74578e4276954b78cfbfb1f3f8034`；绿色版与 release EXE 字节一致；签名状态 `NotSigned`。
- 相比旧 release EXE `14,003,712` bytes，当前 EXE 减少 `61.72%`；相比旧 NSIS `3,382,646` bytes，当前安装器减少 `43.22%`。
- 当前 `C:\Users\Tony\AppData\Local\DSHtray\dshtray.exe` 仍是用户正在运行的旧安装版，本轮未覆盖；当前真实 DSH listener 也未操作。

## 2026-08-28 托盘单击/双击行为调整

- 根据最新交互要求，托盘左键 `Click::Up` 在 500ms 双击判定窗口结束后调用 `open_dsh_url_with_app`，使用 `tauri-plugin-opener` 的 `open_url(url, None)` 通过默认浏览器打开当前配置的 DSH 页面。
- Windows 左键 `TrayIconEvent::DoubleClick` 会递增取消令牌、取消待执行的单击打开动作，并立即显示 DSHtray 管理器；双击序列尾部的 `Click::Up` 会被抑制，不会再次打开浏览器。
- 左键菜单自动弹出保持关闭，右键托盘菜单保持启用；右键/中键双击、鼠标按下事件不会触发上述两个动作。
- 事件分流测试已更新为：左键单击打开 DSH 页面、左键双击打开管理器、右键双击忽略、按下事件忽略；聚焦测试 **10 passed、0 failed**。
- 最新 release/绿色版 `dshtray.exe`：5,362,688 bytes，SHA-256 `6b1176240740d62bfe6ffacce223310c3370bc1029ab40063df2fe5b3995e105`；NSIS `DSHtray_0.1.0_x64-setup.exe`：1,920,143 bytes，SHA-256 `ab6bc7602d2d4e92a9b3db32b1503b8d7f6478bf30d70fc905e8231d16f87f00`；绿色版与 release EXE 字节一致；签名状态 `NotSigned`。
- 本轮前端质量门禁、Rust 全目标测试、fmt、Clippy、Tauri release、NSIS、绿色版和 `package:verify` 均通过。

## 2026-08-28 真实 Windows 源码 listener 识别修复

- 真实只读复现：当前 `127.0.0.1:3080` listener PID `30184` 为 `node.exe`，命令行包含 `C:\\Users\\Tony\\Documents\\Default Project\\deepseek-harness\\apps\\cli\\src\\bin.ts web`；进程工作目录为源码目录下的 `apps\\cli\\src`，父 PID `29032` 已不存在。
- 根因确认：Windows 命令行使用反斜杠，而原入口标记只匹配 `/`；同时 listener 自身工作目录是已配置源码目录的子目录，原逻辑要求完全相等。父链已退出时，两项差异共同导致合法 DSH 被错误归类为 `PortConflict`。
- 修复范围：命令行标记匹配前只规范化 `\\`/`/`；listener 入口进程的工作目录允许配置源码目录本身或带路径边界的子目录；`pnpm dsh web` launcher 仍要求工作目录精确等于配置根目录。端口占用、入口标记、`web` 参数和外部默认只观察/确认接管边界均未放宽。
- TDD 回归：新增 Windows 反斜杠入口 + 嵌套工作目录测试；修复前失败为 `PortConflict`，修复后 `1 passed、0 failed`。
- 真实 smoke probe 使用当前配置和真实 `WindowsProcessAdapter` 只读调用 `refresh_external_state()`，结果为 `LifecycleState::External`、`Ownership::External`、PID `30184`；未执行接管、停止、终止或配置写入。
- 修复后发布门禁：`pnpm tauri build --no-bundle --ci`、`pnpm tauri build --ci --bundles nsis --no-sign`、`pnpm package:portable` 和 `pnpm package:verify` 均通过；安装器签名状态为 `NotSigned`。
- 最终 release/绿色版 `dshtray.exe` 均为 `5,362,688 bytes`，SHA-256 `57856ab2d1141566c0fc49fa220cec0485054aac59c79a4d36270a155fd8f92c`；NSIS `DSHtray_0.1.0_x64-setup.exe` 为 `1,920,630 bytes`，SHA-256 `36908c3f9fb77988f0b52ca7a89b03b8670e5fb5ff80ea1ed308a236fa1cd985`；绿色版与 release 字节一致。
- 当前运行中的安装版仍为旧实例：PID `35500`，路径 `C:\\Users\\Tony\\AppData\\Local\\DSHtray\\dshtray.exe`，SHA-256 `B6AFE9BB7B2AA3F1B765CC91D26EBB2F010E4FCFD25460EBEA9032152A43305E`；本轮未覆盖或退出该实例，也未操作 PID `30184` 的真实 DSH。

- 因当前仍有旧安装版 `dshtray.exe` 及真实 DSH listener 运行，本轮没有强制退出、覆盖旧实例或执行真实托盘点击 smoke test。