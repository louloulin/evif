# EVIF 100 插件开发计划

> 创建时间：2026-04-30
> 更新：2026-04-30
> 目标：基于 MCP 生态分析，设计 EVIF 作为 Agent 统一接入平台的 100 个插件

---

## 一、EVIF 定位：Agent 统一接入平台

### 1.1 核心使命

**EVIF = Everything Is a Virtual Filesystem**

EVIF 将所有外部服务暴露为文件系统接口，让 AI Agent 可以用统一的方式访问：
- 本地文件
- 云存储
- 数据库
- API 服务
- AI 能力
- 协作工具

### 1.2 MCP 对齐

MCP (Model Context Protocol) 是 Anthropic 2024 年 11 月推出的开放标准，用于连接 AI 应用与外部系统。

**EVIF 与 MCP 的关系**：
| MCP 概念 | EVIF 实现 |
|---------|-----------|
| MCP Server | EVIF Plugin |
| MCP Resources | EVIF File System |
| MCP Tools | EVIF File Operations |
| MCP Prompts | EVIF Skills |
| MCP Sampling | EVIF Pipeline |

**EVIF 是 MCP 的文件系统实现**——所有 MCP 能力都通过 VFS 接口暴露。

---

## 二、当前状态

### 2.1 已实现插件（39 个）

| 分类 | 插件 | 数量 |
|------|------|------|
| 本地存储 | localfs, memfs, encryptedfs, tieredfs, streamrotatefs | 5 |
| 数据库 | sqlfs, sqlfs2, kvfs, queuefs | 4 |
| 云存储 | s3fs, gcsfs, azureblobfs, aliyunossfs, tencentcosfs, huaweiobsfs, miniofs | 7 |
| 网络协议 | httpfs, webdavfs, ftpfs, sftpfs | 4 |
| AI/LLM | gptfs, vectorfs, contextfs, context_manager | 4 |
| Agent 专用 | skillfs, skill_runtime, pipefs, devfs, streamfs | 5 |
| 系统服务 | heartbeatfs, handlefs, hellofs, catalog, serverinfofs | 5 |
| 统一接入 | opendal (9 个后端) | 1 |
| **MCP 集成** | postgresfs, gmailfs, teamsfs, telegramfs, shopifyfs, **githubfs** | 6 |

### 2.2 与 AGFS 对比

| 特性 | AGFS | EVIF | 差距 |
|------|------|------|------|
| 插件数量 | 17 | 38 | ✅ +21 |
| REST API | ~40 | 108 | ✅ +68 |
| MCP 协议 | ❌ | ✅ 75 工具 | ✅ 独有 |
| 向量搜索 | ✅ | ✅ | 持平 |
| 队列服务 | ✅ | ✅ | 持平 |
| 技能系统 | ❌ | ✅ SKILL.md | ✅ 独有 |
| 上下文分层 | ❌ | ✅ L0/L1/L2 | ✅ 独有 |
| Agent 追踪 | ❌ | ✅ AgentTracker | ✅ 独有 |

**结论：EVIF 已超越 AGFS**

---

## 三、MCP 生态分析（2026）

### 3.1 MCP 服务器分类

