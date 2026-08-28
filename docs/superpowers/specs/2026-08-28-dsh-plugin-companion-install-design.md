# DSH Web 插件 + DSHtray.exe 原生伴侣完整安装设计

- **状态**：设计提案，待用户审阅
- **日期**：2026-08-28
- **目标平台**：Windows 11 x64
- **关联项目**：`C:\Users\Tony\Documents\BaiduSyncdisk\DSH\DSHtray`、`C:\Users\Tony\Documents\Default Project\deepseek-harness`
- **目标包名**：`@gittuyn/dshtray-plugin`
- **实现边界**：本文只定义安装架构、协议、验收和发布约束，不代表已经开始开发

## 1. 决策摘要

目标是让一次插件安装同时完成 DSH Web 集成和 `DSHtray.exe` 原生伴侣安装：

```text
dsh plugin --profile web add @gittuyn/dshtray-plugin
```

或在 DSH Web 插件市场点击一次安装，在明确确认原生程序权限后完成相同流程。

采用以下方案：

1. 插件包同时声明 `dsh.bundle`、`dsh.client` 和 `dsh.companion`。
2. 插件包发布为已经构建好的 npm 包，内置 DSH Web 产物和 current-user NSIS 安装器。
3. 不使用插件的 `postinstall`、`prepare` 或任意自定义脚本安装 Windows 程序。
4. DSH CLI 和插件市场使用同一个受限的 Native Companion 协调器。
5. 协调器只允许当前用户范围、固定安装类型、包内资产、固定参数和已验证签名的安装器。
6. 只有 Web 插件与原生伴侣都验证成功，整个安装才返回成功并激活 Web 集成。
7. 原生安装失败时回滚本次 Web 插件变更；不停止用户当前 DSH，不终止未知进程，不删除已有的独立 DSHtray。
8. 当前 GitHub Release 的 NSIS 安装器仍为 `NotSigned`，在完成代码签名以前不得作为市场自动安装资产发布。

这个方案保留当前 DSHtray 的 Rust/Tauri Windows 能力，不把托盘、Job Object、PID 树控制或代理注入迁移到 Node 插件中。

## 2. 已验证事实与约束

| 项目 | 当前事实 | 对设计的影响 |
| --- | --- | --- |
| DSH 插件入口 | `dsh plugin --profile` 后接 pnpm 参数，当前转发到 profile 目录 | 普通插件安装本身不会自动理解原生伴侣 |
| Bundle 激活 | 包的 `package.json` 可声明 `dsh.bundle.patch`，成功安装后加入 `dsh.profile.bundles` | Web Host 部分可以沿用现有机制 |
| Web Client | 包可以声明 `dsh.client`，客户端 bundle 需要预先构建 | 发布包不能只带 TypeScript 源码 |
| 插件市场安装 | `dshmarket` 的 `/dsh-market/install` 只接受 curated registry 中的 URL，最终调用 DSH 插件安装流程 | 市场需要增加伴侣元数据和安装后协调步骤 |
| 市场安全边界 | 当前安装路由校验 same-origin、curated registry、包入口和回滚 | 新伴侣流程必须继续复用这些边界 |
| pnpm 构建授权 | Git 源码插件的 `prepare` 可能被 pnpm `allowBuilds` 拦截 | 生产包使用已构建 npm tarball，避免消费者安装时构建 |
| DSHtray 安装器 | 当前 Tauri 配置为 NSIS、`installMode: "currentUser"` | 可以设计为不需要管理员权限的原生安装 |
| 当前发布签名 | 当前 `v0.1.0` 安装器状态为 `NotSigned` | 未签名资产只能手工测试，不能进入自动市场安装 |
| 当前 Release 安装器 SHA-256 | `36908c3f9fb77988f0b52ca7a89b03b8670e5fb5ff80ea1ed308a236fa1cd985` | 签名后必须重新计算并更新包元数据 |
| 凭据边界 | DSHtray 不保存 API 密钥、令牌、密码或其他凭据值 | 安装收据和日志不得引入凭据字段 |

