import type { RuntimeSnapshot } from "../types";

const stateLabels: Record<RuntimeSnapshot["state"], string> = {
  stopped: "已停止",
  starting: "启动中",
  running: "运行中",
  stopping: "停止中",
  failed: "失败",
  external: "检测到外部 DSH",
  portConflict: "端口冲突",
};

export function StatusCard({ snapshot, url }: { snapshot: RuntimeSnapshot; url: string }) {
  return (
    <section className="card status-card" aria-labelledby="status-heading">
      <div className="status-card__header">
        <div>
          <p className="eyebrow">DeepSeek Harness</p>
          <h2 id="status-heading">服务状态</h2>
        </div>
        <span className={`status-pill status-pill--${snapshot.state}`}>
          {stateLabels[snapshot.state]}
        </span>
      </div>
      <dl className="status-grid">
        <div><dt>服务地址</dt><dd>{url}</dd></div>
        <div><dt>目标</dt><dd>{snapshot.target === "source" ? "源码模式" : "DSH.exe"}</dd></div>
        <div><dt>进程 PID</dt><dd>{snapshot.pid ?? "—"}</dd></div>
        <div><dt>归属</dt><dd>{snapshot.ownership === "external" ? "外部观察" : snapshot.ownership === "adopted" ? "已接管" : snapshot.ownership === "managed" ? "管理器负责" : "—"}</dd></div>
      </dl>
      {snapshot.lastError && <p className="error-text">{snapshot.lastError.message}</p>}
    </section>
  );
}
