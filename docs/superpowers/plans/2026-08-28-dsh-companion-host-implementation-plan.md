# DSH CLI Native Companion Host Coordinator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 DSH CLI/Host 中实现可信的 Native Companion 事务协调器，使 Web 插件安装只有在原生资产校验、用户确认、启动和版本握手全部满足时才算成功。

**Architecture:** 保留现有 `apps/cli/src/plugin.ts` 的 pnpm/profile 工作流，在其外围增加 `companion.ts` 协调器；纯 manifest/路径/receipt 逻辑与 Windows 平台执行逻辑分离，所有外部动作通过注入的受限 adapter 完成。Web profile 变更和 current-user NSIS 安装构成一个可回滚事务，外部已有但无匹配 receipt 的 DSHtray 永远是 observe-only。

**Tech Stack:** TypeScript/Node 22+, Commander 参数解析，Node `fs`/`crypto`/`child_process`，Windows native WinVerifyTrust adapter，Vitest，DSH CLI 现有 `pnpm`/profile manifest 代码。

**Spec:** `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/docs/superpowers/specs/2026-08-28-dsh-plugin-companion-install-design.md`

## Global Constraints

- 命令必须保留 `dsh plugin --profile web add @gittuyn/dshtray-plugin`，非 TTY 自动安装必须额外提供 `--accept-native-companion`。
- 原生伴侣协议必须是 `1`；id 必须是 `com.deepseek.dshtray`；平台必须是 `win32`；架构必须是 `x64`；安装类型必须是 `nsis-current-user`。
- 只允许包内 `assets/DSHtray_0.1.0_x64-setup.exe`、固定 `/S` 参数和 current-user 目标；不得信任浏览器传入的路径、参数或下载地址。
- 不得通过 `postinstall`、`prepare`、PowerShell、`cmd /c`、`shell: true` 或字符串拼接命令安装/启动安装器；安装器必须用直接文件执行并传入参数数组。
- 运行时签名检查必须使用 G2 批准的原生 WinVerifyTrust/等效 adapter；未签名、发布者不匹配、大小/hash 不匹配都必须 fail closed。
- DSHtray 已存在但 receipt 缺失、id/version/hash/path/publisher 不匹配时必须标记 `external`，不得自动覆盖、接管、卸载或强杀。
- 升级不得结束正在运行的 DSHtray 或 DSH；旧安装器必须进入可验证 cache，失败必须恢复 profile/receipt/asset，或明确返回 `half-complete`。
- 不保存、上传或转发任何 API 密钥、令牌、密码或其他凭据值。
- Host 只能调用 DSHtray 的声明式安装/状态握手，不得替代或削弱现有 Rust `Job Object`、精确 PID 树和进程身份校验；不得提供无保护的本机 HTTP 控制接口。
- DSHtray 的默认代理语义 `http://127.0.0.1:7897` 由原生伴侣保留；Host 不改系统代理、不把代理或凭据写入 receipt，并只展示状态。

## Execution Gates

- **H0 — CLI flags:** 执行前确认 `--accept-native-companion` 和 `--remove-native-companion` 作为 DSH plugin-level flags，且不会被转发给 pnpm。
- **H1 — Signature adapter:** 必须选定并审计不依赖 PowerShell/cmd 的 WinVerifyTrust 实现；没有 H1 不能实现自动安装。
- **H2 — Receipt path:** 默认使用 `%LOCALAPPDATA%\\DSH\\companions\\com.deepseek.dshtray\\receipt.json`，安装目录默认 `%LOCALAPPDATA%\\DSHtray`；若发行版已有其他 current-user 路径，执行前修改常量和验收用例。
- **H3 — Existing profile state:** DSH 仓库当前存在既有未跟踪文件；实现时只能 stage 明确列出的 CLI 文件和测试文件，不得使用 `git add .`。

## Verified Baseline and File Map

