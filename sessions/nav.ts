// sessions/nav.ts — the wiki's chapter tree (sidebar + pagers).

export type NavItem = { slug: string; title: string };
export type NavSection = { title: string; items: NavItem[] };

// Slugs map 1:1 to content/<slug>.md unless generated (timeline, sessions,
// numbers are built from data by build.ts but still live in this nav).
export const WIKI_NAV: NavSection[] = [
  {
    title: "序",
    items: [{ slug: "", title: "这是什么" }],
  },
  {
    title: "编年史 · 从零到 v0.2.0",
    items: [
      { slug: "prehistory", title: "前史：dreamcart 里的 PSP 梦" },
      { slug: "day-one", title: "Day 1 · 抽取（7·3）" },
      { slug: "identity", title: "Day 2 · 名字与门面（7·4）" },
      { slug: "big-bang", title: "Day 3 · 大爆炸（7·5）" },
      { slug: "first-release", title: "Day 4–5 · 首个 release（7·6–7·7）" },
    ],
  },
  {
    title: "剖析",
    items: [
      { slug: "steering", title: "作者如何思考与 steering" },
      { slug: "agent", title: "Agent 如何拆解与推进" },
      { slug: "numbers", title: "数字全景" },
    ],
  },
  {
    title: "档案",
    items: [
      { slug: "epilogue", title: "后记 · v0.3.0 → v0.5.0" },
      { slug: "timeline", title: "全量时间线" },
      { slug: "sessions", title: "Session 档案馆" },
      { slug: "colophon", title: "方法论 · 本站如何生成" },
    ],
  },
];
