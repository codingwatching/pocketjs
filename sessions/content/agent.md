# Agent 如何拆解与推进

> 本章基于全部 69 个 session 的转录统计与四个深潜样本（Day 1 建站、gallery、devtools/发布、pocket-youtube）。先给规模感：**888 条人类消息，驱动了 37,227 条 assistant 消息（1 : 42）与约 9,000 次 shell 命令**。

## 1 · 接令：从「ultracode」到任务板

作者的开工消息带一个触发词——**「ultracode」在语料中出现 76 次**，含义是：研究 → 设计 → 实现 → 验证 → 开 PR，全程不等待。与之配对的是光杆的「继续」（36 次）。收到指令后，agent 的第一动作不是写代码，而是**侦察叙事**：读代码、报告事实、收敛出承重问题。Day 1 建站 session 的范式时刻——agent 拒绝在风险未拆除前扇出：

:::agent S21 · 2026-07-03 · Day 1 建站
Two things are on the critical path and I need to de-risk them before fanning out: 1. The rename surface … 2. The playground's hardest assumption — that @babel/core + babel-preset-solid (generate:'universal') can be bundled to run *in the browser*. If that fails, the whole live-recompile design changes.
:::

spike 三次失败（CJS/ESM 边界）、精确修通之后才有一句「**Critical risk retired.**」，然后才建任务板。整个语料共 **425 次建任务 / 762 次更新任务**，任务板有稳定语法：**第一项是设计文档，最后一项是"验证 + draft PR"**。devtools 的板子是典型：「写 DEVTOOLS 设计文档」→「框架侧 shim：树注册、命名、rect 查询、高亮」→「输入磁带录制/回放 + 暂停单步」→「调试通道：WS hub + PSP host0: 信箱」→「UI」→「接入测试设施」→「验证并开 draft PR」。

有趣的反事实：**正式的 plan mode 在 69 个 session 里只用过一次**（PocketShell，[[S53]]）。那份唯一的计划书以「已确认的关键事实（探索阶段验证过源码）」开头，每条都带 file:line 证据——包括"`active:` 变体已编译但从未有调用方（core/src/lib.rs:704）"这种级别的考据——并把路线图显式切成"本波实现"与"声明但不实现"。其余时间，计划都是公开进行的侦察叙事 + 任务板。

什么时候跳过计划？作者自带设计文档时（prompt 就是 `@design.md`）；小而可观察的改动（直接 Bash/Edit）；以及纯文案 session（任务板在那里只是表演）。

## 2 · 拆解的四种形状

**(a) 规格先行。** 大系统的第一项任务是写设计工件：DEVTOOLS.md、RUNTIMES.md（本体论）、PocketShell 计划书。

**(b) 垂直切片。** PocketJS 的特性是一刀切穿全部层次的。一条真实的任务原文：

:::agent S24 · gallery session 的一条任务
Implement sprite atlas: spec + Rust core (tick/draw) + wasm + PSP native — Add sprite node/op + pak sprite-atlas entry; core ticks frames (fixed dt) + draws atlas UV sub-rect; wasm raster UV sampling; native pak.rs raw read + ge.rs sprite draw (flag PSP compile-unverified).
:::

层序稳定：契约/spec → core → 各宿主 → demo 作为证明。注意括号里的「flag PSP compile-unverified」——**不能验证的部分被显式标注，而不是假装完成**。

**(c) 实现 → 对抗式评审 → 重构，作为显式循环。** 从 dreamcart 时代作者就要求循环而非线性（「在合并之前要开一次比较大的对抗性的评审……这个过程可以重复几次」），agent 把它制度化成常设任务项：「Review & adversarially verify the diff」→「Apply **9 confirmed** review fixes + re-verify」——评审发现本身也被当作待证假设，先确认再修，所以数字精确到"9 个确认"。

**(d) 移植换永久资产。** OpenStrike 上 PSP 的任务板里藏着一条「**Refactor pocket3d: portable core + renderer backend seam**」——一次移植支付一道永久的多后端接缝。demo 也一样：figma viewer、IM、YouTube，每个都是为了逼出引擎缺口（「借此完善 pocket 的引擎基建」）。

