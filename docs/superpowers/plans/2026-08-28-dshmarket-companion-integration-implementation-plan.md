# dshmarket Native Companion Consent Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 dshmarket 上游实现可信 registry 元数据、同源安装确认和 CLI flag 转发，使市场不会把 npm/Web 包安装成功误报为原生伴侣安装成功。

**Architecture:** dshmarket 继续使用现有 `src/routes.ts` → `src/dsh-cli.ts` → DSH CLI 子进程链路；registry 只用于展示和服务端 allowlist，真正的资产/hash/signature/ownership/rollback 由 DSH Host 协调器负责。安装 UI 在执行带伴侣的包之前明确展示 current-user、签名发布者、版本和资产信息，用户确认后才向 CLI 传递布尔选项；浏览器永远不能传入路径、参数或下载地址。

**Tech Stack:** TypeScript, Node HTTP routes, React 18, existing `@deepseek-ai/dsh-client-ui-primitives`, `vitest`, `jsdom`, `playwright`, `tsdown`, current dshmarket `typecheck/build/test` scripts。

**Spec:** `C:/Users/Tony/Documents/BaiduSyncdisk/DSH/DSHtray/docs/superpowers/specs/2026-08-28-dsh-plugin-companion-install-design.md`

## Global Constraints

- 正式修改目标是上游仓库 `https://github.com/dsh-market/dsh-market.git`；不得直接修改 `C:/Users/Tony/.dsh/profiles/web/node_modules/dshmarket` 安装副本。
- 执行 checkout 默认使用 `C:/Users/Tony/Documents/Default Project/dsh-market`；checkout 前确认该路径不存在未保存用户工作。
- 安装 route 只能接受 same-origin POST 和 curated registry 中的目标；不能接受浏览器提供的任意 URL、文件路径、安装器参数或二次下载地址。
- 伴侣 id 必须是 `com.deepseek.dshtray`；协议号必须是 `1`；平台 `win32`；架构 `x64`；安装类型 `nsis-current-user`；静默参数只能是 `/S`。
- 只有用户明确确认后才能转发 `--accept-native-companion`；没有确认时返回可操作的 consent-required 响应，不执行 DSH CLI。
- Web 插件移除默认只移除 Web/profile 层；只有显式确认才转发 `--remove-native-companion`。
- dshmarket 不做 Authenticode/hash/ownership 判断的替代实现；Host 失败、回滚或 half-complete 必须原样显示为失败/需处理状态。
- 安装、升级、卸载和 UI 日志不得保存、上传或转发 API 密钥、令牌、密码或其他凭据值。

## Execution Gates

- **M0 — Upstream checkout:** 当前安装包只有 42 个 source TypeScript 文件且不含测试；必须在上游 checkout 中确认测试目录、branch 和 `AGENTS.md` 后再执行。
- **M1 — HTTP status contract:** 默认使用 `428 Precondition Required` 和 `{ error: 'native-companion-consent-required', companion: RegistryCompanionSummary }`；若 dshmarket 现有错误枚举已定义等价状态，执行前保持同一语义但不得静默改为 200 成功。
- **M2 — CLI flag order:** 默认生成 `dsh plugin --profile <profile> --accept-native-companion add <target>`；DSH CLI H0 若采用其他合法位置，必须同步更新 adapter 测试。
- **M3 — Catalog publisher:** registry 的 publisher/size/version/digest 只做界面和 allowlist 输入；Host 仍以签名和本地 manifest 为最终权威，不能把 registry 字段直接写入 receipt。

## Verified Baseline and File Map

安装副本的可读参考文件如下，全部只读：