## 3. 目标与非目标

### 3.1 目标

1. 在 Windows 11 x64 上通过一次 CLI 或市场操作安装 Web 插件和 DSHtray。
2. 安装器默认使用当前用户范围，不要求管理员权限，不修改系统级设置。
3. 在执行原生安装前校验包内安装器的路径、精确字节数、SHA-256 和 Authenticode 签名。
4. 安装完成后验证预期的 `DSHtray.exe`、版本和签名，并记录无敏感信息的安装收据。
5. 安装失败时让 profile 恢复到本次操作之前的可启动状态。
6. 支持有明确边界的升级和卸载，不因插件移除误删用户独立使用的 DSHtray。
7. 让旧版 DSH 或旧版市场在不支持伴侣协议时安全失败，而不是静默只安装 Web 半部。

### 3.2 非目标

第一版不包含：

- 将现有 DSHtray 改写成纯 DSH Node 插件。
- 通过 Web 插件直接实现启动、停止、接管或强制终止 DSH。
- 让任意插件包执行任意 PowerShell、批处理或 shell 命令。
- 从未固定的 GitHub URL 下载第二份 EXE。
- 通过插件保存、读取、上传或转发 API 密钥、令牌、密码或其他凭据。
- 自动修改 Windows 全局代理、服务、计划任务或系统范围注册表。
- 在 DSHtray 正在运行时强制结束它以完成升级。
- 在第一版引入远程控制 API 或无认证的本机 HTTP 控制接口。
- 为每个社区插件开放任意安装器类型；第一版只支持固定的 current-user NSIS 类型。

## 4. 安装模型

### 4.1 两个交付层

```text
@gittuyn/dshtray-plugin npm 包
├─ DSH Host bundle
├─ DSH Web client bundle
├─ cordis.patch.yml
└─ companion/
   └─ DSHtray_0.1.0_x64-setup.exe

DSHtray.exe
├─ Windows 托盘
├─ DSH 启动、停止、重启
├─ 代理环境处理
├─ Job Object 和 PID 树安全控制
├─ 开机启动
└─ 独立配置、日志和卸载入口
```

插件包只携带和声明安装资产；DSHtray 仍然拥有原生生命周期和进程安全逻辑。DSH Host 负责把两个层面的安装编排成一个事务，不把原生控制逻辑复制到插件中。

### 4.2 为什么不使用 `postinstall`

`postinstall` 不是可靠的产品边界：

- pnpm 版本和 profile 的 `allowBuilds` 设置可能阻止脚本；
- 普通 npm 安装没有面向用户的原生权限确认界面；
- 脚本可以执行任意进程，市场难以区分安全安装和恶意行为；
- 失败后 pnpm 不会替插件清理安装器、启动项和卸载项；
- CLI、市场和桌面客户端会得到不同的安装结果。

因此，伴侣安装只能通过宿主识别的声明式元数据和固定协调器执行。插件包不提供任意安装脚本入口。

### 4.3 为什么内置安装器而不是安装时下载 Release

第一版把 NSIS 安装器放入已经发布的 npm 包：

- npm/pnpm 已经负责包来源和完整性传输；
- 安装流程只有一个版本对象，不需要再信任第二个未绑定的下载 URL；
- 市场可以在安装前展示伴侣版本和安装范围；
- 失败时可以依据已安装包的版本回滚；
- 不依赖 GitHub Release 在不同网络环境下可达。

GitHub Release 继续用于手工下载、独立安装和发布验证。只有未来明确设计外部资产协议后，才允许使用固定 HTTPS URL，并且仍必须绑定 SHA-256、签名、版本和来源。

## 5. 包元数据协议

### 5.1 `package.json` 形状

插件包应同时包含现有 DSH 元数据和新的伴侣声明：

