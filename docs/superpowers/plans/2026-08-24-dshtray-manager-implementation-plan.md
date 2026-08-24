# DSHtray Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` or `superpowers:executing-plans` to implement this plan task-by-task. Every task ends with its own test or verification gate.

**Goal:** 在 Windows 11 上构建一个独立的 Rust/Tauri DSHtray 托盘应用，安全管理 DeepSeek Harness 的单实例启停、源码/打包双启动目标、代理开关、外部进程观察、日志诊断以及绿色版和 Windows 安装器交付。

**Architecture:** Tauri 2.x 负责 Windows 桌面壳、托盘、单实例和开机启动；Rust 后端持有唯一运行状态，按配置启动 `pnpm dsh web` 或 `DSH.exe`。进程管理使用 Windows Job Object 记录进程归属，健康检查独立绕过代理，React + TypeScript 前端只通过 Tauri commands/events 操作后端。

**Tech Stack:** Rust MSVC toolchain、Tauri 2.x、React + TypeScript + Vite、pnpm、Serde/JSON、Windows APIs、Tauri single-instance/autostart/dialog/opener/notification plugins、Vitest + Testing Library、NSIS installer。

**Spec:** `docs/superpowers/specs/2026-08-24-dshtray-manager-design.md`

## 执行前决策门槛

本计划按以下推荐值编写；开始 Task 1 以前需要用户确认这些实现默认值，或者先修改本计划：

- Tauri：使用当前稳定的 Tauri 2.x，不跨到 Tauri 3。
- 前端：React + TypeScript + Vite，不引入大型 UI 组件库。
- 安装器：Tauri 内置 NSIS，另提供可直接复制的绿色版目录。
- 应用标识：`com.deepseek.dshtray`，产品名 `DSHtray`。
- 当前已发现的首选目标：`C:\Users\Tony\Documents\Default Project\deepseek-harness` 和其下的 `DSH.exe`；仍须在首次向导中由用户确认。

## Global Constraints

- 目标平台为 Windows 11 x64；生产构建使用 `x86_64-pc-windows-msvc`。
- Rust 工具链必须是 MSVC toolchain；Tauri 插件文档要求 Rust 至少 `1.77.2`，实际实现以当时锁定的 Tauri 2.x 依赖要求为准。
- 前端使用 pnpm；当前机器检测到 Node `v24.19.0`、pnpm `11.21.0`，新项目需要写入 lockfile，不能依赖全局未锁定版本。
- 当前环境尚未找到 `rustc` 和 `cargo`；在任何 Rust 文件创建前必须先完成 Rust MSVC、Visual Studio C++ Build Tools、Windows SDK 和 WebView2 Runtime 的前置检查。
- 源码目标的用户可见启动语义必须保持为 `pnpm dsh web`，工作目录为用户确认的 DSH 仓库；不可把它改写成对 DSH 源码的直接内部调用。
- 打包目标直接执行用户确认的 `DSH.exe`，默认工作目录为该文件所在目录，默认参数为空。
- 单实例约束同时适用于 DSHtray 管理器和 DSH 目标：同一时间只允许一个管理器控制一个目标。
- 服务地址默认 `127.0.0.1:3080`；第一版只允许 `127.0.0.1` 或 `localhost`，高级设置只允许修改端口，不允许暴露到局域网网卡。
- 代理开启时仅向新启动的 DSH 子进程注入 `HTTP_PROXY`、`HTTPS_PROXY`、`NODE_USE_ENV_PROXY=1`；保留继承的 `NO_PROXY`，不主动设置 `ALL_PROXY`，不修改 Windows 全局代理。
- 代理关闭时管理器不主动写入或删除父进程已有的代理环境变量；设置页必须明确显示“管理器不主动注入代理”。
- 运行中切换代理必须先确认重启；取消不得改变持久化开关或重启 DSH。
- 管理器开机启动默认启用，`startDshOnLogin` 默认关闭；管理器登录启动不得自动启动 DSH。
- 停止必须先执行尽力的正常退出请求，等待 5 秒后才允许强制结束，并且强制结束范围只能是已归属 Job Object 或用户明确接管的进程树。
- 外部 DSH 默认只观察；不能因为端口相同就停止未知 PID，接管必须经过路径、命令行、进程关系和端口 PID 的再次验证。
- 管理器退出不停止 DSH；Job Object 不能配置 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`。
- 配置和日志写入 `%LOCALAPPDATA%\DeepSeekHarnessManager`；不复用旧 `tray.ps1` 配置，不保存 API key、Token 或 DSH 凭据。
- 新项目不得修改 `C:\Users\Tony\Documents\Default Project\deepseek-harness` 的源码、配置或启动项；它只作为被管理目标和验收对象。

## 当前代码库与外部依赖事实

- `C:\Users\Tony\Documents\BaiduSyncdisk\DSH\DSHtray` 当前只有已批准的设计文档，没有 Rust/Tauri 工程，也不是 Git 仓库。
- DSH 源码仓库的根 `package.json` 使用 pnpm workspace，声明 `packageManager: pnpm@11.7.0`，当前实际脚本为：`"dsh": "node --import tsx/esm apps/cli/src/bin.ts"`。
- DSH README 的源码启动流程为 `pnpm install`、`pnpm run build`、`pnpm dsh web`，默认 Web 地址是 `http://127.0.0.1:3080`。
- 已发现 `C:\Users\Tony\Documents\Default Project\deepseek-harness\DSH.exe`，路径发现结果需要进入首次向导供用户确认，而不是静默写入配置。
- Tauri 官方文档确认 Tauri 2 支持 React/TypeScript 模板、Windows NSIS/MSI 打包，以及 `single-instance` 和 `autostart` 插件；实现时以官方 v2 文档和生成的依赖锁文件为准。

## 目录与文件地图

实现完成后项目预计采用以下边界；每个 Rust 模块只负责一个领域：

```text
DSHtray/
├─ package.json
├─ pnpm-lock.yaml
├─ tsconfig.json
├─ vite.config.ts
├─ index.html
├─ .gitignore
├─ README.md
├─ src/
│  ├─ main.tsx
│  ├─ App.tsx
│  ├─ types.ts
│  ├─ tauri.ts
│  ├─ state.ts
│  ├─ styles.css
│  ├─ components/
│  │  ├─ StatusCard.tsx
│  │  ├─ ActionBar.tsx
│  │  ├─ TargetSelector.tsx
│  │  ├─ ProxySettings.tsx
│  │  ├─ SettingsPanel.tsx
│  │  ├─ DiagnosticsPanel.tsx
│  │  ├─ FirstRunWizard.tsx
│  │  ├─ ConfirmRestartDialog.tsx
│  │  └─ ExternalDshBanner.tsx
│  └─ test/
│     ├─ setup.ts
│     ├─ App.test.tsx
│     ├─ ProxySettings.test.tsx
│     └─ FirstRunWizard.test.tsx
├─ src-tauri/
│  ├─ Cargo.toml
│  ├─ tauri.conf.json
│  ├─ capabilities/default.json
│  ├─ icons/
│  ├─ src/
│  │  ├─ main.rs
│  │  ├─ lib.rs
│  │  ├─ app_error.rs
│  │  ├─ app_state.rs
│  │  ├─ commands.rs
│  │  ├─ events.rs
│  │  ├─ tray.rs
│  │  ├─ config.rs
│  │  ├─ domain.rs
│  │  ├─ discovery.rs
│  │  ├─ health.rs
│  │  ├─ logging.rs
│  │  ├─ diagnostics.rs
│  │  ├─ proxy.rs
│  │  ├─ startup.rs
│  │  ├─ lifecycle.rs
│  │  └─ process/
│  │     ├─ mod.rs
│  │     ├─ job.rs
│  │     ├─ inspect.rs
│  │     └─ graceful_stop.rs
│  ├─ tests/
│  │  ├─ config_store.rs
│  │  ├─ lifecycle.rs
│  │  ├─ process_fixture.rs
│  │  └─ windows_process_integration.rs
│  └─ fixtures/
│     └─ dsh-test-fixture.rs
├─ scripts/
│  ├─ package-portable.ps1
│  └─ verify-release.ps1
└─ docs/
   └─ superpowers/
      ├─ specs/2026-08-24-dshtray-manager-design.md
      └─ plans/2026-08-24-dshtray-manager-implementation-plan.md
```