- `src/registry.ts`: `RegistryPlugin` 当前没有 companion 字段。
- `src/dsh-cli.ts`: `runDshPlugin` 当前接收 profile + args 并生成 DSH CLI 子进程。
- `src/routes.ts`: `mountMarketRoutes` 当前持有 `PluginCommandRuntime`，安装/移除 route 已有同源和 curated registry 保护。
- `src/client/MarketSection.tsx`: 现有 Discover/Themes/Installed 安装、更新、卸载 UI。
- `src/client/InstallToast.tsx`: 现有安装结果/错误提示。
- `src/client/market-data.ts`: registry/status 类型和 API helper。
- `src/client/locales.ts`: 中英文市场文案。
- `src/client/Market.module.css`: 现有 modal/banner/card 样式。

正式上游 checkout 的文件范围：

- Modify `src/registry.ts` and registry fixture/schema: 声明并校验可展示的 `companion` 摘要。
- Modify `src/dsh-cli.ts`: 增加 `PluginRunOptions`，只由服务端布尔值拼接固定 CLI flags。
- Modify `src/routes.ts`: consent preflight、body 类型校验、428 响应、确认后 options 转发、失败状态序列化。
- Modify `src/client/market-data.ts`: `RegistryPlugin.companion` 和 consent response 类型。
- Modify `src/client/MarketSection.tsx`: 安装前 Native Companion 确认 Modal 和二次请求。
- Modify `src/client/InstallToast.tsx`: 显示 native companion install/rollback/half-complete 结果。
- Modify `src/client/locales.ts` and `src/client/Market.module.css`: 安全范围、发布者、版本和失败文案/样式。
- Modify existing `tests/registry.spec.ts`, `tests/dsh-cli.spec.ts`, `tests/routes.spec.ts`, `tests/client/market-section.client.spec.tsx`; add only the companion fixture under `tests/fixtures/` so the existing harnesses (`mount`, `hit`, `post`, `jsonBody`, `stubFetch`, `props`) are reused。

## Shared Interfaces

```ts
export interface RegistryCompanionSummary {
  protocol: 1
  id: 'com.deepseek.dshtray'
  version: string
  platform: 'win32'
  arch: 'x64'
  kind: 'nsis-current-user'
  assetName: 'DSHtray_0.1.0_x64-setup.exe'
  sizeBytes: number
  publisher: string
}

export interface NativeCompanionConsentResponse {
  error: 'native-companion-consent-required'
  companion: RegistryCompanionSummary
  message: string
}

export interface PluginRunOptions {
  acceptNativeCompanion?: boolean
  removeNativeCompanion?: boolean
}
```

`DshPluginRuntime.runPlugin` 的扩展签名必须是：

```ts
(profile: string, args: string[], options?: PluginRunOptions) => Promise<InstallResult>
```

现有调用不传第三参数时行为完全不变。服务端不得接受 `options` 中的路径/字符串参数；只能从经过 registry lookup 的 `true` 布尔值构造两种固定 flag。

### Task 1: Extend registry metadata without trusting it as runtime authority

**Objective:** 让市场能提前知道某条目包含原生伴侣，并显示足够的用户确认信息，同时保持 registry 只读展示/allowlist 角色。

**Files:**
- Modify: `src/registry.ts`
- Modify: registry validation fixture/schema in the upstream repository
- Modify: `src/client/market-data.ts`
- Modify: `tests/registry.spec.ts`
- Create: `tests/fixtures/registry-with-companion.json`

**Interfaces:**
- `RegistryPlugin.companion?: RegistryCompanionSummary`。
- `asRegistry()` 必须拒绝错误 protocol/id/platform/arch/kind、非正 size、空 publisher 和不符合文件名 allowlist 的 companion 摘要。

- [ ] **Step 1: Write the failing test**

```ts
it('accepts only the fixed DSHtray companion summary', () => {
  const registry = loadFixture('registry-with-companion.json')
  expect(registry.plugins[0].companion).toEqual({
    protocol: 1, id: 'com.deepseek.dshtray', version: '0.1.0',
    platform: 'win32', arch: 'x64', kind: 'nsis-current-user',
    assetName: 'DSHtray_0.1.0_x64-setup.exe', sizeBytes: 1920630,
    publisher: 'CN=approved publisher'
  })
})

it('rejects a companion summary that tries to redirect the installer', () => {
  expect(() => parseRegistry({ plugin: { companion: {
    ...validCompanion(), assetName: 'https://evil.example/setup.exe'
  }}})).toThrow()
})
```