```json
{
  "name": "@gittuyn/dshtray-plugin",
  "version": "0.1.0",
  "files": [
    "lib",
    "client",
    "cordis.patch.yml",
    "companion/DSHtray_0.1.0_x64-setup.exe"
  ],
  "dsh": {
    "bundle": {
      "patch": "./cordis.patch.yml"
    },
    "client": {
      "inject": [
        "@deepseek-ai/dsh-client-connection",
        "@deepseek-ai/dsh-client-runtime"
      ],
      "platform": "web"
    },
    "companion": {
      "protocol": 1,
      "id": "com.deepseek.dshtray",
      "platform": "win32",
      "arch": "x64",
      "version": "0.1.0",
      "kind": "nsis-current-user",
      "asset": "./companion/DSHtray_0.1.0_x64-setup.exe",
      "sha256": "36908c3f9fb77988f0b52ca7a89b03b8670e5fb5ff80ea1ed308a236fa1cd985",
      "sizeBytes": 1920630,
      "expectedExecutable": "%LOCALAPPDATA%\\DSHtray\\DSHtray.exe",
      "expectedUninstaller": "%LOCALAPPDATA%\\DSHtray\\uninstall.exe",
      "requiresElevation": false,
      "silentArgs": ["/S"],
      "startAfterInstall": true
    }
  }
}
```

示例中的 SHA-256 是当前未签名 `v0.1.0` 安装器的历史校验值，仅用于说明字段形状；正式发布包必须使用签名后安装器重新计算的 64 位小写十六进制值。发布校验会拒绝空值、格式错误、与实际资产不匹配或仍为未签名的资产。

### 5.2 宿主允许的字段

第一版协调器只接受以下值：

- `protocol` 必须为 `1`；
- `platform` 必须匹配 `win32`；
- `arch` 必须匹配 `x64`；
- `kind` 必须为 `nsis-current-user`；
- `requiresElevation` 必须为 `false`；
- `asset` 必须解析到当前已安装 npm 包目录内的普通文件；
- `asset` 不能包含 `..` 穿越包目录；
- `silentArgs` 必须与受支持的 NSIS 参数集合完全匹配，第一版只允许 `/S`；
- `expectedExecutable` 必须解析到当前用户的 DSHtray 安装目录；
- `expectedUninstaller` 必须解析到同一 current-user 安装目录；
- `sha256` 必须匹配安装器实际字节；
- `sizeBytes` 必须匹配安装器实际字节数；
- 该集成包必须同时有可加载的 `dsh.bundle` 和 `dsh.client` 入口。

其他字段不能改变安装命令、目标目录、提权策略或回滚行为。未知字段可以保留供未来版本读取，但第一版不得执行其语义。

### 5.3 版本兼容声明

插件版本和伴侣版本不要求字符串完全相同，但必须满足显式兼容关系：

```text
plugin companionApi: 1
companion protocol: 1
```

如果未来伴侣升级不兼容，必须提升协议号，旧版 Web 插件不能自动安装新版伴侣。普通 bugfix 或兼容功能升级可以继续使用协议 `1`，但仍要更新版本和 SHA-256。

## 6. 安装事务与用户流程

### 6.1 CLI 安装

交互式终端中的推荐流程：

```text
dsh plugin --profile web add @gittuyn/dshtray-plugin
```

1. DSH 先按现有流程通过 pnpm 解析并暂存插件包。
2. 宿主发现新增包包含 `dsh.companion`。
3. CLI 显示名称、版本、平台、安装范围、是否需要管理员权限和预期路径。
4. 用户明确确认后，协调器执行伴侣预检和安装。
5. 宿主验证 `DSHtray.exe` 后完成 profile bundle reconciliation，并刷新 Web client。
6. 只有两个层面都成功，命令返回 `0`。

无 TTY 的脚本或 CI 环境不得隐式安装原生程序。必须使用显式确认选项：

```text
dsh plugin --profile web --accept-native-companion add @gittuyn/dshtray-plugin
```

`--accept-native-companion` 由 DSH 处理，不转发给 pnpm。没有该选项时，非交互安装在伴侣阶段失败并保留可操作的提示。

### 6.2 插件市场安装

市场卡片在用户点击安装前展示：

