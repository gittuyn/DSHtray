import type { ProxyConfig } from "../types";

export function ProxySettings({
  proxy,
  onToggle,
}: {
  proxy: ProxyConfig;
  onToggle: (enabled: boolean) => void;
}) {
  return (
    <section className="card settings-card">
      <div className="setting-row">
        <div>
          <h3>网络代理</h3>
          <p>关闭时，管理器不主动注入或清理 DSH 环境变量。</p>
          <code>{proxy.url}</code>
        </div>
        <label className="switch-label">
          <span>使用代理</span>
          <input
            type="checkbox"
            role="switch"
            aria-label="使用代理"
            checked={proxy.enabled}
            onChange={(event) => onToggle(event.currentTarget.checked)}
          />
        </label>
      </div>
    </section>
  );
}
