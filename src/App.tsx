import { useState, useEffect, createContext, useContext } from "react";
import { invoke } from "@tauri-apps/api/core";
import Setup from "./pages/Setup";
import Chat from "./pages/Chat";
import { darkTheme, lightTheme, Theme } from "./theme";
import { Lang } from "./i18n";
import "./App.css";

interface AppContextType {
  theme: Theme;
  isDark: boolean;
  toggleTheme: () => void;
  lang: Lang;
  setLang: (l: Lang) => void;
}

export const AppContext = createContext<AppContextType>({
  theme: darkTheme,
  isDark: true,
  toggleTheme: () => {},
  lang: "en",
  setLang: () => {},
});

export const useApp = () => useContext(AppContext);

function App() {
  const [page, setPage] = useState<"loading" | "setup" | "chat">("loading");
  const [isDark, setIsDark] = useState(() => {
    return localStorage.getItem("theme") !== "light";
  });
  const [lang, setLang] = useState<Lang>(() => {
    return (localStorage.getItem("lang") as Lang) || __PACKAGING_CONFIG__?.defaults?.language || "zh";
  });

  const theme = isDark ? darkTheme : lightTheme;

  const toggleTheme = () => {
    setIsDark((prev) => {
      const next = !prev;
      localStorage.setItem("theme", next ? "dark" : "light");
      return next;
    });
  };

  const handleSetLang = (l: Lang) => {
    setLang(l);
    localStorage.setItem("lang", l);
  };

  useEffect(() => {
    invoke<{ api_key: string; base_url: string } | null>("load_config").then(
      (config) => {
        const nextPage = config && config.api_key ? "chat" : "setup";
        // Show splash for at least 1 second after config loads
        setTimeout(() => setPage(nextPage), 1000);
      },
    );
  }, []);

  // Apply theme to body
  useEffect(() => {
    document.body.style.background = theme.bg;
    document.body.style.color = theme.text;
  }, [theme]);

  return (
    <AppContext.Provider value={{ theme, isDark, toggleTheme, lang, setLang: handleSetLang }}>
      {page === "loading" && (
        <div style={{
          display: "flex", alignItems: "center", justifyContent: "center",
          height: "100vh", background: theme.bg,
        }}>
          <img src="/splash.png" alt="" style={{
            maxWidth: "80%", maxHeight: "80%", objectFit: "contain",
          }} />
        </div>
      )}
      {page === "setup" && <Setup onSaved={() => setPage("chat")} />}
      {page === "chat" && <Chat onSettings={() => setPage("setup")} />}
    </AppContext.Provider>
  );
}

export default App;