- DSH Web 插件名称和版本；
- 原生伴侣名称 `DSHtray` 和版本；
- `Windows x64` 平台限制；
- current-user 安装范围；
- 不需要管理员权限；
- 预期安装路径；
- 将启动托盘程序；
- 代码签名发布者。

用户确认原生伴侣后，市场向现有 same-origin 安装路由提交伴侣同意状态。服务器仍必须重新读取 curated registry 和已安装包的 `package.json`，不能信任浏览器提交的路径、URL、哈希或命令参数。

旧版市场不支持 `dsh.companion` 时，不允许把结果显示为“安装完成”。市场应提示升级到支持 Native Companion 的版本，或提供 GitHub Release 手工安装入口；不能静默降级为只安装 Web 插件。

### 6.3 新安装事务顺序

```text
读取现状
  ↓
若已有 DSHtray 但没有匹配安装收据，则标记为外部伴侣并中止自动覆盖
  ↓
读取并校验 curated registry
  ↓
快照 profile package.json / dsh.profile.bundles
  ↓
pnpm 安装预构建 Web 插件
  ↓
验证 dsh.bundle / dsh.client 入口
  ↓
读取并校验 dsh.companion
  ↓
验证包内安装器路径、字节数、SHA-256、签名和平台
  ↓
将已验证安装器保存到版本化回滚缓存
  ↓
执行 current-user NSIS 安装器
  ↓
验证 DSHtray.exe、版本、签名和安装收据
  ↓
启动 DSHtray（不启动 DSH）
  ↓
刷新或重启 DSH Web 以激活 client bundle
  ↓
返回整体成功
```

### 6.4 失败处理

以下任一情况都不是成功：

- 包没有可加载的 Web 入口；
- 伴侣声明缺失或协议不支持；
- 平台或架构不匹配；
- 安装器路径越界或不是普通文件；
- SHA-256 不匹配；
- Authenticode 验证失败；
- 安装器退出码非零或超时；
- 预期的 `DSHtray.exe` 不存在；
- 安装后的文件版本或签名不匹配；
- 安装收据无法原子写入。

新安装失败时：

1. 如果原生安装器尚未执行，只回滚本次 Web profile 变更。
2. 如果原生安装器已经执行，先调用协调器的受限卸载流程，成功后再回滚 Web profile。
3. 只有本次事务创建并记录为本插件所有的伴侣才允许执行上述受限卸载；预先存在但没有匹配收据的 DSHtray 一律视为外部伴侣，不能覆盖、卸载或接管。
4. 如果卸载也失败，不删除用户已有文件，不强制终止 DSHtray；标记“伴侣已安装、Web 插件未完成”，给出精确的手工恢复路径。
5. 绝不停止、接管或终止用户当前 DSH。
6. DSHmarket 使用已有 profile manifest 快照和失败回滚机制；回滚失败必须明确报告，不能伪造整体成功。

## 7. 升级与卸载

### 7.1 升级顺序

升级必须区分 Web 包变化和伴侣变化：

1. 检查当前 profile 是否有运行中的 Agent；运行中时沿用市场现有规则阻止替换插件文件。
2. 读取当前 Web 包、伴侣版本和安装收据。
3. 预验证新包和新伴侣的协议、平台、哈希、签名及兼容范围。
4. 如果 DSHtray 正在运行，不得强制结束它；安装器可以返回“请退出 DSHtray 后重试”。
5. 先保存 profile 和伴侣收据快照，并把旧版本的已验证安装器保存在版本化回滚缓存中，再执行可回滚的更新。
6. 新伴侣验证成功后再完成 Web bundle/client 更新。
7. Web 更新失败时恢复原 profile；如果新伴侣已替换旧伴侣，必须使用回滚缓存中的旧版本安装资产恢复，恢复失败则明确标记人工处理，不宣称事务成功。缺少旧版本缓存时，必须在更新前中止，不得进入不可回滚状态。
8. 更新后再次验证 DSHtray 路径、版本、签名和收据。

第一版不支持后台静默强制升级，不使用 `taskkill` 关闭 DSHtray，也不在升级期间修改 DSH 的代理或生命周期状态。

