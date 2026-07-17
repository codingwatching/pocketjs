# 前史：dreamcart 里的 PSP 梦

> **时间跨度** 2021-09-27 → 2026-07-02 · **舞台** `doodlewind/dreamcart` · **史料** [[S2]] [[S4]] [[S5]] [[S9]] [[S10]] [[S11]] [[S19]]

PocketJS 的独立仓库只活了 5 天就发布了首个 release。这个速度的全部秘密，藏在另一个仓库里：在被抽取成 PocketJS 之前，这套东西叫 **psp-ui**，生长在一个叫 **dreamcart** 的游戏项目内部。而 dreamcart 自己的故事，要从五年前讲起。

## 五年引线（2021 → 2026-06-15）

dreamcart 仓库最老的 commit 是 `7b03993`，时间 **2021 年 9 月 27 日**：一个把 QuickJS 跑上索尼 PSP 的 Rust PoC，当时叫 psp-js，带着 `quickjs-rs` 和 `rust-psp` 两个 submodule。然后是四年半的沉寂。

2026 年 6 月 15 日，`489643d` 让它复活：web playground、3DS target、同构 TS 游戏框架。第二天改名 DreamCart，口号是 "Self-contained game cartridges for tiny worlds"——同一份游戏 JS，不改一行跑在 PSP、Web、3DS 上，每个平台只实现一个极小的 native 契约。

所以 PSP 从来不是 dreamcart 里"新增的一个 target"。**这个项目本来就是从 PSP 里长出来的。**

## 三天打穿 3D（06-16 → 06-18）

6 月 16 日清晨的开题指令，定下了整个夏天的基调：

:::quote 2026-06-16 · S2 · dreamcart
ultracode 系统研究一下如何下一步设计实现 psp 3ds 和 web 上 JS 业务逻辑同构的 3d 游戏，设想是游戏引擎各自实现，但逻辑同一份。示例应该是一个 3D cube 旋转、3D 赛车游戏和一个简单小房间 FPS 游戏
:::

同一天里：设计文档落地（每帧一次批量 `submit(buffer)`＝一次 FFI 穿越；一个纯软件参考光栅器作为 CI 的 ground truth；自带 `dsin/dcos`，因为 `Math.cos` 在 QuickJS 和浏览器之间会差 1 ULP——**确定性从第一天就是契约**）、透视 bug 排查、web 与 3DS 对齐。傍晚，"榨干硬件"的总纲领出现：

:::quote 2026-06-16 · S2 · dreamcart
你需要通过下一个 PoC 去尝试，看看这些场景到底怎么被建模，最后怎么让业务逻辑用 JS 来写。要尽可能把它走通，把硬件的机能发挥到极致，以此来做一个测试。
:::

次日是硬件蒙皮与性能战争：glTF 烘焙、`sceGuBoneMatrix` 硬件蒙皮、VRAM 预算表。当 agent 过早宣称"已经榨到极限"时，出现了整个前史最著名的一次爆发：

:::quote 2026-06-17 · S2 · dreamcart
你他妈在逗我吗？这肯定完全没有榨到极限，你认为就现在这种情况，能有什么极限？……你要不要看一看这台硬件是什么配置再说话？
:::

结果是几小时内挖出真正的瓶颈——`linked_list_allocator` 让 QuickJS 每次内存分配都要 ~1ms，换成 O(1) 分区 arena 分配器（`037281c`），再加 native 蒙皮与动画采样，狐狸模型从 **6 FPS → 30 → 60**。60fps 信仰是从一场争吵里开始的。

## 确定性成为方法论（06-19 → 06-21）

6 月 19 日有一个纯思考 session [[S5]]，预演了后来 PocketJS 的核心分工——"native 渲染，JS 编排"：

:::quote 2026-06-19 · S5 · dreamcart
我在考虑这样一个事情：基于 TypeScript 编写的游戏 Gameplay 逻辑，在 PSP 的机能限制下（64MB 内存、333MHz 处理器），使用 QuickJS 到底能支持到什么程度？……如果 Node3D 对象不由 JS 控制，而是直接在 Rust 里面管理游戏场景，JS 仅提供有限的扩展、少量的编排，或者通过某些回调来定制行为。
:::

同期的 CS 1.6 地图（BSP）导入暴露出相机移动时的闪烁、黑面 bug——而且**连复现都做不到**。作者的回应成了这个项目的认识论，也是日后《The UI Runtime That Can't Flake》的胚胎：

:::quote 2026-06-20 · S4 · dreamcart
我的预期是，尽可能快地看到在一个场景里面，PSP 的引擎能够很自信地渲染出和我们的 Ground Truth 一致的内容。然后，它在摄像机移动的时候，不会产生各种闪烁、跳变之类的渲染 bug。现在甚至都没有足够多的能力去抓取、去把这个问题复现出来，这是必须要解决的。
:::

于是有了 `bun run bsp-loop`：PPSSPP 无头截帧 vs WebGL vs CPU oracle 三方比对、IoU 打分的相机扫描、脚本化输入。靠这套回路诊断出 guard-band 细分修复。**先造能复现问题的仪器，再修问题**——这个顺序此后再未颠倒过。

## 真机回路：「千万不要做写操作」（06-20 → 07-02）

6 月 20 日，PSPLINK 真机调试链路的搭建请求发出，带着一个非常生活化的约束：

:::quote 2026-06-20 · S9 · dreamcart
我正在准备搭建索尼 PSP 真机的调试链路……但是请注意，现在不要做任何写操作。因为我的 PSP 正在作为 U盘连接备份数据，所以千万不要做写操作。
:::

