# DSHtray Native Companion Assets and Plugin Package Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将现有 DSHtray Rust/Tauri 应用包装成可由 DSH Host 验证、安装、握手和回滚的 `@gittuyn/dshtray-plugin@0.1.0` 原生伴侣资产，同时提供只读 Web 状态入口。

**Architecture:** DSHtray 继续是独立的托盘和进程生命周期所有者；npm 包只携带预构建 Web 插件、声明式 `dsh.companion` 元数据和包内 NSIS 安装器。安装动作由 DSH Host 执行，DSHtray 本身只提供 current-user 应用、受限只读状态握手和“打开管理器”协议入口，不提供无认证的进程控制 HTTP API。

**Tech Stack:** Rust, Tauri 2, NSIS current-user installer, Windows named pipe, Authenticode/WinVerifyTrust, TypeScript, React 18, DSH client runtime, pnpm 11.7.0, Vitest, PowerShell release scripts.

**Spec:** `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/docs/superpowers/specs/2026-08-28-dsh-plugin-companion-install-design.md`

## Global Constraints

- Companion id 必须是 `com.deepseek.dshtray`；协议号必须是 `1`；目标平台必须是 `win32`；目标架构必须是 `x64`。
- npm 包必须是 `@gittuyn/dshtray-plugin@0.1.0`；宿主运行时兼容当前 DSH `0.1.0-rc.7` 的 client/host contracts。
- 当前 release 安装器的真实文件名是 `DSHtray_0.1.0_x64-setup.exe`，大小为 `1920630` 字节；当前远端资产为 `NotSigned`，不能作为自动安装生产资产。
- 安装类型只能是 `nsis-current-user`；唯一静默参数只能是 `/S`；`requiresElevation` 必须是 `false`。
- 资产必须来自包内固定路径；脚本不能从网络下载第二个安装器，不能执行 `postinstall`、`prepare` 或任意自定义安装钩子。
- 安装前必须检查平台、架构、相对路径、文件大小、SHA-256、Authenticode 有效性、发布者匹配和外部所有权；任何失败都不执行安装。
- Web 插件卸载默认不卸载独立 DSHtray；只有显式的原生卸载选项才可进入卸载流程。
- 第一版 Web UI 只读显示安装状态、运行状态、代理状态、管理归属和等待接管状态；不提供停止、重启、接管或强制终止按钮。
- 安装、升级、卸载和状态协议不得保存、上传或转发任何 API 密钥、令牌、密码或其他凭据值。

## Execution Gates

- **G0 — 包源码位置：** 本计划默认将目标包放在 `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/packages/dshtray-plugin`，并由 DSHtray 仓库的 pnpm workspace 构建；若改为独立仓库，必须在执行前改写本计划中的包路径和构建命令。
- **G1 — 管理器入口：** 本计划默认使用 current-user `dshtray://open` URI scheme；安装器注册 `HKCU\\Software\\Classes\\dshtray`，应用只接受固定的 `dshtray://open`，不解析任意命令参数。若不批准注册 URI scheme，必须在执行前提供已认证的 DSH Host opener capability 替代方案。
- **G2 — 运行时签名验证：** DSH Host 必须使用原生 WinVerifyTrust 或已审计的等效 native adapter；不得用 PowerShell、`cmd /c`、`shell: true` 代替运行时验证。G2 未通过前不能把自动安装标记为可用。
- **G3 — 发布者身份：** 生产发布前必须在 CI secret/安全签名环境中确定 Authenticode signer subject 和证书链；证书私钥、令牌和密码不进入仓库、npm 包或日志。

## File Map

