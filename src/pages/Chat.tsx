import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { useApp } from "../App";
import { t } from "../i18n";

interface Props { onSettings: () => void }

export default function Chat({ onSettings }: Props) {
  const { theme: T, isDark, toggleTheme, lang } = useApp();
  const [workingDir, setWorkingDir] = useState("");
  const [page, setPage] = useState<"home" | "terminal">("home");
  const [error, setError] = useState("");
  const [showDirPrompt, setShowDirPrompt] = useState(false);
  const [updating, setUpdating] = useState(false);
  const [updateResult, setUpdateResult] = useState<{ ok: boolean; msg: string } | null>(null);
  const [connStatus, setConnStatus] = useState<"unknown" | "ok" | "error">("unknown");
  const [connMsg, setConnMsg] = useState("");

  const termRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);

  useEffect(() => {
    invoke<{ api_key: string; base_url: string; working_dir: string; model: string } | null>("load_config").then((cfg) => {
      if (cfg?.working_dir) setWorkingDir(cfg.working_dir);
      else setShowDirPrompt(true);
      // Show config status without network request
      if (cfg?.api_key && cfg?.base_url) {
        setConnStatus("ok");
        setConnMsg(lang === "zh" ? "已配置" : "Configured");
      } else {
        setConnStatus("error");
        setConnMsg(lang === "zh" ? "未配置" : "Not configured");
      }
    });
  }, []);

  const handleUpdate = async () => {
    setUpdating(true);
    setUpdateResult(null);
    try {
      const msg = await invoke<string>("update_claude_code");
      setUpdateResult({ ok: true, msg });
    } catch (e) {
      setUpdateResult({ ok: false, msg: String(e) });
    }
    setUpdating(false);
  };

  const chooseDir = async () => {
    const sel = await open({ directory: true, multiple: false });
    if (sel) {
      const d = sel as string;
      setWorkingDir(d);
      await invoke("save_working_dir", { dir: d });
    }
  };

  // Start embedded terminal
  const launchInApp = async () => {
    if (!workingDir) {
      await chooseDir();
    }
    setError("");
    setPage("terminal");
  };

  // Initialize xterm when terminal page shows
  useEffect(() => {
    if (page !== "terminal" || !termRef.current) return;

    const xterm = new Terminal({
      cursorBlink: true,
      fontSize: 14,
      fontFamily: "'Cascadia Code', 'Fira Code', Consolas, monospace",
      theme: isDark ? {
        background: "#0f0f23",
        foreground: "#e0e0e0",
        cursor: "#e07a5f",
        selectionBackground: "#3d3d5c",
      } : {
        background: "#ffffff",
        foreground: "#1d1d1f",
        cursor: "#d4603a",
        selectionBackground: "#b4d7ff",
      },
      allowProposedApi: true,
    });
    xtermRef.current = xterm;

    const fit = new FitAddon();
    fitRef.current = fit;
    xterm.loadAddon(fit);
    xterm.open(termRef.current);

    // Debounced fit — prevents rapid resize causing misaligned rows
    let fitTimer: ReturnType<typeof setTimeout> | null = null;
    const debouncedFit = () => {
      if (fitTimer) clearTimeout(fitTimer);
      fitTimer = setTimeout(() => {
        try { fit.fit(); } catch { /* ignore */ }
      }, 50);
    };

    // Initial fit after render
    requestAnimationFrame(() => { fit.fit(); xterm.focus(); });

    // User input → PTY
    xterm.onData((data) => invoke("pty_write", { data }));
    xterm.onResize(({ cols, rows }) => invoke("pty_resize", { cols, rows }));

    // PTY output → xterm
    const unOutput = listen<string>("pty-output", (e) => xterm.write(e.payload));
    const unExit = listen<number>("pty-exit", (e) => {
      xterm.write(`\r\n\x1b[33m[Process exited with code ${e.payload}]\x1b[0m\r\n`);
    });

    // Resize handler — debounced, both window and container
    window.addEventListener("resize", debouncedFit);
    const resizeObs = new ResizeObserver(debouncedFit);
    resizeObs.observe(termRef.current);

    // Spawn Claude Code
    invoke("spawn_claude").catch((e) => {
      const msg = String(e);
      let friendly = msg;
      if (msg.includes("not found")) friendly = "Claude Code resources not found. Please reinstall the app.";
      else if (msg.includes("Working directory")) friendly = "Working directory does not exist. Please choose a valid directory.";
      else if (msg.includes("No config")) friendly = "Please configure your API key and Base URL in Settings first.";
      xterm.write(`\x1b[31m${friendly}\x1b[0m\r\n`);
    });

    return () => {
      if (fitTimer) clearTimeout(fitTimer);
      window.removeEventListener("resize", debouncedFit);
      resizeObs.disconnect();
      unOutput.then((f) => f());
      unExit.then((f) => f());
      invoke("kill_claude").catch(() => {});
      xterm.dispose();
      xtermRef.current = null;
      fitRef.current = null;
    };
  }, [page]);

  // Update theme without recreating terminal
  useEffect(() => {
    if (xtermRef.current) {
      xtermRef.current.options.theme = isDark ? {
        background: "#0f0f23", foreground: "#e0e0e0", cursor: "#e07a5f", selectionBackground: "#3d3d5c",
      } : {
        background: "#ffffff", foreground: "#1d1d1f", cursor: "#d4603a", selectionBackground: "#b4d7ff",
      };
    }
  }, [isDark]);

  // --- Directory prompt ---
  if (showDirPrompt) {
    return (
      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", background: T.bg }}>
        <div style={{ background: T.bgSecondary, borderRadius: 12, padding: 40, maxWidth: 420, textAlign: "center" }}>
          <h2 style={{ fontSize: 20, fontWeight: 600, color: T.text, marginBottom: 8 }}>{lang === "zh" ? "选择工作目录" : "Choose Working Directory"}</h2>
          <p style={{ fontSize: 14, color: T.textMuted, marginBottom: 24 }}>{lang === "zh" ? "Claude Code 将在此目录下工作" : "Claude Code will work in this directory"}</p>
          <div style={{ display: "flex", gap: 12, justifyContent: "center" }}>
            <button onClick={async () => { await chooseDir(); setShowDirPrompt(false); }} style={{ padding: "10px 24px", borderRadius: 8, border: "none", background: T.accent, color: "#fff", fontSize: 14, fontWeight: 600, cursor: "pointer" }}>
              {lang === "zh" ? "选择目录" : "Choose Directory"}
            </button>
            <button onClick={() => setShowDirPrompt(false)} style={{ padding: "10px 24px", borderRadius: 8, border: `1px solid ${T.border}`, background: "transparent", color: T.textSecondary, fontSize: 14, cursor: "pointer" }}>
              {lang === "zh" ? "跳过" : "Skip"}
            </button>
          </div>
        </div>
      </div>
    );
  }

  // --- Terminal view ---
  if (page === "terminal") {
    return (
      <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
        <div style={{ display: "flex", alignItems: "center", padding: "6px 12px", background: T.bgSecondary, borderBottom: `1px solid ${T.border}`, gap: 8 }}>
          <button onClick={() => { invoke("kill_claude").catch(() => {}); setPage("home"); }} style={btnStyle(T)}>
            {lang === "zh" ? "返回" : "Back"}
          </button>
          <span style={{ fontSize: 12, color: T.textMuted, flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {workingDir}
          </span>
          <button onClick={() => invoke("spawn_claude")} style={btnStyle(T)}>
            {lang === "zh" ? "重启" : "Restart"}
          </button>
        </div>
        <div ref={termRef} style={{ flex: 1, overflow: "hidden", background: isDark ? "#0f0f23" : "#ffffff" }} />
      </div>
    );
  }

  // --- Home view ---
  return (
    <div style={{ display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", height: "100%", background: T.bg, gap: 24 }}>
      <h1 style={{ fontSize: 28, fontWeight: 700, color: T.text }}>Claude Code Launcher</h1>
      <span style={{ fontSize: 12, color: T.textMuted, marginTop: -16 }}>v0.9.3</span>

      {/* Connection status */}
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <span style={{ width: 8, height: 8, borderRadius: "50%", background: connStatus === "ok" ? T.success : connStatus === "error" ? T.error : T.textMuted }} />
        <span style={{ fontSize: 13, color: connStatus === "ok" ? T.success : connStatus === "error" ? T.error : T.textMuted }}>
          {connMsg || (lang === "zh" ? "未配置" : "Not configured")}
        </span>
      </div>

      <div style={{ background: T.bgSecondary, borderRadius: 12, padding: 32, width: 400, display: "flex", flexDirection: "column", gap: 16 }}>
        <div>
          <label style={{ fontSize: 13, fontWeight: 600, color: T.textSecondary, display: "block", marginBottom: 6 }}>
            {lang === "zh" ? "工作目录" : "Working Directory"}
          </label>
          <div style={{ display: "flex", gap: 8 }}>
            <div style={{ flex: 1, padding: "10px 14px", borderRadius: 8, border: `1px solid ${T.border}`, background: T.bg, color: T.text, fontSize: 13, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {workingDir || (lang === "zh" ? "未选择" : "Not selected")}
            </div>
            <button onClick={chooseDir} style={{ padding: "10px 14px", borderRadius: 8, border: `1px solid ${T.border}`, background: T.bg, color: T.accent, fontSize: 13, cursor: "pointer", whiteSpace: "nowrap", fontWeight: 600 }}>
              {lang === "zh" ? "选择" : "Browse"}
            </button>
          </div>
        </div>

        <button onClick={launchInApp}
          style={{ width: "100%", padding: "14px 0", borderRadius: 8, border: "none", background: T.accent, color: "#fff", fontSize: 16, fontWeight: 700, cursor: "pointer" }}>
          {lang === "zh" ? "启动 Claude Code" : "Launch Claude Code"}
        </button>

        {error && <p style={{ color: T.error, fontSize: 13, textAlign: "center", wordBreak: "break-word" }}>{error}</p>}
      </div>

      <div style={{ display: "flex", gap: 12, alignItems: "center" }}>
        <button onClick={onSettings} style={btnStyle(T)}>{t(lang, "settings")}</button>
        <button onClick={toggleTheme} style={btnStyle(T)}>{isDark ? "Light" : "Dark"}</button>
        <button onClick={handleUpdate} disabled={updating} style={{ ...btnStyle(T), opacity: updating ? 0.6 : 1 }}>
          {updating ? (lang === "zh" ? "更新中..." : "Updating...") : (lang === "zh" ? "更新 Claude Code" : "Update Claude Code")}
        </button>
      </div>
      {updateResult && <p style={{ fontSize: 13, color: updateResult.ok ? T.success : T.error }}>{updateResult.msg}</p>}
    </div>
  );
}

function btnStyle(T: ReturnType<typeof import("../App").useApp>["theme"]): React.CSSProperties {
  return { padding: "8px 16px", borderRadius: 6, border: `1px solid ${T.border}`, background: T.bg, color: T.textSecondary, cursor: "pointer", fontSize: 13 };
}
