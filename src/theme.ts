export interface Theme {
  bg: string;
  bgSecondary: string;
  bgTertiary: string;
  border: string;
  text: string;
  textSecondary: string;
  textMuted: string;
  accent: string;
  userMsg: string;
  assistantMsg: string;
  error: string;
  errorBg: string;
  success: string;
}

export const darkTheme: Theme = {
  bg: "#0f0f23",
  bgSecondary: "#16213e",
  bgTertiary: "#1e1e3a",
  border: "#2a2a4a",
  text: "#e0e0e0",
  textSecondary: "#aaa",
  textMuted: "#666",
  accent: "#e07a5f",
  userMsg: "#1a3a5c",
  assistantMsg: "#1e1e3a",
  error: "#ff6b6b",
  errorBg: "#3a1a1a",
  success: "#4caf50",
};

export const lightTheme: Theme = {
  bg: "#ffffff",
  bgSecondary: "#f5f5f7",
  bgTertiary: "#eeeef0",
  border: "#d1d1d6",
  text: "#1d1d1f",
  textSecondary: "#555",
  textMuted: "#999",
  accent: "#d4603a",
  userMsg: "#e3f2fd",
  assistantMsg: "#f5f5f7",
  error: "#d32f2f",
  errorBg: "#fdecea",
  success: "#2e7d32",
};
