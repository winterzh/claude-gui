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