---

### Task 1: 开发环境门槛、Git 基线和 Tauri 工程脚手架

**Objective:** 建立可编译的 Tauri 2 + React/TypeScript 基线，但不连接 DSH 生命周期。

**Files:**
- Create: `package.json`, `pnpm-lock.yaml`, `tsconfig.json`, `vite.config.ts`, `index.html`
- Create: `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`, `src-tauri/capabilities/default.json`
- Create: `.gitignore`, `README.md`
- Preserve: `docs/superpowers/specs/2026-08-24-dshtray-manager-design.md`

**Interfaces:**
- Produces the Tauri application shell and `pnpm tauri dev` command used by every later task.
- Does not yet expose DSH process commands.

- [ ] **Step 1: 验证 Windows 构建前置条件**

执行：

```text
where.exe rustc cargo rustup node pnpm
rustc --version
cargo --version
node --version
pnpm --version
```

预期：`rustc`、`cargo`、`rustup`、Node、pnpm 都能解析；当前检查中 Rust 三项缺失，因此在实现前必须先安装 Rust MSVC toolchain、Visual Studio C++ Build Tools、Windows SDK，并重新执行同一组命令。若任一命令失败，不创建 Rust 源文件。

- [ ] **Step 2: 初始化工程模板**

在项目根目录执行官方 Tauri 2 脚手架：

```text
pnpm create tauri-app@latest . --template react-ts
pnpm install
```

选择或确认：pnpm、React、TypeScript、应用标识 `com.deepseek.dshtray`、产品名 `DSHtray`。若脚手架检测到已有 `docs` 目录，必须保留该目录及设计文档。

- [ ] **Step 3: 固定基础脚本与应用元数据**

`package.json` 至少提供：

```json
{
  "name": "dshtray",
  "private": true,
  "packageManager": "pnpm@11.21.0",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "build:desktop": "pnpm tauri build",
    "build:portable": "pnpm tauri build --no-bundle",
    "tauri": "tauri",
    "typecheck": "tsc --noEmit",
    "test": "vitest run",
    "test:watch": "vitest",
    "lint": "eslint ."
  }
}
```

版本号由脚手架实际生成并写入 lockfile；不要手工删除脚手架的 Tauri 依赖。

- [ ] **Step 4: 设置 Tauri 窗口和能力清单**

`src-tauri/tauri.conf.json` 设置产品名、标识、开发 URL、前端构建路径和 Windows NSIS bundle 目标；`src-tauri/capabilities/default.json` 只授予后续实际使用的窗口、dialog、opener、notification、autostart 和 clipboard 权限，不使用全量权限。`Cargo.toml` 同时加入 `serde`、`serde_json`、`thiserror`、`url`、`reqwest`、`tokio`、`windows`，测试依赖加入 `tempfile`；每个依赖的 feature 只覆盖对应模块所需 API。

- [ ] **Step 5: 运行基线验证**

执行：

```text
pnpm typecheck
pnpm test
pnpm tauri build --no-bundle
```

预期：TypeScript 检查、空基线测试和 Tauri release 编译全部退出码为 0，并在 `src-tauri/target/release/` 产生 `dshtray.exe`。如果 Tauri CLI 参数与脚手架版本不同，先使用 `pnpm tauri build --help` 对齐命令，再把实际可用命令写回本计划对应章节。

- [ ] **Step 6: 建立 Git 基线**

由于当前目录不是 Git 仓库，在确认用户希望使用 Git 后执行：

```text
git init
git add .
git commit -m "chore: scaffold DSHtray Tauri app"
```

后续每个任务至少保留一个可独立回滚的提交。

---

### Task 2: 领域模型、配置默认值和原子持久化

**Objective:** 实现可测试的配置模型、运行状态模型和配置损坏恢复，不启动任何进程。

**Files:**
- Create: `src-tauri/src/domain.rs`
- Create: `src-tauri/src/config.rs`
- Create: `src-tauri/src/app_error.rs`
- Create: `src-tauri/tests/config_store.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- `domain.rs` exports `TargetId`, `TargetKind`, `AppConfig`, `ManagerConfig`, `TargetConfig`, `ServiceConfig`, `ProxyConfig`, `LifecycleState`, `Ownership`, `RuntimeSnapshot`.
- `config.rs` exports `ConfigStore::load(path) -> ConfigLoad`, `ConfigStore::save(path, &AppConfig)`, `AppConfig::defaults()`, `AppConfig::validate()` and `AppConfig::validate_active_target()`；`ConfigLoad` 包含 `config`、`recovered` 和可选 `backup_path`。
- `app_error.rs` exports serializable `AppError { code: String, message: String, details: Option<String> }` and conversions for I/O, JSON and validation errors.

- [ ] **Step 1: 写配置默认值测试**

在 `src-tauri/tests/config_store.rs` 写入：

```rust
#[test]
fn defaults_match_approved_product_decisions() {
    let config = AppConfig::defaults();
    assert!(config.manager.start_on_login);
    assert!(!config.manager.start_dsh_on_login);
    assert!(config.manager.close_to_tray);
    assert_eq!(config.active_target, TargetId::Source);
    assert_eq!(config.service.host, "127.0.0.1");
    assert_eq!(config.service.port, 3080);
    assert!(config.proxy.enabled);
    assert_eq!(config.proxy.url, "http://127.0.0.1:7897");
}
```

- [ ] **Step 2: 运行失败测试**

执行：

```text
cargo test --manifest-path src-tauri/Cargo.toml defaults_match_approved_product_decisions
```

预期：在模型尚未实现时因类型或方法不存在而失败。

- [ ] **Step 3: 实现 Serde 模型和校验**

使用 `#[serde(rename_all = "camelCase")]`，让 JSON 字段与设计文档一致。`AppConfig::validate()` 必须拒绝：

- service host 不是 `127.0.0.1` 或 `localhost`；
- service port 不在 `1..=65535`；
- proxy URL 不是 `http` 或 `https`，或缺少 host；
- 已填写的 source target working directory 不存在或不是目录；
- 已填写的 packaged target executable 不存在、不是文件或不是 `.exe`。

首次运行允许两个 target 路径都为空；`validate_active_target()` 在真正启动或切换到目标时拒绝当前目标为空，并返回 `target_not_configured`。