- Modify `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/package.json`: 增加不执行安装的包构建、校验和 `pack:companion` scripts。
- Modify `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/pnpm-workspace.yaml`: 只加入 `packages/*` workspace，保持根应用脚本不变。
- Create `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/packages/dshtray-plugin/package.json`: 发布元数据、`dsh.bundle`、`dsh.client`、`dsh.companion` 声明和 package allowlist。
- Create `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/packages/dshtray-plugin/cordis.patch.yml`: 将 Host bundle 插入 DSH bundle tree。
- Create `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/packages/dshtray-plugin/src/index.ts`: `dshtrayCompanion` 只读 Host service 和 bundle apply 入口。
- Create `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/packages/dshtray-plugin/src/status.ts`: named-pipe 状态读取、receipt 归属合并和 `DshtrayCompanionStatus` 类型。
- Create `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/packages/dshtray-plugin/src/client/index.tsx`: Web client bundle 入口。
- Create `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/packages/dshtray-plugin/src/client/DshtrayStatusSection.tsx`: 只读状态卡和固定 `dshtray://open` 入口。
- Create `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/packages/dshtray-plugin/src/client/DshtrayStatusSection.test.tsx`: 状态映射和禁用控制按钮回归测试。
- Create `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/packages/dshtray-plugin/src/test/package-manifest.test.ts`: 包声明和生命周期脚本回归测试。
- Create `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/packages/dshtray-plugin/src/test/package-tarball.test.ts`: tarball allowlist 回归测试。
- Create `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/packages/dshtray-plugin/tsconfig.json` and `vite.client.config.ts`: Host 类型检查与 DSH client bundle 构建配置，复用 DSH `ui-settings-general` 的 injection contract。
- Create `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/packages/dshtray-plugin/scripts/verify-package.mjs`: 只读检查发布 tarball 内容、脚本和 manifest allowlist。
- Create `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/scripts/package-companion.ps1`: 生成包内 manifest、拷贝 NSIS 资产和执行发布前签名检查。
- Modify `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/scripts/verify-release.ps1`: 增加 current-user companion 参数和强制签名模式。
- Create `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/src-tauri/src/companion.rs`: named pipe status handshake、URI 输入 allowlist 和 Windows-only 状态桥。
- Modify `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/src-tauri/src/lib.rs`: 注册 companion status server 和 deep-link handler。
- Modify `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/src-tauri/Cargo.toml`: 添加当前 Tauri 主版本兼容的 deep-link/native Windows API 依赖及 feature。
- Modify `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/src-tauri/tauri.conf.json`: 注册 `dshtray` scheme、current-user NSIS hook 和包资源。
- Create `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/src-tauri/windows/installer-hooks.nsh`: 仅写入/移除 HKCU URI scheme，不写 HKLM、不保存凭据。
- Create `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/scripts/tests/package-manifest.test.mjs`: manifest/allowlist/签名门槛测试。
- Create `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/docs/superpowers/acceptance/dshtray-companion-release.md`: 人工安装、升级、回滚、卸载和外部所有权验收记录模板。

### Task 1: 建立目标包 manifest 和最小 workspace

**Objective:** 先固定 npm 包的可审计边界，让 Host 后续只依赖声明式字段而不是猜目录。

**Files:**
- Create: `packages/dshtray-plugin/package.json`
- Create: `packages/dshtray-plugin/cordis.patch.yml`
- Modify: `pnpm-workspace.yaml`
- Test: `packages/dshtray-plugin/src/test/package-manifest.test.ts`

**Interfaces:**
- `package.json` 的 `dsh.companion` 只声明固定身份和 `manifest: './dsh.companion.json'`；打包生成的 `dsh.companion.json` 才携带 `asset`, `sizeBytes`, `sha256`, `expectedExecutable`, `expectedUninstaller`, `requiresElevation`, `silentArgs`, `startAfterInstall`。
- Produces `dshtray://open` 作为客户端入口；不产生 `install`, `stop`, `restart`, `kill` 等 client capability。

- [ ] **Step 1: Write the failing test**