## 3 · 验证阶梯，与那根作为调度原语的 USB 线

每个严肃 session 都爬同一架梯子：`tsc --noEmit` → `bun test` → **byte-exact golden** → 无头截图（站点用 Chrome，主机用 PPSSPP）→ PSPLINK 真机 → 作者本人的眼睛和耳朵。深潜样本里的命令关键词计数：

| session | bun test | golden | ppsspp | psplink |
|---|---|---|---|---|
| S21 Day 1 建站 | 3 | 0 | 0 | 0 |
| S24 gallery | 31 | 57 | 0 | 0 |
| S42 devtools/发布 | 28 | 40 | 84 | 52 |
| S43 OpenStrike 真机 | 13 | 35 | 69 | 6 |

真机验证依赖一根物理 USB 线，转录里能看到真实的人机硬件握手被编进任务排序：PocketShell 计划书里写着「需 PSP 接 PSPLINK；若线不在手边，PPSSPP e2e 先行，真机留待用户插线」；作者宣布资源到位（「psplink 连上了，开始吧」「重插了」），agent 把积压的硬件验收项批量打进那个窗口。而最后一级永远是人：重写 PSP 音频路径后，agent 的收尾是——「等你耳朵的最终裁决：滋滋声还在不在」。

## 4 · 四个从失败里爬出来的故事

**npm EOTP 拉锯（07-07）。** 诊断 → 给出带用户操作步骤的 A/B 方案 → 从截图解码 npm UI 陷阱 → 换 B 计划本地 bootstrap 发布 → 切 OIDC 让 CI 永久免 token → 当天用 v0.2.1 端到端验证 → 蒸馏进 memory 和 release skill。（详见[首个 release](/first-release/)。）

**会 flake 的发布闸门（v0.4.0，07-13）。** 两次失败运行：npm 12 改了 `npm pack --json` 的输出形状（本地 npm 11 掩盖了它）；Bun 5 秒默认测试超时在慢 CI 机上抖动。产出的教义:「**重打 tag 就是正确的重试**——publish 步骤的 `npm view` 守卫让重跑幂等，而失败的 run 什么都没发出去（闸门先跑）」。

**过期 dist 的 golden 陷阱。** Vita e2e 会把 dist bundle 重建成 Vita 目标，而 golden 脚本只补缺失文件——合并日一次性出现 30 个假 golden 失败。蒸馏出的规则（golden 裁决前先 `rm -f dist/*`）后来**抓住了一个真回归**：v0.5.0 首次打 tag 就失败在两张像素 golden 上，那正是被同一陷阱在本地掩盖的 stale 像素——memory 里记为「First real gate catch」。

**跨仓库 ABI 破坏当作状态来记账。** vendored quickjs-rs 的一个 vita commit 用 `<limits.h>` 换掉了 `#define CHAR_BIT 8`，打断所有 PSP 构建。memory 卡片记录修复 commit、哪个 clone 有它、哪些下游仓库还 pin 着坏 sha、以及"未推送"的悬置状态——直到 07-13 标记 RESOLVED。

错误的日常质感其实很小：S42 里 640 次 Bash 有 53 次报错——瞬时 SSL 失败（重试）、harness 守卫、沙箱提醒。硬件调试风格是假设驱动的，agent 自己的复盘说得最好：「这套系统里几乎每个 bug 都留下了可以读取的证据，而每个假设都能做出可证伪的预言……flex 压缩是从截图里量出圆角位置偏了 56px、而 456+56 恰好等于 512 的那一刻破案的。」

## 5 · 编排习惯