`LifecycleState` 至少包含 `Stopped`、`Starting`、`Running`、`External`、`Stopping`、`Failed`、`PortConflict`，并为状态提供 JSON 可序列化的稳定字段。

- [ ] **Step 4: 实现配置加载、原子保存和损坏备份**

`ConfigStore::save` 将 JSON 写入同目录临时文件、刷新文件内容，再以 Windows 可用的替换方式覆盖正式文件；`ConfigStore::load` 在 JSON 解析失败时把原文件改名为 `config.json.corrupt-<timestamp>`，返回默认配置和 `config-recovered` 诊断事件。测试路径必须可注入临时目录，不能直接写真实 `%LOCALAPPDATA%`。

- [ ] **Step 5: 补充恢复与校验测试**

测试至少覆盖：

```rust
#[test]
fn invalid_host_is_rejected() {
    let mut config = AppConfig::defaults();
    config.service.host = "0.0.0.0".into();
    let error = config.validate().expect_err("non-loopback host must fail");
    assert_eq!(error.code, "invalid_service_host");
}

#[test]
fn invalid_proxy_scheme_is_rejected() {
    let mut config = AppConfig::defaults();
    config.proxy.url = "socks5://127.0.0.1:7897".into();
    let error = config.validate().expect_err("socks5 is outside the MVP");
    assert_eq!(error.code, "invalid_proxy_url");
}

#[test]
fn empty_default_target_is_allowed_until_first_run_is_completed() {
    let config = AppConfig::defaults();
    config.validate().expect("empty targets are valid for first run");
    let error = config
        .validate_active_target()
        .expect_err("active source target still needs a path");
    assert_eq!(error.code, "target_not_configured");
}

#[test]
fn corrupt_config_is_backed_up_and_defaults_are_returned() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join("config.json");
    std::fs::write(&path, b"{not-json").expect("write corrupt config");
    let loaded = ConfigStore::load(&path).expect("corruption recovery");
    assert!(loaded.recovered);
    assert!(loaded.backup_path.as_ref().is_some_and(|backup| backup.exists()));
    assert_eq!(loaded.config.service.port, 3080);
}

#[test]
fn save_then_load_round_trips_camel_case_json() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join("config.json");
    let original = AppConfig::defaults();
    ConfigStore::save(&path, &original).expect("save config");
    let loaded = ConfigStore::load(&path).expect("load config");
    assert_eq!(loaded.config, original);
    let json = std::fs::read_to_string(path).expect("read config");
    assert!(json.contains("startDshOnLogin"));
}
```

- [ ] **Step 6: 运行通过验证**

执行：

```text
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --test config_store
```

预期：格式检查通过，配置测试全部通过。

---

### Task 3: 代理语义和源码/打包目标命令构建

**Objective:** 把代理开关和两个 DSH 启动适配器变成纯函数式、可单元测试的 Rust 逻辑。

**Files:**
- Create: `src-tauri/src/proxy.rs`
- Create: `src-tauri/src/targets.rs`
- Modify: `src-tauri/src/domain.rs`, `src-tauri/src/lib.rs`
- Test: `src-tauri/src/proxy.rs`, `src-tauri/src/targets.rs`

**Interfaces:**
- `proxy.rs` exports `validate_proxy_url(&str) -> Result<(), AppError>` and `build_child_environment(&ProxyConfig, parent: &[(OsString, OsString)]) -> Vec<(OsString, OsString)>`.
- `targets.rs` exports `TargetCommand { program: PathBuf, args: Vec<OsString>, working_directory: PathBuf }`, `build_source_command(&TargetConfig)`, `build_packaged_command(&TargetConfig)` and `resolve_pnpm_command()`.
- Test-only `enabled_proxy()`, `disabled_proxy()`, `value(env, name)`, `source_target(path)` and `packaged_target(path)` are defined in the module test section with concrete `ProxyConfig`/`TargetConfig` values.

- [ ] **Step 1: 写代理和命令构建测试**

```rust
#[test]
fn enabled_proxy_adds_only_approved_variables() {
    let parent = vec![(OsString::from("NO_PROXY"), OsString::from("127.0.0.1"))];
    let env = build_child_environment(&enabled_proxy(), &parent);
    assert_eq!(value(&env, "HTTP_PROXY"), Some("http://127.0.0.1:7897"));
    assert_eq!(value(&env, "HTTPS_PROXY"), Some("http://127.0.0.1:7897"));
    assert_eq!(value(&env, "NODE_USE_ENV_PROXY"), Some("1"));
    assert_eq!(value(&env, "NO_PROXY"), Some("127.0.0.1"));
    assert_eq!(value(&env, "ALL_PROXY"), None);
}

#[test]
fn disabled_proxy_does_not_add_or_remove_environment_values() {
    let parent = vec![(OsString::from("HTTP_PROXY"), OsString::from("inherited"))];
    assert_eq!(build_child_environment(&disabled_proxy(), &parent), parent);
}

#[test]
fn source_command_preserves_pnpm_dsh_web_argv() {
    let target = source_target(PathBuf::from(r"C:\deepseek-harness"));
    let command = build_source_command(&target).expect("valid source target");
    assert_eq!(command.args, vec![OsString::from("dsh"), OsString::from("web")]);
    assert_eq!(command.working_directory, PathBuf::from(r"C:\deepseek-harness"));
}
#[test]
fn packaged_command_uses_executable_directory_as_cwd() {
    let target = packaged_target(PathBuf::from(r"C:\DSH\DSH.exe"));
    let command = build_packaged_command(&target).expect("valid packaged target");
    assert_eq!(command.program, PathBuf::from(r"C:\DSH\DSH.exe"));
    assert_eq!(command.working_directory, PathBuf::from(r"C:\DSH"));
    assert!(command.args.is_empty());
}
```

- [ ] **Step 2: 运行失败测试**

执行：

```text
cargo test --manifest-path src-tauri/Cargo.toml proxy
cargo test --manifest-path src-tauri/Cargo.toml targets
```

预期：因函数尚未存在而失败。

- [ ] **Step 3: 实现代理环境构建**

代理开启时在传入父环境的副本上覆盖三个明确变量，保留其他变量，包括 `NO_PROXY`。代理关闭时直接返回父环境副本，不调用 `remove_var`，不新增任何变量。验证 URL 时只接受 `http`/`https` 且必须有 host；错误消息不得把完整 URL 中可能存在的用户信息写入日志。

- [ ] **Step 4: 实现 Windows pnpm 解析和目标命令**

Windows 下优先解析 `pnpm.cmd`，再解析 `pnpm.exe`；使用 PATH 查找，不写死 `C:\Users\Tony\AppData\Roaming\npm`。源码目标的 `TargetCommand` 必须是等价于：

```text
program: <resolved pnpm.cmd>
args: ["dsh", "web"]
working_directory: <confirmed source directory>
```

打包目标不得通过 shell 拼接命令，直接使用 `DSH.exe` 路径和用户配置参数。

- [ ] **Step 5: 运行通过验证**

执行：

```text
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml proxy
cargo test --manifest-path src-tauri/Cargo.toml targets
```

预期：代理变量、继承语义、源码命令和打包命令测试全部通过。

---

### Task 4: 本地健康检查和端口归属发现

**Objective:** 以不受系统代理影响的方式判断 DSH 是否就绪，并把 listener 映射到 PID。

