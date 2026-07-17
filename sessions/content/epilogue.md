# 后记 · v0.3.0 → v0.5.0（2026-07-08 → 07-17）

> 十天、三个版本、五个新仓库、七篇博客。首个 release 之后，PocketJS 不再是"正在被造的框架"，而变成了**上面正在长应用的平台**——而每个应用都是被刻意挑选来逼出一项新框架能力的。本章是浓缩时间线；细节请循 session 链接进档案馆。

## 第一周：证明与宣言（07-08 → 07-11）

**07-08 · OpenStrike 登上真机。** [[S43]]（46.6MB）给 Pocket3D 装上 PSP 后端：cooked `.p3d` 世界、sceGu 渲染器、PVS 剔除（[[PR69]]/[[PR70]]）。一个 CS 形状的 FPS 带着 JSX HUD 在 2004 年的掌机上锁 60 FPS。当晚博客《Shipping OpenStrike》（[[PR72]]）发布。

**07-09 · 确定性宣言。** 同一个 session 的次日早晨：**虚拟时钟**——`simulationHz`、帧边缘 effect、无头确定性 sim host（[[PR75]]）。作者交出一份私人架构笔记并委托旗舰文章：「这篇更可能是一篇开宗明义提出 ui=f(state) 或者 redux time travel 级别的宏观技术路线理念的文章……充分发挥你的才能，创造出这篇文章吧」。文章以《Time Is an Input》发布（[[PR76]]），数小时内改题为**《The UI Runtime That Can't Flake》**（[[PR77]]）——标题、结尾、URL 全部在线迭代。同期的暗面：[[S44]] 的 NEON RUSH 移植后来被发现分支是空指针，**代码永久丢失**——这个节奏罕见的伤亡。

**07-09–11 · Pocket Figma。** [[S47]]：一晚上的 prompt——把一个真实的 14,430 节点 Figma 社区文件编译期烘焙到能在 PSP 上流畅平移缩放——产出 `.fig` cooker、TILESET/CLUT8 流式纹理、跨三后端的 `<DeepZoom>` 组件（[[PR81]]）。同一个 session 里颁布了**家族命名法令**（pocket-figma、pocket-notion、pocket-linear……），并筹划了 Show HN 的北京时间投放窗口。

**07-10–11 · GameBlocks playset。** [[S51]]：`scene3d` surface + 由 ~10 个并行 agent 移植的 74 个 TypeScript 模块（[[PR85]]）——扇出途中撞上组织 API 月度上限，agent 下个 session 被逐一唤醒续命。"波次"模式的极端形态。

## 第二周：平台版本与系统软件（07-12 → 07-17）

**07-12 · 四线并行日。** PocketShell（[[S53]]，sheru 的 Win98/XP/Aqua 主题上 PSP，引擎收获 bevel [[PR93]] 与 `active:` 按压态 [[PR94]]，还有只在真机出现的 GE 纹理缓存 gotcha）；Pocket Talk IM demo（[[S55]]，「覆盖一个除了网络连接是 mock 以外的 IM 应用应该具备的所有能力」→ [[PR95]]，其间的换行成本讨论直接催生 capability 思想，作者当晚授权 agent 自审自合 [[PR98]] 平台契约与 [[PR92]] PS Vita 原生宿主）；paperworld（[[S54]]，把 can't-flake 宣言当作别人的地基）；以及首页改版（[[PR100]]）。

**07-13 · v0.4.0 与 can't-flake 的反讽。** Vita 拿到原生密度渲染（960×544 @2x、触摸，[[PR99]]），博客标题被否决式命名（「不对，我更喜欢 Twice the Pixels, Zero Forks」）。然后是戏剧性一幕：**宣言是"不许 flake"的框架，自己的发布管线连续两次被 CI flake 拦下**——npm 12 改了 `pack --json` 输出形状、bun 5 秒默认超时杀死慢 CI 上的确定性测试套件。回应是纯教义式的：修闸门（[[PR107]]/[[PR108]]）、发布自动建 GitHub Release（[[PR109]]）、**重打 tag 作为正确的重试**。同日 [[PR106]] input.cursor（opt-in 虚拟指针 capability）从「pocket shell 目前的按键操作太不方便了」里诞生。

**07-16–17 · Pocket YouTube 与 v0.5.0。** [[S64]]+[[S66]]（合计 50MB）：「实现一个新的 pocket-youtube 应用，支持在 PSP 上看 youtube……不需要支持 wifi，仅支持 USB 插上 Mac 时使用即可」。DevTools 信箱泛化成 `pocket-svc` 应用服务通道（ops 30–37），`.pkst` 环形文件运送 CLUT8 帧与 PCM 音频，Mac 侧 yt-dlp/ffmpeg（[[PR113]]）。真机调试找到三个模拟器永远不会暴露的 bug：GE 纹理竞态（噪点闪烁）、泄漏的音频硬件通道（无声）、以及撒谎的 stale 构建。应用抽取成独立仓库（[[PR115]]），博客带手持真机视频上线并投放 Hacker News。**在设备上找到的 DX 债务直接变成框架特性**：bundle-hash 握手 + 设备诊断计数器（[[PR118]]），LVGL 风格的**系统 OSK** 取代每个 demo 手搓的键盘（「它应该被提升成一种系统级的组件，而不是每个应用自己在 demo 里去实现」→ [[PR119]]/[[PR120]]）。**07-17，v0.5.0 打 tag**——changelog 的标题是"The console grows system software"。

同一个窗口的收束三连：[[S65]] 在做 VueConf 2026 幻灯片（讲述整段历史），[[S67]] 用 3 条人类消息近自治地建成 Pocket Static（替换最老的 aot 原型），而 [[S68]]——窗口里的最后一个 session——委托建造了你正在读的这座 wiki。**项目开始把自己的历史当作产品来做。**

## 版本节奏表

| Tag | 日期 | 主题 | 距上一版 |
|---|---|---|---|
| v0.1.0（追认） | 07-06 | Initial public release —《Introducing PocketJS》 | — |
| v0.2.0 | 07-07 | 动画引擎（烘焙关键帧时间轴）· **npm 首发** | 1 天 |
| v0.2.1 | 07-07 | OIDC 管线的验证性发布 | 40 分钟 |
| v0.3.0 | 07-08 | Pocket DevTools：时间旅行成为框架原语 | 1 天 |
| v0.4.0 | 07-13 | One app, two PlayStations：Vita + 平台契约 | 5 天 |
| v0.5.0 | 07-17 | 主机长出系统软件：svc、视频平面、OSK、光标、devStats | 4 天 |

## 复利模式：应用即强制函数

这十天的结构性故事，是"demo → 系统软件"的自动扶梯：**Figma 逼出流式纹理与 DeepZoom；Shell 逼出样式引擎与光标；IM 逼出平台契约；YouTube 逼出应用服务、视频平面与 OSK**——然后每个应用被抽取进 pocket-stack 家族，带着同一套发布装备（独立仓库、EBOOT/VPK 封面、真机视频博客、中文推文草稿、两次 Hacker News）。v0.5.0 changelog 把教义写成了一句话：这些能力"every handheld app needs, now owned by the framework instead of copy-pasted per demo"。

steering 也在上移一层：作者不再指挥代码，而在指挥标题、语气、战略（iOS/Android 的未来、Pocket Studio/Store）与物理验收——实现层则在常设授权 + golden 的护栏下自开自合 PR。

完整的逐日记录在[全量时间线](/timeline/)；每一天背后的原话在 [Session 档案馆](/sessions/)。
