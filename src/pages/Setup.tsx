import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useApp } from "../App";
import { t, LANGS } from "../i18n";

interface Profile {
  name: string;
  api_key: string;
  base_url: string;
}

interface Props {
  onSaved: () => void;
}

const PRESETS: Profile[] = [
  { name: "Anthropic (Direct)", api_key: "", base_url: "https://api.anthropic.com" },
  { name: "Pincc.ai", api_key: "", base_url: "https://v2.pincc.ai" },
  { name: "MiniMaxi", api_key: "", base_url: "https://api.minimaxi.com/anthropic" },
];

function mask(s: string): string {
  if (!s) return "";
  if (s.length <= 8) return "*".repeat(s.length);
  return s.slice(0, 4) + "*".repeat(Math.min(s.length - 8, 20)) + s.slice(-4);
}

export default function Setup({ onSaved }: Props) {
  const { theme: T, isDark, toggleTheme, lang, setLang } = useApp();
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [activeIdx, setActiveIdx] = useState(-1);
  const [editing, setEditing] = useState(false);
  const [editKey, setEditKey] = useState("");
  const [editUrl, setEditUrl] = useState("");
  const [editName, setEditName] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<{ ok: boolean; msg: string } | null>(null);

  useEffect(() => {
    invoke<{ api_key: string; base_url: string; profiles: Profile[]; active_profile: string } | null>("load_config").then((cfg) => {
      if (cfg?.profiles?.length) {
        setProfiles(cfg.profiles);
        const idx = cfg.profiles.findIndex((p) => p.name === cfg.active_profile);
        if (idx >= 0) setActiveIdx(idx);
      }
    });
  }, []);

  const startNew = () => {
    setEditing(true);
    setEditName("");
    setEditKey("");
    setEditUrl("");
    setTestResult(null);
    setError("");
  };

  const startEdit = (idx: number) => {
    setActiveIdx(idx);
    setEditing(true);
    setEditName(profiles[idx].name);
    setEditKey(profiles[idx].api_key);
    setEditUrl(profiles[idx].base_url);
    setTestResult(null);
    setError("");
  };

  const deleteProfile = (idx: number) => {
    const updated = profiles.filter((_, i) => i !== idx);
    setProfiles(updated);
    if (activeIdx === idx) setActiveIdx(updated.length > 0 ? 0 : -1);
    else if (activeIdx > idx) setActiveIdx(activeIdx - 1);
    invoke("save_profiles", { profiles: updated, activeProfile: updated.length > 0 ? updated[Math.min(activeIdx, updated.length - 1)]?.name || "" : "" });
  };

  const handleTest = async () => {
    if (!editKey.trim() || !editUrl.trim()) { setError(lang === "zh" ? "请填写 Key 和 URL" : "Fill in Key and URL"); return; }
    setTesting(true);
    setTestResult(null);
    try {
      const msg = await invoke<string>("test_connection", { apiKey: editKey.trim(), baseUrl: editUrl.trim() });
      setTestResult({ ok: true, msg });
    } catch (e) {
      setTestResult({ ok: false, msg: String(e) });
    }
    setTesting(false);
  };

  const handleSave = async () => {
    if (!editName.trim()) { setError(lang === "zh" ? "请输入配置名称" : "Enter a profile name"); return; }
    if (!editKey.trim()) { setError(lang === "zh" ? "请输入 API Key" : "Enter API Key"); return; }
    if (!editUrl.trim()) { setError(lang === "zh" ? "请输入 Base URL" : "Enter Base URL"); return; }
    setSaving(true);
    setError("");
    try {
      const p: Profile = { name: editName.trim(), api_key: editKey.trim(), base_url: editUrl.trim() };
      // Update or add
      const existing = profiles.findIndex((x) => x.name === p.name);
      let updated: Profile[];
      if (existing >= 0) {
        updated = [...profiles];
        updated[existing] = p;
      } else {
        updated = [...profiles, p];
      }
      setProfiles(updated);
      const newIdx = updated.findIndex((x) => x.name === p.name);
      setActiveIdx(newIdx);

      await invoke("save_config", { apiKey: p.api_key, baseUrl: p.base_url });
      await invoke("save_profiles", { profiles: updated, activeProfile: p.name });
      setEditing(false);
      onSaved();
    } catch (e) {
      setError(String(e));
    }
    setSaving(false);
  };

  const selectAndLaunch = async (idx: number) => {
    const p = profiles[idx];
    setActiveIdx(idx);
    await invoke("save_config", { apiKey: p.api_key, baseUrl: p.base_url });
    await invoke("save_profiles", { profiles, activeProfile: p.name });
    onSaved();
  };

  const applyPreset = (preset: Profile) => {
    setEditUrl(preset.base_url);
    if (!editName) setEditName(preset.name);
  };

  const shadow = isDark ? "0 8px 32px rgba(0,0,0,0.3)" : "0 8px 32px rgba(0,0,0,0.08)";

  // --- Editing view ---
  if (editing) {
    return (
      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", padding: 20, background: T.bg, overflowY: "auto" }}>
        <div style={{ background: T.bgSecondary, borderRadius: 12, padding: 36, width: "100%", maxWidth: 480, boxShadow: shadow }}>
          <h2 style={{ fontSize: 20, fontWeight: 700, color: T.text, marginBottom: 20 }}>
            {editName ? (lang === "zh" ? "编辑配置" : "Edit Profile") : (lang === "zh" ? "新建配置" : "New Profile")}
          </h2>

          {/* Name */}
          <div style={{ marginBottom: 14 }}>
            <label style={labelStyle(T)}>{lang === "zh" ? "配置名称" : "Profile Name"}</label>
            <input value={editName} onChange={(e) => setEditName(e.target.value)} placeholder="My Config"
              style={inputStyle(T)} />
          </div>

          {/* Presets */}
          <div style={{ marginBottom: 14 }}>
            <label style={labelStyle(T)}>{lang === "zh" ? "快速选择" : "Presets"}</label>
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              {PRESETS.map((p) => (
                <button key={p.name} onClick={() => applyPreset(p)}
                  style={{ padding: "5px 12px", borderRadius: 6, border: `1px solid ${editUrl === p.base_url ? T.accent : T.border}`, background: editUrl === p.base_url ? T.accent : T.bg, color: editUrl === p.base_url ? "#fff" : T.textSecondary, cursor: "pointer", fontSize: 12 }}>
                  {p.name}
                </button>
              ))}
            </div>
          </div>

          {/* API Key */}
          <div style={{ marginBottom: 14 }}>
            <label style={labelStyle(T)}>API Key</label>
            <input type="password" value={editKey} onChange={(e) => setEditKey(e.target.value)}
              placeholder="sk-ant-..." style={inputStyle(T)} autoComplete="off" />
          </div>

          {/* Base URL */}
          <div style={{ marginBottom: 14 }}>
            <label style={labelStyle(T)}>API Base URL</label>
            <input type="password" value={editUrl} onChange={(e) => setEditUrl(e.target.value)}
              placeholder="https://api.example.com" style={inputStyle(T)} autoComplete="off" />
          </div>

          {/* Test result */}
          {testResult && (
            <p style={{ fontSize: 13, marginBottom: 10, color: testResult.ok ? T.success : T.error, wordBreak: "break-word" }}>
              {testResult.msg}
            </p>
          )}
          {error && <p style={{ fontSize: 13, marginBottom: 10, color: T.error }}>{error}</p>}

          {/* Buttons */}
          <div style={{ display: "flex", gap: 8 }}>
            <button onClick={() => setEditing(false)}
              style={{ flex: 1, padding: "11px 0", borderRadius: 8, border: `1px solid ${T.border}`, background: "transparent", color: T.textSecondary, fontSize: 14, cursor: "pointer" }}>
              {lang === "zh" ? "取消" : "Cancel"}
            </button>
            <button onClick={handleTest} disabled={testing}
              style={{ flex: 1, padding: "11px 0", borderRadius: 8, border: `1px solid ${T.border}`, background: T.bg, color: T.text, fontSize: 14, fontWeight: 600, cursor: "pointer", opacity: testing ? 0.6 : 1 }}>
              {testing ? "..." : (lang === "zh" ? "测试连接" : "Test")}
            </button>
            <button onClick={handleSave} disabled={saving}
              style={{ flex: 1, padding: "11px 0", borderRadius: 8, border: "none", background: T.accent, color: "#fff", fontSize: 14, fontWeight: 600, cursor: "pointer", opacity: saving ? 0.6 : 1 }}>
              {saving ? "..." : (lang === "zh" ? "保存" : "Save")}
            </button>
          </div>
        </div>
      </div>
    );
  }

  // --- Profile list view ---
  return (
    <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", padding: 20, background: T.bg, overflowY: "auto" }}>
      <div style={{ background: T.bgSecondary, borderRadius: 12, padding: 36, width: "100%", maxWidth: 480, boxShadow: shadow }}>
        <h1 style={{ fontSize: 22, fontWeight: 700, color: T.text, marginBottom: 4 }}>Claude Code Launcher</h1>
        <p style={{ fontSize: 13, color: T.textMuted, marginBottom: 20 }}>{t(lang, "configTitle")}</p>

        {/* Profile list */}
        {profiles.length === 0 && (
          <p style={{ fontSize: 14, color: T.textMuted, marginBottom: 20, textAlign: "center", padding: 24 }}>
            {lang === "zh" ? "还没有配置，点击下方按钮新建" : "No profiles yet. Click below to create one."}
          </p>
        )}

        <div style={{ display: "flex", flexDirection: "column", gap: 8, marginBottom: 16 }}>
          {profiles.map((p, i) => (
            <div key={p.name} style={{ display: "flex", alignItems: "center", gap: 8, padding: "12px 14px", borderRadius: 8, background: T.bg, border: `1px solid ${activeIdx === i ? T.accent : T.border}` }}>
              <div style={{ flex: 1, cursor: "pointer" }} onClick={() => selectAndLaunch(i)}>
                <div style={{ fontSize: 14, fontWeight: 600, color: T.text }}>{p.name}</div>
                <div style={{ fontSize: 12, color: T.textMuted, marginTop: 2 }}>
                  Key: {mask(p.api_key)} &nbsp; URL: {mask(p.base_url)}
                </div>
              </div>
              <button onClick={() => startEdit(i)}
                style={{ padding: "5px 10px", borderRadius: 4, border: `1px solid ${T.border}`, background: "transparent", color: T.textSecondary, cursor: "pointer", fontSize: 12 }}>
                {lang === "zh" ? "编辑" : "Edit"}
              </button>
              <button onClick={() => deleteProfile(i)}
                style={{ padding: "5px 8px", borderRadius: 4, border: `1px solid ${T.border}`, background: "transparent", color: T.error, cursor: "pointer", fontSize: 12, fontWeight: 700 }}>
                x
              </button>
            </div>
          ))}
        </div>

        {/* New profile button */}
        <button onClick={startNew}
          style={{ width: "100%", padding: "12px 0", borderRadius: 8, border: `2px dashed ${T.border}`, background: "transparent", color: T.accent, fontSize: 14, fontWeight: 600, cursor: "pointer" }}>
          + {lang === "zh" ? "新建配置" : "New Profile"}
        </button>

        {/* Language + Theme */}
        <div style={{ display: "flex", gap: 12, marginTop: 20 }}>
          <select value={lang} onChange={(e) => setLang(e.target.value as "en" | "zh")}
            style={{ flex: 1, padding: "8px 10px", borderRadius: 6, border: `1px solid ${T.border}`, background: T.bg, color: T.text, fontSize: 12, cursor: "pointer", outline: "none" }}>
            {LANGS.map((l) => <option key={l.id} value={l.id}>{l.label}</option>)}
          </select>
          <button onClick={toggleTheme}
            style={{ flex: 1, padding: "8px 10px", borderRadius: 6, border: `1px solid ${T.border}`, background: T.bg, color: T.textSecondary, fontSize: 12, cursor: "pointer" }}>
            {isDark ? "Light" : "Dark"}
          </button>
        </div>
      </div>
    </div>
  );
}

function labelStyle(T: ReturnType<typeof import("../App").useApp>["theme"]): React.CSSProperties {
  return { display: "block", fontSize: 13, fontWeight: 600, marginBottom: 6, color: T.textSecondary };
}

function inputStyle(T: ReturnType<typeof import("../App").useApp>["theme"]): React.CSSProperties {
  return { width: "100%", padding: "10px 14px", borderRadius: 8, border: `1px solid ${T.border}`, background: T.bg, color: T.text, fontSize: 14, outline: "none" };
}
