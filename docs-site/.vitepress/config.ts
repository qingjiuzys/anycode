import { defineConfig } from 'vitepress'

// GitHub Pages（仓库 Pages）：https://<user>.github.io/<repo>/
// 本地开发不设环境变量时用 '/'；CI 中设置 VITEPRESS_BASE=/anycode/ 再 build。
const base = process.env.VITEPRESS_BASE || '/'

const guideSidebarEn = [
  { text: 'Docs directory', link: '/guide/docs-directory' },
  { text: 'Getting started', link: '/guide/getting-started' },
  { text: 'Install', link: '/guide/install' },
  {
    text: 'CLI',
    collapsed: false,
    items: [
      { text: 'Overview', link: '/guide/cli' },
      { text: 'Run, REPL & TUI', link: '/guide/cli-sessions' },
      { text: 'Daemon', link: '/guide/cli-daemon' },
      { text: 'Model commands', link: '/guide/cli-model' },
      { text: 'Discovery & test-security', link: '/guide/cli-diagnostics' },
      { text: 'Agent skills', link: '/guide/skills' },
      { text: 'WeChat & onboard', link: '/guide/wechat' }
    ]
  },
  { text: 'Models', link: '/guide/models' },
  { text: 'Architecture', link: '/guide/architecture' },
  { text: 'Routing', link: '/guide/routing' },
  { text: 'Config & security', link: '/guide/config-security' },
  { text: 'Troubleshooting', link: '/guide/troubleshooting' },
  { text: 'Development', link: '/guide/development' },
  { text: 'Roadmap', link: '/guide/roadmap' },
  { text: 'All pages (hub)', link: '/guide/hubs' }
]

const guideSidebarZh = [
  { text: '文档地图', link: '/zh/guide/docs-directory' },
  { text: '快速开始', link: '/zh/guide/getting-started' },
  { text: '安装', link: '/zh/guide/install' },
  {
    text: '命令行 CLI',
    collapsed: false,
    items: [
      { text: '总览', link: '/zh/guide/cli' },
      { text: 'run / REPL / TUI', link: '/zh/guide/cli-sessions' },
      { text: '守护进程', link: '/zh/guide/cli-daemon' },
      { text: '模型子命令', link: '/zh/guide/cli-model' },
      { text: '发现与 test-security', link: '/zh/guide/cli-diagnostics' },
      { text: 'Agent skills', link: '/zh/guide/skills' },
      { text: '微信与 onboard', link: '/zh/guide/wechat' }
    ]
  },
  { text: '模型与端点', link: '/zh/guide/models' },
  { text: '架构', link: '/zh/guide/architecture' },
  { text: '路由', link: '/zh/guide/routing' },
  { text: '配置与安全', link: '/zh/guide/config-security' },
  { text: '排错', link: '/zh/guide/troubleshooting' },
  { text: '开发与贡献', link: '/zh/guide/development' },
  { text: '路线图', link: '/zh/guide/roadmap' },
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
        nav: [
          { text: 'Docs directory', link: '/guide/docs-directory' },
          { text: 'Guide', link: '/guide/getting-started' },
          { text: '中文', link: '/zh/' }
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
        nav: [
          { text: '文档地图', link: '/zh/guide/docs-directory' },
          { text: '文档', link: '/zh/guide/getting-started' },
          { text: 'English', link: '/' }
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
