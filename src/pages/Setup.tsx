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
  return s.slice(0, 4) + "*".repeat(s.length - 8) + s.slice(-4);
}

export default function Setup({ onSaved }: Props) {
  const { theme: T, isDark, toggleTheme, lang, setLang } = useApp();
  const [apiKey, setApiKey] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<{ ok: boolean; msg: string } | null>(null);
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [activeProfile, setActiveProfile] = useState("");
  // Track whether fields have been loaded from saved config (show masked) or are being edited
  const [keyEdited, setKeyEdited] = useState(false);
  const [urlEdited, setUrlEdited] = useState(false);
  const [savedKey, setSavedKey] = useState("");
  const [savedUrl, setSavedUrl] = useState("");

  useEffect(() => {
    invoke<{ api_key: string; base_url: string; profiles: Profile[]; active_profile: string } | null>("load_config").then(
      (config) => {
        if (config) {
          setSavedKey(config.api_key || "");
          setSavedUrl(config.base_url || "");
          setApiKey(config.api_key || "");
          setBaseUrl(config.base_url || "");
          if (config.profiles?.length) setProfiles(config.profiles);
          if (config.active_profile) setActiveProfile(config.active_profile);
        }
      },
    );
  }, []);

  const handleSave = async () => {
    const finalKey = keyEdited ? apiKey.trim() : savedKey;
    const finalUrl = urlEdited ? baseUrl.trim() : savedUrl;
    if (!finalKey) { setError(t(lang, "enterApiKey")); return; }
    if (!finalUrl) { setError(t(lang, "enterBaseUrl")); return; }
    setSaving(true);
    setError("");
    try {
      await invoke("save_config", { apiKey: finalKey, baseUrl: finalUrl });
      const updated = profiles.map((p) =>
        p.name === activeProfile ? { ...p, api_key: finalKey, base_url: finalUrl } : p
      );
      await invoke("save_profiles", { profiles: updated, activeProfile });
      onSaved();
    } catch (e) {
      setError(String(e));
      setSaving(false);
    }
  };

  const handleTest = async () => {
    const finalKey = keyEdited ? apiKey.trim() : savedKey;
    const finalUrl = urlEdited ? baseUrl.trim() : savedUrl;
    if (!finalKey || !finalUrl) { setError(lang === "zh" ? "请先填写 Key 和 URL" : "Fill in Key and URL first"); return; }
    setTesting(true);
    setTestResult(null);
    setError("");
    try {
      const msg = await invoke<string>("test_connection", { apiKey: finalKey, baseUrl: finalUrl });
      setTestResult({ ok: true, msg });
    } catch (e) {
      setTestResult({ ok: false, msg: String(e) });
    }
    setTesting(false);
  };

  const selectProfile = (name: string) => {
    setActiveProfile(name);
    const p = profiles.find((x) => x.name === name);
    if (p) {
      setSavedKey(p.api_key);
      setSavedUrl(p.base_url);
      setApiKey(p.api_key);
      setBaseUrl(p.base_url);
      setKeyEdited(false);
      setUrlEdited(false);
    }
    setTestResult(null);
  };

  const applyPreset = (preset: Profile) => {
    setBaseUrl(preset.base_url);
    setSavedUrl(preset.base_url);
    setUrlEdited(false);
    if (!profiles.find((p) => p.name === preset.name)) {
      const newP = { ...preset, api_key: savedKey || apiKey };
      setProfiles([...profiles, newP]);
    }
    setActiveProfile(preset.name);
    setTestResult(null);
  };

  const addProfile = () => {
    const name = prompt(lang === "zh" ? "输入配置名称:" : "Enter profile name:");
    if (!name) return;
    const finalKey = keyEdited ? apiKey : savedKey;
    const finalUrl = urlEdited ? baseUrl : savedUrl;
    const newP: Profile = { name, api_key: finalKey, base_url: finalUrl };
    const updated = profiles.filter((p) => p.name !== name);
    updated.push(newP);
    setProfiles(updated);
    setActiveProfile(name);
    setSavedKey(finalKey);
    setSavedUrl(finalUrl);
    setKeyEdited(false);
    setUrlEdited(false);
    invoke("save_profiles", { profiles: updated, activeProfile: name });
  };

  const deleteProfile = (name: string) => {
    const updated = profiles.filter((p) => p.name !== name);
    setProfiles(updated);
    if (activeProfile === name) setActiveProfile("");
    invoke("save_profiles", { profiles: updated, activeProfile: activeProfile === name ? "" : activeProfile });
  };

  const shadow = isDark ? "0 8px 32px rgba(0,0,0,0.3)" : "0 8px 32px rgba(0,0,0,0.08)";

  return (
    <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", padding: 20, background: T.bg, overflowY: "auto" }}>
      <div style={{ background: T.bgSecondary, borderRadius: 12, padding: 36, width: "100%", maxWidth: 500, boxShadow: shadow }}>
        <h1 style={{ fontSize: 22, fontWeight: 700, color: T.text, marginBottom: 4 }}>Claude Code Launcher</h1>
        <p style={{ fontSize: 13, color: T.textMuted, marginBottom: 24 }}>{t(lang, "configTitle")}</p>

        {/* Presets */}
        <div style={{ marginBottom: 16 }}>
          <label style={labelStyle(T)}>{lang === "zh" ? "快速选择" : "Presets"}</label>
          <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
            {PRESETS.map((p) => (
              <button key={p.name} onClick={() => applyPreset(p)}
                style={{ padding: "5px 12px", borderRadius: 6, border: `1px solid ${T.border}`, background: (urlEdited ? baseUrl : savedUrl) === p.base_url ? T.accent : T.bg, color: (urlEdited ? baseUrl : savedUrl) === p.base_url ? "#fff" : T.textSecondary, cursor: "pointer", fontSize: 12 }}>
                {p.name}
              </button>
            ))}
          </div>
        </div>

        {/* Saved Profiles */}
        <div style={{ marginBottom: 16 }}>
          <label style={labelStyle(T)}>{lang === "zh" ? "已保存配置" : "Profiles"}</label>
          <div style={{ display: "flex", gap: 6, flexWrap: "wrap", alignItems: "center" }}>
            {profiles.map((p) => (
              <div key={p.name} style={{ display: "flex", alignItems: "center" }}>
                <button onClick={() => selectProfile(p.name)}
                  style={{ padding: "5px 12px", borderRadius: "6px 0 0 6px", border: `1px solid ${activeProfile === p.name ? T.accent : T.border}`, background: activeProfile === p.name ? T.accent : T.bg, color: activeProfile === p.name ? "#fff" : T.textSecondary, cursor: "pointer", fontSize: 12 }}>
                  {p.name}
                </button>
                <button onClick={() => deleteProfile(p.name)}
                  style={{ padding: "5px 6px", borderRadius: "0 6px 6px 0", border: `1px solid ${T.border}`, borderLeft: "none", background: T.bg, color: T.error, cursor: "pointer", fontSize: 11, fontWeight: 700 }}>
                  x
                </button>
              </div>
            ))}
            <button onClick={addProfile}
              style={{ padding: "5px 12px", borderRadius: 6, border: `1px dashed ${T.border}`, background: "transparent", color: T.textMuted, cursor: "pointer", fontSize: 12 }}>
              + {lang === "zh" ? "新建" : "New"}
            </button>
          </div>
        </div>

        {/* API Key */}
        <div style={{ marginBottom: 16 }}>
          <label style={labelStyle(T)}>{t(lang, "apiKey")}</label>
          <input
            type="password"
            value={keyEdited ? apiKey : (savedKey ? mask(savedKey) : "")}
            onFocus={() => { if (!keyEdited) { setApiKey(""); setKeyEdited(true); } }}
            onChange={(e) => { setApiKey(e.target.value); setKeyEdited(true); }}
            placeholder={savedKey ? (lang === "zh" ? "点击修改 Key" : "Click to change key") : (lang === "zh" ? "输入 API Key" : "Enter API Key")}
            style={inputStyle(T)} />
          {keyEdited && <p style={{ fontSize: 11, color: T.textMuted, marginTop: 4 }}>{lang === "zh" ? "正在编辑，保存后生效" : "Editing, save to apply"}</p>}
        </div>

        {/* Base URL */}
        <div style={{ marginBottom: 16 }}>
          <label style={labelStyle(T)}>{t(lang, "baseUrl")}</label>
          <input
            type="password"
            value={urlEdited ? baseUrl : (savedUrl ? mask(savedUrl) : "")}
            onFocus={() => { if (!urlEdited) { setBaseUrl(""); setUrlEdited(true); } }}
            onChange={(e) => { setBaseUrl(e.target.value); setUrlEdited(true); }}
            placeholder={savedUrl ? (lang === "zh" ? "点击修改 URL" : "Click to change URL") : (lang === "zh" ? "输入 Base URL" : "Enter Base URL")}
            style={inputStyle(T)} />
          {urlEdited && <p style={{ fontSize: 11, color: T.textMuted, marginTop: 4 }}>{lang === "zh" ? "正在编辑，保存后生效" : "Editing, save to apply"}</p>}
        </div>

        {/* Language + Theme */}
        <div style={{ display: "flex", gap: 12, marginBottom: 16 }}>
          <div style={{ flex: 1 }}>
            <label style={labelStyle(T)}>{t(lang, "language")}</label>
            <select value={lang} onChange={(e) => setLang(e.target.value as "en" | "zh")}
              style={{ ...inputStyle(T), cursor: "pointer" }}>
              {LANGS.map((l) => <option key={l.id} value={l.id}>{l.label}</option>)}
            </select>
          </div>
          <div style={{ flex: 1 }}>
            <label style={labelStyle(T)}>{t(lang, "theme")}</label>
            <button onClick={toggleTheme}
              style={{ ...inputStyle(T), cursor: "pointer", textAlign: "center", width: "100%" }}>
              {isDark ? t(lang, "dark") : t(lang, "light")}
            </button>
          </div>
        </div>

        {/* Test result */}
        {testResult && (
          <p style={{ fontSize: 13, marginBottom: 12, color: testResult.ok ? T.success : T.error, wordBreak: "break-word" }}>
            {testResult.msg}
          </p>
        )}
        {error && <p style={{ fontSize: 13, marginBottom: 12, color: T.error }}>{error}</p>}

        {/* Buttons */}
        <div style={{ display: "flex", gap: 8 }}>
          <button onClick={handleTest} disabled={testing}
            style={{ flex: 1, padding: "12px 0", borderRadius: 8, border: `1px solid ${T.border}`, background: T.bg, color: T.text, fontSize: 14, fontWeight: 600, cursor: "pointer", opacity: testing ? 0.6 : 1 }}>
            {testing ? (lang === "zh" ? "测试中..." : "Testing...") : (lang === "zh" ? "测试连接" : "Test Connection")}
          </button>
          <button onClick={handleSave} disabled={saving}
            style={{ flex: 1, padding: "12px 0", borderRadius: 8, border: "none", background: T.accent, color: "#fff", fontSize: 14, fontWeight: 600, cursor: "pointer", opacity: saving ? 0.6 : 1 }}>
            {saving ? t(lang, "saving") : t(lang, "saveAndLaunch")}
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
