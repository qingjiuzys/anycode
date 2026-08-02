type ConceptId = "atlas" | "signal" | "studio" | "paper" | "orbit";

type Concept = {
  id: ConceptId;
  name: string;
  tag: string;
  thesis: string;
  palette: [string, string, string, string];
  headline: string;
  description: string;
  cta: string;
  nav: string[];
  navLinks: string[];
  features: string[];
};

const CONCEPTS: Concept[] = [
  {
    id: "atlas",
    name: "Atlas",
    tag: "产品优先",
    thesis: "深色工程工作台，强调任务轨迹与本地掌控。",
    palette: ["#11131a", "#1d2432", "#dff35b", "#edf1f7"],
    headline: "把复杂工作，交给可见的执行轨迹。",
    description: "面向技术团队的 agent 工作台。每一次判断、工具调用与交付都可追踪。",
    cta: "下载桌面版",
    nav: ["产品", "案例", "定价", "文档"],
    navLinks: ["/product", "/cases/launch-ppt", "/plans", "/docs"],
    features: ["会话接力", "本地执行", "交付物归档"],
  },
  {
    id: "signal",
    name: "Signal",
    tag: "可信执行",
    thesis: "冷静、高信任的信息设计，适合企业采购与团队协作。",
    palette: ["#edf1f4", "#ffffff", "#1769e0", "#172131"],
    headline: "每个 agent，都有清晰边界。",
    description: "审批、权限与执行记录放在同一条工作流里，团队能放心把任务交出去。",
    cta: "开始使用",
    nav: ["能力", "安全", "套餐", "帮助中心"],
    navLinks: ["/features", "/product", "/plans", "/docs/help"],
    features: ["审批可控", "模型自由", "团队工作区"],
  },
  {
    id: "studio",
    name: "Studio",
    tag: "创作协作",
    thesis: "更有温度的品牌站，突出人和 agent 一起完成交付。",
    palette: ["#f4efe8", "#fffdfa", "#a8341d", "#19211e"],
    headline: "从一个念头，到可以交付的作品。",
    description: "anyCode 把研究、制作和检查串成一个有节奏的协作过程。",
    cta: "体验工作台",
    nav: ["如何工作", "案例", "社区", "下载"],
    navLinks: ["/features", "/cases/launch-ppt", "/docs", "/downloads"],
    features: ["需求拆解", "技能组合", "成果打包"],
  },
  {
    id: "paper",
    name: "Paper",
    tag: "清晰叙事",
    thesis: "轻盈的中文排版系统，让产品价值与案例内容成为主角。",
    palette: ["#fbfaf7", "#e7e2d8", "#203a63", "#1d1d1f"],
    headline: "让智能体做事，也让每一步说得清楚。",
    description: "以案例、方法与产品能力为叙事主线，适合把 anyCode 做成可理解的品牌。",
    cta: "查看案例",
    nav: ["首页", "案例", "方法", "文档"],
    navLinks: ["/", "/cases/launch-ppt", "/features", "/docs"],
    features: ["任务脉络", "真实案例", "可读文档"],
  },
  {
    id: "orbit",
    name: "Orbit",
    tag: "未来感",
    thesis: "克制的空间感与动态层次，为 AI 原生产品建立辨识度。",
    palette: ["#10121c", "#20283d", "#a9a8ff", "#f5f6ff"],
    headline: "让每一个想法，进入自己的运行轨道。",
    description: "以项目为中心组织模型、工具与产物，形成持续积累的个人工作系统。",
    cta: "进入 anyCode",
    nav: ["工作台", "生态", "价格", "开发者"],
    navLinks: ["/product", "/features", "/plans", "/docs"],
    features: ["项目上下文", "多模型协作", "持续记忆"],
  },
];

const B_CONCEPTS = [
  {
    id: "command",
    name: "Command",
    tag: "执行系统",
    thesis: "把官网做成一台正在工作的机器。命令、状态和结果就是品牌语言。",
    palette: ["#080b09", "#d7ff43", "#b7c0b7", "#212721"],
  },
  {
    id: "bauhaus",
    name: "Bauhaus",
    tag: "瑞士平面",
    thesis: "极强的网格、字号反差与红色切面，建立直接而国际化的技术品牌。",
    palette: ["#f2f0e8", "#171717", "#ee3b2f", "#2d5cff"],
  },
  {
    id: "shanshui",
    name: "Shanshui",
    tag: "东方留白",
    thesis: "用当代中文排版与水墨空间讲述智能体，不再复制西方 SaaS 的视觉模板。",
    palette: ["#f5f2e9", "#171914", "#a62920", "#a9af9e"],
  },
  {
    id: "blocks",
    name: "Blocks",
    tag: "模块玩具",
    thesis: "像搭积木一样组合模型、技能和产物，明快、亲近且具产品解释力。",
    palette: ["#ffdc3a", "#1768ff", "#ff6b55", "#15211d"],
  },
  {
    id: "cinema",
    name: "Cinema",
    tag: "空间叙事",
    thesis: "用电影标题、景深和聚焦式构图，塑造更高端、更有发布会感的产品形象。",
    palette: ["#090b12", "#f1eee7", "#ff7849", "#29324a"],
  },
] as const;

