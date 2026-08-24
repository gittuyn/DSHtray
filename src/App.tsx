import { useEffect, useState } from "react";
import { ActionBar } from "./components/ActionBar";
import { ExternalDshBanner } from "./components/ExternalDshBanner";
import { FirstRunWizard } from "./components/FirstRunWizard";
import { ProxySettings } from "./components/ProxySettings";
import { StatusCard } from "./components/StatusCard";
import { api, subscribeState } from "./tauri";
import type {
  AppError,
  AppStateDto,
  DiscoveredTarget,
  FirstRunSetup,
  ProxyChangePlan,
  RuntimeSnapshot,
  SelfTestReport,
} from "./types";
import "./styles.css";

function toError(error: unknown): AppError {
  if (typeof error === "object" && error !== null && "message" in error) {
    const value = error as Partial<AppError>;
    return {
      code: value.code ?? "unknown_error",
      message: value.message ?? "操作失败",
      details: value.details,
    };
  }
  return { code: "unknown_error", message: String(error) };
}

export default function App({ initialState }: { initialState?: AppStateDto }) {
  const [appState, setAppState] = useState<AppStateDto | null>(initialState ?? null);
  const [error, setError] = useState<AppError | null>(null);
  const [candidates, setCandidates] = useState<DiscoveredTarget[]>([]);
  const [proxyPlan, setProxyPlan] = useState<ProxyChangePlan | null>(null);
  const [selfTest, setSelfTest] = useState<SelfTestReport | null>(null);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    const load = async () => {
      try {
        const loaded = initialState ?? (await api.getAppState());
        if (cancelled) return;
        setAppState(loaded);
        if (loaded.firstRun) {
          try {
            setCandidates(await api.scanTargets());
          } catch {
            setCandidates([]);
          }
        }
        unlisten = await subscribeState((next) => {
          if (!cancelled) setAppState(next);
        });
      } catch (cause) {
        if (!cancelled) setError(toError(cause));
      }
    };
    void load();
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [initialState]);

  const updateRuntime = (runtime: RuntimeSnapshot) => {
    setAppState((current) => (current ? { ...current, runtime } : current));
  };

  const run = async (operation: () => Promise<RuntimeSnapshot>) => {
    setError(null);
    try {
      updateRuntime(await operation());
    } catch (cause) {
      setError(toError(cause));
    }
  };

  const handleProxyToggle = async (enabled: boolean) => {
    if (!appState) return;
    setError(null);
    try {
      const plan = await api.prepareProxyChange(enabled);
      if (plan.requiresRestart) {
        setProxyPlan(plan);
        return;
      }
      updateRuntime(await api.applyProxyChange(enabled, false));
      setAppState((current) => current && { ...current, proxy: { ...current.proxy, enabled } });
    } catch (cause) {
      setError(toError(cause));
    }
  };

  const confirmProxyChange = async () => {
    if (!proxyPlan) return;
    try {
      updateRuntime(await api.applyProxyChange(proxyPlan.enabled, true));
      setAppState((current) => current && { ...current, proxy: { ...current.proxy, enabled: proxyPlan.enabled } });
      setProxyPlan(null);
    } catch (cause) {
      setError(toError(cause));
    }
  };

  const completeFirstRun = async (setup: FirstRunSetup) => {
    try {
      setAppState(await api.completeFirstRun(setup));
    } catch (cause) {
      setError(toError(cause));
    }
  };

  if (!appState) {
    return <main className="app-shell"><p className="loading">正在读取管理器状态…</p></main>;
  }

  return (
    <main className="app-shell">
      <header className="app-header">
        <div>
          <p className="eyebrow">Windows 托盘管理器</p>
          <h1>DSHtray</h1>
        </div>
        <span className="version-tag">本机服务</span>
      </header>

      {appState.firstRun ? (
        <FirstRunWizard
          candidates={candidates}
          proxyUrl={appState.proxy.url}
          onComplete={completeFirstRun}
        />
      ) : (
        <>
          <StatusCard snapshot={appState.runtime} url={appState.runtime.serviceUrl} />
          <ExternalDshBanner snapshot={appState.runtime} onAdopt={() => void api.adoptExternalDsh().then(updateRuntime).catch((cause) => setError(toError(cause)))} />
          <ActionBar
            state={appState.runtime.state}
            onStart={() => void run(api.startDsh)}
            onStop={() => void run(api.stopDsh)}
            onRestart={() => void run(api.restartDsh)}
          />
          <ProxySettings proxy={appState.proxy} onToggle={(enabled) => void handleProxyToggle(enabled)} />
          <section className="card utility-card">
            <button type="button" className="secondary" onClick={() => void api.openDshUrl().catch((cause) => setError(toError(cause)))}>打开 DSH 页面</button>
            <button type="button" className="secondary" onClick={() => void api.runSelfTest().then(setSelfTest).catch((cause) => setError(toError(cause)))}>运行自检</button>
            <button type="button" className="secondary" onClick={() => void api.openLogDirectory().catch((cause) => setError(toError(cause)))}>打开日志目录</button>
          </section>
          {selfTest && <section className="card"><h3>自检结果：{selfTest.healthy ? "通过" : "需要处理"}</h3>{selfTest.checks.map((check) => <p key={check.name} className={check.passed ? "check-pass" : "check-fail"}>{check.passed ? "✓" : "!"} {check.message}</p>)}</section>}
        </>
      )}

      {proxyPlan && (
        <div className="dialog-backdrop" role="presentation">
          <section className="dialog" role="dialog" aria-modal="true" aria-labelledby="proxy-dialog-title">
            <h2 id="proxy-dialog-title">确认重启 DSH</h2>
            <p>{proxyPlan.message}</p>
            <div className="dialog-actions">
              <button type="button" className="secondary" onClick={() => setProxyPlan(null)}>取消</button>
              <button type="button" onClick={() => void confirmProxyChange()}>确认重启</button>
            </div>
          </section>
        </div>
      )}
      {error && <div className="error-banner" role="alert"><strong>{error.code}</strong><span>{error.message}</span><button type="button" onClick={() => setError(null)} aria-label="关闭错误">×</button></div>}
    </main>
  );
}
