import type { LifecycleState } from "../types";

export function ActionBar({
  state,
  onStart,
  onStop,
  onRestart,
}: {
  state: LifecycleState;
  onStart: () => void;
  onStop: () => void;
  onRestart: () => void;
}) {
  const busy = state === "starting" || state === "stopping";
  return (
    <div className="action-bar">
      <button type="button" onClick={onStart} disabled={busy || state !== "stopped"}>
        启动 DSH
      </button>
      <button type="button" onClick={onStop} disabled={busy || !["running", "external"].includes(state)}>
        停止 DSH
      </button>
      <button type="button" className="secondary" onClick={onRestart} disabled={busy || state !== "running"}>
        重启 DSH
      </button>
    </div>
  );
}
