# Day 4–5 · 首个 release（2026-07-06 → 07-08）

> **37 小时内三个 npm 版本**：v0.2.0（07-07 20:59）→ v0.2.1（21:39）→ v0.3.0（07-08 09:53） · **史料** [[S39]] [[S41]] [[S42]] + PR #53–#68 · 本章时间为北京时间

## 发布始于一条推文

7 月 6 日 09:24，大爆炸的硝烟未散，作者开的不是代码 session，而是发行 session：

:::quote 2026-07-06 09:24 · S39
基于现在 PocketJS 的现状，我准备发 Twitter 的第一条推。开头应该叫"Introducing PocketJS"，给我一些你觉得适合第一条推的备选项。请你提炼一下这个项目最激动人心的一些地方，直接把这些内容写在第一条推里面。这条推文用英文写。
:::

agent 先读 README 和 landing 文案，蒸馏出四条**可安全声明**的事实（2004 年的 PSP 上 8MB 内 60 FPS；用的是 npm 上的真 `solid-js` 不是 fork；QuickJS 组件驱动 no_std Rust 核心；同一核心跑真机/PPSSPP/wasm/Bun），再给出五个带字数统计的推文选项。11 分钟后，推文升级成基础设施：「官网上面需要有一个 blog 的入口，其中第一篇……需要你把你理解的、它在技术上到底有哪些前无古人的创新点都写出来」——注意这个句式：**让 agent 自己陈述什么是前无古人**，作者把它当合著者而非速记员。[[PR53]] 当日合入，agent 在汇报完成前先验证了线上 URL 渲染出署名 Yifeng "Evan" Wang。changelog 后来把这一天追认为 **0.1.0 "Initial public release"**。

## 「顺便一起加一下」：一句话立项的 npm 管线

7 月 7 日白天在做动画引擎（yui540 烘焙时间轴，[[PR55]]）。20:24，作者在审动画博客标题时顺手定下一条编辑方针——「我不想每篇博客都像第一次 launch 的推文，整个博客目录一眼扫下来应该更体现出一些项目在连续实现技术突破的感觉」。20:45，第二次改标题的**同一句话里**，整个发布工程被当作从句立项：

:::quote 2026-07-07 20:45 · S41
这个博客标题也差一点，至少要体现出 PocketJS 这个关键词吧。然后首页 nav 这里要隐藏 3D 这个入口，再加一个 Changelog 部分，包的发布链路也 npm login 好了你处理一下（肯定是需要 github action 自动化的），然后我希望有个类似 react native 或 flutter 的 CLI 方便本地配好工具链环境的 CLI 可以顺便一起加一下。
:::

「顺便一起加一下」就是 `@pocketjs/cli` 的全部需求文档。**作者给的是要什么，agent 设计了其余一切**（[[PR61]]，20:57 开、20:59 合）：`/changelog/` 页面并**追认 0.1.0/0.2.0 两个历史条目**；`@pocketjs/framework` 的发布准备（MIT LICENSE、`files` 白名单产出 1.1MB/86 文件的 tarball，刻意把 CI 构建的 wasm 和三个 Rust crate 源码都装进去，让 `psp`/`hw` 命令能从 tarball 直接工作）；零依赖的 `@pocketjs/cli`，`pocketjs doctor` 对每个缺失项都给一行具体修复命令；以及 `release.yml`——推 `v*` tag → 测试 → wasm 构建 → 双包 `npm publish --provenance`，**并跳过 registry 上已存在的版本**（这个跳过守卫让当晚的多次重跑全部安全）。

**为什么首个 tag 是 v0.2.0 而不是 v0.1.0？** 没有任何一条消息争论过版本号——理由是结构性的：`package.json` 从出生 commit 起就写着 0.1.0；changelog 把 7 月 6 日的站点发布追认为 0.1.0；动画引擎显然值一个 minor，所以 npm 首发 0.2.0——"**The animation engine.** The Tailwind style table learned motion."。agent 打 tag 时甚至发现一个**指向 Pocket3D 合并的陈旧 v0.2.0 tag**，查证无任何 release/workflow 消费过之后安全地强制移动了它。