- [ ] **Step 2: Run test to verify failure**

Run: `pnpm test -- tests/registry.spec.ts`

Expected: FAIL because `RegistryPlugin` and registry validation have no companion field.

- [ ] **Step 3: Implement schema and type guards**

```ts
function isCompanionSummary(value: unknown): value is RegistryCompanionSummary {
  if (value === null || typeof value !== 'object') return false
  const c = value as Partial<RegistryCompanionSummary>
  return c.protocol === 1 && c.id === 'com.deepseek.dshtray'
    && c.platform === 'win32' && c.arch === 'x64'
    && c.kind === 'nsis-current-user'
    && c.assetName === 'DSHtray_0.1.0_x64-setup.exe'
    && typeof c.version === 'string' && c.version !== ''
    && typeof c.sizeBytes === 'number' && Number.isSafeInteger(c.sizeBytes) && c.sizeBytes > 0
    && typeof c.publisher === 'string' && c.publisher !== ''
}
```

Do not copy `sha256` into a browser response unless the existing registry policy explicitly exposes it; the user needs publisher/version/size/scope, while Host validates the package asset itself.

- [ ] **Step 4: Run test to verify pass**

Run: `pnpm test -- tests/registry.spec.ts && pnpm typecheck`

Expected: PASS; legacy registry entries without `companion` remain valid.

- [ ] **Step 5: Commit**

```bash
git add src/registry.ts src/client/market-data.ts tests/registry.spec.ts tests/fixtures/registry-with-companion.json
 git commit -m "feat: describe native companions in registry"
```

### Task 2: Add a typed CLI runner option seam

**Objective:** 使 dshmarket 只把服务端确认结果转成固定 DSH CLI flag，不改变既有 pnpm 参数顺序和输出采集。

**Files:**
- Modify: `src/dsh-cli.ts`
- Modify: `tests/dsh-cli.spec.ts`

- [ ] **Step 1: Write the failing test**

```ts
const spawned: string[][] = []
const capturedSpawnArgs = (): string[] => spawned.at(-1) ?? []

it('adds the accept flag only when the server supplies true', async () => {
  vi.doMock('node:child_process', async () => ({
    ...(await vi.importActual<typeof import('node:child_process')>('node:child_process')),
    spawn: (_file: string, args: readonly string[]) => {
      spawned.push([...args])
      const child = new EventEmitter()
      queueMicrotask(() => child.emit('close', 0))
      return child
    },
  }))
  vi.resetModules()
  const { runDshPlugin } = await import('../src/dsh-cli.ts')
  await runDshPlugin('web', ['add', '@gittuyn/dshtray-plugin'], { acceptNativeCompanion: true })
  expect(spawned.at(-1)).toEqual([
    'plugin', '--profile', 'web', '--accept-native-companion',
    'add', '@gittuyn/dshtray-plugin',
  ])
})

it('does not add any browser-controlled installer argument', async () => {
  const options: PluginRunOptions = { acceptNativeCompanion: true }
  await runDshPlugin('web', ['add', '@gittuyn/dshtray-plugin'], options)
  const spawnedArgs = capturedSpawnArgs().at(-1) ?? []
  expect(spawnedArgs).not.toContain('--installer-path')
  expect(spawnedArgs).not.toContain('--installer-args')
})
```

- [ ] **Step 2: Run test to verify failure**

Run: `pnpm test -- tests/dsh-cli.spec.ts`

Expected: FAIL because `runDshPlugin` accepts no options and builds no companion flag.

- [ ] **Step 3: Implement the fixed options mapper**

```ts
function companionFlags(options: PluginRunOptions | undefined): string[] {
  if (options?.acceptNativeCompanion === true) return ['--accept-native-companion']
  if (options?.removeNativeCompanion === true) return ['--remove-native-companion']
  return []
}
```