- Existing parser: `C:/Users/Tony/Documents/Default Project/deepseek-harness/apps/cli/src/args.ts`。
- Existing dispatch: `C:/Users/Tony/Documents/Default Project/deepseek-harness/apps/cli/src/bin.ts`。
- Existing plugin flow: `C:/Users/Tony/Documents/Default Project/deepseek-harness/apps/cli/src/plugin.ts`。
- Existing profile boot conventions: `C:/Users/Tony/Documents/Default Project/deepseek-harness/apps/cli/src/profile-boot.ts`。
- Existing CLI tests: `apps/cli/tests/args.spec.ts`, `apps/cli/tests/built-bin.e2e.ts`, `apps/cli/tests/windows-shell.spec.ts`。
- Create `apps/cli/src/companion.ts`: public transaction coordinator and shared result/state types。
- Create `apps/cli/src/companion-manifest.ts`: unknown-to-validated manifest parser and path/size/hash checks。
- Create `apps/cli/src/companion-receipt.ts`: receipt/cache paths, atomic JSON write/read, ownership comparison。
- Create `apps/cli/src/companion-win32.ts`: native signature adapter seam, direct installer spawn, named-pipe handshake client, uninstall launcher。
- Modify `apps/cli/src/args.ts`, `apps/cli/src/bin.ts`, `apps/cli/src/plugin.ts`: parse flags, preserve pnpm args, wrap install/update/remove transaction。
- Create `apps/cli/tests/companion-manifest.spec.ts`, `apps/cli/tests/companion-receipt.spec.ts`, `apps/cli/tests/companion-transaction.spec.ts`, `apps/cli/tests/companion-win32.spec.ts`。
- Create `apps/cli/tests/companion-test-helpers.ts`: 统一的临时 profile、manifest、receipt 和 fake platform helper，避免每个测试文件自行实现一套副作用替身。
- Modify `apps/cli/tests/args.spec.ts` and `apps/cli/tests/built-bin.e2e.ts` for public CLI behavior。
- Create `apps/cli/tests/fixtures/companion-package/package.json`, `dsh.companion.json`, and a deterministic fake installer file; fixture contains no credential value。

## Shared Interfaces

These names are the cross-plan contract consumed by the DSHtray package and dshmarket adapter:

```ts
export const COMPANION_ID = 'com.deepseek.dshtray' as const
export const COMPANION_PROTOCOL = 1 as const
export const COMPANION_PACKAGE = '@gittuyn/dshtray-plugin' as const
export const COMPANION_INSTALL_ARGS = ['/S'] as const

export type CompanionState =
  | 'not-installed' | 'asset-invalid' | 'signature-invalid' | 'consent-required'
  | 'external' | 'installing' | 'installed' | 'handshake-failed'
  | 'rollback' | 'half-complete' | 'uninstalled'

export interface NativeCompanionOptions {
  acceptNativeCompanion: boolean
  removeNativeCompanion: boolean
}

export interface CompanionResult {
  state: CompanionState
  id: typeof COMPANION_ID
  version: string | null
  managed: boolean
  detail: string | null
}

export interface CompanionPlatform {
  verifyAuthenticode(assetPath: string, expectedPublisher: string): Promise<{ valid: boolean; publisher: string | null; detail: string }>
  launchInstaller(assetPath: string, args: readonly string[]): Promise<{ exitCode: number; timedOut: boolean }>
  queryStatus(): Promise<{ protocol: 1; id: typeof COMPANION_ID; version: string; pid: number; managedMode: 'owned' | 'external' } | null>
  launchUninstaller(path: string): Promise<{ exitCode: number; timedOut: boolean }>
}

export interface PluginRunOptions {
  acceptNativeCompanion?: boolean
  removeNativeCompanion?: boolean
}
```

## Test helper contract

`apps/cli/tests/companion-test-helpers.ts` supplies the fixtures consumed by
Tasks 3–6. It uses Node's `mkdtempSync`, `join`, `readFileSync`, and
`rmSync` in `beforeEach`/`afterEach`; it never invokes a real installer:

