# Day 3 · 大爆炸（2026-07-05）

> **一天 38 个 commit**（git 可查证），PR #15–#51 当日开当日合 · **至少 10 个 Claude worktree + 一支 Codex 编队**并行 · **史料** [[S26]]–[[S38]] · 本章时间为北京时间；这一"天"实际从 07-05 08:00 连续跑到 07-06 09:30，约 25 小时

## 双厂牌军团

大爆炸日的第一个史实会颠覆多数人的想象：它是**两个 AI 厂牌的联合作战**。

早晨 08:00 起的 90 分钟里，一支 **Codex 编队**打出 11 个 PR 的齐射——landing 刷新、SEO/OG、PSP 文字渲染性能、benchmark skill、诚实的「8 MB heap」文案修正、solid API 导入规范（[[PR21]]，今天 CLAUDE.md 里的导入纪律源头）——多数 PR 从创建到合并只隔二十几秒。**PR 是 changelog，不是关卡。**

而 Claude 的 worktree 承担深水区的从零到一。当天的战线图：

| worktree | 任务 | 产出 |
|---|---|---|
| humorous-cough | Gallery、CI 自动部署、画布 HUD、掌机外壳 | [[PR26]] [[PR28]] [[PR29]] [[PR30]] [[PR31]] [[PR35]] |
| faceted-fahrenheit | `@pocketjs/aot`：TSX 部分求值编译成 **GBA 卡带**，真 mGBA E2E | [[PR34]] |
| amazing-suggestion | landing 的设备路线图（3DS/iOS/Android "Coming Soon"） | [[PR33]] [[PR39]] |
| spectrum-yamamomo | Pocket3D 竞速者 A（设计文档版 prompt） | PR #49 · **落选关闭** |
| jealous-burn | Pocket3D 竞速者 B（自然语言版 prompt）· 75 MB 转录 | [[PR48]] · **胜出** |
| wax-star | Pocket3D 竞速者 C（ultracode 版 prompt） | 无 · **死于组织月度用量上限** |
| aspiring-birch | AOT 扩展到 **Game Boy + NES** + 神雕侠侣同人 RPG | PR #52 |
| wheat-cupboard | **iOS host**：PocketJS 做 React Native 替代，跑生产级 IM | 闭源 fork |
| comet-linseed / plum-watcher | 昨夜的 Video 组件与 3DS 移植，凌晨转 draft PR | draft #17 等 |

最令人瞠目的细节：**第二个前端框架 Vue Vapor（[[PR27]]）来自 Codex 分支，创建到合并 21 秒**。它之所以便宜——后来量化过：renderer 108 行 + DOM facade 136 行，**约 244 行接入一个新框架**——是因为运行时真正的契约是 HostOps/DrawList 边界，不是框架本身。当天四个后续 PR 立刻把它扶正为一等公民，一年后它撑起了一场 VueConf 演讲。

## 18:36 · 一个下载文件夹引发的军备竞赛

`~/Downloads/cs-maps-20260705-1836`——文件夹名字本身就是时间戳：18:36，CS 1.6 地图落盘。接下来两个半小时：

- 18:56 spectrum-yamamomo 收到「ultracode 完整实现 @pocket3d_openstrike_design.md 方案」
- 19:01 jealous-burn 收到自然语言版创始简报
- 19:19 第四个 worktree 已经有 vertical-slice PR 挂出
- 21:18 wax-star 收到 ultracode 版同一简报

**同一个 3D 引擎，三种 prompt 变体，四条战线竞速。** 落选者的 PR 直接关闭；wax-star 的对抗式 review 群在 9/12 个 agent 报错「You've hit your org's monthly spend limit」后倒下——那天唯一能阻止任何事情的力量。jealous-burn 用 75 MB 的转录跑完全程。

创始简报值得全文引用，它是"验收标准写进立项书"的范本——**目标、非目标、可证伪的验收回路，一段话说完**：

:::quote 2026-07-05 19:01 · S31 · jealous-burn
现在想在 Pocket 项目里做一个独立的 Rust 3D runtime，暂定叫 Pocket3D，它不耦合现有 PocketJS/PSP 2D UI 代码，而是作为一个现代、轻量、可扩展的 3D 基础设施存在；它的第一个验证案例是 OpenStrike：一个单机 CS-like FPS example，把 BSP，尤其是 Dust2 这种地图，作为一等公民支持。第一版不追求复刻经典 CS 手感，也不做联网或完整通用引擎，只需要证明：在 Pocket3D 里能加载 BSP 场景，第一人称走路，拿枪，遇到会移动和播放动画的简单 Bot，开枪击杀它，并完成一局胜负与自动重开循环。
:::

