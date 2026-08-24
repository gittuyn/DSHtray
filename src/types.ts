export type TargetId = "source" | "packaged";
export type TargetKind = "source" | "packaged";
export type LifecycleState =
  | "stopped"
  | "starting"
  | "running"
  | "stopping"
  | "failed"
  | "external"
  | "portConflict";
export type Ownership = "none" | "managed" | "external" | "adopted";

export interface RuntimeSnapshot {
  state: LifecycleState;
  target: TargetId;
  pid: number | null;
  ownership: Ownership;
  serviceUrl: string;
  proxyEnabled: boolean;
  lastError: AppError | null;
  startedAt?: string | null;
}

export interface AppError {
  code: string;
  message: string;
  details?: string;
}

export interface TargetConfig {
  label: string;
  kind: TargetKind;
  workingDirectory: string;
  command: string;
  arguments: string[];
  executable: string;
}

export interface TargetsConfig {
  source: TargetConfig;
  packaged: TargetConfig;
}

export interface ProxyConfig {
  enabled: boolean;
  url: string;
}

export interface ManagerConfig {
  startOnLogin: boolean;
  startDshOnLogin: boolean;
  closeToTray: boolean;
}

export interface AppStateDto {
  firstRun: boolean;
  runtime: RuntimeSnapshot;
  manager: ManagerConfig;
  activeTarget: TargetId;
  targets: TargetsConfig;
  serviceHost: string;
  servicePort: number;
  proxy: ProxyConfig;
}

export interface ProxyChangePlan {
  enabled: boolean;
  currentEnabled: boolean;
  requiresRestart: boolean;
  message: string;
}

export interface DiscoveredTarget {
  id: TargetId;
  kind: TargetKind;
  label: string;
  workingDirectory: string;
  executable: string | null;
  valid: boolean;
  needsUserConfirmation: boolean;
  reason: string;
}

export interface FirstRunSetup {
  source: DiscoveredTarget | null;
  packaged: DiscoveredTarget | null;
  activeTarget: TargetId;
  proxyEnabled: boolean;
  proxyUrl: string;
  startOnLogin: boolean;
  startDshOnLogin: boolean;
}

export interface SelfTestCheck {
  name: string;
  passed: boolean;
  message: string;
}

export interface SelfTestReport {
  healthy: boolean;
  checks: SelfTestCheck[];
}