```ts
import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

it('declares the fixed DSHtray companion contract', () => {
  const pkg = JSON.parse(readFileSync('packages/dshtray-plugin/package.json', 'utf8'))
  expect(pkg.name).toBe('@gittuyn/dshtray-plugin')
  expect(pkg.version).toBe('0.1.0')
  expect(pkg.dsh.companion).toMatchObject({
    manifest: './dsh.companion.json',
    protocol: 1,
    id: 'com.deepseek.dshtray',
    platform: 'win32',
    arch: 'x64',
    kind: 'nsis-current-user',
  })
  expect(pkg.scripts.postinstall).toBeUndefined()
  expect(pkg.scripts.prepare).toBeUndefined()
})
```

- [ ] **Step 2: Run test to verify failure**

Run: `pnpm exec vitest run packages/dshtray-plugin/src/test/package-manifest.test.ts`

Expected: FAIL because `packages/dshtray-plugin/package.json` does not exist yet.

- [ ] **Step 3: Write the minimal manifest**

```json
{
  "name": "@gittuyn/dshtray-plugin",
  "version": "0.1.0",
  "private": false,
  "type": "module",
  "main": "lib/index.js",
  "types": "lib/types/index.d.ts",
  "files": ["lib", "client", "assets", "cordis.patch.yml", "dsh.companion.json"],
  "dsh": {
    "bundle": "./cordis.patch.yml",
    "client": {
      "entry": "./client/client.js",
      "platform": "web",
      "inject": [
        "@deepseek-ai/dsh-client-runtime",
        "@deepseek-ai/dsh-client-connection",
        "@deepseek-ai/dsh-client-locale",
        "@deepseek-ai/dsh-client-ui-primitives"
      ]
    },
    "companion": {
      "manifest": "./dsh.companion.json",
      "protocol": 1,
      "id": "com.deepseek.dshtray",
      "platform": "win32",
      "arch": "x64",
      "kind": "nsis-current-user"
    }
  },
  "scripts": {
    "bundle": "tsdown",
    "build:client": "vite build --config vite.client.config.ts",
    "verify": "node scripts/verify-package.mjs"
  }
}
```

`dsh.companion.json` is generated only after the final installer is copied and signed; its `sha256` must contain exactly 64 lowercase hexadecimal characters and its `sizeBytes` must be read from that same copied file. The source `package.json` carries no asset digest or byte count, so a stale release value cannot be published accidentally.

- [ ] **Step 4: Run test to verify pass**

Run: `pnpm exec vitest run packages/dshtray-plugin/src/test/package-manifest.test.ts`

Expected: PASS and zero package lifecycle scripts that can execute an installer.

- [ ] **Step 5: Commit**

```bash
git add package.json pnpm-workspace.yaml packages/dshtray-plugin
 git commit -m "feat: declare dshtray companion package"
```

### Task 2: Add the read-only Windows status handshake

**Objective:** 让 Host 能在不依赖 HTTP 控制接口的情况下确认 `id/protocol/version/pid/managedMode`。

**Files:**
- Create: `src-tauri/src/companion.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml`
- Test: `src-tauri/src/companion.rs` unit tests

**Interfaces:**
- Request: `{"protocol":1,"op":"status"}\n`。
- Response: `{"protocol":1,"id":"com.deepseek.dshtray","version":"0.1.0","pid":1234,"managedMode":"owned","running":true}\n`。
- Named pipe: `\\.\\pipe\\dshtray-com.deepseek.dshtray.v1`，ACL 仅允许当前交互用户；只读操作集合为 `{status}`。

- [ ] **Step 1: Write the failing Rust test**

```rust
#[test]
fn status_protocol_rejects_every_non_status_operation() {
    assert!(parse_status_request(br#"{"protocol":1,"op":"status"}"#).is_ok());
    assert!(parse_status_request(br#"{"protocol":1,"op":"stop"}"#).is_err());
    assert!(parse_status_request(br#"{"protocol":2,"op":"status"}"#).is_err());
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml companion::tests::status_protocol_rejects_every_non_status_operation`

Expected: FAIL because `parse_status_request` and the pipe server do not exist.

