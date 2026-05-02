#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginSupportTier {
    Core,
    Experimental,
}

impl PluginSupportTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Experimental => "experimental",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PluginCatalogEntry {
    pub id: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
    pub plugin_type: &'static str,
    pub support_tier: PluginSupportTier,
    pub aliases: &'static [&'static str],
    pub is_mountable: bool,
}

const CORE_PLUGIN_CATALOG: [PluginCatalogEntry; 13] = [
    PluginCatalogEntry {
        id: "contextfs",
        display_name: "ContextFS",
        description: "Layered L0/L1/L2 context filesystem for agent working memory",
        plugin_type: "context",
        support_tier: PluginSupportTier::Core,
        aliases: &["context"],
        is_mountable: true,
    },
    PluginCatalogEntry {
        id: "memfs",
        display_name: "MemFS",
        description: "High-speed in-memory filesystem for temporary data",
        plugin_type: "other",
        support_tier: PluginSupportTier::Core,
        aliases: &["mem", "memoryfs"],
        is_mountable: true,
    },
    PluginCatalogEntry {
        id: "skillfs",
        display_name: "SkillFS",
        description: "Standard SKILL.md discovery and invocation filesystem",
        plugin_type: "agent",
        support_tier: PluginSupportTier::Core,
        aliases: &["skill", "skills"],
        is_mountable: true,
    },
    PluginCatalogEntry {
        id: "pipefs",
        display_name: "PipeFS",
        description: "Bidirectional pipe primitives for multi-agent coordination",
        plugin_type: "agent",
        support_tier: PluginSupportTier::Core,
        aliases: &["pipe", "pipes"],
        is_mountable: true,
    },
    PluginCatalogEntry {
        id: "localfs",
        display_name: "LocalFS",
        description: "Mount a host directory into the EVIF namespace",
        plugin_type: "local",
        support_tier: PluginSupportTier::Core,
        aliases: &["local"],
        is_mountable: true,
    },
    PluginCatalogEntry {
        id: "hellofs",
        display_name: "HelloFS",
        description: "Minimal demo filesystem for smoke testing and onboarding",
        plugin_type: "other",
        support_tier: PluginSupportTier::Core,
        aliases: &["hello"],
        is_mountable: true,
    },
    PluginCatalogEntry {
        id: "kvfs",
        display_name: "KVFS",
        description: "Key-value storage exposed through file and directory semantics",
        plugin_type: "database",
        support_tier: PluginSupportTier::Core,
        aliases: &[],
        is_mountable: true,
    },
    PluginCatalogEntry {
        id: "queuefs",
        display_name: "QueueFS",
        description: "FIFO queue interface for task and message workflows",
        plugin_type: "database",
        support_tier: PluginSupportTier::Core,
        aliases: &[],
        is_mountable: true,
    },
    PluginCatalogEntry {
        id: "sqlfs2",
        display_name: "SQLFS2",
        description: "SQLite-backed file interface for structured data access",
        plugin_type: "database",
        support_tier: PluginSupportTier::Core,
        aliases: &["sqlfs"],
        is_mountable: true,
    },
    PluginCatalogEntry {
        id: "streamfs",
        display_name: "StreamFS",
        description: "Streaming read and append workflows for event-style data",
        plugin_type: "other",
        support_tier: PluginSupportTier::Core,
        aliases: &[],
        is_mountable: true,
    },
    PluginCatalogEntry {
        id: "heartbeatfs",
        display_name: "HeartbeatFS",
        description: "Liveness and lease heartbeat filesystem primitives",
        plugin_type: "other",
        support_tier: PluginSupportTier::Core,
        aliases: &[],
        is_mountable: true,
    },
    PluginCatalogEntry {
        id: "proxyfs",
        display_name: "ProxyFS",
        description: "Proxy file operations to another EVIF-compatible endpoint",
        plugin_type: "other",
        support_tier: PluginSupportTier::Core,
        aliases: &[],
        is_mountable: true,
    },
    PluginCatalogEntry {
        id: "serverinfofs",
        display_name: "ServerInfoFS",
        description: "Expose server health, version, and runtime metadata as files",
        plugin_type: "other",
        support_tier: PluginSupportTier::Core,
        aliases: &[],
        is_mountable: true,
    },
];

