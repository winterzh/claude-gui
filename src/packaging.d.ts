interface PackagingConfig {
  enabled: boolean;
  appName?: string;
  appSlug?: string;
  identifier?: string;
  version?: string;
  company?: {
    name?: string;
    authors?: string[];
    copyright?: string;
    website?: string;
  };
  icon?: string;
  splash?: string;
  activationCodes?: Record<
    string,
    {
      name: string;
      api_key: string;
      base_url: string;
      model?: string;
      auth_env?: string;
      extra_env?: Record<string, string>;
      msg_zh: string;
      msg_en: string;
    }
  >;
  defaults?: {
    language?: "en" | "zh";
    showSkipPermissions?: boolean;
    isolationDir?: string;
  };
  features?: {
    showActivationCode?: boolean;
    showUpdateButton?: boolean;
  };
  build?: {
    nodeVersion?: string;
    gitVersion?: string;
  };
}

declare const __PACKAGING_CONFIG__: PackagingConfig | null;
declare const __APP_VERSION__: string;