```ts
export const fixtureRoot: string
export const fixtureProfile: string
export function validReceipt(root: string): CompanionReceipt
export function fakePlatform(overrides?: {
  calls?: Array<{ file: string; args: readonly string[] }>
  signature?: { valid: boolean; publisher: string | null; detail: string }
}): CompanionPlatform & { uninstallCalls: string[] }
export function snapshotProfile(profileDir: string): string
export function readProfileManifest(profileDir: string): string
```

`fakePlatform` returns a successful owned status for the fixed protocol/id,
records installer/uninstaller calls, and accepts an explicit signature result;
`validReceipt(root)` uses `join(root, 'DSHtray')` and the fixed companion id,
package name, protocol, version, executable and publisher fields. Every test
that imports these helpers removes `fixtureRoot` in `afterEach`; tests that
need a real built binary continue to use the local `runBuiltBin` helper already
defined in `apps/cli/tests/built-bin.e2e.ts`.

### Task 1: Parse and isolate plugin-level companion flags

**Objective:** 让 DSH CLI 能表达用户确认/显式原生卸载，同时保证两个 flag 不会流入 pnpm。

**Files:**
- Modify: `apps/cli/src/args.ts`
- Modify: `apps/cli/src/bin.ts`
- Modify: `apps/cli/src/plugin.ts`
- Modify: `apps/cli/tests/args.spec.ts`
- Create: `apps/cli/tests/companion-cli-flags.spec.ts`

**Interfaces:**
- `parseDshArgs` produces `PluginInvocation.companion: NativeCompanionOptions`。
- `runPlugin(profile, args, options?: PluginRunOptions): Promise<number>` preserves the existing numeric exit-code contract。

- [ ] **Step 1: Write the failing test**

```ts
it('parses companion flags without forwarding them to pnpm', () => {
  const parsed = parseDshArgs([
    'plugin', '--profile', 'web', '--accept-native-companion', 'add', '@gittuyn/dshtray-plugin'
  ])
  expect(parsed.plugin.companion).toEqual({ acceptNativeCompanion: true, removeNativeCompanion: false })
  expect(parsed.plugin.args).toEqual(['add', '@gittuyn/dshtray-plugin'])
})

it('rejects both companion flags together', () => {
  expect(() => parseDshArgs([
    'plugin', '--profile', 'web', '--accept-native-companion', '--remove-native-companion', 'remove', '@gittuyn/dshtray-plugin'
  ])).toThrow(/mutually exclusive/)
})
```

- [ ] **Step 2: Run test to verify failure**

Run: `pnpm exec vitest run apps/cli/tests/companion-cli-flags.spec.ts`

Expected: FAIL because `PluginInvocation` has no companion options.

- [ ] **Step 3: Implement the parser seam**

```ts
export interface PluginRunOptions {
  acceptNativeCompanion?: boolean
  removeNativeCompanion?: boolean
}

// plugin command definition
.option('--accept-native-companion', 'confirm the package-owned native companion')
.option('--remove-native-companion', 'explicitly remove the package-owned native companion')
```

Extract these options before calling the existing pnpm argument builder. Reject `accept` on `remove` and `remove` on `add/update`; preserve every other argument byte-for-byte.

- [ ] **Step 4: Run test to verify pass**

Run: `pnpm exec vitest run apps/cli/tests/args.spec.ts apps/cli/tests/companion-cli-flags.spec.ts`

Expected: PASS; a spy around the pnpm invocation receives no companion flag.

- [ ] **Step 5: Commit**

```bash
git add apps/cli/src/args.ts apps/cli/src/bin.ts apps/cli/src/plugin.ts apps/cli/tests/args.spec.ts apps/cli/tests/companion-cli-flags.spec.ts
 git commit -m "feat: parse native companion plugin flags"
```

### Task 2: Validate manifest and prevent path/argument escapes

**Objective:** 将包内 unknown JSON 转成严格 manifest，并在任何文件或进程动作前拒绝路径穿越、绝对路径和非固定参数。