const EXPERIMENTAL_PLUGIN_CATALOG: [PluginCatalogEntry; 10] = [
    PluginCatalogEntry {
        id: "devfs",
        display_name: "DevFS",
        description: "Device and pseudo-file examples for experimentation",
        plugin_type: "other",
        support_tier: PluginSupportTier::Experimental,
        aliases: &[],
        is_mountable: true,
    },
    PluginCatalogEntry {
        id: "httpfs",
        display_name: "HTTPFS",
        description: "HTTP-backed filesystem adapter for remote content access",
        plugin_type: "other",
        support_tier: PluginSupportTier::Experimental,
        aliases: &[],
        is_mountable: true,
    },
    PluginCatalogEntry {
        id: "postgresfs",
        display_name: "PostgresFS",
        description: "PostgreSQL database filesystem interface with Plan 9 style paths",
        plugin_type: "database",
        support_tier: PluginSupportTier::Experimental,
        aliases: &["postgres", "pgfs"],
        is_mountable: true,
    },
    PluginCatalogEntry {
        id: "gmailfs",
        display_name: "GmailFS",
        description: "Gmail/IMAP email filesystem interface with Plan 9 style paths",
        plugin_type: "email",
        support_tier: PluginSupportTier::Experimental,
        aliases: &["gmail", "email", "mail"],
        is_mountable: true,
    },
    PluginCatalogEntry {
        id: "teamsfs",
        display_name: "TeamsFS",
        description: "Microsoft Teams filesystem interface with Plan 9 style paths",
        plugin_type: "collaboration",
        support_tier: PluginSupportTier::Experimental,
        aliases: &["teams", "msteams"],
        is_mountable: true,
    },
    PluginCatalogEntry {
        id: "telegramfs",
        display_name: "TelegramFS",
        description: "Telegram Bot filesystem interface with Plan 9 style paths",
        plugin_type: "messaging",
        support_tier: PluginSupportTier::Experimental,
        aliases: &["telegram", "tg"],
        is_mountable: true,
    },
    PluginCatalogEntry {
        id: "shopifyfs",
        display_name: "ShopifyFS",
        description: "Shopify e-commerce filesystem interface with Plan 9 style paths",
        plugin_type: "ecommerce",
        support_tier: PluginSupportTier::Experimental,
        aliases: &["shopify", "shop"],
        is_mountable: true,
    },
    PluginCatalogEntry {
        id: "handlefs",
        display_name: "HandleFS",
        description: "Handle-oriented filesystem wrapper that needs a backing plugin",
        plugin_type: "other",
        support_tier: PluginSupportTier::Experimental,
        aliases: &[],
        is_mountable: false,
    },
    PluginCatalogEntry {
        id: "tieredfs",
        display_name: "TieredFS",
        description: "Multi-tier storage orchestration that needs explicit backend wiring",
        plugin_type: "other",
        support_tier: PluginSupportTier::Experimental,
        aliases: &[],
        is_mountable: false,
    },
    PluginCatalogEntry {
        id: "encryptedfs",
        display_name: "EncryptedFS",
        description: "Encryption wrapper plugin that must be composed with another mount",
        plugin_type: "other",
        support_tier: PluginSupportTier::Experimental,
        aliases: &[],
        is_mountable: false,
    },
];

pub fn normalize_plugin_id(name: &str) -> String {
    match name.to_ascii_lowercase().as_str() {
        "context" => "contextfs".to_string(),
        "skill" | "skills" => "skillfs".to_string(),
        "pipe" | "pipes" => "pipefs".to_string(),
        "mem" | "memoryfs" => "memfs".to_string(),
        "hello" => "hellofs".to_string(),
        "local" => "localfs".to_string(),
        "sqlfs" => "sqlfs2".to_string(),
        other => other.to_string(),
    }
}

pub fn core_supported_plugins() -> Vec<PluginCatalogEntry> {
    CORE_PLUGIN_CATALOG.to_vec()
}

pub fn experimental_plugins() -> Vec<PluginCatalogEntry> {
    EXPERIMENTAL_PLUGIN_CATALOG.to_vec()
}

pub fn plugin_catalog() -> Vec<PluginCatalogEntry> {
    let mut plugins = core_supported_plugins();
    plugins.extend(experimental_plugins());
    plugins
}

pub fn find_plugin_catalog_entry(name: &str) -> Option<PluginCatalogEntry> {
    let normalized = normalize_plugin_id(name);
    plugin_catalog().into_iter().find(|entry| {
        entry.id == normalized
            || entry
                .aliases
                .iter()
                .any(|alias| normalize_plugin_id(alias) == normalized)
    })
}