### 7.2 卸载默认行为

卸载 Web 插件和卸载 DSHtray 是两个不同动作：

```text
dsh plugin --profile web remove @gittuyn/dshtray-plugin
```

默认只移除 Web 插件，不删除 DSHtray。原因是 DSHtray 是可以独立使用的 Windows 管理器，用户可能已经从 GitHub Release 或其他 DSH profile 使用它。

市场可以提供明确的复选项“同时卸载 DSHtray”。只有用户选中后，协调器才：

1. 检查 DSHtray 是否正在运行；运行中时提示用户从托盘退出并重试，不强制结束；
2. 调用已验证的 current-user NSIS 卸载入口；
3. 验证预期程序文件和卸载收据已经消失；
4. 成功后移除 Web 插件。

如果用户只执行 Web 插件移除，伴侣收据可以保留为独立安装记录，但不能继续把已移除的 Web 插件显示为已集成。

### 7.3 回滚收据

协调器保存的收据只包含安装管理所需信息：

```json
{
  "protocol": 1,
  "id": "com.deepseek.dshtray",
  "package": "@gittuyn/dshtray-plugin",
  "companionVersion": "0.1.0",
  "installerSha256": "36908c3f9fb77988f0b52ca7a89b03b8670e5fb5ff80ea1ed308a236fa1cd985",
  "installerSizeBytes": 1920630,
  "installerCachePath": "%LOCALAPPDATA%\\DSH\\companions\\com.deepseek.dshtray\\0.1.0\\installer.exe",
  "executablePath": "%LOCALAPPDATA%\\DSHtray\\DSHtray.exe",
  "installedAt": "2026-08-28T00:00:00.000Z"
}
```

时间字段只用于诊断，不能用于权限判断。收据不包含 API key、Token、密码、Cookie、授权头、DSH 会话内容或完整环境变量。

## 8. 组件职责

### 8.1 DSH CLI / Host

需要新增一个受限的 companion 协调边界，职责是：

- 识别新增、更新或移除包的 `dsh.companion`；
- 处理 CLI 交互确认和非 TTY 显式确认；
- 校验协议、平台、架构、路径、哈希和签名；
- 使用无 shell 的直接进程启动执行固定类型安装器；
- 维护 profile 和伴侣收据的事务快照；
- 在失败时恢复 profile，保留精确错误码；
- 不承担 DSHtray 的生命周期控制。

现有 `dsh plugin` 的普通 pnpm 转发能力继续保留。没有 `dsh.companion` 的普通插件行为不改变。

### 8.2 dshmarket

市场需要增加：

- registry 条目的伴侣摘要字段，用于安装前展示；
- 对已安装包 `dsh.companion` 的服务器端复核；
- 原生伴侣确认 UI 和平台不匹配提示；
- 安装状态中区分 `webInstalled`、`companionInstalled` 和 `complete`；
- 伴侣阶段进度、失败和重试结果；
- 卸载时“仅移除 Web 插件”和“同时卸载 DSHtray”的明确分支。

市场仍使用 same-origin、curated registry、profile mutation lock、Agent busy guard 和现有 manifest rollback。浏览器不得直接提交任意安装器路径或 shell 命令。

### 8.3 DSH Web Client

第一版只提供：

- DSHtray 是否已安装；
- 已安装伴侣版本；
- 当前用户安装路径；
- DSHtray 是否正在运行的只读状态（能可靠取得时显示）；
- “打开 DSHtray 管理器”入口；
- 未安装、版本不兼容、安装失败和需要重试的状态。

第一版不在 Web 中提供停止、重启、接管或强制终止按钮。原生控制仍由 DSHtray 管理器完成。

### 8.4 DSHtray

DSHtray 继续负责：

- 托盘和托盘图标；
- DSH 源码/打包目标发现；
- DSH 启动、停止、重启和健康检查；
- 代理环境设置；
- Job Object、PID 身份校验和精确进程树控制；
- 自身开机启动和配置；
- 脱敏日志和诊断。