**Files:**
- Create: `apps/cli/src/companion-manifest.ts`
- Create: `apps/cli/tests/companion-manifest.spec.ts`
- Create: `apps/cli/tests/fixtures/companion-package/dsh.companion.json`

**Interfaces:**

```ts
export interface NativeCompanionManifest {
  protocol: 1
  id: 'com.deepseek.dshtray'
  platform: 'win32'
  arch: 'x64'
  version: string
  kind: 'nsis-current-user'
  asset: string
  sizeBytes: number
  sha256: string
  expectedExecutable: 'DSHtray.exe'
  expectedUninstaller: 'uninstall.exe'
  requiresElevation: false
  silentArgs: ['/S']
  startAfterInstall: true
}

export function parseCompanionManifest(raw: unknown, packageRoot: string): NativeCompanionManifest
export function resolveCompanionAsset(manifest: NativeCompanionManifest, packageRoot: string): string
```

- [ ] **Step 1: Write the failing test**

```ts
function validManifest(): NativeCompanionManifest {
  return {
    protocol: 1,
    id: 'com.deepseek.dshtray',
    platform: 'win32',
    arch: 'x64',
    version: '0.1.0',
    kind: 'nsis-current-user',
    asset: 'assets/DSHtray_0.1.0_x64-setup.exe',
    sizeBytes: 1,
    sha256: '0'.repeat(64),
    expectedExecutable: 'DSHtray.exe',
    expectedUninstaller: 'uninstall.exe',
    requiresElevation: false,
    silentArgs: ['/S'],
    startAfterInstall: true,
  }
}

it.each([
  ['absolute asset', { asset: 'C:/evil.exe' }],
  ['traversal asset', { asset: '../evil.exe' }],
  ['wrong args', { silentArgs: ['/S', '/D=C:/x'] }],
  ['elevation', { requiresElevation: true }],
  ['wrong id', { id: 'other' }],
] as Array<[string, Record<string, unknown>]>)('rejects %s', (_name, patch) => {
  expect(() => parseCompanionManifest({ ...validManifest(), ...patch }, '/profile/node_modules/pkg')).toThrow()
})
```

- [ ] **Step 2: Run test to verify failure**

Run: `pnpm exec vitest run apps/cli/tests/companion-manifest.spec.ts`

Expected: FAIL because the parser does not exist.

- [ ] **Step 3: Implement strict validation**

```ts
export function resolveCompanionAsset(manifest: NativeCompanionManifest, packageRoot: string): string {
  const root = resolve(packageRoot)
  const asset = resolve(root, manifest.asset)
  if (relative(root, asset).startsWith('..') || isAbsolute(relative(root, asset))) {
    throw new Error('companion asset escapes package root')
  }
  return asset
}
```

Validate exact protocol/id/platform/arch/kind/args/booleans, positive safe integer size, lowercase 64-hex digest, basename expectations, and `packageRoot` containment. Hash and size are checked after file read, before signature verification.

- [ ] **Step 4: Run test to verify pass**

Run: `pnpm exec vitest run apps/cli/tests/companion-manifest.spec.ts`

Expected: PASS for the valid fixture and every malicious path/field case.

- [ ] **Step 5: Commit**

```bash
git add apps/cli/src/companion-manifest.ts apps/cli/tests/companion-manifest.spec.ts apps/cli/tests/fixtures/companion-package/dsh.companion.json
 git commit -m "feat: validate native companion manifests"
```

### Task 3: Add receipt, ownership and rollback storage

**Objective:** 用原子 receipt 和安装器 cache 区分 DSH 所有资产与外部 DSHtray，并让升级失败有真实旧资产可恢复。

**Files:**
- Create: `apps/cli/src/companion-receipt.ts`
- Create: `apps/cli/tests/companion-receipt.spec.ts`
- Create: `apps/cli/tests/fixtures/receipt.json`

**Interfaces:**