- [ ] **Step 3: Implement only the read-only bridge**

```rust
#[derive(Deserialize)]
struct StatusRequest { protocol: u8, op: String }

fn parse_status_request(bytes: &[u8]) -> Result<StatusRequest, CompanionError> {
    let request: StatusRequest = serde_json::from_slice(bytes)?;
    if request.protocol != 1 || request.op != "status" {
        return Err(CompanionError::InvalidRequest);
    }
    Ok(request)
}
```

Create the named pipe during application setup, use a current-user security descriptor, cap one request and one response to a fixed byte limit, close the client after the response, and never deserialize a command or executable path from the request.

- [ ] **Step 4: Run test to verify pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml companion::tests`

Expected: PASS; no test may mention a stop/restart/kill command as an accepted operation.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/companion.rs src-tauri/src/lib.rs
 git commit -m "feat: add dshtray read-only status handshake"
```

### Task 3: Register the fixed manager URI entry

**Objective:** 在 G1 获得批准后，让 Web 的“打开 DSHtray 管理器”只触发固定 URI，而不是执行浏览器提供的路径或参数。

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/tauri.conf.json`
- Create: `src-tauri/windows/installer-hooks.nsh`
- Modify: `src-tauri/src/companion.rs`
- Test: `src-tauri/src/companion.rs` URI parser tests and Windows acceptance script

**Interfaces:**
- Accepted URI: exactly `dshtray://open` (case-insensitive scheme only after URI parsing; path must equal `/open`; no query/fragment/userinfo).
- Registry scope: `HKCU\\Software\\Classes\\dshtray`; uninstaller removes only the value it created and only when the installation receipt says it owns it.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn manager_uri_accepts_only_the_fixed_open_route() {
    assert!(parse_manager_uri("dshtray://open").is_ok());
    assert!(parse_manager_uri("dshtray://open?command=stop").is_err());
    assert!(parse_manager_uri("dshtray://C:/Windows/System32/cmd.exe").is_err());
    assert!(parse_manager_uri("https://example.com").is_err());
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml companion::tests::manager_uri_accepts_only_the_fixed_open_route`

Expected: FAIL because no URI parser or registry hook exists.

- [ ] **Step 3: Implement the fixed URI path**

```rust
fn parse_manager_uri(value: &str) -> Result<(), CompanionError> {
    let uri = url::Url::parse(value).map_err(|_| CompanionError::InvalidUri)?;
    if uri.scheme() != "dshtray" || uri.host_str() != Some("open")
        || uri.path() != "" || uri.query().is_some() || uri.fragment().is_some()
    {
        return Err(CompanionError::InvalidUri);
    }
    Ok(())
}
```

Use the current Tauri major's deep-link plugin/configuration and a current-user NSIS hook. Do not place an unescaped user path into the registry command; pass the fixed URI to the already installed executable and let the app discard every URI except the one route above.

- [ ] **Step 4: Run test to verify pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml companion::tests` and `pnpm tauri build --debug`

Expected: URI parser tests PASS; debug installer builds without requesting elevation. A manual registry read must show only HKCU entries created by this installer.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/tauri.conf.json src-tauri/windows src-tauri/src/companion.rs
 git commit -m "feat: register dshtray manager deep link"
```

### Task 4: Build and verify the package-contained installer

**Objective:** 将实际 NSIS 资产复制到包内，生成真实 digest/size，并在发布前拒绝 unsigned 或错误发布者资产。

**Files:**
- Create: `scripts/package-companion.ps1`
- Modify: `scripts/verify-release.ps1`
- Modify: `package.json`
- Create: `scripts/tests/package-manifest.test.mjs`
- Test: `scripts/tests/package-manifest.test.mjs`

**Interfaces:**
- Input: `src-tauri/target/release/bundle/nsis/DSHtray_0.1.0_x64-setup.exe`。
- Output: `packages/dshtray-plugin/assets/DSHtray_0.1.0_x64-setup.exe` and `packages/dshtray-plugin/dsh.companion.json`。
- Verification result must include `path`, `sizeBytes`, `sha256`, `signatureStatus`, `publisherSubject`; it must never print a certificate private key or credential value.

- [ ] **Step 1: Write the failing manifest test**

```js
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFileSync, statSync } from 'node:fs'
import test from 'node:test'