**Files:**
- Create: `src-tauri/src/health.rs`
- Create: `src-tauri/src/network.rs`
- Modify: `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml`
- Test: `src-tauri/src/health.rs`, `src-tauri/src/network.rs`

**Interfaces:**
- `health.rs` exports `HealthChecker::with_proxy_disabled() -> HealthChecker` and `HealthChecker::check(&self, &ServiceConfig) -> HealthResult`；`HealthResult::{Ready, Unreachable, UnexpectedStatus}`。
- `network.rs` exports `find_listener(&ServiceConfig) -> Result<Option<ListenerOwner>, AppError>` and `ListenerOwner { pid: u32, local_address: String, port: u16 }`.
- Test-only `TestHttpServer::responding_with(status)`, `ServiceConfig::loopback(port)` and `unused_local_port()` are defined in the health test module; production code does not expose them.

- [ ] **Step 1: 写健康检查测试**

使用测试 HTTP server 验证：

```rust
#[tokio::test]
async fn health_check_accepts_2xx_without_using_environment_proxy() {
    let server = TestHttpServer::responding_with(204).await;
    let config = ServiceConfig::loopback(server.port());
    let checker = HealthChecker::with_proxy_disabled();
    let result = checker.check(&config).await;
    assert!(matches!(result, HealthResult::Ready { status: 204 }));
}

#[tokio::test]
async fn health_check_reports_unreachable_port() {
    let config = ServiceConfig::loopback(unused_local_port());
    let result = HealthChecker::with_proxy_disabled().check(&config).await;
    assert!(matches!(result, HealthResult::Unreachable { .. }));
}
```

- [ ] **Step 2: 实现无代理 HTTP client**

使用显式 `no_proxy` 的 client 和连接/请求超时；健康检查必须只访问配置快照中的 loopback host/port。接受所有 `2xx` 和 `3xx`；其他 HTTP 状态返回 `UnexpectedStatus`，连接拒绝和超时返回 `Unreachable`，并保留结构化错误码。

- [ ] **Step 3: 实现 Windows listener 查询**

在 `cfg(windows)` 模块中使用 `GetExtendedTcpTable` 查询 IPv4 TCP table，将 `127.0.0.1:<port>` 的 listener 映射到 PID。不能调用 `tasklist`、`netstat` 的文本输出作为正式协议。非 Windows 编译路径返回明确的 unsupported 错误，便于 Rust 单元测试编译。

- [ ] **Step 4: 运行验证**

执行：

```text
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml health
cargo test --manifest-path src-tauri/Cargo.toml network
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

预期：健康检查不会读取代理、listener 查询编译通过，Clippy 无 warning。

---

### Task 5: Windows 进程检查、Job Object 和尽力正常退出

**Objective:** 实现安全的进程归属和停止底层能力，确保管理器退出不会连带停止 DSH。

**Files:**
- Create: `src-tauri/src/process/mod.rs`
- Create: `src-tauri/src/process/job.rs`
- Create: `src-tauri/src/process/inspect.rs`
- Create: `src-tauri/src/process/graceful_stop.rs`
- Modify: `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`
- Test: `src-tauri/tests/process_fixture.rs`, `src-tauri/tests/windows_process_integration.rs`

**Interfaces:**
- `JobOwner::create_or_open(name)`, `JobOwner::assign(pid)`, `JobOwner::process_ids()`, `JobOwner::is_empty()`, `JobOwner::terminate()`, `JobOwner::close_without_termination()`。
- `ProcessInspector::inspect(pid) -> ProcessIdentity { pid, executable, command_line, parent_pid }`。
- `GracefulStop::request(process_group_id) -> GracefulStopResult`，只做尽力请求，不宣称已经退出。
- `OwnedProcessTree { root_pid, listener_pid, target_id, job_name, ownership }`。
- Test-only `FixtureProcess::spawn_parent_with_child()`, `parent_pid()`, `is_alive()`, `terminate_tree_for_test()` and `unique_job_name()` are defined in `windows_process_integration.rs` and always clean up their own fixtures.

- [ ] **Step 1: 编写 Job Object 行为测试场景**

`windows_process_integration.rs` 需要使用 `dsh-test-fixture` 启动一个父进程和子进程，验证以下可观察行为：

```rust
#[test]
#[cfg(windows)]
fn closing_manager_handle_does_not_kill_owned_fixture() {
    let fixture = FixtureProcess::spawn_parent_with_child();
    let job = JobOwner::create_or_open(unique_job_name()).expect("create job");
    job.assign(fixture.parent_pid()).expect("assign fixture");
    job.close_without_termination();
    assert!(fixture.is_alive());
    fixture.terminate_tree_for_test();
}

#[test]
#[cfg(windows)]
fn terminate_only_kills_processes_assigned_to_the_job() {
    let owned = FixtureProcess::spawn_parent_with_child();
    let unrelated = FixtureProcess::spawn_parent_with_child();
    let job = JobOwner::create_or_open(unique_job_name()).expect("create job");
    job.assign(owned.parent_pid()).expect("assign owned parent");
    job.terminate().expect("terminate owned job");
    assert!(!owned.is_alive());
    assert!(unrelated.is_alive());
    unrelated.terminate_tree_for_test();
}
```

测试不能使用用户当前 DSH 进程，不能使用固定 PID，也不能执行递归 `taskkill`。

- [ ] **Step 2: 实现命名 Job Object**

使用当前用户范围的确定性名称，例如 `Local\\DeepSeekHarnessManager-<user-sid>`；如果管理器重启且之前的 DSH 仍存活，允许重新打开该 Job Object 进行观察。不要设置 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`。`Drop` 只关闭句柄；只有显式 `terminate()` 才调用 `TerminateJobObject`。

- [ ] **Step 3: 实现进程身份检查**

通过 Windows API 获取 PID 的完整可执行路径和父 PID；通过可靠的进程信息 API 获取命令行。对外部 DSH 形成 `ProcessIdentity`，后续接管判断至少比较：

- listener PID 与观察到的 PID 一致；
- executable 路径与 packaged `DSH.exe` 规范化后相同，或命令行包含已确认源码目录且包含 `dsh`、`web`；
- 进程未退出且 PID 在检查期间没有复用。

路径比较必须大小写不敏感、规范化分隔符，但不能模糊匹配任意同名 `DSH.exe`。

- [ ] **Step 4: 实现尽力正常退出**

优先对由管理器创建的进程组发送 `CTRL_BREAK_EVENT`；如果目标没有可用控制台进程组，则记录 `graceful-stop-unavailable` 并进入等待窗口。实现不得把 `TerminateJobObject` 当作第一步。等待逻辑由上层生命周期管理器执行 5 秒；底层只提供请求和存活查询。

- [ ] **Step 5: 运行 Windows 集成验证**

执行：