```ts
export interface CompanionReceipt {
  protocol: 1
  id: 'com.deepseek.dshtray'
  version: string
  installPath: string
  executable: string
  installerSha256: string
  publisher: string
  managedBy: 'dsh'
  packageName: '@gittuyn/dshtray-plugin'
  installedAt: string
}

export function receiptPath(id: string, env?: NodeJS.ProcessEnv): string
export function readReceipt(id: string, env?: NodeJS.ProcessEnv): CompanionReceipt | null
export function writeReceipt(receipt: CompanionReceipt, env?: NodeJS.ProcessEnv): void
export function receiptsMatch(actual: CompanionReceipt | null, expected: Pick<CompanionReceipt, 'id' | 'version' | 'installerSha256' | 'installPath'>): boolean
export function cacheInstaller(id: string, installerPath: string, version: string, env?: NodeJS.ProcessEnv): string
```

- [ ] **Step 1: Write the failing test**

```ts
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { fixtureRoot, validReceipt } from './companion-test-helpers'

it('classifies a present executable without a matching receipt as external', () => {
  const receipt = readReceipt('com.deepseek.dshtray', { LOCALAPPDATA: fixtureRoot })
  expect(receiptsMatch(receipt, {
    id: 'com.deepseek.dshtray', version: '0.1.0',
    installerSha256: '0'.repeat(64), installPath: join(fixtureRoot, 'DSHtray')
  })).toBe(false)
})

it('writes receipt atomically and never includes credential-shaped fields', () => {
  writeReceipt(validReceipt(fixtureRoot), { LOCALAPPDATA: fixtureRoot })
  const text = readFileSync(receiptPath('com.deepseek.dshtray', { LOCALAPPDATA: fixtureRoot }), 'utf8')
  expect(text).not.toMatch(/api[_-]?key|token|password|secret/i)
})
```

- [ ] **Step 2: Run test to verify failure**

Run: `pnpm exec vitest run apps/cli/tests/companion-receipt.spec.ts`

Expected: FAIL because receipt helpers do not exist.

- [ ] **Step 3: Implement atomic receipt/cache helpers**

```ts
export function writeReceipt(receipt: CompanionReceipt, env = process.env): void {
  const target = receiptPath(receipt.id, env)
  mkdirSync(dirname(target), { recursive: true })
  const temp = `${target}.${process.pid}.tmp`
  writeFileSync(temp, JSON.stringify(receipt) + '\n', { encoding: 'utf8', flag: 'wx' })
  renameSync(temp, target)
}
```

Use a fixed `%LOCALAPPDATA%\\DSH\\companions\\<id>` path, reject symlinked receipt/cache parents where the Windows adapter can inspect them, and retain the previous installer before any upgrade. Do not write credentials or arbitrary package metadata into the receipt.

- [ ] **Step 4: Run test to verify pass**

Run: `pnpm exec vitest run apps/cli/tests/companion-receipt.spec.ts`

Expected: PASS; receipt write is atomic, malformed receipt reads as `null`, and missing receipt plus executable is `external`.

- [ ] **Step 5: Commit**

```bash
git add apps/cli/src/companion-receipt.ts apps/cli/tests/companion-receipt.spec.ts apps/cli/tests/fixtures/receipt.json
 git commit -m "feat: track owned companion receipts"
```

### Task 4: Implement the Windows adapter seam and native verification gate

**Objective:** 让所有真实 Windows 副作用集中在一个受限 adapter 中，直接执行固定 installer，并在 G2/H1 未满足时拒绝自动安装。

**Files:**
- Create: `apps/cli/src/companion-win32.ts`
- Create: `apps/cli/tests/companion-win32.spec.ts`
- Modify: `apps/cli/package.json` only if the approved native adapter requires a declared runtime dependency

**Interfaces:**

```ts
export function createWin32CompanionPlatform(): CompanionPlatform

export function spawnFixedInstaller(
  installerPath: string,
  args: readonly ['/S'],
): Promise<{ exitCode: number; timedOut: boolean }>
```

- [ ] **Step 1: Write the failing test**