Insert the returned flag after `plugin --profile <profile>` and before the caller’s command args. The public type contains only two optional booleans; do not add path, URL, environment, or arbitrary argument fields.

- [ ] **Step 4: Run test to verify pass**

Run: `pnpm test -- tests/dsh-cli.spec.ts && pnpm typecheck`

Expected: PASS; all legacy callers still spawn the same argv when options are omitted.

- [ ] **Step 5: Commit**

```bash
git add src/dsh-cli.ts tests/dsh-cli.spec.ts
 git commit -m "feat: forward explicit companion consent to dsh"
```

### Task 3: Gate install route on explicit user consent

**Objective:** 对带 companion 的 registry entry 先返回 428 consent card，确认后才调用 DSH CLI，并把 Host 的失败/回滚结果保留下来。

**Files:**
- Modify: `src/routes.ts`
- Modify: `src/dsh-cli.ts: PluginRunOptions and PluginRunner types`
- Modify: `tests/routes.spec.ts`

**Interfaces:**

```ts
type InstallBody = {
  url?: unknown
  acceptNativeCompanion?: unknown
}

// 428 response
{
  error: 'native-companion-consent-required',
  companion: RegistryCompanionSummary,
  message: 'This package also installs DSHtray for the current Windows user.'
}
```

- [ ] **Step 1: Write the failing route tests**

```ts
const commandRuntime: PluginCommandRuntime = {
  runPlugin: vi.fn(async () => ({ exitCode: 0, timedOut: false, stdout: '', stderr: '', cancelled: false })),
  probePnpm: vi.fn(async () => true),
  provisionPnpm: vi.fn(async () => ({ ok: true })),
  cancelActive: vi.fn(() => false),
}
const trustedTarget = 'https://github.com/gittuyn/DSHtray'
const postInstall = (body: unknown) => hit(routes, '/dsh-market/install', post('/dsh-market/install', body))

beforeEach(() => {
  routes = mount(commandRuntime).routes
  vi.clearAllMocks()
})

// Add this hoisted mock with the existing imports at the top of this spec
// file; it makes `loadRegistry()` return the fixture and never performs a
// network request. Keep the fixture's URL equal to trustedTarget.
vi.mock('../src/registry.ts', async () => {
  const actual = await vi.importActual<typeof import('../src/registry.ts')>('../src/registry.ts')
  return {
    ...actual,
    loadRegistry: vi.fn(async () => JSON.parse(readFileSync(
      new URL('./fixtures/companion-registry.json', import.meta.url), 'utf8'
    )) as Awaited<ReturnType<typeof actual.loadRegistry>>),
  }
})

it('returns consent-required and does not spawn dsh', async () => {
  const response = await postInstall({ url: trustedTarget, acceptNativeCompanion: false })
  expect(response.status).toBe(428)
  expect(response.body.error).toBe('native-companion-consent-required')
  expect(commandRuntime.runPlugin).not.toHaveBeenCalled()
})

it('passes only true consent to the CLI runner', async () => {
  const response = await postInstall({ url: trustedTarget, acceptNativeCompanion: true })
  expect(response.status).toBe(200)
  expect(commandRuntime.runPlugin).toHaveBeenCalledWith(
    'web', ['add', trustedTarget], { acceptNativeCompanion: true }
  )
})
```

- [ ] **Step 2: Run test to verify failure**

Run: `pnpm test -- tests/routes.spec.ts`

Expected: FAIL because route body parsing and companion consent gate do not exist.

- [ ] **Step 3: Implement the route gate**

```ts
const registry = await loadRegistry()
const entry = registry.plugins.find(plugin => plugin.url.toLowerCase() === String(body.url).toLowerCase())
if (entry === undefined) {
  sendJson(response, 404, { error: 'plugin-not-in-curated-registry' })
  return
}
if (entry.companion !== undefined && body.acceptNativeCompanion !== true) {
  sendJson(response, 428, {
    error: 'native-companion-consent-required',
    companion: entry.companion,
    message: 'This package also installs DSHtray for the current Windows user.',
  })
  return
}
const options = entry.companion === undefined ? undefined : { acceptNativeCompanion: true }
const result = await runPlugin(config.profile, ['add', entry.url], options)
```