```text
cargo test --manifest-path src-tauri/Cargo.toml --test windows_process_integration -- --nocapture
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

预期：管理器句柄关闭后 fixture 仍存活；显式终止只影响 Job Object 内的 fixture；Clippy 无 warning。若当前环境尚未准备好 Rust/Windows SDK，记录真实阻塞原因，不用模拟输出替代测试结果。

---

### Task 6: 生命周期状态机和 DSH 启停/重启控制器

**Objective:** 将配置、目标命令、健康检查和进程归属组合成可串行化的 DSH 生命周期控制器。

**Files:**
- Create: `src-tauri/src/lifecycle.rs`
- Modify: `src-tauri/src/domain.rs`, `src-tauri/src/health.rs`, `src-tauri/src/process/mod.rs`, `src-tauri/src/lib.rs`
- Test: `src-tauri/src/lifecycle.rs`, `src-tauri/tests/lifecycle.rs`

**Interfaces:**
- `LifecycleController::start()`, `stop()`, `restart()`, `refresh_external_state()`。
- `LifecycleController::snapshot() -> RuntimeSnapshot`。
- `LifecycleActionError` 至少包含 `AlreadyRunning`, `PortConflict`, `TargetInvalid`, `ExternalNotAdopted`, `StartupTimeout`, `StopTimeout`。
- 所有动作通过同一异步互斥锁串行执行；健康检查任务携带 generation ID，旧任务不得覆盖新状态。
- 测试用 `controller_with(adapter, health)`、`FakeAdapter::{ready,no_spawn,hanging,external}`、`FakeHealth::{ready,unreachable}`、`FakeListener::unknown(pid)` 和 `FakeClock` 在 `src-tauri/tests/lifecycle.rs` 定义；它们不启动真实 DSH。

- [ ] **Step 1: 写状态转换测试**

```rust
#[test]
fn start_transitions_stopped_to_starting_to_running() {
    let mut controller = controller_with(FakeAdapter::ready(), FakeHealth::ready());
    assert_eq!(controller.snapshot().state, LifecycleState::Stopped);
    controller.start().expect("start fake target");
    assert_eq!(controller.snapshot().state, LifecycleState::Running);
}
#[test]
fn unknown_listener_enters_port_conflict_without_killing_pid() {
    let listener = FakeListener::unknown(pid(4012));
    let mut controller = controller_with(FakeAdapter::no_spawn(), FakeHealth::unreachable());
    controller.set_listener(listener);
    let error = controller.start().expect_err("unknown listener must block start");
    assert_eq!(error.code(), "port_conflict");
    assert_eq!(controller.process_port.terminate_calls(), 0);
}
#[test]
fn stop_waits_five_seconds_before_forcing_owned_job() {
    let mut controller = controller_with(FakeAdapter::hanging(), FakeHealth::ready());
    controller.start().expect("start fake target");
    controller.stop().expect("force after graceful wait");
    assert_eq!(controller.clock.elapsed(), std::time::Duration::from_secs(5));
    assert_eq!(controller.process_port.terminate_calls(), 1);
}
#[test]
fn external_process_is_observe_only_until_adopted() {
    let mut controller = controller_with(FakeAdapter::external(), FakeHealth::ready());
    controller.refresh_external_state().expect("observe external");
    let error = controller.stop().expect_err("external process is observe-only");
    assert_eq!(error.code(), "external_not_adopted");
}
#[test]
fn restart_uses_one_config_snapshot_for_command_and_health_url() {
    let mut controller = controller_with(FakeAdapter::ready(), FakeHealth::ready());
    controller.start().expect("initial start");
    controller.restart().expect("restart fake target");
    assert_eq!(controller.last_start_command().port(), controller.last_health_url().port());
    assert_eq!(controller.last_start_command().target_id(), controller.last_health_url().target_id());
}
```

- [ ] **Step 2: 实现启动前检查**

`start()` 按以下顺序执行：加载并校验配置；查询 listener；无 listener 时创建目标命令和 Job Object；有 listener 且身份匹配时进入 `External`；有 listener 且身份未知时进入 `PortConflict`；不得对未知 PID 发送停止或终止请求。

- [ ] **Step 3: 实现启动和就绪轮询**

启动目标后立即记录根 PID并加入命名 Job Object；每 500ms 执行一次绕过代理的健康检查，最多 90 秒。就绪后记录 listener PID、target ID、ownership 和 started_at；进程提前退出或超时则记录 stdout/stderr 摘要，进入 `Failed` 并清理已归属 Job。

- [ ] **Step 4: 实现停止和重启**

`stop()` 只允许 managed/adopted ownership；先调用 `GracefulStop::request`，每 100ms 检查 Job 是否为空，最长等待 5 秒；仍存活时调用 `JobOwner::terminate()`，再等待进程列表为空。`restart()` 保存一个不可变配置快照，执行 stop 后使用同一快照 start，避免切换端口或目标时前后不一致。

- [ ] **Step 5: 运行生命周期验证**

执行：

```text
cargo test --manifest-path src-tauri/Cargo.toml lifecycle
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
```

预期：状态转换、未知端口保护、5 秒停止顺序、外部观察和配置快照测试全部通过。

---

### Task 7: Tauri 应用状态、commands、事件、托盘和单实例

**Objective:** 将 Rust 生命周期控制器安全暴露给前端和托盘，并实现关闭窗口隐藏、退出管理器不停止 DSH。

**Files:**
- Create: `src-tauri/src/app_state.rs`
- Create: `src-tauri/src/commands.rs`
- Create: `src-tauri/src/events.rs`
- Create: `src-tauri/src/tray.rs`
- Modify: `src-tauri/src/lib.rs`, `src-tauri/src/main.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src-tauri/Cargo.toml`
- Test: `src-tauri/src/commands.rs`, `src-tauri/src/tray.rs`

**Interfaces:**

后端注册以下 Tauri commands，命令内部必须重新校验状态，不能信任前端传来的 PID 或路径：

```text
get_app_state() -> Result<AppStateDto, AppError>
start_dsh() -> Result<RuntimeSnapshot, AppError>
stop_dsh() -> Result<RuntimeSnapshot, AppError>
restart_dsh() -> Result<RuntimeSnapshot, AppError>
prepare_proxy_change(enabled: bool) -> Result<ProxyChangePlan, AppError>
apply_proxy_change(enabled: bool, confirmed_restart: bool) -> Result<RuntimeSnapshot, AppError>
set_active_target(target_id: TargetId) -> Result<AppStateDto, AppError>
save_settings(settings: SettingsPatch) -> Result<AppStateDto, AppError>
scan_targets() -> Result<Vec<DiscoveredTarget>, AppError>
adopt_external_dsh() -> Result<RuntimeSnapshot, AppError>
open_dsh_url() -> Result<(), AppError>
open_log_directory() -> Result<(), AppError>
run_self_test() -> Result<SelfTestReport, AppError>
```

事件名称固定为：`state_changed`、`log_appended`、`startup_progress`、`notification_requested`。
- `AppStateDto`、`SettingsPatch`、`ProxyChangePlan` 和 `SelfTestReport` 在 `commands.rs` 中定义为 camelCase 可序列化 DTO；前端只依赖这些 DTO，不依赖内部 Job Object 类型。
- 测试辅助 `state_with_external_process()`, `state_with_running_process(proxy)`, `proxy_enabled(value)`, `stop_dsh_for_test(state)`, `prepare_proxy_change_for_test(state, enabled)` 和 `apply_proxy_change_for_test(state, enabled, confirmed_restart)` 在 `commands.rs` 的 `#[cfg(test)]` 模块中定义。

- [ ] **Step 1: 写 command 状态保护测试**

使用 fake `LifecyclePort` 验证：