```ts
import { fakePlatform } from './companion-test-helpers'

it('passes only the fixed silent argument to the direct process runner', async () => {
  const calls: Array<{ file: string; args: readonly string[] }> = []
  const platform = fakePlatform({ calls })
  await platform.launchInstaller('C:/profile/assets/DSHtray_0.1.0_x64-setup.exe', ['/S'])
  expect(calls).toEqual([{
    file: 'C:/profile/assets/DSHtray_0.1.0_x64-setup.exe', args: ['/S']
  }])
})

it('does not install when native signature verification is unavailable', async () => {
  const platform = fakePlatform({ signature: { valid: false, publisher: null, detail: 'WinVerifyTrust unavailable' } })
  await expect(platform.verifyAuthenticode('asset.exe', 'approved-subject')).resolves.toMatchObject({ valid: false })
})
```

- [ ] **Step 2: Run test to verify failure**

Run: `pnpm exec vitest run apps/cli/tests/companion-win32.spec.ts`

Expected: FAIL because the adapter and direct spawn wrapper do not exist.

- [ ] **Step 3: Implement the minimal direct process adapter**

```ts
const child = spawn(installerPath, ['/S'], {
  shell: false,
  windowsHide: true,
  stdio: ['ignore', 'ignore', 'pipe'],
})
```

Reject every argument except the literal `'/S'`, require an absolute installer path resolved under the package root, set a bounded timeout, and never use PowerShell/cmd. Implement `verifyAuthenticode` with the approved native WinVerifyTrust adapter from H1/G2; do not silently downgrade to hash-only verification.

- [ ] **Step 4: Run test to verify pass**

Run: `pnpm exec vitest run apps/cli/tests/companion-win32.spec.ts`

Expected: PASS; test output proves exact file/args and nonzero/timeout behavior. On a Windows host, add a native adapter smoke test that verifies a known signed system binary and rejects an unsigned fixture without installing it.

- [ ] **Step 5: Commit**

```bash
git add apps/cli/src/companion-win32.ts apps/cli/tests/companion-win32.spec.ts apps/cli/package.json
 git commit -m "feat: isolate win32 companion execution"
```

### Task 5: Implement the install transaction around the existing pnpm flow

**Objective:** 让 add/update 只有在 Web package 和原生伴侣都成功时返回零退出码，并在失败时恢复 profile manifest。

**Files:**
- Create: `apps/cli/src/companion.ts`
- Modify: `apps/cli/src/plugin.ts`
- Create: `apps/cli/tests/companion-transaction.spec.ts`

**Interfaces:**

```ts
export async function installNativeCompanion(
  packageDirectory: string,
  expectedPublisher: string,
  platform: CompanionPlatform,
  options: NativeCompanionOptions,
): Promise<CompanionResult>

export async function coordinatePluginInstall(
  profile: string,
  pluginArgs: readonly string[],
  options: PluginRunOptions,
  dependencies?: { platform?: CompanionPlatform },
): Promise<number>
```

- [ ] **Step 1: Write the failing end-to-end transaction test**

```ts
import {
  fixtureProfile, fixtureRoot, readProfileManifest, snapshotProfile,
} from './companion-test-helpers'

it('restores the Web profile when the companion signature fails', async () => {
  const platform = fakePlatform({
    signature: { valid: false, publisher: null, detail: 'signature-invalid' },
  })
  const before = snapshotProfile(fixtureProfile)
  const result = await coordinatePluginInstall('web', ['add', '@gittuyn/dshtray-plugin'], {
    acceptNativeCompanion: true,
    removeNativeCompanion: false,
  }, { platform })
  expect(result).not.toBe(0)
  expect(readProfileManifest(fixtureProfile)).toEqual(before)
  expect(readReceipt('com.deepseek.dshtray', { LOCALAPPDATA: fixtureRoot })).toBeNull()
})
```

- [ ] **Step 2: Run test to verify failure**

Run: `pnpm exec vitest run apps/cli/tests/companion-transaction.spec.ts`

Expected: FAIL because plugin installation has no companion transaction.