## 40 分钟 2FA 拉锯：v0.2.0 是手工发布的

tag 触发 workflow，publish 失败。接下来是 40 分钟的四轮二重奏——**一个能读 CI 日志的 agent，和一个握着验证器的人**：

- **EOTP**——npm 账号 2FA 是 auth-and-writes，CI 无法提供动态口令。「创建绕过 OTP 的 token 需要你的验证器,我无法代办。两个包目前都还是 404,没有部分发布。」
- 作者配了 token：「配了，你重试一下」→ **E404**。agent 从作者的**截图**里找出 npm 自己 UI 的陷阱：「你勾的 pocketjs 在 'Organizations' 区块下——那是组织管理权限…往上滚,顶部那个显示 'No access' 的是 'Packages and scopes' 区块,它才控制发包」。
- 作者撞上 npm 新政的 90 天 token 上限（「怎么只能用 90 天啊 那我怎么发包」），agent 重构问题：90 天无所谓，因为这只是 **bootstrap**——「首发之后切到 Trusted Publishing(OIDC)，以后 CI 完全不需要 token…这个 90 天 token 过期就过期,再也不用管。」
- 再试两轮仍 EOTP（账号级 2FA 连 granular token 也拦）。agent 切到 B 计划：让**作者本人**在 session 里执行 `! npm publish --access public --otp=你的六位码`（还贴心提示「两条命令用同一个码可能超时,第二条换新码」）。`!` 前缀意味着：人执行，agent 永远不碰 OTP。

21:16，双包上线。所以历史事实是：**v0.2.0——这个庆祝自动化管线的版本——最终是作者本人在本地用动态口令手工发布的**，没有 provenance（「0.2.0 这一版少个 provenance 标记,无实际影响」）。agent 随即逐字段口述 npm Trusted Publisher 表单，[[PR62]] 切换到 OIDC 免 token 发布，[[PR63]] 顺手把 LICENSE 署名修正（并做了比例判断：已发布的 tarball 不值得为此重发）。

## v0.2.1：一个"验证发布"

40 分钟后的 v0.2.1（[[PR64]]）不是 hotfix。表面内容是 README 徽章和安装文档，**声明的目的却是验证**——agent 原话：「Tagging `v0.2.1` after merge doubles as the first end-to-end verification of the trusted-publishing (OIDC) pipeline——0.2.0 是本地 OTP bootstrap 发布的，自动化路径还没证明过自己。」跑完之后：

:::agent v0.2.1 发布验证 · S41
发布验证(核心) — v0.2.1 标签触发的 release 工作流全绿:测试 → wasm 构建 → 两个包均通过 OIDC 免 token 发布成功…已上线且带 provenance 证明(dist.attestations.url 均存在)。自动化链路从此被真实验证过。
:::

验证手段全是**摸产物而不是看流水线**：后台 `gh run watch`、`npm view` 查 attestation URL、在干净目录里 smoke install 双包并跑 `pocketjs doctor`、检查 tarball 里的 wasm 与 LICENSE——然后做卫生学收尾：**删掉已经无用的 NPM_TOKEN secret**（「仓库里少一个活凭据」）。

## 22:52 · 两次发布之间的哲学夜谈，选出了 v0.3.0

:::quote 2026-07-07 22:52 · S41
另外一个命题是：之前前端很流行"时间旅行"（Time Travel）的调试，但那只是 Redux 时代的东西，针对的还只是非常少的状态。在现在这种可控性更高、管理更多内容的架构下面，有什么更高级的时间旅行调试能力可以被完整实现呢？
:::

agent 的回答给整个特性定了纲："**Redux 时代的时间旅行是'录状态'，你的架构可以'录宇宙'——因为整个世界是封闭的**"——固定 dt、帧是纯函数、资产全烘焙、没有墙钟：一场会话不是内存快照，而是一条**输入磁带**，「PSP 每帧输入就是一个按键位掩码,一场十分钟的会话 ≈ 70KB」。作者用三连问收口（最难的是什么/最兴奋的是什么/最空白的是什么），agent 两次都选时间旅行，动机自陈得很坦白：「私心很直白:它升级的是我自己的工作条件…**确定性复现是把 agent 调试从'猜测'变成'必然'的那个开关**」「独占的空白,才是最值得占的」。