然后这个 worktree 等了 **11 天**——等那台 PSP 从"备份 U 盘"的岗位上退下来。7 月 1 日深夜接上真机，一夜排错（PSPLINK 不认 EBOOT.PBP 要加载裸 .prx；rust-psp 模块不释放内存，每次 reload 前必须 `reset`；FAT32 上的 AppleDouble `._` 垃圾文件）。7 月 2 日早晨：

:::quote 2026-07-02 · S9 · dreamcart
可以了,walk3d 在跑了,你写一个面向普通开发者的使用这个工具的英语文档,然后常用命令封装一下不要让我填这么多环境变量,开 PR 提交合并
:::

这就是 dreamcart PR #37：`bun run psp:hw`，编辑 → 构建 → 真机运行一条命令，不用拔记忆棒。**真机验证成为习惯，是因为它先变成了一条命令。**

## 40 分钟后：psp-ui 的出生证明（07-02 08:45 UTC）

真机回路 merge 后 40 分钟，同一个 worktree 里，作者敲下了那段日后成为 PocketJS 的话——一条消息里包含了全部承重决策：

:::quote 2026-07-02 · S10/S11 · dreamcart
ultracode 现在我 PSP 真机环境搭建好了，也有 PPSSPP，我想做一个极致的 JS 跨平台 UI 技术栈，参考现有的接入方式，利用 rust 平台 UI 库 + JSX + 自建 tailwind 子集编译器，然后 JS 引擎用 QuickJS，前端框架一定是需要一个流行的，至少是兼容 JSX，也可以考虑（但我不确定）类似 solid 或 vue 或 react 的方案，也需要支持流畅动画，文字可以烘焙，最后要有一些适合做 landing page 展示的酷炫 demo 基于这套 UI 引擎实现出来（并端到端测通）。可以在 dreamcart 下的一个独立的 psp-ui 目录从头写所有的代码，不需要复用现有的 dreamcart 面向游戏场景的建模，因为后续会迁出独立仓库维护。
:::

注意最后一句：**「后续会迁出独立仓库维护」——抽取是预谋的，写在出生证明上。**

Agent 做了框架选型对比（Solid universal renderer vs Preact + DOM shim vs Vue 自定义渲染器），结论是 Solid：无 VDOM，信号更新只跑一个微小 effect、正好对应一次 `ui.setStyle`——这对 333MHz 的解释器是本质优势。约 13 小时后，PR #38 合入：**94 个文件，+14,778 行**。no_std Rust core（节点 arena + taffy flexbox + 固定步长 tween），一份核心编译两次（PSP EBOOT 与 wasm 软光栅），Tailwind 子集编译器把 class 字面量在编译期变成二进制样式表，wasm 与 PPSSPP 双侧 byte-exact golden，`0:0,58:0x40,62:0` 这样的帧输入 tape——PocketJS input tape 的直系祖先。合并前还跑了一轮对抗式 review，当天修掉 22 个确认缺陷。

Merge 前五分钟，条件反射再次出现：「几个 demo 给我打包出 eboot pbp 然后我要真机测」。

## 模拟器说了谎：抽取前夜（07-02 深夜 → 07-03）

psp-ui 第一次跑上真机：**黑屏**。PPSSPP 从未暴露过这个问题——真机上 PSPLINK 拉起的用户线程可能带着 FPU invalid-operation 陷阱启动，而 taffy 的 flexbox 故意用 NaN 作为 auto 尺寸的哨兵值，第一轮布局就触发浮点异常。修复是**一条 MIPS 指令**：`ctc1 $zero, $31`（清 FCSR，让 NaN 照常传播）。因为黑屏没有任何日志，这个 session 还发明了 host0: trace 文件。这就是 dreamcart PR #39，也是"模拟器会说谎，真机才算数"教义的定案时刻。

同一晚还有两件小事，说明这套栈开始有品味而不只是能力：确定键全线改成圈（「确定键应该是"圈"，而不是"叉"」——日版硬件的文化本能，引擎级改动 + 全量 golden 重录）；以及凌晨两点多的五轮 XMB 封面美术指导（「不要 dark mode 感觉,要 light mode,再改一波,但现在方向非常对」）。前史的最后一条消息，07-03 02:51：**「连上了」**。

7 月 3 日 15:32，`17fe56b` "Remove extracted PocketJS UI stack" 从 dreamcart 里删掉 psp-ui——**PocketJS 这个名字在历史记录中的第一次出现**，就在这条移除 commit 里。psp-ui 在 dreamcart 主干上的一生，只有 17 个小时。

## 本章时间表

| 时间 | 事件 |
|---|---|
| 2021-09-27 | `7b03993` psp-js PoC：QuickJS + rust-psp |
| 2026-06-15 | 复活：web playground、3DS、同构框架 |
| 2026-06-16 | 改名 DreamCart；3D 同构开题 [[S2]]；确定性数学入约 |
| 2026-06-17 | 「你他妈在逗我吗？」→ O(1) arena 分配器；Fox 6→60 FPS |
| 2026-06-20 | 闪烁宣言 → ground-truth 比对回路；PSPLINK 请求（「千万不要做写操作」） |
| 2026-07-02 07:50 | 「可以了,walk3d 在跑了」→ PR #37 `bun run psp:hw` |
| 2026-07-02 08:45 | **psp-ui 出生证明**（40 分钟后） |
| 2026-07-02 22:03 | PR #38 psp-ui 合入：94 文件 +14,778 行 |
| 2026-07-03 00:23 | PR #39 真机黑屏修复：`ctc1 $zero, $31` |
| 2026-07-03 15:32 | psp-ui 移出 dreamcart，"PocketJS" 首次见于史册 |

**下一章**，独立仓库的第一天：抽取本身没有留下任何 Claude 转录——它是怎么被重建出来的，以及 Day 1 的四个 PR 里都有什么。