而当竞速者 A 交付时，作者的验收标准浓缩成七个字：

:::quote 2026-07-05 21:04 · S30
怎么体验？……我要真玩
:::

不是测试报告，不是截图——是亲手在 Dust2 里走一圈。

## 凌晨 1 点 · bug 报告变成宣言

jealous-burn 的第二幕由当天最有分量的一条消息开启。它同时是 bug 反馈（枪械视模抖动、「人物我不喜欢用现在这个卡通人物」）和一篇本体论宣言：

:::quote 2026-07-06 01:19 · S35 · jealous-burn
我的愿景是可以模糊 App 和 Game 之间的边界……整个 pocketjs 不是一个想要做成 Unreal 那样大而全的通用引擎，而是特化出很多种小的专有引擎，并在此之上封装出最适合相应场景的扩展 API（比如对 2D UI 是 jsx，对 FPS 应该是 mod）。我希望据此探索出一种更像 Roblox 的扩展架构，这是极其有价值的事情，所以希望你从本体论的视角出发，抽象出最适合最优雅的扩展机制。
:::

Claude 的回答成了 `RUNTIMES.md`：「Roblox 的价值在于『创作单元是脚本而非引擎构建』，但代价是一个万能 DataModel。Pocket 的对偶命题是：**不共享世界本体，共享定义世界本体的语法**」——形式化为 `Runtime = ⟨Cores, Surfaces, Guest⟩`。落地物同样具体：`pocket-mod`（rquickjs host）、`pocket-ui-wgpu`（继 PSP GE 和 wasm 之后的第三个 DrawList 后端）、macOS `uihost` 让**未改一字的 PSP bundle 跑进桌面窗口**、open-strike 里 Rust 侧的回合状态机被删掉——`game/rules.ts` 是"第一个 mod"，`game/hud.tsx` 是一个完整的 PocketJS Solid 应用合成在 3D 画面上。

合并纪律与野心同规格：agent 主动列出 submodule re-pin 风险（「否则外部用户 clone --recursive 会拉到一个还指向 feature 分支的引擎」），对合并后的 main 重跑 walk 脚本，open-strike 打 v0.1.0 tag。

## 凌晨 4 点 · 还在开辟新平台

3D 尚未着陆，作者又写了两份完整的凌晨立项书：aspiring-birch 把卡带编译器扩展到 Game Boy + NES（「我还是希望用 TypeScript，因为我希望推广 TS DSL 作为 authoring model 来使得创作（和 remix）这些平台上的游戏可以不需要用到系统编程的知识」），附一个神雕侠侣同人 RPG 的**真机验收**要求；wheat-cupboard 开启 iOS host（「从第一性原理出发最高效的 pocket mobile ios 方案」），给 Paperboy IM 做 React Native 替代。

## 尾声 · 告诉世界

07-06 上午，jealous-burn 在贴图驱动的收尾轮次后（「README 里面截图的顺序调换了吗？第一张也要是 Dust」），作者要求一份**合并前简报**（「接下来我准备要发布 3D 这整个所有仓库相关的代码了」），[[PR48]] 于 09:24 合入。而 **4 分钟之前**，主 checkout 里 [[S39]] 已经打开：

:::quote 2026-07-06 09:24 · S39
基于现在 PocketJS 的现状，我准备发 Twitter 的第一条推。开头应该叫"Introducing PocketJS"…请你提炼一下这个项目最激动人心的一些地方……需要你把你理解的、它在技术上到底有哪些前无古人的创新点都写出来。你写好以后在本地环境运行一下，我来验收。
:::

launch blog（[[PR53]]）当日中午合入，署名 Yifeng "Evan" Wang。大爆炸以向世界宣告收场——而首个 release 的倒计时，从这篇博客开始。

## 本章要点

- **指挥风格在这一天定型**：立项书长（200–400 字，含目标/非目标/验收回路），steering 短（「继续」「开 PR」「我要真玩」）；输入预先备好在 ~/Downloads，prompt 是点火器不是需求会。
- **worktree 是牲畜不是宠物**：同一简报三种变体竞速，输家 PR 直接关闭。
- **治理规则在战斗中立法**：Conventional Commits、draft PR 流程、merge 即部署，全是当天顺手定下的，至今有效。
- **上限真实存在**：这一天唯一停下来的原因，是组织月度用量额度。

**下一章**，07-06 深夜到 07-08 清晨：release pipeline、v0.2.0、40 分钟后的 v0.2.1、以及次日早晨的 v0.3.0 时间旅行 devtools。
