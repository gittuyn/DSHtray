import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppStateDto,
  DiscoveredTarget,
  FirstRunSetup,
  ProxyChangePlan,
  RuntimeSnapshot,
  SelfTestReport,
  TargetId,
} from "./types";

export interface SettingsPatch {
  startOnLogin?: boolean;
  startDshOnLogin?: boolean;
  servicePort?: number;
  proxyEnabled?: boolean;
  proxyUrl?: string;
}

export const api = {
  getAppState: () => invoke<AppStateDto>("get_app_state"),
  startDsh: () => invoke<RuntimeSnapshot>("start_dsh"),
  stopDsh: () => invoke<RuntimeSnapshot>("stop_dsh"),
  restartDsh: () => invoke<RuntimeSnapshot>("restart_dsh"),
  prepareProxyChange: (enabled: boolean) =>
    invoke<ProxyChangePlan>("prepare_proxy_change", { enabled }),
  applyProxyChange: (enabled: boolean, confirmedRestart: boolean) =>
    invoke<RuntimeSnapshot>("apply_proxy_change", { enabled, confirmedRestart }),
  setActiveTarget: (targetId: TargetId) =>
    invoke<AppStateDto>("set_active_target", { targetId }),
  saveSettings: (settings: SettingsPatch) =>
    invoke<AppStateDto>("save_settings", { settings }),
  scanTargets: () => invoke<DiscoveredTarget[]>("scan_targets"),
  completeFirstRun: (setup: FirstRunSetup) =>
    invoke<AppStateDto>("complete_first_run", { setup }),
  adoptExternalDsh: () => invoke<RuntimeSnapshot>("adopt_external_dsh"),
  runSelfTest: () => invoke<SelfTestReport>("run_self_test"),
  openDshUrl: () => invoke<void>("open_dsh_url"),
  openLogDirectory: () => invoke<void>("open_log_directory"),
};

export function subscribeState(handler: (state: AppStateDto) => void): Promise<UnlistenFn> {
  return listen<AppStateDto>("state_changed", (event) => handler(event.payload));
}
