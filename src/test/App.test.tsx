import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AppStateDto } from "../types";
import App from "../App";

const { mockApi, mockSubscribeState } = vi.hoisted(() => ({
  mockApi: {
    getAppState: vi.fn(),
    prepareProxyChange: vi.fn(),
    applyProxyChange: vi.fn(),
    completeFirstRun: vi.fn(),
    startDsh: vi.fn(),  },
  mockSubscribeState: vi.fn().mockResolvedValue(() => undefined),
}));

vi.mock("../tauri", () => ({ api: mockApi, subscribeState: mockSubscribeState }));

function state(overrides: Partial<AppStateDto> = {}): AppStateDto {
  return {
    firstRun: false,
    activeTarget: "source",
    serviceHost: "127.0.0.1",
    servicePort: 3080,
    manager: { startOnLogin: true, startDshOnLogin: false, closeToTray: true },
    proxy: { enabled: true, url: "http://127.0.0.1:7897" },
    targets: {
      source: { label: "源码", kind: "source", workingDirectory: "", command: "pnpm", arguments: ["dsh", "web"], executable: "" },
      packaged: { label: "DSH.exe", kind: "packaged", workingDirectory: "", command: "", arguments: [], executable: "" },
    },
    runtime: {
      state: "stopped",
      target: "source",
      pid: null,
      ownership: "none",
      serviceUrl: "http://127.0.0.1:3080",
      proxyEnabled: true,
      lastError: null,
      startedAt: null,
    },
    ...overrides,
  };
}

describe("DSHtray App", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockSubscribeState.mockResolvedValue(() => undefined);
  });

  it("renders stopped state and enables start", async () => {
    mockApi.getAppState.mockResolvedValue(state());
    render(<App />);
    expect(await screen.findByText("已停止")).toBeVisible();
    expect(screen.getByRole("button", { name: "启动 DSH" })).toBeEnabled();
  });

  it("asks before restarting when proxy changes while running", async () => {
    mockApi.prepareProxyChange.mockResolvedValue({
      enabled: false,
      currentEnabled: true,
      requiresRestart: true,
      message: "需要重启 DSH，当前会话可能中断",
    });
    render(
      <App
        initialState={state({
          runtime: {
            ...state().runtime,
            state: "running",
            ownership: "managed",
            pid: 21420,
          },
        })}
      />,
    );
    await userEvent.click(screen.getByRole("switch", { name: "使用代理" }));
    expect(await screen.findByText("需要重启 DSH，当前会话可能中断")).toBeVisible();
    expect(mockApi.applyProxyChange).not.toHaveBeenCalled();
  });

  it("does not auto-start after first-run wizard submission", async () => {
    mockApi.completeFirstRun.mockResolvedValue(state());
    render(<App initialState={state({ firstRun: true })} />);
    await userEvent.click(screen.getByRole("button", { name: "完成配置" }));
    expect(mockApi.completeFirstRun).toHaveBeenCalled();
    expect(mockApi.startDsh).not.toHaveBeenCalled();
  });
});