```rust
#[test]
fn stop_rejects_unadopted_external_process() {
    let state = state_with_external_process();
    let error = stop_dsh_for_test(state).expect_err("external process is not adopted");
    assert_eq!(error.code, "external_not_adopted");
}
#[test]
fn proxy_change_plan_does_not_persist_before_confirmation() {
    let state = state_with_running_process(proxy_enabled(true));
    let before = state.config.proxy.enabled;
    let plan = prepare_proxy_change_for_test(&state, false).expect("prepare change");
    assert!(plan.requires_restart);
    assert_eq!(state.config.proxy.enabled, before);
}
#[test]
fn apply_proxy_change_requires_confirmation_when_running() {
    let state = state_with_running_process(proxy_enabled(true));
    let error = apply_proxy_change_for_test(&state, false, false)
        .expect_err("running DSH requires explicit confirmation");
    assert_eq!(error.code, "confirmation_required");
}
```

- [ ] **Step 2: 实现 AppState 和事件桥**

`AppState` 持有配置存储、生命周期控制器、日志句柄和事件发布器。每次状态改变只由 Rust 发布一次 `state_changed`；前端不能直接写运行状态。事件 payload 使用与 `RuntimeSnapshot` 相同的 camelCase DTO。

- [ ] **Step 3: 实现托盘菜单**

使用 Tauri `TrayIconBuilder` 创建状态图标和菜单：状态项、启动、停止、重启、代理开关、当前目标、打开 DSH 页面、打开管理器窗口、打开日志目录、退出管理器。菜单项按状态动态禁用；`external` 状态只显示观察和接管，不显示可直接强制停止。

- [ ] **Step 4: 实现单实例和窗口行为**

初始化 `tauri-plugin-single-instance`；第二次启动只激活现有窗口，不创建第二个控制器。拦截主窗口 `CloseRequested`，默认 prevent close 并隐藏到托盘。“退出管理器”调用 `app.exit(0)`，退出路径不调用 `stop_dsh()`，且 Job Object 句柄释放不能终止 DSH。

- [ ] **Step 5: 运行后端集成验证**

执行：

```text
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
pnpm typecheck
pnpm tauri dev
```

手工验证第二次启动只激活同一窗口，关闭窗口后托盘仍在，退出管理器后一个已运行的测试 DSH 仍可访问。

---

### Task 8: 开机启动、目标自动发现和首次配置向导后端

**Objective:** 实现当前用户开机启动注册、源码/打包目标扫描和首次运行配置流程。

**Files:**
- Create: `src-tauri/src/startup.rs`
- Create: `src-tauri/src/discovery.rs`
- Modify: `src-tauri/src/commands.rs`, `src-tauri/src/app_state.rs`, `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml`
- Test: `src-tauri/src/startup.rs`, `src-tauri/src/discovery.rs`

**Interfaces:**
- `StartupRegistration::reconcile(enabled: bool)` 使用 autostart plugin，默认只写当前用户范围。
- `discover_targets() -> Vec<DiscoveredTarget>` 不启动进程、不修改配置。
- `validate_source_directory(path) -> TargetValidation` 检查 `package.json`、根脚本 `dsh` 和 `apps/cli/src/bin.ts`。
- `validate_packaged_executable(path) -> TargetValidation` 检查存在、为文件、扩展名为 `.exe`，并规范化父目录。
- 测试辅助 `fixture_source_with(script_name, entrypoint)`, `discover_targets_from(paths)` 和 `DiscoveryProbe::launch_calls()` 只存在测试模块；`discover_targets_from` 返回 `(Vec<DiscoveredTarget>, DiscoveryProbe)`。

- [ ] **Step 1: 写发现测试**

```rust
#[test]
fn source_candidate_requires_dsh_script_and_cli_entrypoint() {
    let root = fixture_source_with("dsh", "apps/cli/src/bin.ts");
    let result = validate_source_directory(&root);
    assert!(result.is_valid);
}
#[test]
fn packaged_candidate_requires_existing_exe_file() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let missing = dir.path().join("missing.exe");
    assert!(!validate_packaged_executable(&missing).is_valid);
    let valid = dir.path().join("DSH.exe");
    std::fs::write(&valid, b"fixture").expect("write exe fixture");
    assert!(validate_packaged_executable(&valid).is_valid);
}
#[test]
fn discovery_returns_current_known_candidates_without_starting_them() {
    let (result, probe) = discover_targets_from(vec![fixture_source_with("dsh", "apps/cli/src/bin.ts")]);
    assert_eq!(result.len(), 1);
    assert!(result[0].needs_user_confirmation);
    assert_eq!(probe.launch_calls(), 0);
}
```

fixture 使用临时目录，不读取真实 DSH 配置，不执行 `pnpm`。

- [ ] **Step 2: 实现候选路径扫描**

候选顺序固定为：

1. `C:\Users\Tony\Documents\Default Project\deepseek-harness`；
2. `%USERPROFILE%\Documents\Default Project\deepseek-harness`；
3. `%USERPROFILE%\Documents\BaiduSyncdisk\DSH\deepseek-harness`；
4. 已保存配置中的旧路径。

对每个源码候选同时检查根目录 `package.json` 和 `DSH.exe`，但返回结果必须带 `needs_user_confirmation: true`。禁止全盘递归扫描。

- [ ] **Step 3: 实现首次配置流程后端**

当配置不存在或两个目标都未配置时，`get_app_state()` 返回 `firstRun: true`。向导提交后一次性写入已确认目标、activeTarget、代理默认值和管理器启动设置；提交后状态仍是 `Stopped`，不得自动调用 `start_dsh()`。

- [ ] **Step 4: 实现 autostart reconcile**

管理器启动时只根据 `manager.startOnLogin` 启用/禁用自身启动项；即使 `startDshOnLogin` 字段未来被设置为 true，第一版也必须由独立设置和明确用户动作触发，默认值必须是 false。启动项命令不能带 `start_dsh` 参数。

- [ ] **Step 5: 运行验证**

执行：

```text
cargo test --manifest-path src-tauri/Cargo.toml discovery
cargo test --manifest-path src-tauri/Cargo.toml startup
```

手工验证首次向导完成后 DSH 仍停止，注销/登录后管理器可启动但不启动 DSH。

---

### Task 9: React 状态层、托盘状态窗口和首次运行 UI

**Objective:** 提供状态卡、启停按钮、目标切换、代理设置、确认对话框和首次向导。

**Files:**
- Create: `src/types.ts`, `src/tauri.ts`, `src/state.ts`, `src/components/StatusCard.tsx`, `src/components/ActionBar.tsx`, `src/components/TargetSelector.tsx`, `src/components/ProxySettings.tsx`, `src/components/SettingsPanel.tsx`, `src/components/FirstRunWizard.tsx`, `src/components/ConfirmRestartDialog.tsx`, `src/components/ExternalDshBanner.tsx`, `src/components/DiagnosticsPanel.tsx`, `src/styles.css`
- Modify: `src/main.tsx`, `src/App.tsx`, `src/test/setup.ts`
- Test: `src/test/App.test.tsx`, `src/test/ProxySettings.test.tsx`, `src/test/FirstRunWizard.test.tsx`