基于 [Awesome MCP Servers](https://github.com/punkpeye/awesome-mcp-servers) 和 [Best of MCP](https://github.com/tolkonepiu/best-of-mcp-servers)：

| 分类 | 数量 | 代表插件 |
|------|------|----------|
| **代码/开发** | 80+ | GitHub, GitLab, VSCode |
| **数据库** | 60+ | PostgreSQL, MySQL, MongoDB, Redis |
| **通信** | 50+ | Slack, Discord, Teams, Email |
| **生产力** | 40+ | Notion, Linear, Asana, Jira |
| **云服务** | 30+ | AWS, GCP, Azure |
| **搜索** | 20+ | Brave, Google, Bing |
| **AI/ML** | 30+ | OpenAI, Anthropic, Stability AI |
| **媒体** | 20+ | Figma, YouTube, Twitter |
| **存储** | 25+ | S3, Google Drive, Dropbox |
| **其他** | 40+ | Browser, Filesystem |

### 3.2 Top MCP 服务器（按 stars）

| 排名 | 插件 | Stars | 分类 |
|------|------|-------|------|
| 1 | filesystem | 极高 | 本地文件 |
| 2 | GitHub | 极高 | 代码管理 |
| 3 | Slack | 高 | 通信 |
| 4 | Notion | 高 | 知识库 |
| 5 | PostgreSQL | 高 | 数据库 |
| 6 | Google Calendar | 中高 | 日历 |
| 7 | Linear | 中高 | 项目管理 |
| 8 | Sentry | 中 | 监控 |
| 9 | Brave Search | 中 | 搜索 |
| 10 | Puppeteer | 中 | 浏览器 |

---

## 四、EVIF 100 插件计划

### P0 - MCP 核心对应（20 个）

**直接实现 MCP 服务器能力的 EVIF 插件**

| # | 插件 | MCP 对应 | 功能 | 优先级 | 工作量 |
|---|------|---------|------|--------|--------|
| 1 | **filesystem** | filesystem MCP | 本地文件系统完整实现 | P0 | 3 天 |
| 2 | **githubfs** ✅ | GitHub MCP | 仓库、Issue、PR 管理 | P0 | ✅ 已实现 |
| 3 | **slackfs** | Slack MCP | 消息发送/归档/搜索 | P0 | 3 天 |
| 4 | **discordfs** | Discord MCP | 消息归档/频道操作 | P0 | 3 天 |
| 5 | **notionfs** | Notion MCP | 页面读写/数据库 | P0 | 4 天 |
| 6 | **postgresfs** | PostgreSQL MCP | SQL 查询/数据管理 | P0 | 3 天 |
| 7 | **mysqlfs** | MySQL MCP | SQL 查询 | P0 | 2 天 |
| 8 | **redisfs** | Redis MCP | KV 操作/缓存 | P0 | 2 天 |
| 9 | **mongodbfs** | MongoDB MCP | NoSQL 操作 | P0 | 3 天 |
| 10 | **s3fs** | S3 MCP | 对象存储 | P0 | 3 天 |
| 11 | **googledrivefs** | Google Drive MCP | 云盘文件 | P0 | 3 天 |
| 12 | **dropboxfs** | Dropbox MCP | 云盘文件 | P1 | 2 天 |
| 13 | **gmailfs** | Gmail MCP | 邮件读写/搜索 | P0 | 3 天 |
| 14 | **googlesearchfs** | Brave Search MCP | 网络搜索 | P0 | 2 天 |
| 15 | **linearfs** | Linear MCP | Issue/项目 | P0 | 3 天 |
| 16 | **gitlabfs** | GitLab MCP | 仓库/Issue/MR | P1 | 3 天 |
| 17 | **jirafs** | Jira MCP | Issue/敏捷板 | P1 | 4 天 |
| 18 | **asanafs** | Asana MCP | 任务管理 | P2 | 3 天 |
| 19 | **trellofs** | Trello MCP | 看板/卡片 | P2 | 2 天 |
| 20 | **sentryfs** | Sentry MCP | 错误监控 | P1 | 2 天 |

### P1 - 通信/协作（15 个）

| # | 插件 | MCP 对应 | 功能 | 优先级 | 工作量 |
|---|------|---------|------|--------|--------|
| 21 | **teamsfs** | Teams MCP | 消息/频道/会议 | P1 | 3 天 |
| 22 | **zoomfs** | Zoom MCP | 会议/录制 | P2 | 3 天 |
| 23 | **telegramfs** | Telegram MCP | Bot/消息 | P1 | 2 天 |
| 24 | **whatsappfs** | WhatsApp MCP | 消息/媒体 | P2 | 3 天 |
| 25 | **wechatfs** | WeChat MCP | 公众号/小程序 | P2 | 4 天 |
| 26 | **dingtalkfs** | DingTalk MCP | 钉钉消息 | P2 | 3 天 |
| 27 | **feishufs** | Feishu MCP | 飞书文档/消息 | P2 | 3 天 |
| 28 | **larkfs** | Lark MCP | 文档/表格 | P2 | 3 天 |
| 29 | **intercomfs** | Intercom MCP | 客服消息 | P2 | 2 天 |
| 30 | **zendeskfs** | Zendesk MCP | 支持工单 | P2 | 3 天 |
| 31 | **salesforcefs** | Salesforce MCP | CRM 操作 | P2 | 4 天 |
| 32 | **hubspotfs** | HubSpot MCP | 营销 CRM | P2 | 3 天 |
| 33 | **sendgridfs** | SendGrid MCP | 邮件发送 | P1 | 2 天 |
| 34 | **mailgunfs** | Mailgun MCP | 邮件服务 | P2 | 2 天 |
| 35 | **twiliofs** | Twilio MCP | SMS/语音 | P2 | 2 天 |

### P2 - 云服务/基础设施（15 个）

| # | 插件 | MCP 对应 | 功能 | 优先级 | 工作量 |
|---|------|---------|------|--------|--------|
| 36 | **awsfs** | AWS MCP | EC2/S3/Lambda | P1 | 4 天 |
| 37 | **gcpfs** | GCP MCP | GCE/GCS/Cloud Functions | P1 | 4 天 |
| 38 | **azurefs** | Azure MCP | VM/Blob/Functions | P1 | 4 天 |
| 39 | **digitaloceanfs** | DigitalOcean MCP | Droplet/Spaces | P1 | 3 天 |
| 40 | **herokufs** | Heroku MCP | 应用/Addon | P2 | 2 天 |
| 41 | **vercelfs** | Vercel MCP | 部署/函数 | P0 | 3 天 |
| 42 | **netlifyfs** | Netlify MCP | 部署/函数 | P1 | 3 天 |
| 43 | **cloudflarefs** | Cloudflare MCP | Workers/D1/R2 | P1 | 3 天 |
| 44 | **k8sfs** | K8s MCP | Pod/Service/Deployment | P1 | 5 天 |
| 45 | **dockerfs** | Docker MCP | 容器/镜像 | P1 | 3 天 |
| 46 | **terraformfs** | Terraform MCP | IaC 状态 | P2 | 3 天 |
| 47 | **ansiblefs** | Ansible MCP | Playbook 执行 | P2 | 3 天 |
| 48 | **prometheusfs** | Prometheus MCP | 指标查询 | P1 | 2 天 |
| 49 | **grafanafs** | Grafana MCP | 仪表板/告警 | P1 | 3 天 |
| 50 | **datadogfs** | Datadog MCP | 监控/日志 | P2 | 3 天 |

### P3 - AI/ML 能力（15 个）

| # | 插件 | MCP 对应 | 功能 | 优先级 | 工作量 |
|---|------|---------|------|--------|--------|
| 51 | **openaifs** | OpenAI MCP | GPT/DALL-E/Whisper | P0 | 3 天 |
| 52 | **anthropicfs** | Anthropic MCP | Claude API | P0 | 3 天 |
| 53 | **gemini_fs** | Gemini MCP | Gemini API | P1 | 3 天 |
| 54 | **llama_fs** | Ollama MCP | 本地 LLM | P1 | 4 天 |
| 55 | **midjourneyfs** | Midjourney MCP | AI 图像生成 | P1 | 4 天 |
| 56 | **stable_diffusion_fs** | SD MCP | 本地图像生成 | P1 | 4 天 |
| 57 | **dalle_fs** | DALL-E MCP | OpenAI 图像 | P1 | 2 天 |
| 58 | **replicatefs** | Replicate MCP | AI 模型调用 | P2 | 3 天 |
| 59 | **falaisfs** | fal.ai MCP | 视频/语音生成 | P2 | 3 天 |
| 60 | **whisper_fs** | Whisper MCP | 语音转文字 | P1 | 2 天 |
| 61 | **elevenlabsfs** | ElevenLabs MCP | 语音合成 | P2 | 2 天 |
| 62 | **qdrantfs** | Qdrant MCP | 向量搜索 | P0 | 3 天 |
| 63 | **pinecone_fs** | Pinecone MCP | 向量存储 | P1 | 3 天 |
| 64 | **weaviatefs** | Weaviate MCP | 向量+GraphQL | P1 | 3 天 |
| 65 | **chromafs** | Chroma MCP | 本地向量库 | P2 | 2 天 |

### P4 - 媒体/内容（15 个）

| # | 插件 | MCP 对应 | 功能 | 优先级 | 工作量 |
|---|------|---------|------|--------|--------|
| 66 | **figmafs** | Figma MCP | 设计文件 | P1 | 3 天 |
| 67 | **youtubefs** | YouTube MCP | 视频/字幕 | P2 | 3 天 |
| 68 | **twitterfs** | Twitter/X MCP | 推文/搜索 | P1 | 3 天 |
| 69 | **linkedinfs** | LinkedIn MCP | 帖子/消息 | P2 | 2 天 |
| 70 | **redditfs** | Reddit MCP | 帖子/搜索 | P2 | 2 天 |
| 71 | **mediumfs** | Medium MCP | 文章发布 | P2 | 2 天 |
| 72 | **substackfs** | Substack MCP | 通讯订阅 | P2 | 2 天 |
| 73 | **spotifyfs** | Spotify MCP | 播放列表/播放 | P2 | 3 天 |
| 74 | **pinterestfs** | Pinterest MCP | 图钉/搜索 | P3 | 2 天 |
| 75 | **instagramfs** | Instagram MCP | 帖子/故事 | P3 | 3 天 |
| 76 | **tiktokfs** | TikTok MCP | 视频数据 | P3 | 3 天 |
| 77 | **shopifyfs** | Shopify MCP | 商品/订单 | P2 | 3 天 |
| 78 | **wordpressfs** | WordPress MCP | 文章/媒体 | P2 | 3 天 |
| 79 | **webhookfs** | Webhook MCP | 事件接收 | P1 | 2 天 |
| 80 | **rssfs** | RSS MCP | 订阅聚合 | P2 | 2 天 |

### P5 - 安全/监控（10 个）

| # | 插件 | MCP 对应 | 功能 | 优先级 | 工作量 |
|---|------|---------|------|--------|--------|
| 81 | **vaultfs** | HashiCorp Vault MCP | 密钥管理 | P1 | 3 天 |
| 82 | **awssecretsfs** | AWS Secrets MCP | 密钥轮换 | P1 | 2 天 |
| 83 | **1passwordfs** | 1Password MCP | 密码管理 | P2 | 3 天 |
| 84 | **lastpassfs** | LastPass MCP | 密码共享 | P3 | 2 天 |
| 85 | **oktafs** | Okta MCP | SSO/身份 | P2 | 3 天 |
| 86 | **auth0fs** | Auth0 MCP | 身份管理 | P2 | 3 天 |
| 87 | **cloudflare_warpfs** | Cloudflare WARP MCP | VPN/网络 | P3 | 3 天 |
| 88 | **pagerdutyfs** | PagerDuty MCP | 告警响应 | P2 | 2 天 |
| 89 | **opsgeniefs** | OpsGenie MCP | 告警管理 | P3 | 2 天 |
| 90 | **victoriametricsfs** | VictoriaMetrics MCP | 时序数据 | P2 | 3 天 |

### P6 - 开发工具（10 个）

| # | 插件 | MCP 对应 | 功能 | 优先级 | 工作量 |
|---|------|---------|------|--------|--------|
| 91 | **vscodefs** | VSCode MCP | 编辑/终端 | P1 | 4 天 |
| 92 | **jetbrainsfs** | JetBrains MCP | IDE 集成 | P2 | 4 天 |
| 93 | **jirafs** | Jira MCP | Issue 管理 | P2 | 4 天 |
| 94 | **confluencefs** | Confluence MCP | Wiki 文档 | P2 | 3 天 |
| 95 | **bitbucketfs** | Bitbucket MCP | 仓库/MR | P2 | 3 天 |
| 96 | **npmfs** | NPM MCP | 包查询 | P3 | 2 天 |
| 97 | **cratesiofs** | Crates.io MCP | Rust 包查询 | P3 | 2 天 |
| 98 | **pypi_fs** | PyPI MCP | Python 包查询 | P3 | 2 天 |
| 99 | **dockerhubfs** | Docker Hub MCP | 镜像搜索 | P3 | 2 天 |
| 100 | **githubactionsfs** | GitHub Actions MCP | CI/CD 工作流 | P2 | 3 天 |

---

## 五、实施路线图

### Phase 1: MCP 核心（1-2 月）

| 周 | 插件 | 输出 |
|---|------|------|
| Week 1 | filesystem | 本地文件完整支持 |
| Week 2 | githubfs, gitlabfs | 代码管理 |
| Week 3 | slackfs, discordfs | 通信 |
| Week 4 | notionfs | 知识库 |
| Week 5 | postgresfs, mysqlfs | 数据库 |
| Week 6 | redisfs, mongodbfs | KV/NoSQL |
| Week 7 | s3fs, googledrivefs | 对象存储 |
| Week 8 | gmailfs, linearfs | 邮件/项目 |

### Phase 2: 云/AI（3-4 月）

| 周 | 插件 | 输出 |
|---|------|------|
| Week 9-10 | **openaifs**, **anthropicfs** | AI 能力 |
| Week 11-12 | **awsfs**, **gcpfs**, **azurefs** | 云服务 |
| Week 13-14 | **vercelfs**, **netlifyfs**, **k8sfs** | 部署 |
| Week 15-16 | **llama_fs**, **midjourneyfs**, **whisper_fs** | 本地 AI |

### Phase 3: 协作/媒体（5-6 月）

| 周 | 插件 | 输出 |
|---|------|------|
| Week 17-18 | teamsfs, telegramfs, wechatfs | 企业通信 |
| Week 19-20 | shopifyfs, wordpressfs | 电商/CMS |
| Week 21-22 | youtube, twitter, linkedin | 社媒 |
| Week 23-24 | spotify, figma, notion | 创意工具 |

### Phase 4: 安全/监控（5-6 月）

| 周 | 插件 | 输出 |
|---|------|------|
| Week 25-26 | **vaultfs**, **awssecretsfs** | 密钥 |
| Week 27-28 | **grafanafs**, **prometheusfs** | 监控 |
| Week 29-30 | **pagerdutyfs**, **datadogfs** | 告警 |

---

## 六、技术实现模板

### 6.1 MCP Server 转换模板

```rust
// 将 MCP Server 能力转换为 EVIF Plugin
pub struct SlackFsPlugin {
    client: SlackClient,
    base_path: String,
}

#[async_trait]
impl EvifPlugin for SlackFsPlugin {
    async fn read(&self, path: &str) -> Result<Vec<u8>> {
        match path {
            "/channels" => self.list_channels().await,
            "/messages/{channel}" => self.get_messages(path).await,
            _ => Err(EvifError::NotFound),
        }
    }

    async fn write(&self, path: &str, data: Vec<u8>) -> Result<()> {
        match path {
            "/send/{channel}" => self.send_message(path, data).await,
            _ => Err(EvifError::PermissionDenied),
        }
    }

    async fn readdir(&self, path: &str) -> Result<Vec<DirEntry>> {
        match path {
            "/" => vec![
                DirEntry::dir("channels"),
                DirEntry::dir("messages"),
                DirEntry::dir("users"),
            ],
            "/channels" => self.list_channels_dir().await,
            _ => Err(EvifError::NotFound),
        }
    }
}
```

### 6.2 统一认证模式

```rust
// 所有插件支持统一认证
pub struct EvifAuth {
    api_key: Option<String>,
    jwt: Option<String>,
    oauth: Option<OAuthToken>,
}

impl EvifAuth {
    pub fn from_env() -> Self {
        Self {
            api_key: std::env::var("EVIF_API_KEY").ok(),
            jwt: std::env::var("EVIF_JWT").ok(),
            oauth: Self::load_oauth(),
        }
    }
}
```

---

## 七、商业价值评估

### 7.1 高价值插件

| 插件 | TAM | 竞品 | EVIF 优势 |
|------|-----|------|----------|
| **githubfs** | 100M 开发者 | GitHub CLI | VFS 接口 + MCP |
| **slackfs** | 20M 企业 | Slack API | 统一文件访问 |
| **openai_fs** | $5B LLM 市场 | OpenAI SDK | 上下文管理 |
| **k8sfs** | $30B K8s 市场 | kubectl | AI 驱动运维 |
| **notionfs** | 20M 用户 | Notion API | 本地优先 |

### 7.2 差异化

| 能力 | EVIF | 其他 MCP |
|------|------|----------|
| 文件系统接口 | ✅ VFS | ❌ |
| MCP + REST + CLI | ✅ 3 层 | ❌ |
| 向量记忆 | ✅ VectorFS | ❌ |
| 技能系统 | ✅ SKILL.md | ❌ |
| 上下文分层 | ✅ L0/L1/L2 | ❌ |
| 多租户安全 | ✅ RBAC | ❌ |

---

## 八、参考资料

- [Awesome MCP Servers](https://github.com/punkpeye/awesome-mcp-servers)
- [Best of MCP Servers](https://github.com/tolkonepiu/best-of-mcp-servers)
- [Model Context Protocol](https://modelcontextprotocol.io)
- [Claude MCP Servers](https://github.com/modelcontextprotocol/servers)
- [Awesome AI Agents 2026](https://github.com/Zijian-Ni/awesome-ai-agents-2026)
- [Database MCP Servers](https://claudemarketplaces.com/mcp/category/database)
- [MCP Directory](https://mcp.directory/categories/databases)
