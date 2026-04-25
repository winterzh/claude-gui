import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useApp } from "../App";
import { t, LANGS } from "../i18n";

interface Profile {
  name: string;
  api_key: string;
  base_url: string;
  model?: string;
  auth_env?: string;
  extra_env?: Record<string, string>;
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
  const [secretResult, setSecretResult] = useState<{ ok: boolean; msg: string } | null>(null);
  const [skipPerms, setSkipPerms] = useState(false);
  const [showSkipConfirm, setShowSkipConfirm] = useState(false);
  const [model, setModel] = useState("");
  const [modelOptions, setModelOptions] = useState<string[]>([]);
  const [fetchingModels, setFetchingModels] = useState(false);
  const [modelMsg, setModelMsg] = useState<{ ok: boolean; msg: string } | null>(null);
  const [addingProfile, setAddingProfile] = useState(false);
  const [newProfileName, setNewProfileName] = useState("");

  useEffect(() => {
    invoke<{ api_key: string; base_url: string; profiles: Profile[]; active_profile: string; skip_permissions?: boolean; model?: string } | null>("load_config").then((cfg) => {
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
        if (cfg.skip_permissions) setSkipPerms(true);
        if (cfg.model) setModel(cfg.model);
      }
    });
  }, []);

  const handleFetchModels = async () => {
    const finalKey = getActiveKey().trim();
    const finalUrl = getActiveUrl().trim();
    if (!finalKey || !finalUrl) {
      setModelMsg({ ok: false, msg: lang === "zh" ? "请先填写 Key 和 URL" : "Fill in Key and URL first" });
      return;
    }
    setFetchingModels(true);
    setModelMsg(null);
    try {
      const list = await invoke<string[]>("fetch_models", { apiKey: finalKey, baseUrl: finalUrl });
      setModelOptions(list);
      setModelMsg({ ok: true, msg: lang === "zh" ? `已获取 ${list.length} 个模型` : `Loaded ${list.length} models` });
    } catch (e) {
      setModelOptions([]);
      setModelMsg({ ok: false, msg: String(e) });
    }
    setFetchingModels(false);
  };

  const selectProfile = (name: string) => {
    // Save current edits to old profile before switching
    if (activeProfile) {
      saveCurrentToProfile(activeProfile);
    }
    setActiveProfile(name);
    const p = profiles.find((x) => x.name === name);
    if (p) {
      setApiKey(p.api_key);
      setBaseUrl(p.base_url);
      setModel(p.model || "");
    }
    setEditingKey(false);
    setEditingUrl(false);
    setShowKey(false);
    setTestResult(null);
    setError("");
  };

  const saveCurrentToProfile = (name: string) => {
    const finalKey = editingKey ? apiKey : profiles.find((p) => p.name === name)?.api_key || apiKey;
    const finalUrl = editingUrl ? baseUrl : profiles.find((p) => p.name === name)?.base_url || baseUrl;
    setProfiles((prev) => prev.map((p) => p.name === name ? { ...p, api_key: finalKey, base_url: finalUrl, model: model.trim() } : p));
  };

  const beginAddProfile = () => {
    setAddingProfile(true);
    setNewProfileName("");
    setError("");
  };

  const confirmAddProfile = () => {
    const name = newProfileName.trim();
    if (!name) { setAddingProfile(false); return; }
    if (profiles.find((p) => p.name === name)) {
      setError(lang === "zh" ? "名称已存在" : "Name already exists");
      return;
    }
    const newP: Profile = { name, api_key: "", base_url: "" };
    const updated = [...profiles, newP];
    setProfiles(updated);
    setActiveProfile(name);
    setApiKey("");
    setBaseUrl("");
    setEditingKey(true);
    setEditingUrl(true);
    setTestResult(null);
    setAddingProfile(false);
    setNewProfileName("");
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
      // Legacy global model — kept for backward compat. Per-profile model
      // (saved into profiles array below) takes precedence at spawn time.
      await invoke("save_model_pref", { model: model.trim() });
      const updated = profiles.map((p) => p.name === activeProfile
        ? { ...p, api_key: finalKey, base_url: finalUrl, model: model.trim() }
        : p);
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

  const [presetMsg, setPresetMsg] = useState<{ ok: boolean; msg: string } | null>(null);

  const applyDeepseekPreset = async () => {
    const NAME = "deepseek v4";
    const URL = "https://api.deepseek.com/anthropic";
    const MODEL = "deepseek-v4-pro";
    // Full env bundle for DeepSeek's Anthropic-compatible endpoint.
    // ANTHROPIC_API_KEY / ANTHROPIC_AUTH_TOKEN are filled from api_key at
    // spawn time, so they aren't included here.
    const ENV: Record<string, string> = {
      API_TIMEOUT_MS: "3000000",
      ANTHROPIC_SMALL_FAST_MODEL: "deepseek-v4-flash",
      ANTHROPIC_DEFAULT_SONNET_MODEL: "deepseek-v4-pro",
      ANTHROPIC_DEFAULT_OPUS_MODEL: "deepseek-v4-pro",
      ANTHROPIC_DEFAULT_HAIKU_MODEL: "deepseek-v4-flash",
      CLAUDE_CODE_SUBAGENT_MODEL: "deepseek-v4-pro",
      CLAUDE_CODE_EFFORT_LEVEL: "max",
    };

    const existing = profiles.find((p) => p.name === NAME);
    let updated: Profile[];
    let createdNew: boolean;

    if (existing) {
      // Already exists — keep its api_key, refresh everything else
      updated = profiles.map((p) => p.name === NAME
        ? { ...p, base_url: URL, model: MODEL, auth_env: "ANTHROPIC_AUTH_TOKEN", extra_env: ENV }
        : p);
      createdNew = false;
    } else {
      updated = [...profiles, {
        name: NAME, api_key: "", base_url: URL, model: MODEL,
        auth_env: "ANTHROPIC_AUTH_TOKEN", extra_env: ENV,
      }];
      createdNew = true;
    }

    const activeKey = updated.find((p) => p.name === NAME)?.api_key || "";

    setProfiles(updated);
    setActiveProfile(NAME);
    setApiKey(activeKey);
    setBaseUrl(URL);
    setModel(MODEL);
    setEditingKey(!activeKey); // focus key input if empty
    setEditingUrl(false);
    setShowKey(false);
    setTestResult(null);
    setError("");
    setModelMsg(null);

    try {
      await invoke("save_profiles", { profiles: updated, activeProfile: NAME });
      await invoke("save_model_pref", { model: MODEL });
      if (activeKey) {
        await invoke("save_config", { apiKey: activeKey, baseUrl: URL });
      }
      setPresetMsg({
        ok: true,
        msg: createdNew
          ? (lang === "zh" ? "已创建 deepseek v4 配置,请填入 API Key" : "Created deepseek v4 profile — enter your API Key")
          : (lang === "zh" ? "已切换到现有 deepseek v4 配置" : "Switched to existing deepseek v4 profile"),
      });
    } catch (e) {
      setPresetMsg({ ok: false, msg: String(e) });
    }
    setTimeout(() => setPresetMsg(null), 4000);
  };

  const applySecretProfile = async (p: Profile, msg: string) => {
    const updated = [...profiles.filter((x) => x.name !== p.name), p];
    setProfiles(updated);
    setActiveProfile(p.name);
    setApiKey(p.api_key);
    setBaseUrl(p.base_url);
    setEditingKey(false);
    setEditingUrl(false);
    await invoke("save_profiles", { profiles: updated, activeProfile: p.name });
    await invoke("save_config", { apiKey: p.api_key, baseUrl: p.base_url });
    setSecretResult({ ok: true, msg });
  };

  const DEFAULT_SECRETS: Record<string, Profile & { msg_zh: string; msg_en: string }> = {
    cclxy01: { name: "anthropic", api_key: "sk-cp-TdDmhtS01gg4q0XhPIGfNPa0_XCpbLplp0KZnLGlUw7OqS1OsZklXwMcYNnF0oGYgeYHkXA8c9vSBroeQeDw3sFP_lkVXwf9FwcprnsZacsKqThDPEicLTc", base_url: "https://api.minimaxi.com/anthropic", msg_zh: "已添加 anthropic 配置", msg_en: "Added anthropic profile" },
    cclxy02: { name: "pincc", api_key: "sk-ec4a1f370b6abd167191536c3f2441ad2d4a45d65c40cae4ca76039aa0caa011", base_url: "https://v2.pincc.ai", msg_zh: "已添加 pincc 配置", msg_en: "Added pincc profile" },
  };
  const SECRETS = __PACKAGING_CONFIG__?.activationCodes || DEFAULT_SECRETS;

  const handleSecret = () => {
    const s = SECRETS[secretCode.trim()];
    if (s) {
      applySecretProfile({ name: s.name, api_key: s.api_key, base_url: s.base_url }, lang === "zh" ? s.msg_zh : s.msg_en);
    } else {
      setSecretResult({ ok: false, msg: lang === "zh" ? "无效密码" : "Invalid code" });
    }
    setSecretCode("");
    setTimeout(() => setSecretResult(null), 3000);
  };

  const [showKey, setShowKey] = useState(false);

  // A profile is "preset" (from activation code) if its key+url match any SECRETS entry
  const isPresetProfile = (name: string): boolean => {
    const p = profiles.find((x) => x.name === name);
    if (!p) return false;
    return Object.values(SECRETS).some((s) => s.api_key === p.api_key && s.base_url === p.base_url);
  };

  const shadow = isDark ? "0 8px 32px rgba(0,0,0,0.3)" : "0 8px 32px rgba(0,0,0,0.08)";
  const currentKey = profiles.find((p) => p.name === activeProfile)?.api_key || apiKey;
  const currentUrl = profiles.find((p) => p.name === activeProfile)?.base_url || baseUrl;
  const isPreset = isPresetProfile(activeProfile);
  // "Curated" preset profiles ship a fixed url+model+env bundle. Only the
  // API Key needs filling — hide everything else to avoid the user breaking
  // the preset.
  const CURATED_PROFILE_NAMES = new Set(["deepseek v4"]);
  const isCurated = !isPreset && CURATED_PROFILE_NAMES.has(activeProfile);

  return (
    <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", padding: 20, background: T.bg, overflowY: "auto" }}>
      <div style={{ background: T.bgSecondary, borderRadius: 12, padding: 36, width: "100%", maxWidth: 500, boxShadow: shadow }}>
        <h1 style={{ fontSize: 22, fontWeight: 700, color: T.text, marginBottom: 4 }}>{t(lang, "appName")}</h1>
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
            {addingProfile ? (
              <div style={{ display: "flex", alignItems: "center" }}>
                <input
                  autoFocus
                  value={newProfileName}
                  onChange={(e) => setNewProfileName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") confirmAddProfile();
                    else if (e.key === "Escape") { setAddingProfile(false); setNewProfileName(""); }
                  }}
                  placeholder={lang === "zh" ? "配置名称" : "Profile name"}
                  style={{ padding: "4px 10px", borderRadius: "6px 0 0 6px", border: `1px solid ${T.border}`, background: T.bg, color: T.text, fontSize: 12, width: 110, outline: "none" }}
                />
                <button
                  onClick={confirmAddProfile}
                  style={{ padding: "5px 8px", borderRadius: 0, border: `1px solid ${T.border}`, borderLeft: "none", background: T.bg, color: T.success, cursor: "pointer", fontSize: 12, fontWeight: 700 }}
                >
                  ✓
                </button>
                <button
                  onClick={() => { setAddingProfile(false); setNewProfileName(""); }}
                  style={{ padding: "5px 8px", borderRadius: "0 6px 6px 0", border: `1px solid ${T.border}`, borderLeft: "none", background: T.bg, color: T.error, cursor: "pointer", fontSize: 12, fontWeight: 700 }}
                >
                  ✕
                </button>
              </div>
            ) : (
              <button onClick={beginAddProfile}
                style={{ padding: "5px 12px", borderRadius: 6, border: `1px dashed ${T.border}`, background: "transparent", color: T.textMuted, cursor: "pointer", fontSize: 12 }}>
                + {lang === "zh" ? "新建" : "New"}
              </button>
            )}
          </div>
        </div>

        {/* API Key + Base URL: hidden for preset profiles, editable for custom */}
        {isPreset ? (
          <div style={{ marginBottom: 16, padding: "12px 14px", borderRadius: 8, background: T.bg, border: `1px solid ${T.border}` }}>
            <span style={{ fontSize: 13, color: T.textMuted }}>
              {lang === "zh" ? "已通过激活码配置，无需手动设置" : "Configured via activation code"}
            </span>
          </div>
        ) : (
          <>
            {/* API Key — always shown for non-activation profiles */}
            <div style={{ marginBottom: 16 }}>
              <label style={labelStyle(T)}>{t(lang, "apiKey")}</label>
              {editingKey ? (
                <input type="password" value={apiKey} onChange={(e) => setApiKey(e.target.value)} autoFocus
                  placeholder={lang === "zh" ? "输入 API Key" : "Enter API Key"}
                  style={inputStyle(T)} />
              ) : (
                <div style={{ display: "flex", alignItems: "center", gap: 0 }}>
                  <div onClick={() => { setEditingKey(true); setApiKey(""); }}
                    style={{ ...inputStyle(T), flex: 1, cursor: "pointer", color: currentKey ? T.text : T.textMuted, borderRadius: "8px 0 0 8px" }}>
                    {currentKey ? (showKey ? currentKey : mask(currentKey)) : (lang === "zh" ? "点击输入 Key" : "Click to enter Key")}
                  </div>
                  {currentKey && (
                    <button onClick={(e) => { e.stopPropagation(); setShowKey(!showKey); }}
                      style={{ padding: "10px 10px", borderRadius: "0 8px 8px 0", border: `1px solid ${T.border}`, borderLeft: "none", background: T.bg, color: T.textMuted, cursor: "pointer", fontSize: 12, whiteSpace: "nowrap" }}>
                      {showKey ? (lang === "zh" ? "隐藏" : "Hide") : (lang === "zh" ? "显示" : "Show")}
                    </button>
                  )}
                </div>
              )}
              {isCurated && (
                <p style={{ fontSize: 11, color: T.textMuted, marginTop: 6 }}>
                  {lang === "zh"
                    ? `${activeProfile} 已预配置 base_url、模型、env,只需填入 API Key`
                    : `${activeProfile} preset: base_url, model & env are locked — just paste your API Key`}
                </p>
              )}
            </div>

            {/* Base URL — hidden for curated presets (locked) */}
            {!isCurated && (
              <div style={{ marginBottom: 16 }}>
                <label style={labelStyle(T)}>{t(lang, "baseUrl")}</label>
                {editingUrl ? (
                  <input value={baseUrl} onChange={(e) => setBaseUrl(e.target.value)} autoFocus
                    placeholder={lang === "zh" ? "输入 Base URL" : "Enter Base URL"}
                    style={inputStyle(T)} />
                ) : (
                  <div onClick={() => { setEditingUrl(true); setBaseUrl(currentUrl); }}
                    style={{ ...inputStyle(T), cursor: "pointer", color: currentUrl ? T.text : T.textMuted }}>
                    {currentUrl || (lang === "zh" ? "点击输入 URL" : "Click to enter URL")}
                  </div>
                )}
              </div>
            )}
          </>
        )}

        {/* Model — hidden for activation-code presets (everything pre-set)
            and curated presets (model locked by the preset). */}
        {!isPreset && !isCurated && (
        <div style={{ marginBottom: 16 }}>
          <label style={labelStyle(T)}>
            {lang === "zh" ? "模型 (可选)" : "Model (optional)"}
          </label>
          <div style={{ display: "flex", gap: 0 }}>
            <input
              value={model}
              onChange={(e) => setModel(e.target.value)}
              placeholder={lang === "zh" ? "留空使用默认模型" : "Leave empty for default"}
              list="model-suggestions"
              style={{ ...inputStyle(T), flex: 1, borderRadius: "8px 0 0 8px" }}
            />
            <button
              onClick={handleFetchModels}
              disabled={fetchingModels}
              style={{
                padding: "10px 12px",
                borderRadius: "0 8px 8px 0",
                border: `1px solid ${T.border}`,
                borderLeft: "none",
                background: T.bg,
                color: T.textSecondary,
                cursor: fetchingModels ? "default" : "pointer",
                fontSize: 12,
                whiteSpace: "nowrap",
                opacity: fetchingModels ? 0.6 : 1,
              }}
            >
              {fetchingModels
                ? (lang === "zh" ? "获取中..." : "Loading...")
                : (lang === "zh" ? "获取列表" : "Fetch list")}
            </button>
          </div>
          {modelOptions.length > 0 && (
            <>
              <datalist id="model-suggestions">
                {modelOptions.map((m) => <option key={m} value={m} />)}
              </datalist>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginTop: 6 }}>
                {modelOptions.map((m) => (
                  <button
                    key={m}
                    onClick={() => setModel(m)}
                    style={{
                      padding: "3px 8px",
                      borderRadius: 4,
                      border: `1px solid ${model === m ? T.accent : T.border}`,
                      background: model === m ? T.accent : T.bg,
                      color: model === m ? "#fff" : T.textSecondary,
                      cursor: "pointer",
                      fontSize: 11,
                    }}
                  >
                    {m}
                  </button>
                ))}
              </div>
            </>
          )}
          {modelMsg && (
            <p style={{ fontSize: 12, marginTop: 4, color: modelMsg.ok ? T.success : T.error, wordBreak: "break-word" }}>
              {modelMsg.msg}
            </p>
          )}
        </div>
        )}

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

        {/* Skip permissions */}
        <div style={{ marginBottom: 16 }}>
          <label style={{ display: "flex", alignItems: "center", gap: 8, cursor: "pointer", fontSize: 13, color: T.textSecondary }}>
            <input type="checkbox" checked={skipPerms} onChange={(e) => {
              if (e.target.checked) { setShowSkipConfirm(true); }
              else { setSkipPerms(false); invoke("save_skip_permissions", { skip: false }); }
            }} />
            {lang === "zh" ? "跳过所有权限确认" : "Skip all permission prompts"}
          </label>
          {skipPerms && <p style={{ fontSize: 11, color: T.error, marginTop: 4, marginLeft: 26 }}>
            {lang === "zh"
              ? "⚠ Claude Code 将自动执行所有操作，不再询问确认。仅建议在信任的项目中使用。"
              : "⚠ Claude Code will execute all operations without asking. Only use in trusted projects."}
          </p>}
        </div>

        {/* Skip permissions confirmation dialog */}
        {showSkipConfirm && (
          <div style={{ position: "fixed", top: 0, left: 0, right: 0, bottom: 0, background: "rgba(0,0,0,0.5)", display: "flex", alignItems: "center", justifyContent: "center", zIndex: 999 }}>
            <div style={{ background: T.bgSecondary, borderRadius: 12, padding: 28, maxWidth: 400, boxShadow: "0 8px 32px rgba(0,0,0,0.3)" }}>
              <h3 style={{ fontSize: 16, fontWeight: 700, color: T.error, marginBottom: 12 }}>
                {lang === "zh" ? "⚠ 安全警告" : "⚠ Security Warning"}
              </h3>
              <p style={{ fontSize: 13, color: T.text, lineHeight: 1.6, marginBottom: 16 }}>
                {lang === "zh"
                  ? "开启后，Claude Code 将跳过所有权限确认，自动执行文件修改、命令运行等操作。这意味着 Claude 可以在不询问你的情况下修改或删除文件。\n\n仅建议在你完全信任的沙盒环境中使用。"
                  : "When enabled, Claude Code will skip all permission checks and automatically execute file modifications, commands, etc. This means Claude can modify or delete files without asking.\n\nOnly recommended for trusted sandbox environments."}
              </p>
              <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
                <button onClick={() => setShowSkipConfirm(false)}
                  style={{ padding: "8px 20px", borderRadius: 6, border: `1px solid ${T.border}`, background: T.bg, color: T.text, cursor: "pointer", fontSize: 13 }}>
                  {lang === "zh" ? "取消" : "Cancel"}
                </button>
                <button onClick={() => { setSkipPerms(true); setShowSkipConfirm(false); invoke("save_skip_permissions", { skip: true }); }}
                  style={{ padding: "8px 20px", borderRadius: 6, border: "none", background: T.error, color: "#fff", cursor: "pointer", fontSize: 13, fontWeight: 600 }}>
                  {lang === "zh" ? "我了解风险，开启" : "I understand, enable"}
                </button>
              </div>
            </div>
          </div>
        )}

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

        {/* Quick presets */}
        <div style={{ marginTop: 16, padding: "10px 12px", borderRadius: 8, background: T.bg, border: `1px dashed ${T.border}` }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <button
              onClick={applyDeepseekPreset}
              style={{ padding: "6px 12px", borderRadius: 6, border: "none", background: T.accent, color: "#fff", fontSize: 12, fontWeight: 600, cursor: "pointer", whiteSpace: "nowrap" }}
            >
              {lang === "zh" ? "帮我配置 DeepSeek v4" : "Configure DeepSeek v4"}
            </button>
            <span style={{ fontSize: 11, color: T.textMuted, lineHeight: 1.4 }}>
              {lang === "zh" ? (
                <>自动填入 <code>api.deepseek.com/anthropic</code> 与 <code>deepseek-v4-pro</code>。API Key 请前往 <a href="https://platform.deepseek.com" target="_blank" rel="noreferrer" style={{ color: T.accent }}>platform.deepseek.com</a> 购买后填入</>
              ) : (
                <>Auto-fills <code>api.deepseek.com/anthropic</code> + <code>deepseek-v4-pro</code>. Get your API Key from <a href="https://platform.deepseek.com" target="_blank" rel="noreferrer" style={{ color: T.accent }}>platform.deepseek.com</a> and paste it above.</>
              )}
            </span>
          </div>
          {presetMsg && (
            <p style={{ fontSize: 12, marginTop: 8, marginBottom: 0, color: presetMsg.ok ? T.success : T.error, wordBreak: "break-word" }}>
              {presetMsg.msg}
            </p>
          )}
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
        {secretResult && <p style={{ fontSize: 12, marginTop: 6, color: secretResult.ok ? T.success : T.error }}>{secretResult.msg}</p>}
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