Reject non-boolean `acceptNativeCompanion`, reject `true` when the entry has no companion, preserve same-origin/curated registry checks before this branch, and map nonzero DSH results to the existing failed operation response. Never report `installed: true` solely because pnpm exited 0.

- [ ] **Step 4: Run test to verify pass**

Run: `pnpm test -- tests/routes.spec.ts && pnpm typecheck`

Expected: PASS for consent-required, explicit accepted, malformed body, no-companion legacy install, DSH nonzero failure, timeout and cancellation.

- [ ] **Step 5: Commit**

```bash
git add src/routes.ts src/dsh-cli.ts tests/routes.spec.ts
 git commit -m "feat: require native companion install consent"
```

### Task 4: Add consent UI and safe result rendering

**Objective:** 在现有 market 安装 Modal 中明确显示原生伴侣影响，用户取消不产生副作用，失败/回滚/half-complete 不显示成功。

**Files:**
- Modify: `src/client/MarketSection.tsx`
- Modify: `src/client/InstallToast.tsx`
- Modify: `src/client/market-data.ts`
- Modify: `src/client/locales.ts`
- Modify: `src/client/Market.module.css`
- Modify: `tests/client/market-section.client.spec.tsx`

**Interfaces:**
- `installPlugin(plugin, { acceptNativeCompanion?: boolean })` only accepts a local boolean selected by the Modal.
- Consent Modal displays `platform`, `arch`, `kind`, `version`, `publisher`, `sizeBytes`, “只安装到当前 Windows 用户”, and “不会保存凭据”。
- Confirm sends `{ acceptNativeCompanion: true }`; cancel closes Modal and does not call POST.

- [ ] **Step 1: Write the failing component tests**

```tsx
const pluginWithCompanion = {
  ...REGISTRY.plugins[0],
  companion: {
    protocol: 1, id: 'com.deepseek.dshtray', version: '0.1.0',
    platform: 'win32', arch: 'x64', kind: 'nsis-current-user',
    assetName: 'DSHtray_0.1.0_x64-setup.exe', sizeBytes: 1920630,
    publisher: 'CN=fixture publisher',
  },
}

it('requires a second explicit click for a registry companion', async () => {
  const fetchMock = stubFetch({
    '/dsh-market/registry': { source: 'fixture', registry: { ...REGISTRY, plugins: [pluginWithCompanion, ...REGISTRY.plugins.slice(1)] } },
    '/dsh-market/install': { ok: true, hot: false },
  })
  render(<MarketSection {...props()} />)
  await screen.findByText('dsh-loop')
  await fireEvent.click(screen.getByRole('button', { name: en.install }))
  expect(screen.getByText(/DSHtray/)).toBeInTheDocument()
  expect(fetchMock.mock.calls.some(([input]) => String(input).includes('/dsh-market/install'))).toBe(false)
  await fireEvent.click(screen.getByRole('button', { name: /确认并安装/ }))
  await waitFor(() => expect(fetchMock.mock.calls.some(([input, init]) => String(input).includes('/dsh-market/install') && String((init as RequestInit).body).includes('"acceptNativeCompanion":true'))).toBe(true))
})

it('renders a native companion failure session as failure, not success', async () => {
  const { InstallToast } = await import('../../src/client/InstallToast.tsx')
  sessionStorage.setItem('dshm-toast', JSON.stringify(['DSHtray']))
  sessionStorage.setItem('dshm-toast-mode', 'native-failed')
  render(<InstallToast t={props().t} />)
  expect(screen.getByText(/DSHtray.*失败/)).toBeInTheDocument()
  expect(screen.queryByText(/安装成功/)).toBeNull()
})
```

