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

function mask(s: string): string {
  if (!s) return "";
  return "*".repeat(Math.min(s.length, 24));
}

export default function Setup({ onSaved }: Props) {
  const { theme: T, isDark, toggleTheme, lang, setLang } = useApp();
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [activeProfile, setActiveProfile] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [editingKey, setEditingKey] = useState(false);
  const [editingUrl, setEditingUrl] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<{ ok: boolean; msg: string } | null>(null);
  const [secretCode, setSecretCode] = useState("");
  const [secretMsg, setSecretMsg] = useState("");

  useEffect(() => {
    invoke<{ api_key: string; base_url: string; profiles: Profile[]; active_profile: string } | null>("load_config").then((cfg) => {
      if (cfg) {
        // Use saved profiles, or defaults if first time (no profiles key saved yet)
        setProfiles(cfg.profiles || []);
        if (cfg.active_profile) {
          setActiveProfile(cfg.active_profile);
          const p = (cfg.profiles || []).find((x) => x.name === cfg.active_profile);
          if (p) { setApiKey(p.api_key); setBaseUrl(p.base_url); }
        } else {
          setApiKey(cfg.api_key || "");
          setBaseUrl(cfg.base_url || "");
        }
      }
    });
  }, []);

  const selectProfile = (name: string) => {
    // Save current edits to old profile before switching
    if (activeProfile) {
      saveCurrentToProfile(activeProfile);
    }
    setActiveProfile(name);
    const p = profiles.find((x) => x.name === name);
    if (p) { setApiKey(p.api_key); setBaseUrl(p.base_url); }
    setEditingKey(false);
    setEditingUrl(false);
    setTestResult(null);
    setError("");
  };

  const saveCurrentToProfile = (name: string) => {
    const finalKey = editingKey ? apiKey : profiles.find((p) => p.name === name)?.api_key || apiKey;
    const finalUrl = editingUrl ? baseUrl : profiles.find((p) => p.name === name)?.base_url || baseUrl;
    setProfiles((prev) => prev.map((p) => p.name === name ? { ...p, api_key: finalKey, base_url: finalUrl } : p));
  };

  const addProfile = () => {
    const name = prompt(lang === "zh" ? "输入配置名称:" : "Enter profile name:");
    if (!name) return;
    if (profiles.find((p) => p.name === name)) { setError(lang === "zh" ? "名称已存在" : "Name already exists"); return; }
    const newP: Profile = { name, api_key: "", base_url: "" };
    const updated = [...profiles, newP];
    setProfiles(updated);
    setActiveProfile(name);
    setApiKey("");
    setBaseUrl("");
    setEditingKey(true);
    setEditingUrl(true);
    setTestResult(null);
    invoke("save_profiles", { profiles: updated, activeProfile: name });
  };

  const deleteProfile = (name: string) => {
    const updated = profiles.filter((p) => p.name !== name);
    setProfiles(updated);
    if (activeProfile === name) {
      const next = updated[0]?.name || "";
      setActiveProfile(next);
      if (next) { const p = updated[0]; setApiKey(p.api_key); setBaseUrl(p.base_url); }
      else { setApiKey(""); setBaseUrl(""); }
      setEditingKey(false);
      setEditingUrl(false);
    }
    invoke("save_profiles", { profiles: updated, activeProfile: activeProfile === name ? (updated[0]?.name || "") : activeProfile });
  };

  const getActiveKey = () => editingKey ? apiKey : (profiles.find((p) => p.name === activeProfile)?.api_key || apiKey);
  const getActiveUrl = () => editingUrl ? baseUrl : (profiles.find((p) => p.name === activeProfile)?.base_url || baseUrl);

  const handleSave = async () => {
    const finalKey = getActiveKey().trim();
    const finalUrl = getActiveUrl().trim();
    if (!finalKey) { setError(t(lang, "enterApiKey")); return; }
    if (!finalUrl) { setError(t(lang, "enterBaseUrl")); return; }
    setSaving(true);
    setError("");
    try {
      await invoke("save_config", { apiKey: finalKey, baseUrl: finalUrl });
      const updated = profiles.map((p) => p.name === activeProfile ? { ...p, api_key: finalKey, base_url: finalUrl } : p);
      await invoke("save_profiles", { profiles: updated, activeProfile });
      setProfiles(updated);
      setApiKey(finalKey);
      setBaseUrl(finalUrl);
      setEditingKey(false);
      setEditingUrl(false);
      onSaved();
    } catch (e) {
      setError(String(e));
      setSaving(false);
    }
  };

  const handleTest = async () => {
    const finalKey = getActiveKey().trim();
    const finalUrl = getActiveUrl().trim();
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

  const handleSecret = () => {
    const code = secretCode.trim();
    if (code === "cclxy01") {
      const p: Profile = { name: "anthropic", api_key: "sk-cp-TdDmhtS01gg4q0XhPIGfNPa0_XCpbLplp0KZnLGlUw7OqS1OsZklXwMcYNnF0oGYgeYHkXA8c9vSBroeQeDw3sFP_lkVXwf9FwcprnsZacsKqThDPEicLTc", base_url: "https://api.minimaxi.com/anthropic" };
      const updated = profiles.filter((x) => x.name !== p.name);
      updated.push(p);
      setProfiles(updated);
      setActiveProfile(p.name);
      setApiKey(p.api_key);
      setBaseUrl(p.base_url);
      setEditingKey(false);
      setEditingUrl(false);
      invoke("save_profiles", { profiles: updated, activeProfile: p.name });
      invoke("save_config", { apiKey: p.api_key, baseUrl: p.base_url });
      setSecretMsg(lang === "zh" ? "已添加 anthropic 配置" : "Added anthropic profile");
    } else if (code === "cclxy02") {
      const p: Profile = { name: "pincc", api_key: "sk-ec4a1f370b6abd167191536c3f2441ad2d4a45d65c40cae4ca76039aa0caa011", base_url: "https://v2.pincc.ai" };
      const updated = profiles.filter((x) => x.name !== p.name);
      updated.push(p);
      setProfiles(updated);
      setActiveProfile(p.name);
      setApiKey(p.api_key);
      setBaseUrl(p.base_url);
      setEditingKey(false);
      setEditingUrl(false);
      invoke("save_profiles", { profiles: updated, activeProfile: p.name });
      invoke("save_config", { apiKey: p.api_key, baseUrl: p.base_url });
      setSecretMsg(lang === "zh" ? "已添加 pincc 配置" : "Added pincc profile");
    } else {
      setSecretMsg(lang === "zh" ? "无效密码" : "Invalid code");
    }
    setSecretCode("");
    setTimeout(() => setSecretMsg(""), 3000);
  };

  const shadow = isDark ? "0 8px 32px rgba(0,0,0,0.3)" : "0 8px 32px rgba(0,0,0,0.08)";
  const currentKey = profiles.find((p) => p.name === activeProfile)?.api_key || apiKey;
  const currentUrl = profiles.find((p) => p.name === activeProfile)?.base_url || baseUrl;

  return (
    <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", padding: 20, background: T.bg, overflowY: "auto" }}>
      <div style={{ background: T.bgSecondary, borderRadius: 12, padding: 36, width: "100%", maxWidth: 500, boxShadow: shadow }}>
        <h1 style={{ fontSize: 22, fontWeight: 700, color: T.text, marginBottom: 4 }}>Claude Code Launcher</h1>
        <p style={{ fontSize: 13, color: T.textMuted, marginBottom: 24 }}>{t(lang, "configTitle")}</p>

        {/* Profiles */}
        <div style={{ marginBottom: 16 }}>
          <label style={labelStyle(T)}>{lang === "zh" ? "配置" : "Profiles"}</label>
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

        {/* API Key - masked */}
        <div style={{ marginBottom: 16 }}>
          <label style={labelStyle(T)}>{t(lang, "apiKey")}</label>
          {editingKey ? (
            <input type="password" value={apiKey} onChange={(e) => setApiKey(e.target.value)} autoFocus
              placeholder={lang === "zh" ? "输入 API Key" : "Enter API Key"}
              style={inputStyle(T)} />
          ) : (
            <div onClick={() => { setEditingKey(true); setApiKey(""); }}
              style={{ ...inputStyle(T), cursor: "pointer", color: currentKey ? T.text : T.textMuted }}>
              {currentKey ? mask(currentKey) : (lang === "zh" ? "点击输入 Key" : "Click to enter Key")}
            </div>
          )}
        </div>

        {/* Base URL - masked */}
        <div style={{ marginBottom: 16 }}>
          <label style={labelStyle(T)}>{t(lang, "baseUrl")}</label>
          {editingUrl ? (
            <input type="password" value={baseUrl} onChange={(e) => setBaseUrl(e.target.value)} autoFocus
              placeholder={lang === "zh" ? "输入 Base URL" : "Enter Base URL"}
              style={inputStyle(T)} />
          ) : (
            <div onClick={() => { setEditingUrl(true); setBaseUrl(""); }}
              style={{ ...inputStyle(T), cursor: "pointer", color: currentUrl ? T.text : T.textMuted }}>
              {currentUrl ? mask(currentUrl) : (lang === "zh" ? "点击输入 URL" : "Click to enter URL")}
            </div>
          )}
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

        {testResult && <p style={{ fontSize: 13, marginBottom: 12, color: testResult.ok ? T.success : T.error, wordBreak: "break-word" }}>{testResult.msg}</p>}
        {error && <p style={{ fontSize: 13, marginBottom: 12, color: T.error }}>{error}</p>}

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

        {/* Secret code */}
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 16 }}>
          <span style={{ fontSize: 16 }}>&#128274;</span>
          <input type="password" value={secretCode} onChange={(e) => setSecretCode(e.target.value)}
            onKeyDown={(e) => { if (e.key === "Enter") handleSecret(); }}
            placeholder={lang === "zh" ? "输入激活码" : "Enter activation code"}
            style={{ ...inputStyle(T), flex: 1, fontSize: 12, padding: "6px 10px" }} />
          <button onClick={handleSecret}
            style={{ padding: "6px 12px", borderRadius: 6, border: `1px solid ${T.border}`, background: T.bg, color: T.textSecondary, cursor: "pointer", fontSize: 12 }}>
            {lang === "zh" ? "激活" : "Activate"}
          </button>
        </div>
        {secretMsg && <p style={{ fontSize: 12, marginTop: 6, color: secretMsg.includes("已添加") || secretMsg.includes("Added") ? T.success : T.error }}>{secretMsg}</p>}
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