function ProductPreview() {
  return (
    <div className="design-lab-product" aria-hidden>
      <aside>
        <span className="design-lab-product__brand">a/</span>
        <span className="is-active">今天</span>
        <span>项目</span>
        <span>技能</span>
      </aside>
      <div className="design-lab-product__body">
        <div className="design-lab-product__top">
          <span>产品发布</span>
          <span>DeepSeek · 已连接</span>
        </div>
        <p className="design-lab-product__prompt">整理本周产品发布计划，并生成可审阅的交付清单。</p>
        <div className="design-lab-product__activity">
          <span>正在分析项目文件</span>
          <span>已找到 4 个相关文档</span>
          <span>准备生成发布计划</span>
        </div>
        <div className="design-lab-product__composer">下一步想完成什么？ <b>↑</b></div>
      </div>
    </div>
  );
}

function CommandPrototype() {
  return (
    <section className="b-prototype b-command" id="command">
      <header><strong>ANYCODE://</strong><span>LOCAL AGENT OPERATING SYSTEM</span><a href="/downloads">INSTALL ↗</a></header>
      <div className="b-command__body">
        <aside aria-label="Command navigation">
          <a href="/product">01 PRODUCT</a><a href="/features">02 PROTOCOL</a>
          <a href="/cases/launch-ppt">03 CASES</a><a href="/docs">04 DOCS</a>
        </aside>
        <div className="b-command__hero">
          <p>$ anycode run</p>
          <h2>Agents that<br />finish the job<span>_</span></h2>
          <div className="b-command__log">
            <p><i>✓</i> context loaded from local workspace</p>
            <p><i>✓</i> plan approved by operator</p>
            <p><i>↳</i> delivery package ready</p>
          </div>
        </div>
        <div className="b-command__metric"><strong>LOCAL</strong><span>your files stay on your machine</span></div>
      </div>
    </section>
  );
}

function BauhausPrototype() {
  return (
    <section className="b-prototype b-bauhaus" id="bauhaus">
      <nav><strong>any<br />Code</strong><div><a href="/product">PRODUCT</a><a href="/cases/launch-ppt">CASES</a><a href="/plans">PLANS</a><a href="/docs">DOCS</a></div><a href="/downloads">GET APP</a></nav>
      <div className="b-bauhaus__stage">
        <div className="b-bauhaus__word"><span>ANY</span><span>CODE</span></div>
        <div className="b-bauhaus__circle">A</div>
        <div className="b-bauhaus__copy">
          <h2>一个能把事情<br />做完的智能体。</h2>
          <p>本地运行。过程可见。结果可交付。</p>
        </div>
        <div className="b-bauhaus__index"><b>01</b><span>WORKBENCH<br />FOR AGENTS</span></div>
      </div>
    </section>
  );
}

function ShanshuiPrototype() {
  return (
    <section className="b-prototype b-shanshui" id="shanshui">
      <nav><strong>anyCode</strong><div><a href="/product">产品</a><a href="/cases/launch-ppt">案例</a><a href="/docs">文档</a></div><a href="/downloads">下载</a></nav>
      <div className="b-shanshui__stage">
        <div className="b-shanshui__copy">
          <span>智能工作台</span>
          <h2>事有始终，<br />智有所托。</h2>
          <p>从一句话开始，梳理脉络、调用工具、检查结果。复杂工作，也能从容完成。</p>
          <a href="/downloads">开始使用</a>
        </div>
        <div className="b-shanshui__landscape" aria-hidden>
          <i /><i /><i />
          <div className="b-shanshui__seal">安<br />码</div>
        </div>
        <div className="b-shanshui__notes">
          <span>本地为本</span><span>人机共议</span><span>成果有据</span>
        </div>
      </div>
    </section>
  );
}

function BlocksPrototype() {
  return (
    <section className="b-prototype b-blocks" id="blocks">
      <nav><strong>anyCode!</strong><div><a href="/features">怎么工作</a><a href="/cases/launch-ppt">案例</a><a href="/plans">价格</a></div><a href="/downloads">免费开始</a></nav>
      <div className="b-blocks__stage">
        <div className="b-blocks__headline">
          <span>把</span><strong>大任务</strong><span>拆成</span><strong>好积木</strong>
        </div>
        <div className="b-blocks__tiles">
          <div className="is-blue"><small>模型</small><b>想</b><span>理解需求</span></div>
          <div className="is-coral"><small>技能</small><b>做</b><span>调用工具</span></div>
          <div className="is-ink"><small>审批</small><b>问</b><span>关键处由你决定</span></div>
          <div className="is-white"><small>产物</small><b>交</b><span>直接拿走成果</span></div>
        </div>
      </div>
    </section>
  );
}