**Interfaces:**
- `src/types.ts` 定义与 Rust DTO 一致的 `AppStateDto`, `RuntimeSnapshot`, `LifecycleState`, `ProxyChangePlan`, `DiscoveredTarget`, `SelfTestReport`。
- `src/tauri.ts` 只封装 `invoke` 和 `listen`，组件不得散落原始 command 字符串。
- `src/state.ts` 提供 `loadState()`, `subscribeToState(setter)`, `runCommand()`，所有 listener 在组件卸载时清理。
- `App` 接受可选 `initialState?: AppStateDto`，只用于 UI 测试；生产启动始终通过 `loadState()` 获取后端快照。测试 `mockInvoke(command, result)` 和 `mockListen()` 在 `src/test/setup.ts` 定义。

- [ ] **Step 1: 写前端行为测试**

```tsx
it('renders stopped state and enables start', async () => {
  mockInvoke('get_app_state', stoppedState());
  render(<App />);
  expect(await screen.findByText('已停止')).toBeVisible();
  expect(screen.getByRole('button', { name: '启动 DSH' })).toBeEnabled();
});

it('asks before restarting when proxy changes while running', async () => {
  mockInvoke('prepare_proxy_change', { requiresRestart: true });
  render(<App initialState={runningState()} />);
  await user.click(screen.getByRole('switch', { name: '使用代理' }));
  expect(screen.getByText('需要重启 DSH，当前会话可能中断')).toBeVisible();
  expect(mockInvoke).not.toHaveBeenCalledWith('apply_proxy_change', expect.anything());
});

it('does not auto-start after first-run wizard submission', async () => {
  render(<App initialState={firstRunState()} />);
  await completeWizard();
  expect(mockInvoke).toHaveBeenCalledWith('save_settings', expect.anything());
  expect(mockInvoke).not.toHaveBeenCalledWith('start_dsh', undefined);
});
```

- [ ] **Step 2: 实现类型和 Tauri API 封装**

Rust 的 camelCase 字段在 TypeScript 中保持相同名称；所有 command 错误统一显示 `AppError.message` 和错误码对应的恢复建议。事件 `state_changed` 到达后替换状态快照，不在前端自行推断 PID 或 ownership。

- [ ] **Step 3: 实现主界面和状态操作**

`StatusCard` 显示状态、当前目标、服务 URL、listener PID、代理状态和最近错误；`ActionBar` 根据状态调用 start/stop/restart；`ExternalDshBanner` 只提供“接管”按钮，并在接管失败时保留 external 状态。

- [ ] **Step 4: 实现代理确认流程**

代理开关点击先调用 `prepare_proxy_change`。如果 `requiresRestart=false`，直接调用 apply；如果为 true，显示确认弹窗；取消则恢复 UI 原值，确认才调用 `apply_proxy_change(enabled, true)`。按钮文案必须明确说明会中断当前 DSH 会话。

- [ ] **Step 5: 实现首次向导和设置页**

向导显示自动发现结果、源码目录/`DSH.exe` 验证结果、默认目标、代理 URL 和代理开关；路径选择通过 dialog plugin，不让用户输入未经校验的 PID 或命令行。设置页显示“关闭代理时，管理器不主动注入或清理环境变量”。

- [ ] **Step 6: 运行 UI 验证**

执行：

```text
pnpm typecheck
pnpm test -- --run
pnpm build
```

预期：TypeScript、Vitest 和前端生产构建全部通过；手工运行 `pnpm tauri dev` 检查窗口关闭隐藏、托盘重新打开、状态按钮禁用逻辑和代理确认流程。

---

### Task 10: 日志、DSH stdout/stderr、诊断和自检

**Objective:** 让用户能定位路径、命令、端口、代理和进程问题，同时不记录凭据或完整环境变量。

**Files:**
- Create: `src-tauri/src/logging.rs`
- Create: `src-tauri/src/diagnostics.rs`
- Modify: `src-tauri/src/app_state.rs`, `src-tauri/src/lifecycle.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/process/mod.rs`
- Modify: `src/components/DiagnosticsPanel.tsx`, `src/types.ts`
- Test: `src-tauri/src/logging.rs`, `src-tauri/src/diagnostics.rs`, `src/test/App.test.tsx`

**Interfaces:**
- `LogManager::init(app_local_data_dir)`, `log_manager_event(event)`, `log_dsh_line(stream, line)`。
- `Diagnostics::snapshot() -> DiagnosticSnapshot`，包含版本、配置摘要、状态、目标验证、listener owner、工具链解析结果和最近错误。
- `run_self_test() -> SelfTestReport`，每个检查项包含 `name`, `status`, `message`, `remediation`。
- 测试辅助 `DiagnosticSnapshot::from_test_state_with_proxy(url)`, `redact_url(url)`, `PnpmResolver::missing()`, `run_self_test_with(resolver)` 和 `SelfTestReport::item(name)` 在对应测试模块中定义。

- [ ] **Step 1: 写日志脱敏测试**

```rust
#[test]
fn diagnostic_output_does_not_include_environment_map() {
    let snapshot = DiagnosticSnapshot::from_test_state_with_proxy("http://127.0.0.1:7897");
    let text = serde_json::to_string(&snapshot).expect("serialize diagnostics");
    assert!(!text.contains("environment"));
    assert!(!text.contains("NODE_USE_ENV_PROXY"));
}
#[test]
fn proxy_credentials_are_redacted_in_log_url() {
    let redacted = redact_url("http://user:secret@127.0.0.1:7897");
    assert_eq!(redacted, "http://***:***@127.0.0.1:7897");
    assert!(!redacted.contains("secret"));
}
#[test]
fn self_test_reports_missing_pnpm_as_actionable_item() {
    let report = run_self_test_with(PnpmResolver::missing());
    let item = report.item("pnpm").expect("pnpm check");
    assert_eq!(item.status, CheckStatus::Failed);
    assert!(!item.remediation.is_empty());
}
```

- [ ] **Step 2: 实现日志文件和子进程输出捕获**

写入：

```text
%LOCALAPPDATA%\DeepSeekHarnessManager\logs\manager.log
%LOCALAPPDATA%\DeepSeekHarnessManager\logs\dsh.log
```

启动目标时分别读取 stdout/stderr，按行写入 `dsh.log`；读取线程退出不能阻塞生命周期状态机。日志事件包含时间、级别、事件名和脱敏详情，不写完整环境 map，不写 API key/Token。

- [ ] **Step 3: 实现诊断快照**

诊断输出只包含配置摘要和可操作信息：目标类型、规范化路径、端口、PID、ownership、状态、pnpm 解析结果、代理是否启用及脱敏 URL、日志路径。复制内容不得包含完整命令环境或凭据。

- [ ] **Step 4: 实现自检**

自检顺序固定为：配置解析、loopback host/port 校验、source/package target 路径、pnpm PATH、proxy URL、listener 状态、健康检查、autostart 状态、日志目录可写。每项返回明确修复建议，不因某项失败而跳过其余独立检查。

- [ ] **Step 5: 运行验证**

执行：

```text
cargo test --manifest-path src-tauri/Cargo.toml logging
cargo test --manifest-path src-tauri/Cargo.toml diagnostics
pnpm test -- App.test.tsx
```

手工在设置窗口点击“运行自检”和“复制诊断信息”，确认日志目录可打开、诊断文本不含环境变量 dump。

---

### Task 11: Windows 绿色版、NSIS 安装器和发布校验脚本

**Objective:** 生成不自动启动 DSH 的绿色版和当前用户范围的 Windows 安装器。

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Create: `scripts/package-portable.ps1`
- Create: `scripts/verify-release.ps1`
- Modify: `README.md`