test('rejects a companion manifest whose digest or size differs from the asset', () => {
  const asset = 'packages/dshtray-plugin/assets/DSHtray_0.1.0_x64-setup.exe'
  const manifest = JSON.parse(readFileSync('packages/dshtray-plugin/dsh.companion.json', 'utf8'))
  const digest = createHash('sha256').update(readFileSync(asset)).digest('hex')
  assert.equal(manifest.sizeBytes, statSync(asset).size)
  assert.equal(manifest.sha256, digest)
  assert.match(manifest.sha256, /^[0-9a-f]{64}$/)
})
```

- [ ] **Step 2: Run test to verify failure**

Run: `node --test scripts/tests/package-manifest.test.mjs`

Expected: FAIL because the package asset and generated manifest are not present.

- [ ] **Step 3: Implement the packaging script**

```powershell
$ErrorActionPreference = 'Stop'
$source = Join-Path $PSScriptRoot '..\src-tauri\target\release\bundle\nsis\DSHtray_0.1.0_x64-setup.exe'
$destination = Join-Path $PSScriptRoot '..\packages\dshtray-plugin\assets\DSHtray_0.1.0_x64-setup.exe'
New-Item -ItemType Directory -Force (Split-Path $destination) | Out-Null
Copy-Item -LiteralPath $source -Destination $destination -Force
$hash = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant()
$size = (Get-Item -LiteralPath $destination).Length
$signature = Get-AuthenticodeSignature -LiteralPath $destination
if ($signature.Status -ne 'Valid') { throw "companion installer signature is $($signature.Status)" }
if ($signature.SignerCertificate.Subject -ne $env:DSHTRAY_SIGNER_SUBJECT) { throw 'companion signer subject mismatch' }
@{
  protocol = 1; id = 'com.deepseek.dshtray'; platform = 'win32'; arch = 'x64'; version = '0.1.0'
  kind = 'nsis-current-user'; asset = 'assets/DSHtray_0.1.0_x64-setup.exe'; sizeBytes = $size; sha256 = $hash
  expectedExecutable = 'DSHtray.exe'; expectedUninstaller = 'uninstall.exe'; requiresElevation = $false
  silentArgs = @('/S'); startAfterInstall = $true
} | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path (Split-Path $destination) '..\dsh.companion.json') -Encoding utf8NoBOM
```

The release script may use PowerShell for packaging-time Authenticode inspection; runtime Host verification remains the native adapter required by G2. The script must fail closed when `DSHTRAY_SIGNER_SUBJECT` is missing instead of accepting `NotSigned`.

- [ ] **Step 4: Run test to verify pass**

Run: `pnpm tauri build` then `pnpm run package:companion` with `DSHTRAY_SIGNER_SUBJECT` supplied by the approved signing environment, followed by `node --test scripts/tests/package-manifest.test.mjs`

Expected: unsigned current `v0.1.0` asset fails with `companion installer signature is NotSigned`; a newly signed asset produces a 64-character digest and a matching size.

- [ ] **Step 5: Commit**

```bash
git add package.json scripts/package-companion.ps1 scripts/verify-release.ps1 scripts/tests/package-manifest.test.mjs
 git commit -m "build: package signed dshtray companion asset"