function CinemaPrototype() {
  return (
    <section className="b-prototype b-cinema" id="cinema">
      <nav><strong>anyCode</strong><div><a href="/product">PRODUCT</a><a href="/cases/launch-ppt">STORIES</a><a href="/downloads">DOWNLOAD</a></div><a href="/login">SIGN IN</a></nav>
      <div className="b-cinema__stage">
        <div className="b-cinema__orb" aria-hidden><i /><i /><i /></div>
        <p className="b-cinema__eyebrow">THE DESKTOP AGENT WORKBENCH</p>
        <h2>Ideas enter.<br /><em>Work</em> comes out.</h2>
        <div className="b-cinema__footer">
          <p>让模型、工具与项目上下文<br />在同一个空间里持续协作。</p>
          <a href="/downloads">下载 anyCode <span>↗</span></a>
          <strong>BUILT<br />TO DELIVER</strong>
        </div>
      </div>
    </section>
  );
}

function BPrototype({ id }: { id: (typeof B_CONCEPTS)[number]["id"] }) {
  switch (id) {
    case "command": return <CommandPrototype />;
    case "bauhaus": return <BauhausPrototype />;
    case "shanshui": return <ShanshuiPrototype />;
    case "blocks": return <BlocksPrototype />;
    case "cinema": return <CinemaPrototype />;
  }
}

type ConceptScreenProps = {
  concept: Concept;
};

function ConceptScreen({ concept }: ConceptScreenProps) {
  return (
    <section className={`design-lab-screen design-lab-screen--${concept.id}`} id={concept.id}>
      <nav className="design-lab-screen__nav">
        <strong>anyCode</strong>
        <div>
          {concept.nav.map((item, index) => (
            <a key={item} href={concept.navLinks[index]}>{item}</a>
          ))}
        </div>
        <a className="design-lab-screen__login" href="/login">登录</a>
      </nav>
      <div className="design-lab-screen__hero">
        <div className="design-lab-screen__copy">
          <p>{concept.tag}</p>
          <h2>{concept.headline}</h2>
          <div className="design-lab-screen__actions">
            <a className="design-lab-screen__primary-cta" href="/downloads">{concept.cta}</a>
            <a href={`#${concept.id}-notes`}>了解设计方向</a>
          </div>
        </div>
        <ProductPreview />
      </div>
      <div className="design-lab-screen__features">
        {concept.features.map((feature, index) => (
          <div key={feature}>
            <span>0{index + 1}</span>
            <strong>{feature}</strong>
          </div>
        ))}
      </div>
    </section>
  );
}

export function DesignLabPage() {
  return (
    <div className="design-lab">
      <header className="design-lab__header">
        <a href="/">anyCode</a>
        <span>anycode.work redesign study</span>
        <a href="#atlas">5 个方向</a>
      </header>

      <main>
        <section className="design-lab__intro">
          <p>重新设计提案</p>
          <h1>同一个 anyCode，五种可落地的品牌表达。</h1>
          <span>五种首页原型同时呈现。每种都包含导航、核心内容层级、工作台视觉语言和可延展的页面系统。</span>
        </section>

        <div className="design-lab__gallery">
          {CONCEPTS.map((concept) => (
            <article className="design-lab__concept" key={concept.id}>
              <header>
                <div>
                  <p>{concept.name}</p>
                  <h2>{concept.tag}</h2>
                </div>
                <span>{concept.thesis}</span>
                <div className="design-lab__swatches" aria-label={`${concept.name} 配色`}>
                  {concept.palette.map((color) => (
                    <i key={color} style={{ background: color }} title={color} />
                  ))}
                </div>
              </header>
              <ConceptScreen concept={concept} />
              <dl id={`${concept.id}-notes`}>
                <div><dt>适用页面</dt><dd>首页、产品、功能、套餐、下载、文档入口</dd></div>
                <div><dt>建议重点</dt><dd>{concept.features.join("、")}</dd></div>
              </dl>
            </article>
          ))}
        </div>

        <section className="design-lab__round-two">
          <header>
            <p>第二轮探索</p>
            <h2>不换皮，换一套表达逻辑。</h2>
            <span>下面五种不共享首屏模板。每个方向从品牌性格、信息结构到交互材料都重新开始。</span>
          </header>
          <div className="design-lab__gallery">
            {B_CONCEPTS.map((concept) => (
              <article className="design-lab__concept design-lab__concept--bold" key={concept.id}>
                <header>
                  <div><p>{concept.name}</p><h2>{concept.tag}</h2></div>
                  <span>{concept.thesis}</span>
                  <div className="design-lab__swatches" aria-label={`${concept.name} 配色`}>
                    {concept.palette.map((color) => <i key={color} style={{ background: color }} title={color} />)}
                  </div>
                </header>
                <BPrototype id={concept.id} />
              </article>
            ))}
          </div>
        </section>
      </main>
    </div>
  );
}
