import type { DiscoveredTarget, FirstRunSetup } from "../types";

export function FirstRunWizard({
  candidates,
  proxyUrl,
  onComplete,
}: {
  candidates: DiscoveredTarget[];
  proxyUrl: string;
  onComplete: (setup: FirstRunSetup) => void;
}) {
  const source = candidates.find((candidate) => candidate.id === "source") ?? null;
  const packaged = candidates.find((candidate) => candidate.id === "packaged") ?? null;
  return (
    <section className="card wizard" aria-labelledby="wizard-heading">
      <p className="eyebrow">首次运行</p>
      <h2 id="wizard-heading">配置 DSHtray</h2>
      <p>请选择一个已发现的 DSH 目标。完成后只保存配置，不会自动启动 DSH。</p>
      <div className="wizard-candidates">
        {candidates.length === 0 ? <p className="muted">尚未发现目标，可在设置中重新扫描。</p> : candidates.map((candidate) => (
          <div className="candidate" key={`${candidate.id}-${candidate.workingDirectory}`}>
            <strong>{candidate.label}</strong>
            <span>{candidate.workingDirectory}</span>
          </div>
        ))}
      </div>
      <button
        type="button"
        onClick={() => onComplete({
          source,
          packaged,
          activeTarget: source ? "source" : "packaged",
          proxyEnabled: true,
          proxyUrl,
          startOnLogin: true,
          startDshOnLogin: false,
        })}
      >
        完成配置
      </button>
    </section>
  );
}