```

### Task 5: Add the read-only DSH client bundle

**Objective:** 让 Web 插件显示 Host 返回的状态和管理归属，且在编译产物中不存在生命周期控制按钮。

**Files:**
- Create: `packages/dshtray-plugin/src/index.ts`
- Create: `packages/dshtray-plugin/src/status.ts`
- Create: `packages/dshtray-plugin/src/client/index.tsx`
- Create: `packages/dshtray-plugin/src/client/DshtrayStatusSection.tsx`
- Create: `packages/dshtray-plugin/src/client/DshtrayStatusSection.test.tsx`
- Create: `packages/dshtray-plugin/tsconfig.json`
- Create: `packages/dshtray-plugin/vite.client.config.ts`
- Test: `packages/dshtray-plugin/src/client/DshtrayStatusSection.test.tsx`

**Interfaces:**

```ts
export type DshtrayCompanionState = 'not-installed' | 'external' | 'installed' | 'incompatible' | 'failed'

export interface DshtrayCompanionStatus {
  protocol: 1
  id: 'com.deepseek.dshtray'
  state: DshtrayCompanionState
  version: string | null
  installPath: string | null
  running: boolean | null
  managed: boolean
  publisher: string | null
  detail: string | null
}
```

The Host service name is exactly `dshtrayCompanion`; its only remote method is `status(): Promise<DshtrayCompanionStatus>`. The client may render an `<a href="dshtray://open">` only when `state === 'installed'`; it must not call an install or process-control method.

- [ ] **Step 1: Write the failing UI test**

```tsx
it('shows external ownership and no process-control actions', async () => {
  render(<DshtrayStatusSection status={{
    protocol: 1, id: 'com.deepseek.dshtray', state: 'external', version: '0.1.0',
    installPath: 'C:/Users/Tony/AppData/Local/DSHtray', running: true,
    managed: false, publisher: null, detail: 'receipt missing'
  }} />)
  expect(screen.getByText('等待确认接管')).toBeInTheDocument()
  expect(screen.queryByRole('button', { name: /停止|重启|接管|终止/ })).toBeNull()
  expect(screen.queryByRole('link', { name: /打开 DSHtray/ })).toBeNull()
})
```

- [ ] **Step 2: Run test to verify failure**

Run: `pnpm exec vitest run packages/dshtray-plugin/src/client/DshtrayStatusSection.test.tsx`

Expected: FAIL because the package client files do not exist.

- [ ] **Step 3: Implement the minimum Host/client path**

```tsx
export function DshtrayStatusSection({ status }: { status: DshtrayCompanionStatus }) {
  return <section aria-label="DSHtray">
    <strong>{status.state}</strong>
    {status.version !== null && <span>{status.version}</span>}
    {status.state === 'installed' && <a href="dshtray://open">打开 DSHtray 管理器</a>}
  </section>
}
```

Host code reads the fixed named pipe and the owned receipt; it maps missing receipt plus an existing executable to `external`, never auto-adopts it, and keeps the status remote read-only. Use the DSH package pattern from `packages/client/ui-settings-general` for the `dsh.client.inject` list and React peer version.

- [ ] **Step 4: Run test and bundle verification**

Run: `pnpm exec vitest run packages/dshtray-plugin/src/client/DshtrayStatusSection.test.tsx` and `pnpm --filter @gittuyn/dshtray-plugin run bundle`

Expected: PASS; client bundle contains the fixed URI string and no `stop`, `restart`, `kill`, or `terminate` action export.

- [ ] **Step 5: Commit**

```bash
git add packages/dshtray-plugin/src packages/dshtray-plugin/tsconfig.json packages/dshtray-plugin/vite.client.config.ts
 git commit -m "feat: expose read-only dshtray web status"
