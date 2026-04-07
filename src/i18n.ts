const translations = {
  en: {
    appName: __PACKAGING_CONFIG__?.appName || "Claude Code Launcher",
    settings: "Settings",
    skills: "Skills",
    chats: "Chat History",
    nodes: "Session Nodes",
    newChat: "New Chat",
    send: "Send",
    thinking: "Thinking...",
    chooseDir: "Choose Dir",
    restart: "Restart",
    // Settings page
    configTitle: "Configure your API connection",
    apiKey: "API Key",
    apiKeyPlaceholder: "sk-ant-...",
    baseUrl: "API Base URL",
    baseUrlPlaceholder: "https://api.example.com",
    saveAndLaunch: "Save & Launch Claude",
    saving: "Saving...",
    show: "Show",
    hide: "Hide",
    enterApiKey: "Please enter your API Key",
    enterBaseUrl: "Please enter the API Base URL",
    language: "Language",
    theme: "Theme",
    dark: "Dark",
    light: "Light",
    // Chat
    typeMessage: "Type a message... (Enter to send, Shift+Enter for new line)",
    emptyHint: "Type a message below to start chatting",
    // Skills panel
    noSkills: "No skills found.\nAdd .md files to ~/.claude/commands/",
    refresh: "Refresh",
    // CLAUDE.md panel
    save: "Save",
    saved: "Saved!",
    claudeMdPlaceholder: "Write your CLAUDE.md content here...",
    // History
    chatHistory: "Chat History",
    noChats: "No saved conversations yet. Click \"New Chat\" to save the current one.",
    conversationNodes: "Conversation Nodes",
    clickToRestore: "Click to restore to that point",
    noMessages: "No messages yet.",
    you: "You",
    claude: "Claude",
    messages: "messages",
  },
  zh: {
    appName: __PACKAGING_CONFIG__?.appName || "Claude Code Launcher",
    settings: "设置",
    skills: "技能",
    chats: "对话历史",
    nodes: "本次对话节点",
    newChat: "新对话",
    send: "发送",
    thinking: "思考中...",
    chooseDir: "选择目录",
    restart: "重启",
    // Settings page
    configTitle: "配置 API 连接",
    apiKey: "API Key",
    apiKeyPlaceholder: "sk-ant-...",
    baseUrl: "API Base URL",
    baseUrlPlaceholder: "https://api.example.com",
    saveAndLaunch: "保存并启动",
    saving: "保存中...",
    show: "显示",
    hide: "隐藏",
    enterApiKey: "请输入 API Key",
    enterBaseUrl: "请输入 API Base URL",
    language: "语言",
    theme: "主题",
    dark: "深色",
    light: "浅色",
    // Chat
    typeMessage: "输入消息... (Enter 发送, Shift+Enter 换行)",
    emptyHint: "在下方输入消息开始对话",
    // Skills panel
    noSkills: "没有找到技能。\n请在 ~/.claude/commands/ 下添加 .md 文件",
    refresh: "刷新",
    // CLAUDE.md panel
    save: "保存",
    saved: "已保存!",
    claudeMdPlaceholder: "在此编写 CLAUDE.md 内容...",
    // History
    chatHistory: "对话历史",
    noChats: "还没有保存的对话。点击\"新对话\"保存当前对话。",
    conversationNodes: "对话节点",
    clickToRestore: "点击回退到该位置",
    noMessages: "还没有消息。",
    you: "你",
    claude: "Claude",
    messages: "条消息",
  },
} as const;

export type Lang = keyof typeof translations;
export type I18nKeys = keyof (typeof translations)["en"];

export function t(lang: Lang, key: I18nKeys): string {
  return translations[lang][key] || translations.en[key] || key;
}

export const LANGS: { id: Lang; label: string }[] = [
  { id: "en", label: "English" },
  { id: "zh", label: "中文" },
];