- [ ] **Step 3: Implement the state machine**

```ts
const states = [
  'not-installed', 'asset-invalid', 'signature-invalid', 'consent-required',
  'external', 'installing', 'installed', 'handshake-failed', 'rollback', 'half-complete'
] as const
```

Sequence: snapshot profile/package/bundle state → run existing `pnpm` add/update → locate `dsh.companion` → validate asset/hash/size/signature/ownership → require interactive prompt or explicit non-TTY flag → cache prior installer → direct `/S` install → query named-pipe handshake → atomically write receipt → activate Web bundle. On failure restore the profile snapshot and receipt/cache; if a newly started DSHtray cannot handshake, do not force-terminate it, leave `half-complete`, disable Web activation, and report the exact manual recovery path.

Do not duplicate `restoreProfileManifest`; call the existing helper in `profile.ts` and preserve its current unportable-dependency warnings.

- [ ] **Step 4: Run test to verify pass**

Run: `pnpm exec vitest run apps/cli/tests/companion-transaction.spec.ts`

Expected: PASS for success, consent decline, non-TTY missing flag, hash/signature failure, external ownership, installer timeout, handshake mismatch, profile rollback, and no DSH/DSHtray force kill.

- [ ] **Step 5: Commit**

```bash
git add apps/cli/src/companion.ts apps/cli/src/plugin.ts apps/cli/tests/companion-transaction.spec.ts
 git commit -m "feat: coordinate plugin and native companion install"
```

### Task 6: Add upgrade, explicit native uninstall and half-complete handling

**Objective:** 覆盖已拥有版本的升级/回滚、Web-only remove 和显式原生卸载，不误伤外部 DSHtray。

**Files:**
- Modify: `apps/cli/src/companion.ts`
- Modify: `apps/cli/src/plugin.ts`
- Modify: `apps/cli/src/companion-receipt.ts`
- Modify: `apps/cli/tests/companion-transaction.spec.ts`
- Modify: `apps/cli/tests/built-bin.e2e.ts`

**Interfaces:**

```ts
export async function uninstallNativeCompanion(
  receipt: CompanionReceipt | null,
  platform: CompanionPlatform,
  explicit: boolean,
): Promise<CompanionResult>
```

- [ ] **Step 1: Write the failing tests**

```ts
import { fakePlatform } from './companion-test-helpers'

it('removes Web only when native removal was not explicit', async () => {
  const platform = fakePlatform()
  await coordinatePluginInstall('web', ['remove', '@gittuyn/dshtray-plugin'], {
    acceptNativeCompanion: false,
    removeNativeCompanion: false,
  }, { platform })
  expect(platform.uninstallCalls).toHaveLength(0)
})

it('refuses to uninstall an external companion', async () => {
  const result = await uninstallNativeCompanion(null, fakePlatform(), true)
  expect(result.state).toBe('external')
})
```

- [ ] **Step 2: Run test to verify failure**

Run: `pnpm exec vitest run apps/cli/tests/companion-transaction.spec.ts apps/cli/tests/built-bin.e2e.ts`

Expected: FAIL until remove/update logic distinguishes owned and external state.

- [ ] **Step 3: Implement owned-only update/remove**

```ts
if (!explicit) return { state: 'uninstalled', id: COMPANION_ID, version: null, managed: true, detail: null }
if (receipt === null || receipt.id !== COMPANION_ID || receipt.managedBy !== 'dsh') {
  return { state: 'external', id: COMPANION_ID, version: null, managed: false, detail: 'receipt mismatch' }
}
```

The coordinator must additionally confirm that the receipt's `installPath` is the fixed `%LOCALAPPDATA%\\DSHtray` location and that the receipt's executable and uninstaller are the exact basenames declared by the validated manifest before invoking `launchUninstaller`.

For upgrade, retain the old installer and receipt before the new installer; if the new handshake/version check fails, reinstall the cached old version only when its signature and digest still verify. Never end the running DSHtray/DSH process from this path. For explicit uninstall, require `--remove-native-companion`, verify receipt ownership, invoke the fixed uninstaller path with fixed arguments, then read back executable/receipt/URI state.

