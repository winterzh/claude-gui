import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useApp } from "../App";
import { t, LANGS } from "../i18n";

interface Props {
  onSaved: () => void;
}

export default function Setup({ onSaved }: Props) {
  const { theme, isDark, toggleTheme, lang, setLang } = useApp();
  const [apiKey, setApiKey] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    invoke<{ api_key: string; base_url: string } | null>("load_config").then(
      (config) => {
        if (config) {
          setApiKey(config.api_key || "");
          setBaseUrl(config.base_url || "");
        }
      },
    );
  }, []);

  const handleSave = async () => {
    if (!apiKey.trim()) {
      setError(t(lang, "enterApiKey"));
      return;
    }
    if (!baseUrl.trim()) {
      setError(t(lang, "enterBaseUrl"));
      return;
    }
    setSaving(true);
    setError("");
    try {
      await invoke("save_config", {
        apiKey: apiKey.trim(),
        baseUrl: baseUrl.trim(),
      });
      onSaved();
    } catch (e) {
      setError(String(e));
      setSaving(false);
    }
  };

  return (
    <div style={{ ...s.container, background: theme.bg }}>
      <div style={{ ...s.card, background: theme.bgSecondary, boxShadow: isDark ? "0 8px 32px rgba(0,0,0,0.3)" : "0 8px 32px rgba(0,0,0,0.08)" }}>
        <h1 style={{ ...s.title, color: theme.text }}>Claude Code Launcher</h1>
        <p style={{ ...s.subtitle, color: theme.textMuted }}>{t(lang, "configTitle")}</p>

        <div style={s.field}>
          <label style={{ ...s.label, color: theme.textSecondary }}>{t(lang, "apiKey")}</label>
          <div style={s.inputRow}>
            <input
              type={showKey ? "text" : "password"}
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder={t(lang, "apiKeyPlaceholder")}
              style={{ ...s.input, background: theme.bg, color: theme.text, borderColor: theme.border }}
            />
            <button
              onClick={() => setShowKey(!showKey)}
              style={{ ...s.toggleBtn, background: theme.bg, color: theme.textSecondary, borderColor: theme.border }}
            >
              {showKey ? t(lang, "hide") : t(lang, "show")}
            </button>
          </div>
        </div>

        <div style={s.field}>
          <label style={{ ...s.label, color: theme.textSecondary }}>{t(lang, "baseUrl")}</label>
          <input
            type="text"
            value={baseUrl}
            onChange={(e) => setBaseUrl(e.target.value)}
            placeholder={t(lang, "baseUrlPlaceholder")}
            style={{ ...s.input, background: theme.bg, color: theme.text, borderColor: theme.border }}
          />
        </div>

        {/* Language + Theme row */}
        <div style={{ ...s.field, display: "flex", gap: 12 }}>
          <div style={{ flex: 1 }}>
            <label style={{ ...s.label, color: theme.textSecondary }}>{t(lang, "language")}</label>
            <select
              value={lang}
              onChange={(e) => setLang(e.target.value as typeof lang)}
              style={{ ...s.input, background: theme.bg, color: theme.text, borderColor: theme.border, cursor: "pointer" }}
            >
              {LANGS.map((l) => (
                <option key={l.id} value={l.id}>{l.label}</option>
              ))}
            </select>
          </div>
          <div style={{ flex: 1 }}>
            <label style={{ ...s.label, color: theme.textSecondary }}>{t(lang, "theme")}</label>
            <button
              onClick={toggleTheme}
              style={{ ...s.input, background: theme.bg, color: theme.text, borderColor: theme.border, cursor: "pointer", textAlign: "center", width: "100%" }}
            >
              {isDark ? t(lang, "dark") : t(lang, "light")}
            </button>
          </div>
        </div>

        {error && <p style={{ ...s.error, color: theme.error }}>{error}</p>}

        <button
          onClick={handleSave}
          disabled={saving}
          style={{ ...s.saveBtn, background: theme.accent, opacity: saving ? 0.6 : 1 }}
        >
          {saving ? t(lang, "saving") : t(lang, "saveAndLaunch")}
        </button>
      </div>
    </div>
  );
}

const s: Record<string, React.CSSProperties> = {
  container: { display: "flex", alignItems: "center", justifyContent: "center", height: "100%", padding: 20 },
  card: { borderRadius: 12, padding: 40, width: "100%", maxWidth: 460 },
  title: { fontSize: 24, fontWeight: 700, marginBottom: 4 },
  subtitle: { fontSize: 14, marginBottom: 28 },
  field: { marginBottom: 20 },
  label: { display: "block", fontSize: 13, fontWeight: 600, marginBottom: 6 },
  inputRow: { display: "flex", gap: 8 },
  input: { flex: 1, padding: "10px 14px", borderRadius: 8, border: "1px solid", fontSize: 14, outline: "none" },
  toggleBtn: { padding: "10px 14px", borderRadius: 8, border: "1px solid", cursor: "pointer", fontSize: 13 },
  error: { fontSize: 13, marginBottom: 16 },
  saveBtn: { width: "100%", padding: "12px 0", borderRadius: 8, border: "none", color: "#fff", fontSize: 15, fontWeight: 600, cursor: "pointer", marginTop: 8 },
};
