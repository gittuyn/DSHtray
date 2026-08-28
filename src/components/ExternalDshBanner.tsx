import type { RuntimeSnapshot } from "../types";

export function ExternalDshBanner({ snapshot, onAdopt }: { snapshot: RuntimeSnapshot; onAdopt: () => void }) {
  if (snapshot.state !== "external") return null;
  return (
    <section className="notice notice--warning">
      <div><strong>发现外部 DSH 进程</strong><p>当前仅观察，不会停止或重启它。确认接管后会重新校验进程身份；即使它属于其他 Job Object，也只会使用已确认 PID 的精确控制。</p></div>
      <button type="button" onClick={onAdopt}>确认接管</button>
    </section>
  );
}