- [ ] **Step 2: Run test to verify failure**

Run: `pnpm test -- tests/client/market-section.client.spec.tsx`

Expected: FAIL because no companion Modal state or result field exists.

- [ ] **Step 3: Implement the smallest UI path**

```tsx
const [companionConsent, setCompanionConsent] = useState<RegistryCompanionSummary | null>(null)
const [selectedPlugin, setSelectedPlugin] = useState<RegistryPlugin | null>(null)

async function beginInstall(plugin: RegistryPlugin) {
  if (plugin.companion !== undefined) {
    setSelectedPlugin(plugin)
    setCompanionConsent(plugin.companion)
    return
  }
  await installPlugin(plugin)
}

async function confirmCompanionInstall() {
  const companion = companionConsent
  const plugin = selectedPlugin
  setCompanionConsent(null)
  setSelectedPlugin(null)
  if (companion !== null && plugin !== null) await installPlugin(plugin, { acceptNativeCompanion: true })
}
```

Handle a server 428 response by reopening the same consent view with the response data; do not automatically retry with `true`. Keep the existing operation lock, cancellation and status refresh logic. Do not add stop/restart/adopt/kill controls.

- [ ] **Step 4: Run test and web typecheck**

Run: `pnpm test -- tests/client/market-section.client.spec.tsx` and `pnpm run typecheck`

Expected: PASS; cancel causes no install request, confirm sends one boolean, and every host failure state renders as actionable failure.

- [ ] **Step 5: Commit**

```bash
git add src/client/MarketSection.tsx src/client/InstallToast.tsx src/client/market-data.ts src/client/locales.ts src/client/Market.module.css tests/client/market-section.client.spec.tsx
 git commit -m "feat: show native companion consent in market"
```

### Task 5: Add explicit native uninstall and operation-state refresh

**Objective:** 保持 Web-only remove 默认行为，并把用户明确勾选的原生卸载安全地交给 DSH Host。

**Files:**
- Modify: `src/routes.ts`
- Modify: `src/dsh-cli.ts`
- Modify: `src/client/MarketSection.tsx`
- Modify: `src/client/InstallToast.tsx`
- Modify: `src/client/operations.ts`
- Modify: `tests/routes.spec.ts`

**Interfaces:**

```ts
type RemoveBody = {
  name?: unknown
  removeNativeCompanion?: unknown
}
```

- [ ] **Step 1: Write the failing tests**

```ts
const postRemove = (body: unknown) => hit(routes, '/dsh-market/uninstall', post('/dsh-market/uninstall', body))

it('removes only Web package by default', async () => {
  const response = await postRemove({ name: '@gittuyn/dshtray-plugin' })
  expect(response.status).toBe(200)
  expect(commandRuntime.runPlugin).toHaveBeenCalledWith('web', ['remove', '@gittuyn/dshtray-plugin'], undefined)
})

it('requires explicit native removal confirmation', async () => {
  const response = await postRemove({ name: '@gittuyn/dshtray-plugin', removeNativeCompanion: true })
  expect(response.status).toBe(200)
  expect(commandRuntime.runPlugin).toHaveBeenCalledWith('web', ['remove', '@gittuyn/dshtray-plugin'], { removeNativeCompanion: true })
})
```

- [ ] **Step 2: Run test to verify failure**

Run: `pnpm test -- tests/routes.spec.ts`

Expected: FAIL because remove route cannot forward an explicit native option.

- [ ] **Step 3: Implement owned-only remove request**

```ts
const removeNative = body.removeNativeCompanion === true
const options = removeNative ? { removeNativeCompanion: true } : undefined
const result = await runPlugin(config.profile, ['remove', name], options)
```

UI must show the same external/owned warning before sending `removeNativeCompanion: true`. A malformed non-boolean value returns 400. The route does not inspect local receipt itself; DSH Host remains the ownership authority.

- [ ] **Step 4: Run test to verify pass**