插件安装器不得绕过这些 Rust 安全边界，不得使用自己的 Node 代码控制 DSH 进程。

## 9. 安全设计

### 9.1 来源和完整性

自动伴侣安装只接受：

1. curated registry 中的插件条目；
2. 已安装包目录中的伴侣资产；
3. 与包元数据完全匹配的 SHA-256；
4. Windows Authenticode 链验证通过且发布者匹配的安装器；
5. `win32/x64/current-user` 固定安装类型。

当前未签名 `v0.1.0` 安装器不得绕过此规则。开发者可以手工运行它进行测试，但市场生产路径必须拒绝 `NotSigned`。

### 9.2 进程执行

- 不使用 `shell: true`、PowerShell、`cmd /c` 或字符串拼接命令；
- 安装器路径必须来自已验证包目录；
- 安装参数由宿主根据 `kind` 固定生成，不能由浏览器或 registry 任意覆盖；
- 设置安装超时，超时只结束安装器自身，不结束 DSHtray 或 DSH；
- 安装完成后使用文件身份、版本和签名检查确认结果；
- 不把凭据或完整环境变量传给安装器。

### 9.3 文件和路径

- `%LOCALAPPDATA%` 是第一版唯一允许的安装根目录；
- 不写入 `Program Files`、系统目录或系统级注册表；
- 安装收据采用临时文件加原子替换；
- 包内资产必须是普通文件，拒绝符号链接、目录和路径穿越；
- DSHtray 自己产生的配置和日志仍位于其既有用户目录。

### 9.4 生命周期安全

- 安装不会自动启动 DSH；
- 安装不会改变 DSH 当前进程的代理环境；
- 升级不会强制结束 DSHtray；
- Web 插件不会停止或终止承载它的 DSH；
- 任意端口号、PID 或 HTTP 200 响应都不能成为进程控制依据；
- 不开放无认证的启动、停止、接管或强制终止 HTTP API。

## 10. 兼容性与发布门槛

### 10.1 宿主兼容性

完整自动安装要求同时满足：

- 支持 `dsh.companion` protocol `1` 的 DSH CLI；
- 支持伴侣确认和事务回滚的 dshmarket；
- Windows 11 x64；
- 可验证的已签名安装器；
- 已构建的 Web client bundle；
- pnpm 可以安装该预构建 npm 包。

任一条件不满足，都必须显示“无法完成完整安装”的原因，不得把半安装状态标成成功。

### 10.2 包发布门槛

发布 `@gittuyn/dshtray-plugin` 前必须完成：

1. Web Host 和 Client bundle 构建；
2. NSIS current-user 安装器构建；
3. 代码签名；
4. 签名状态为 `Valid` 且发布者匹配；
5. 重新计算安装器 SHA-256；
6. 将相同哈希写入包元数据和市场 registry；
7. 校验 npm tarball 内确实包含安装器，且没有本地配置、日志、缓存或凭据；
8. 在干净 Windows 11 x64 环境完成 CLI 和市场安装、升级、卸载验证；
9. 发布失败时不修改 DSHtray 当前已发布的 `v0.1.0` 资产。

## 11. 测试与验收

### 11.1 协议和纯逻辑测试

- 合法 `dsh.companion` 元数据通过；
- 缺失字段、错误协议、平台、架构和安装类型被拒绝；
- `..`、绝对包外路径、符号链接和目录被拒绝；
- SHA-256 不匹配被拒绝；
- 未签名、证书链无效或发布者不匹配被拒绝；
- 安装参数只能产生固定的 `/S`；
- 收据不包含凭据字段；
- 安装前后版本与路径校验正确。

### 11.2 事务测试

- Web 安装成功、伴侣安装成功时整体成功；
- Web 安装失败时不执行伴侣安装；
- 伴侣预检失败时 Web profile 恢复原状；
- 安装器失败时 profile 恢复原状；
- 安装器成功但后置验证失败时执行受限卸载和 profile 回滚；
- 回滚失败时保留可诊断的部分完成状态，不伪造成功；
- 已存在的独立 DSHtray 不会因 Web 插件失败而删除；
- 当前运行中的 DSH 不会被停止或终止。

