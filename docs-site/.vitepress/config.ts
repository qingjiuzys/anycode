import { defineConfig } from 'vitepress'

// GitHub Pages（仓库 Pages）：https://<user>.github.io/<repo>/
// 本地开发不设环境变量时用 '/'；CI 中设置 VITEPRESS_BASE=/anycode/ 再 build。
const base = process.env.VITEPRESS_BASE || '/'

const guideSidebarEn = [
  {
    text: 'First use',
    collapsed: false,
    items: [
      { text: 'Getting started', link: '/guide/getting-started' },
      { text: 'Install', link: '/guide/install' },
      { text: 'WeChat & setup', link: '/guide/wechat' }
    ]
  },
  {
    text: 'Daily use',
    collapsed: false,
    items: [
      { text: 'Overview', link: '/guide/cli' },
      { text: 'Run, REPL & TUI', link: '/guide/cli-sessions' },
      { text: 'Model commands', link: '/guide/cli-model' },
      { text: 'Troubleshooting', link: '/guide/troubleshooting' }
    ]
  },
  {
    text: 'Advanced',
    collapsed: true,
    items: [
      { text: 'Models', link: '/guide/models' },
      { text: 'Config & security', link: '/guide/config-security' },
      { text: 'Routing', link: '/guide/routing' },
      { text: 'Memory notes', link: '/guide/memory' },
      { text: 'Releases & flags', link: '/guide/releases' },
      { text: 'Architecture', link: '/guide/architecture' }
    ]
  },
  {
    text: 'Developer',
    collapsed: true,
    items: [
      { text: 'Development', link: '/guide/development' },
      { text: 'Contributing extensions', link: '/guide/contributing-extensions' },
      { text: 'Agent skills', link: '/guide/skills' },
      { text: 'Discovery & test-security', link: '/guide/cli-diagnostics' },
      { text: 'HTTP daemon (removed)', link: '/guide/cli-daemon' },
      { text: 'Roadmap', link: '/guide/roadmap' },
      {
        text: 'Maintainer roadmap (repo)',
        link: 'https://github.com/qingjiuzys/anycode/blob/main/docs/roadmap.md'
      }
    ]
  },
  { text: 'Docs directory', link: '/guide/docs-directory' },
  { text: 'All pages (hub)', link: '/guide/hubs' }
]

const guideSidebarZh = [
  {
    text: '首次使用',
    collapsed: false,
    items: [
      { text: '快速开始', link: '/zh/guide/getting-started' },
      { text: '安装', link: '/zh/guide/install' },
      { text: '微信与 setup', link: '/zh/guide/wechat' }
    ]
  },
  {
    text: '日常使用',
    collapsed: false,
    items: [
      { text: '总览', link: '/zh/guide/cli' },
      { text: 'run / REPL / TUI', link: '/zh/guide/cli-sessions' },
      { text: '模型子命令', link: '/zh/guide/cli-model' },
      { text: '排错', link: '/zh/guide/troubleshooting' }
    ]
  },
  {
    text: '进阶',
    collapsed: true,
    items: [
      { text: '模型与端点', link: '/zh/guide/models' },
      { text: '配置与安全', link: '/zh/guide/config-security' },
      { text: '路由', link: '/zh/guide/routing' },
      { text: '记忆说明', link: '/zh/guide/memory' },
      { text: '版本与特性开关', link: '/zh/guide/releases' },
      { text: '架构', link: '/zh/guide/architecture' }
    ]
  },
  {
    text: '开发者',
    collapsed: true,
    items: [
      { text: '开发与贡献', link: '/zh/guide/development' },
      { text: '扩展与贡献清单', link: '/zh/guide/contributing-extensions' },
      { text: 'Agent skills', link: '/zh/guide/skills' },
      { text: '发现与 test-security', link: '/zh/guide/cli-diagnostics' },
      { text: 'HTTP 守护进程（已移除）', link: '/zh/guide/cli-daemon' },
      { text: '路线图', link: '/zh/guide/roadmap' },
      {
        text: '维护者路线图（仓库）',
        link: 'https://github.com/qingjiuzys/anycode/blob/main/docs/roadmap.md'
      }
    ]
  },
  { text: '文档地图', link: '/zh/guide/docs-directory' },
  { text: '全量索引', link: '/zh/guide/hubs' }
]

export default defineConfig({
  base,
  title: 'anyCode',
  description: 'Terminal AI agent for developers',
  lastUpdated: true,
  cleanUrls: true,
  // zh/guide links point at repo `crates/` (outside docs-site); VitePress cannot resolve them.
  ignoreDeadLinks: [/^\.?\/?(?:\.\.\/)+crates\//],

  locales: {
    root: {
      label: 'English',
      lang: 'en-US',
      title: 'anyCode',
      description: 'Terminal AI agent for developers',
      themeConfig: {
        logo: '/anycode-logo.png',
        nav: [
          { text: 'Docs directory', link: '/guide/docs-directory' },
          { text: 'Guide', link: '/guide/getting-started' }
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
          { text: '文档地图', link: '/zh/guide/docs-directory' },
          { text: '文档', link: '/zh/guide/getting-started' }
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