Run: `pnpm test -- tests/routes.spec.ts && pnpm typecheck`

Expected: PASS; Web-only remove has no native flag, explicit native remove has exactly one flag, and Host external protection remains visible.

- [ ] **Step 5: Commit**

```bash
git add src/routes.ts src/dsh-cli.ts src/client/MarketSection.tsx src/client/InstallToast.tsx src/client/operations.ts tests/routes.spec.ts
 git commit -m "feat: separate native companion uninstall consent"
```

### Task 6: Add integration fixtures and execute package gates

**Objective:** 用 fake command runtime 证明市场、CLI、Host 之间不会产生假成功，并完成上游发布前检查。

**Files:**
- Create: `tests/fixtures/companion-registry.json`
- Create: `tests/fixtures/host-results.json`
- Modify: `tests/routes.spec.ts`
- Modify: `tests/client/market-section.client.spec.tsx`
- Modify: `README.md` or upstream companion documentation page selected by repository convention

- [ ] **Step 1: Add fixture-driven failure cases**

```ts
it.each([
  ['signature-invalid', 1, '签名校验失败'],
  ['external', 1, '外部伴侣'],
  ['handshake-failed', 1, '版本握手失败'],
  ['installed', 0, '已安装'],
])('maps Host result %s to market operation state', async (state, exitCode, label) => {
  vi.mocked(commandRuntime.runPlugin).mockResolvedValue({ exitCode, timedOut: false, cancelled: false, stdout: '', stderr: state })
  const postInstallForResult = (body: unknown) => hit(routes, '/dsh-market/install', post('/dsh-market/install', body))
  const response = await postInstallForResult({ url: trustedTarget, acceptNativeCompanion: true })
  expect(response.status).toBe(exitCode === 0 ? 200 : 500)
  expect(response.body.message).toContain(label)
})
```

- [ ] **Step 2: Run focused test to verify the new assertions fail first**

Run: `pnpm test -- tests/routes.spec.ts tests/client/market-section.client.spec.tsx`

Expected: the newly added mapping cases fail until result normalization is implemented; existing legacy cases remain runnable.

- [ ] **Step 3: Implement result normalization and docs**

Preserve `exitCode`, `timedOut`, `cancelled`, `stdout`, `stderr`, and the Host companion state in the operation record. A zero exit code without a verified Host `installed` result is not success. Document the exact 428 response, confirm payload, Web-only remove default, and no-control-button scope without including any digest secret or credential.

- [ ] **Step 4: Run all upstream gates**

Run from `C:/Users/Tony/Documents/Default Project/dsh-market`:

```bash
pnpm test -- tests/registry.spec.ts tests/dsh-cli.spec.ts tests/routes.spec.ts tests/client/market-section.client.spec.tsx
pnpm typecheck
pnpm build
pnpm prepack
```

Expected: focused tests PASS, `typecheck` and `build` exit 0, `prepack` verifies package contents without running a native installer. A Windows manual gate must then exercise consent/no-consent, signed/unsigned asset, external receipt, rollback and Web-only removal using the acceptance checklist from the DSHtray plan.

- [ ] **Step 5: Verify upstream diff scope and commit**

Run: `git diff --check` and `git status --short`.

Expected: only the listed dshmarket files and tests/docs are changed. Then:

```bash
git add src/registry.ts src/dsh-cli.ts src/routes.ts src/client tests README.md
git commit -m "feat: integrate native companion consent in market"
```

## Completion Checklist

- [ ] M0–M3 resolved and recorded。
- [ ] Registry metadata validates and remains non-authoritative for local signature/receipt decisions。
- [ ] 428 consent response prevents pre-confirmation DSH CLI spawn。
- [ ] Confirmed install forwards exactly one boolean flag and no browser-controlled path/args。
- [ ] Web-only remove stays default; native removal requires an independent explicit confirmation。
- [ ] DSH nonzero, timeout, rollback and half-complete states never render as success。
- [ ] Existing dshmarket tests, typecheck, build and prepack pass。
