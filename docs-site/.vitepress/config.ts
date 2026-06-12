import { defineConfig } from 'vitepress'

// GitHub Pages（仓库 Pages）：https://<user>.github.io/<repo>/
// 本地开发不设环境变量时用 '/'；CI 中设置 VITEPRESS_BASE=/anycode/ 再 build。
const base = process.env.VITEPRESS_BASE || '/'

const guideSidebarEn = [
  {
    text: 'Get started',
    collapsed: false,
    items: [
      { text: 'Quick start', link: '/guide/getting-started' },
      { text: 'Install', link: '/guide/install' },
      { text: 'Open the Workbench', link: '/guide/dashboard' }
    ]
  },
  {
    text: 'Digital Workbench',
    collapsed: false,
    items: [{ text: 'Workbench tour', link: '/guide/workbench' }]
  },
  {
    text: 'Terminal',
    collapsed: false,
    items: [
      { text: 'Using anyCode in the terminal', link: '/guide/cli' },
      { text: 'Scheduled reminders', link: '/guide/cli-scheduler' },
      { text: 'Common issues', link: '/guide/troubleshooting' }
    ]
  },
  {
    text: 'Learn more',
    collapsed: true,
    items: [
      { text: 'WeChat & setup', link: '/guide/wechat' },
      { text: 'Telegram', link: '/guide/telegram' },
      { text: 'Discord', link: '/guide/discord' },
      { text: 'Run, REPL & TUI', link: '/guide/cli-sessions' },
      { text: 'Models', link: '/guide/models' },
      { text: 'Config & security', link: '/guide/config-security' },
      { text: 'Agents & Skills', link: '/guide/agents' },
      { text: 'Memory', link: '/guide/memory' },
      { text: 'Notifications', link: '/guide/notifications' },
      { text: 'Architecture', link: '/guide/architecture' },
      { text: 'Development', link: '/guide/development' },
      { text: 'All pages (index)', link: '/guide/hubs' }
    ]
  }
]

const guideSidebarZh = [
  {
    text: '开始使用',
    collapsed: false,
    items: [
      { text: '快速开始', link: '/zh/guide/getting-started' },
      { text: '安装', link: '/zh/guide/install' },
      { text: '打开工作台', link: '/zh/guide/dashboard' }
    ]
  },
  {
    text: '数字工作台',
    collapsed: false,
    items: [{ text: '工作台导览', link: '/zh/guide/workbench' }]
  },
  {
    text: '终端助手',
    collapsed: false,
    items: [
      { text: '终端里怎么用', link: '/zh/guide/cli' },
      { text: '定时提醒', link: '/zh/guide/cli-scheduler' },
      { text: '常见问题', link: '/zh/guide/troubleshooting' }
    ]
  },
  {
    text: '了解更多',
    collapsed: true,
    items: [
      { text: '微信与配置', link: '/zh/guide/wechat' },
      { text: 'Telegram', link: '/zh/guide/telegram' },
      { text: 'Discord', link: '/zh/guide/discord' },
      { text: 'run / REPL / 全屏界面', link: '/zh/guide/cli-sessions' },
      { text: '模型与端点', link: '/zh/guide/models' },
      { text: '配置与安全', link: '/zh/guide/config-security' },
      { text: 'Agent 与 Skills', link: '/zh/guide/agents' },
      { text: '记忆', link: '/zh/guide/memory' },
      { text: '会话通知', link: '/zh/guide/notifications' },
      { text: '架构说明', link: '/zh/guide/architecture' },
      { text: '开发与贡献', link: '/zh/guide/development' },
      { text: '全量索引', link: '/zh/guide/hubs' }
    ]
  }
]

export default defineConfig({
  base,
  title: 'anyCode',
  description: 'Terminal AI agent for developers',
  lastUpdated: true,
  cleanUrls: true,
  // zh/guide links point at repo `crates/` and `docs/` (outside docs-site); VitePress cannot resolve them.
  ignoreDeadLinks: [
    /^\.?\/?(?:\.\.\/)+crates\//,
    /^\.?\/?(?:\.\.\/)+docs\//,
  ],

  locales: {
    root: {
      label: 'English',
      lang: 'en-US',
      title: 'anyCode',
      description: 'Terminal AI agent for developers',
      themeConfig: {
        logo: '/anycode-logo.png',
        nav: [
          { text: 'Workbench', link: '/guide/workbench' },
          { text: 'Get started', link: '/guide/getting-started' }
        ],
        sidebar: {
          '/guide/': guideSidebarEn
        },
        outline: { level: [2, 3] },
        footer: {
          message: 'MIT License',
          copyright: 'anyCode Contributors'
        }
      }
    },
    zh: {
      label: '简体中文',
      lang: 'zh-CN',
      link: '/zh/',
      title: 'anyCode',
      description: '终端 AI 编程助手',
      themeConfig: {
        logo: '/anycode-logo.png',
        nav: [
          { text: '工作台', link: '/zh/guide/workbench' },
          { text: '快速开始', link: '/zh/guide/getting-started' }
        ],
        sidebar: {
          '/zh/guide/': guideSidebarZh
        },
        outline: { level: [2, 3] },
        footer: {
          message: 'MIT License',
          copyright: 'anyCode Contributors'
        }
      }
    }
  },

  themeConfig: {
    i18nRouting: true,
    search: {
      provider: 'local'
    },
    socialLinks: [
      { icon: 'github', link: 'https://github.com/qingjiuzys/anycode' }
    ]
  }
})