### 11.3 Windows 集成测试

在 Windows 11 x64 上验证：

- 干净用户目录执行 CLI 一次安装；
- 市场确认后执行一次安装；
- 安装不弹出管理员提权；
- `DSHtray.exe` 出现在预期 current-user 路径；
- DSHtray 启动并显示托盘；
- DSH 保持停止，除非用户在 DSHtray 中另行启动；
- 重复安装不会创建多个 DSHtray 实例；
- DSHtray 运行时升级不会被强制结束；
- 退出 DSHtray 后重试升级成功；
- 仅移除 Web 插件时 DSHtray 仍存在；
- 选择同时卸载时 DSHtray、卸载项和收据均被清理；
- 代理、Job Object、外部 DSH 识别和 PID 安全测试继续通过现有 DSHtray 验收。

### 11.4 市场验收

- registry 卡片准确展示伴侣版本、平台、范围和签名发布者；
- 用户未确认原生伴侣时操作不会执行 EXE；
- 非 Windows 或非 x64 环境不会显示可完成安装；
- 安装进度区分 Web 阶段和伴侣阶段；
- 失败后显示具体阶段和重试建议；
- 市场重启或刷新后不会把半安装状态显示为完整成功；
- 恶意或被篡改的 registry URL、路径、哈希和参数不能改变实际安装目标。

## 12. 未来实施文件范围

本文确认后，实施计划再细化精确代码任务。当前预计涉及以下文件范围：

### 12.1 DSHtray 仓库

- `package.json`：发布资产和打包校验脚本的最小调整；
- `src-tauri/tauri.conf.json`：确认 current-user NSIS 的稳定安装行为；
- `scripts/verify-release.ps1`：增加签名发布门槛和伴侣资产元数据输出；
- `scripts/package-companion.ps1`：生成插件包使用的安装器和校验清单；
- `docs/superpowers/acceptance/`：记录完整安装验收证据；
- 本设计文档及后续实现说明。

上述清单不代表现在已经修改这些文件。

### 12.2 DSH 仓库

- `apps/cli/src/args.ts`：解析 companion 确认选项；
- `apps/cli/src/plugin.ts`：在 `add`、`update`、`remove` 的明确边界接入协调器；
- `apps/cli/src/companion.ts`：固定类型、路径、签名、哈希、事务和收据逻辑；
- `apps/cli/reference/README.md` 与中文文档：记录完整安装命令和失败状态；
- CLI 测试：协议、非 TTY、事务和 Windows 进程启动测试。

### 12.3 dshmarket 仓库

- `src/registry.ts`：增加可选伴侣摘要类型和严格读取；
- `src/routes.ts`：接入伴侣预检、安装后验证和回滚；
- `src/dsh-cli.ts`：复用宿主协调器或受限的 Windows 启动边界；
- 市场客户端安装确认、进度、失败和卸载分支；
- registry fixture、Windows 集成测试和安全回归测试。

实现阶段必须优先复用现有 profile snapshot、mutation lock、same-origin、curated registry、入口验证和回滚工具，不重复实现另一套 pnpm 管理器。

## 13. 设计验收门槛

进入实现计划前，需要用户确认以下产品决策：

1. 接受“预构建 npm 包内置已签名 current-user NSIS 安装器”的交付方式；
2. 接受 CLI 交互式确认和非 TTY 下必须显式使用 `--accept-native-companion`；
3. 接受 Web 插件卸载默认不删除独立 DSHtray，只有明确勾选或参数才同时卸载；
4. 接受未签名安装器不能进入市场自动安装；
5. 接受完整安装需要更新 DSH CLI 和 dshmarket 的伴侣协议支持，旧版宿主只报告不兼容，不执行半自动降级。

本文批准只代表批准设计方向，不代表批准开发。设计批准后还需要单独生成实现计划，并在计划确认后才修改 DSH 或 DSHtray 源码。
