import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useApp } from "../App";
import { t } from "../i18n";

interface Props { onSettings: () => void }

export default function Chat({ onSettings }: Props) {
  const { theme: T, isDark, toggleTheme, lang } = useApp();
  const [workingDir, setWorkingDir] = useState("");
  const [running, setRunning] = useState(false);
  const [error, setError] = useState("");
  const [showDirPrompt, setShowDirPrompt] = useState(false);

  useEffect(() => {
    invoke<{ api_key: string; base_url: string; working_dir: string; model: string } | null>("load_config").then((cfg) => {
      if (cfg?.working_dir) setWorkingDir(cfg.working_dir);
      else setShowDirPrompt(true);
    });
  }, []);

  const chooseDir = async () => {
    const sel = await open({ directory: true, multiple: false });
    if (sel) {
      const d = sel as string;
      setWorkingDir(d);
      await invoke("save_working_dir", { dir: d });
    }
  };

  const chooseDirPrompt = async () => {
    await chooseDir();
    setShowDirPrompt(false);
  };

  const launchClaude = async () => {
    setRunning(true);
    setError("");
    try {
      await invoke("launch_claude_code");
    } catch (e) {
      setError(String(e));
    }
    setRunning(false);
  };

  if (showDirPrompt) {
    return (
      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", background: T.bg }}>
        <div style={{ background: T.bgSecondary, borderRadius: 12, padding: 40, maxWidth: 420, textAlign: "center" }}>
          <h2 style={{ fontSize: 20, fontWeight: 600, color: T.text, marginBottom: 8 }}>{lang === "zh" ? "选择工作目录" : "Choose Working Directory"}</h2>
          <p style={{ fontSize: 14, color: T.textMuted, marginBottom: 24 }}>{lang === "zh" ? "Claude Code 将在此目录下工作" : "Claude Code will work in this directory"}</p>
          <div style={{ display: "flex", gap: 12, justifyContent: "center" }}>
            <button onClick={chooseDirPrompt} style={{ padding: "10px 24px", borderRadius: 8, border: "none", background: T.accent, color: "#fff", fontSize: 14, fontWeight: 600, cursor: "pointer" }}>
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

  return (
    <div style={{ display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", height: "100%", background: T.bg, gap: 24 }}>
      <h1 style={{ fontSize: 28, fontWeight: 700, color: T.text }}>Claude Code Launcher</h1>

      <div style={{ background: T.bgSecondary, borderRadius: 12, padding: 32, width: 400, display: "flex", flexDirection: "column", gap: 16 }}>
        {/* Working Directory */}
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

        {/* Launch Button */}
        <button
          onClick={launchClaude}
          disabled={running}
          style={{ width: "100%", padding: "14px 0", borderRadius: 8, border: "none", background: T.accent, color: "#fff", fontSize: 16, fontWeight: 700, cursor: "pointer", opacity: running ? 0.6 : 1 }}
        >
          {running
            ? (lang === "zh" ? "Claude Code 运行中..." : "Claude Code Running...")
            : (lang === "zh" ? "启动 Claude Code" : "Launch Claude Code")}
        </button>

        {error && <p style={{ color: T.error, fontSize: 13, marginTop: 8, textAlign: "center", wordBreak: "break-word" }}>{error}</p>}
      </div>

      {/* Bottom buttons */}
      <div style={{ display: "flex", gap: 12 }}>
        <button onClick={onSettings} style={btnStyle(T)}>{t(lang, "settings")}</button>
        <button onClick={toggleTheme} style={btnStyle(T)}>{isDark ? "Light" : "Dark"}</button>
      </div>
    </div>
  );
}

function btnStyle(T: ReturnType<typeof import("../App").useApp>["theme"]): React.CSSProperties {
  return { padding: "8px 16px", borderRadius: 6, border: `1px solid ${T.border}`, background: T.bg, color: T.textSecondary, cursor: "pointer", fontSize: 13 };
}