00:39 的正式立项书里，作者两次要求 agent **为自己而建**：

:::quote 2026-07-08 00:39 · S42
基于前面的讨论，完整地去设计实现时间旅行和整套调试工具链。它应该是一个能在嵌入式 JS UI 领域对标 React DevTools 的东西……它们都应该变成 PocketJS 框架一等公民的基建。……你应该在这个问题空间里面，去做那些最能够满足你需要的功能。
:::

四个并行侦察子 agent 在 7 分钟内交回地图，其中的"决定性事实"直接变成设计：输入本来就是每帧一个 int（磁带＝Uint16 数组，「永远在录的黑匣子」）；高亮必须画在核心的绘制遍历里，三个后端免费共享；PSP 的 JS 没有文件 IO 和 console，调试通道走 usbhostfs 上的 `host0:` JSONL 信箱——顺便让 PSP「第一次拥有 console.log」。spec ops 18–22 全部 debug-only、默认关闭，35/35 像素 golden 不变证明发布渲染 byte-identical。

## 07-08 早晨 · 90 分钟压缩一个产品周期

05:37 真机联调（「device is talking」——信箱在真硬件上通了）；08:24 第二轮五条指令（一行命令的 DX「参考一下现在最丝滑、最有名的那些前端命令行工具」→ vite 风格 banner「⚡ Pocket DevTools ready in 21 ms」；09:07「cli 命令从 pocketjs 改为 pocket 吧，这个不需要考虑向后兼容」）。然后：

- **09:23**「我刚刚点了 screenshot 之后，真机就卡死了」→ agent 的法医式排查：`shot.raw` 0 字节、心跳冻在 870 帧；读本地 psplinkusb 驱动源码，找到根因——**把 VRAM 非缓存镜像地址直接喂给了 usbhostfs**，其发送路径的 dcache 回写 + USB bulk DMA 都不能作用于 VRAM 地址，第一个 64KB 块就把 JS 线程挂死在 `sceIoWrite` 里。修复：32KB RAM 弹跳缓冲、17 次分块写。顺藤摸出第二个 bug（GE 写 alpha=0，PNG 全透明）和第三个（Solid bundle 里根本没有 `console` 对象）。
- **09:39** 发布令 + release skill 的诞生：

:::quote 2026-07-08 09:39 · S42
确认修复了，然后代码合并进去，都部署上去吧 然后能发布一个 0.3，也就一起发布吧。Changelogs 什么的你也注意一下。这个发布应该也抽象成一个 skill。等这次发布结束，你可以再开一个 PR 把它总结提炼出来，然后提交进去。这些都是你自己开 PR 合并到主干就可以
:::

最后一句成为后续 session 反复引用的**常设自合并授权**。09:53 [[PR65]] squash 合入（5000+ 行、64 文件），v0.3.0 tag → OIDC 发布全绿；09:55 [[PR67]] 把这次实操蒸馏成 `skills/pocketjs-release/SKILL.md`（「验证产物而非工作流」清单、squash-merge worktree 陷阱、changelog 条目是发布的承重输入）；10:09 [[PR68]] 补文档——agent 主动审计发现 0.2 的动画特性在参考文档里**命中数为零**。每道伤疤都在结痂前变成了教义。

## 首个 release，结账

从 `fdfdebc` 到 v0.2.0 上 npm：**4 天 5 小时**。从 2021 年那个 PoC 算起：4 年 9 个月。三个版本的 37 小时里，napm 管线从不存在到被真实验证、devtools 从一场夜谈变成框架原语、发布本身变成一个可复用的 skill。

**下一章剖析**这两个主角各自的方法：[作者如何思考与 steering](/steering/)，以及 [Agent 如何拆解与推进](/agent/)。