- **worktree-per-feature**：全语料 28 个 worktree；07-05 一天 7 个并行。
- **子 agent 117 次**：两大用途——设计前的并行只读侦察（devtools 动工前 4 个 Explore 同时测绘 core/宿主/PSPLINK/测试设施），和可并行的生产（GameBlocks 一个 session 扇出 20 个 worker 移植 74 个模块）。
- **agent 自写编排脚本（Workflow 78 次）**：Day 1 在 site/ 里留下了四个——`.docs-workflow.js`（"one agent per doc page"，共享简报里明令禁止写出字符串 "psp-ui"）、3 变体 PSP 外框、4 变体 landing。作者甚至通过 agent 去催 agent 的 agent：「没有进展，催一下他们」。
- **后台进程 + Monitor**：usbhostfs/pspsh 守护、510 帧 dust2 巡游截帧、bench JSONL 的在线 reducer。
- **draft PR 纪律**：CLAUDE.md 立法（这条规则本身也是作者一句话装进去的），S42 一个 session 开了 16 个 draft。
- **提问极度克制**：一个月只有 29 次 AskUserQuestion——技术决策靠 spike 自答，**只有身份决策才打断人**（品牌名、设计方向、发布与否），且问题总是带着预验证的选项和推荐。
- **skill 与 memory 是工作流的结晶**：5 个 repo skill（release、devtools、benchmark、imagegen、video-outro），每个都诞生于先手工做过一遍的 session；约 30 张 memory 卡片以「Why / How to apply」格式跨 session 携带活状态（未推送的修复、"hardware acceptance pending"、"NEON RUSH 代码已丢失"）。

## 6 · 数字里的工作方式

| 工具 | 次数 | 说明 |
|---|---|---|
| Bash | 8,930 | 构建、测试、PPSSPP、PSPLINK、gh、部署都住在这里 |
| Edit / Write | 4,058 / 1,164 | 写代码 |
| Read | 3,112 | 读代码 |
| TaskUpdate / TaskCreate | 762 / 425 | 任务板 |
| Agent（子 agent） | 117 | 侦察与并行生产 |
| Workflow | 78 | 自写编排脚本 |
| AskUserQuestion | 29 | 一个月仅 29 次 |
| WebSearch | 4 | **一个月只搜了 4 次网**——这个项目几乎完全靠本地证据运转 |

人类消息的分布是双峰的：马拉松 steering session（S42：83 条人类消息，多为验收反馈）对阵 fire-and-forget 的 ultracode run（[[S32]]/[[S34]]：**1 条人类消息、283 条 assistant 消息**；[[S67]] 用 3 条人类消息近自治地建成了 Pocket Static）。会话卫生也有数据：`/model` 44 次、`/compact` 28 次、`/effort` 14 次；32 个 session 带着"从上一个耗尽上下文的对话继续"的摘要开场——多日马拉松是压缩链，靠任务板和 memory 卡片存活。

## 7 · 纠偏如何改写了行为

- **性能上的低野心 → 证据或沉默**：06-17 的爆发之后，同 session 就出现「这个优化过程，你也需要存成项目 docs 目录里的一些经验」；后代是 benchmark skill、bench JSONL Monitor 背书的锁 60fps 声明。
- **流程漂移 → 一次立法，永不再犯**：Conventional Commits（07-05）与「不要写 sh，一律用 bun shell」（07-10）各自产生 CLAUDE.md 规则 + memory 卡片，**此后任何 digest 里都没有再出现同类纠偏**。
- **文案翻车 → 成文的声音契约**：blog-writing-voice 卡片（hackathon 语气、个人段落短/真/第一人称、绝不虚构轶事）之后，Vita 与 YouTube 两篇发布文案都一两轮过审。
- **修不掉的弱项：像素级品味。** 一个肩键圆角迭代五轮（「懂不懂？」）。没有规则能修好它，最终的缓解是结构性的：先贴截图再请验收、尽早索要参考图、N 变体扇出让作者**挑选而不是迭代**。
- **作者也在调试 agent 本身**：里程碑后的复盘（「你觉得这次你的表现怎么样，你喜欢这个事情吗」）把 agent 自述的痛点变成排期——device-stats 与 bundle-hash 陷阱线（[[PR118]]）的存在，就是因为 agent 说真机调试很疼。

## 一句话总结

如果把作者的方法概括为"把每个重复动作烘焙成工件"，那么 agent 的方法就是它的镜像：**把每个指令展开成一架可验证的梯子**——事实先行、风险先拆、垂直切片、每层留证据、失败结痂成教义。两者合在一起，就是 15 天从抽取到 v0.5.0 的那台机器。

想看这台机器的原始记录，去 [Session 档案馆](/sessions/)；想看它的产出速率，去[数字全景](/numbers/)。