**Interfaces:**
- `scripts/package-portable.ps1 -Version <version>` 生成 `artifacts/portable/DSHtray-<version>-windows-x64/`。
- `scripts/verify-release.ps1 -ArtifactRoot <path>` 检查可执行文件、安装包和默认配置行为。

- [ ] **Step 1: 配置 NSIS bundle**

在 `tauri.conf.json` 中启用 `nsis` target，产品名、标识、图标、版本和安装器语言来自同一份配置。安装器不附带 DSH，也不带 `start_dsh` 参数；当前用户安装模式优先，不要求管理员权限才能使用管理器自身。

- [ ] **Step 2: 实现绿色版打包脚本**

脚本执行：

```text
pnpm tauri build --no-bundle
```

然后把 `src-tauri/target/release/dshtray.exe` 和 `README.md` 复制到版本化 portable 目录。脚本不能复制 `%LOCALAPPDATA%` 配置，不在程序目录写默认配置，不启动可执行文件。

- [ ] **Step 3: 实现发布校验脚本**

`verify-release.ps1` 检查：

- portable 目录存在唯一 `dshtray.exe`；
- NSIS `*-setup.exe` 存在；
- 可执行文件版本信息中的 product name 为 `DSHtray`；
- 安装包不包含 DSH 二进制；
- 安装后启动项只指向 DSHtray，不带 DSH 启动参数；
- 卸载测试删除 DSHtray 启动项，但按用户选择保留或删除配置日志。

- [ ] **Step 4: 更新 README 发布说明**

记录：前置环境、绿色版运行方式、安装器运行方式、配置目录、默认代理语义、源码目标要求、如何打开诊断日志，以及卸载不会停止已经独立运行的 DSH。

- [ ] **Step 5: 运行构建验证**

在 Windows MSVC 环境执行：

```text
pnpm tauri build
pwsh -NoProfile -File scripts/package-portable.ps1 -Version 0.1.0
pwsh -NoProfile -File scripts/verify-release.ps1 -ArtifactRoot artifacts
```

预期：NSIS setup exe 和 portable 目录都生成，发布校验退出码为 0；如果安装器需要管理员权限，必须在 README 和验收记录中如实标注，不把它描述成当前用户无 UAC 安装。

---

### Task 12: 完整 Windows 验收和回归门禁

**Objective:** 用真实 Windows 运行结果证明所有已批准验收标准，而不是只证明单元测试通过。

**Files:**
- Modify: `README.md`（补充实际命令和已验证平台）
- Create: `docs/superpowers/acceptance/2026-08-24-dshtray-acceptance.md`
- Use: `src-tauri/tests/windows_process_integration.rs`, `scripts/verify-release.ps1`

- [ ] **Step 1: 运行 Rust 和前端静态门禁**

```text
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
pnpm typecheck
pnpm test
pnpm build
```

记录每条命令的真实退出码和测试数量；任何失败都进入修复，不用“基本通过”替代。

- [ ] **Step 2: 验收源码目标**

在首次向导确认 `C:\Users\Tony\Documents\Default Project\deepseek-harness` 后启动源码目标，验证：

- 管理器执行的 argv 等价于 `pnpm dsh web`；
- 状态从 `starting` 进入 `running`；
- `http://127.0.0.1:3080/` 可访问；
- 代理开启时通过受控测试子进程确认三个变量准确存在；
- 代理关闭时确认管理器没有主动删除或添加变量。

- [ ] **Step 3: 验收打包目标**

停止源码目标后选择确认的 `DSH.exe`，验证打包目标可启动、健康检查使用同一端口、停止只清理归属树。两个目标不得同时占用端口。

- [ ] **Step 4: 验收安全边界**

启动一个与 DSH 无关的本地 listener 占用配置端口，点击启动，确认状态为 `port-conflict`，未知 PID 仍存活。启动外部 DSH，确认默认 `external`、停止按钮受保护；明确接管后才允许停止，身份校验失败时拒绝接管。

- [ ] **Step 5: 验收生命周期和代理切换**

验证：

- 正常停止优先发出尽力退出请求；无法退出时至少等待 5 秒后才强制清理已归属 Job；
- 管理器退出后 DSH 仍能访问；
- 代理切换取消不重启，确认才重启；
- 管理器登录启动后 DSH 保持停止；
- 关闭窗口只隐藏到托盘。

- [ ] **Step 6: 验收安装和卸载**

安装 NSIS 包、启动管理器、开启/关闭当前用户开机启动、注销并登录、检查单实例、卸载并验证启动项清理。配置和日志保留行为必须与卸载选择一致。

- [ ] **Step 7: 写入验收记录**

`docs/superpowers/acceptance/2026-08-24-dshtray-acceptance.md` 逐条记录命令、日期、Windows 版本、工具版本、结果和失败证据。只有所有设计验收标准都有真实证据，才能把版本标记为可交付。

## 依赖顺序

```text
Task 1
  ├─ Task 2 ─┐
  ├─ Task 3 ─┼─ Task 6 ─ Task 7 ─ Task 9 ─ Task 10
  ├─ Task 4 ─┤              └─ Task 8
  └─ Task 5 ─┘
Task 11 依赖 Task 7、Task 9、Task 10
Task 12 依赖 Task 5、Task 6、Task 7、Task 9、Task 10、Task 11
```

## 主要风险与应对

1. **当前没有 Rust/cargo。** 先完成 MSVC 工具链门禁；在工具链可用前不开始代码任务。
2. **pnpm 版本不一致。** DSH 仓库声明 `pnpm@11.7.0`，当前环境为 `11.21.0`；管理器只执行外部配置的命令，不修改 DSH 仓库的 packageManager，真实验收时记录两者版本。
3. **Windows graceful stop 能力受进程控制台形态影响。** 发送 `CTRL_BREAK_EVENT` 只能视为尽力请求；5 秒后是否强制终止由 Job Object 归属决定，日志必须记录请求不可用的原因。
4. **管理器退出后重新识别旧 DSH。** 使用确定性命名 Job Object 和 listener/PID/路径复核；复核失败时回到 external 或 port-conflict，不自动接管。
5. **Tauri 插件权限过宽。** 每增加一个 command，先补 capability 最小权限，再补 Rust command 测试和前端调用测试。
6. **NSIS 当前用户安装语义。** 先以官方 Tauri 2 schema 验证 `installMode`，构建结果决定 README 的权限说明；不能根据配置文件推断安装器行为。
7. **DSH 源码自身启动失败。** 管理器只报告 pnpm/工作目录/子进程 stderr/健康检查结果，不修改 DSH 源码来规避启动失败。

## 计划完成后的执行交接

计划文件保存为：

```text
docs/superpowers/plans/2026-08-24-dshtray-manager-implementation-plan.md
```

开始实现时有两种方式：

1. **Subagent-Driven（推荐）**：按任务逐个派发新子代理，每个任务完成后先做规格符合性检查，再做代码质量检查。
2. **Inline Execution**：在当前会话按依赖顺序执行，每个任务完成后运行该任务的测试并停下来复核。

无论采用哪种方式，必须先确认执行前决策门槛中的 Tauri 2.x、React/TypeScript/Vite、NSIS 和应用标识；然后先解决 Rust MSVC 工具链缺失问题。本文是实现计划，不表示已经创建代码或安装依赖。