- [ ] **Step 4: Run test to verify pass**

Run: `pnpm exec vitest run apps/cli/tests/companion-transaction.spec.ts apps/cli/tests/built-bin.e2e.ts`

Expected: PASS for Web-only removal, owned native removal, external protection, upgrade rollback and half-complete reporting.

- [ ] **Step 5: Commit**

```bash
git add apps/cli/src/companion.ts apps/cli/src/plugin.ts apps/cli/src/companion-receipt.ts apps/cli/tests/companion-transaction.spec.ts apps/cli/tests/built-bin.e2e.ts
 git commit -m "feat: protect companion upgrades and uninstall"
```

### Task 7: Run DSH CLI integration and contract gates

**Objective:** 证明新参数不破坏既有 plugin/profile/bundle 行为，并给 dshmarket 一个稳定的子进程接口。

**Files:**
- Modify: `apps/cli/tests/args.spec.ts`
- Modify: `apps/cli/tests/built-bin.e2e.ts`
- Create: `apps/cli/tests/fixtures/companion-package/package.json`
- Create: `apps/cli/tests/fixtures/companion-package/dsh.companion.json`
- Modify: `apps/cli/src/companion.ts` only for test-observed contract gaps

- [ ] **Step 1: Add fixture-driven integration assertions**

```ts
it('prints a stable companion failure code for the market runner', async () => {
  const fixtureHome = mkdtempSync(join(tmpdir(), 'dsh-companion-cli-'))
  try {
    const result = await runBuiltBin([
      'plugin', '--profile', 'web', 'add', '@gittuyn/dshtray-plugin'
    ], { DSH_HOME: fixtureHome }, fixtureHome)
    expect(result.code).not.toBe(0)
    expect(result.stderr).toContain('native-companion')
  } finally {
    rmSync(fixtureHome, { recursive: true, force: true })
  }
})
```

- [ ] **Step 2: Run the focused red/green suite**

Run: `pnpm exec vitest run apps/cli/tests/args.spec.ts apps/cli/tests/companion-*.spec.ts apps/cli/tests/built-bin.e2e.ts`

Expected: all focused tests PASS; any failure must identify either a parser regression or a companion state/rollback mismatch.

- [ ] **Step 3: Run repository gates**

Run: `pnpm --filter @deepseek-ai/dsh typecheck`, `pnpm test -- apps/cli/tests`, `pnpm run build:lib:host`, and `pnpm run lint:contracts-ready`。

Expected: exit code 0; no existing CLI tests regress. Do not run a real installer, registry modification, startup registration or process-control command in this task.

- [ ] **Step 4: Verify the diff scope**

Run: `git diff --check` and `git status --short -- apps/cli/src apps/cli/tests`

Expected: only explicitly planned DSH CLI files are listed; pre-existing unrelated untracked files remain untouched.

- [ ] **Step 5: Commit**

```bash
git add apps/cli/src/args.ts apps/cli/src/bin.ts apps/cli/src/plugin.ts apps/cli/src/companion*.ts apps/cli/tests/args.spec.ts apps/cli/tests/companion-*.spec.ts apps/cli/tests/built-bin.e2e.ts
git commit -m "test: verify native companion CLI contract"
```

## Completion Checklist

- [ ] H0–H3 resolved and recorded.
- [ ] All manifest/path/hash/signature/ownership checks happen before installer spawn.
- [ ] Direct process execution uses absolute fixed asset path, `shell:false`, and `['/S']` only.
- [ ] Web profile rollback is verified after every failed native step.
- [ ] Existing external DSHtray is observe-only.
- [ ] Upgrade retains a verified old installer and never force-kills DSH/DSHtray.
- [ ] Receipt has no credential fields or values.
- [ ] dshmarket can pass explicit consent through flags and receives a nonzero exit code plus actionable stderr on failure.