```

### Task 6: Verify package contents and release acceptance

**Objective:** 在任何 npm publish 或 DSH profile 安装前，证明 tarball 只包含预期文件、签名资产和无安装生命周期脚本。

**Files:**
- Create: `packages/dshtray-plugin/scripts/verify-package.mjs`
- Modify: `package.json`
- Modify: `scripts/verify-release.ps1`
- Create: `docs/superpowers/acceptance/dshtray-companion-release.md`
- Test: `packages/dshtray-plugin/src/test/package-tarball.test.ts`

- [ ] **Step 1: Write the failing tarball test**

```ts
import { execFileSync } from 'node:child_process'
import { readdirSync, rmSync, mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'

function readTarballFileNames(file: string): string[] {
  const tar = process.platform === 'win32' ? 'tar.exe' : 'tar'
  return execFileSync(tar, ['-tf', file], { encoding: 'utf8' }).trim().split(/\r?\n/).filter(Boolean)
}

it('contains only package runtime, client, manifest, patch, and signed asset', () => {
  const packageDir = join(process.cwd(), 'packages', 'dshtray-plugin')
  const outputDir = mkdtempSync(join(tmpdir(), 'dshtray-pack-'))
  try {
    const pnpm = process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm'
    execFileSync(pnpm, ['pack', '--pack-destination', outputDir], { cwd: packageDir, encoding: 'utf8' })
    const archive = readdirSync(outputDir).find(name => name.endsWith('.tgz'))
    if (archive === undefined) throw new Error('pnpm pack produced no .tgz archive')
    const files = readTarballFileNames(join(outputDir, archive))
    expect(files).toEqual(expect.arrayContaining([
      'package/lib/index.js', 'package/client/client.js', 'package/dsh.companion.json',
      'package/cordis.patch.yml', 'package/assets/DSHtray_0.1.0_x64-setup.exe'
    ]))
    expect(files.some(name => /(^|\/)(scripts|test|\.env)/.test(name))).toBe(false)
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})
```

- [ ] **Step 2: Run test to verify failure**

Run: `pnpm exec vitest run packages/dshtray-plugin/src/test/package-tarball.test.ts`

Expected: FAIL until the package build and verifier exist.

- [ ] **Step 3: Implement allowlist verification**

```js
const allowed = /^(package\/(lib|client|assets)\/|package\/(dsh\.companion\.json|cordis\.patch\.yml|package\.json))$/
for (const name of files) {
  if (!allowed.test(name)) throw new Error(`unexpected tarball entry: ${name}`)
}
const pkg = JSON.parse(readFileSync('package/package.json', 'utf8'))
if (pkg.scripts?.postinstall || pkg.scripts?.prepare) throw new Error('package lifecycle install script is forbidden')
```

Add acceptance cases for clean first install, user decline, wrong hash, unsigned asset, publisher mismatch, external executable without receipt, owned upgrade, failed upgrade rollback, Web-only removal, explicit native removal, and URI/pipe read-back. Do not run the real installer in CI; use a signed staging asset and a Windows manual gate for UAC/Start Menu/receipt.

- [ ] **Step 4: Run final package gates**

Run: `pnpm run typecheck`, `pnpm test -- packages/dshtray-plugin`, `pnpm run build`, `pnpm pack --dry-run` and `pwsh -NoProfile -File scripts/verify-release.ps1 -RequireSignature -PublisherSubject $env:DSHTRAY_SIGNER_SUBJECT`

Expected: source and client checks PASS; `pack --dry-run` lists only the allowlisted files; release verification exits 0 only for a valid signed asset. With the current `NotSigned` asset it must exit non-zero.

- [ ] **Step 5: Commit**

```bash
git add package.json packages/dshtray-plugin scripts/verify-release.ps1 docs/superpowers/acceptance/dshtray-companion-release.md
 git commit -m "test: gate dshtray companion release artifacts"
```

## Completion Checklist

- [ ] G0–G3 have explicit approvals/values.
- [ ] Named pipe accepts only `protocol=1, op=status` and is current-user scoped.
- [ ] URI handler accepts only `dshtray://open`.
- [ ] Package contains no install lifecycle script and no credential value.
- [ ] Release script rejects the current unsigned asset.
- [ ] Signed staging asset passes size/hash/publisher/package allowlist checks.
- [ ] DSHtray/Tauri tests, package tests, and manual Windows acceptance evidence are attached before release.
