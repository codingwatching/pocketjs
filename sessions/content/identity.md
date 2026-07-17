# Day 2 · 名字与门面（2026-07-04）

> **主题** 一个框架在 24 小时里获得名字、口号、脸面和自己的 GitHub 组织 · **史料** [[S21]] [[S23]] [[S24]] [[S25]] + PR #7–#14 · 本章时间为北京时间

[[S21]] 的马拉松从凌晨一直跑到当晚 19:15。前一章讲了它的夜场（建站、定名、「what」）；这一章讲白天与傍晚——**身份的落地是三条并行轨道完成的**：Claude 在建站，Codex 在改名，作者在两者之间做裁决。

## 命名三部曲（18:26 → 20:49）

名字本身没有经过辩论——凌晨 01:52，agent 用一个选择题问「站点和文档用什么品牌名？(仓库是 psp-ui，域名是 pocketjs.dev)」，作者的回答是这次改名的宪法文本：

:::quote 2026-07-04 01:52 · S21
PSP-UI 这个东西要完全从仓库里面移除掉，都叫 PocketJS，然后导入的时候，路径可以是 '@pocketjs/xxx' 之类
:::

真正花了一晚上的，是**包路径**。三步走，九个小时：

| 时间 | PR | 事件 |
|---|---|---|
| 18:34 | [[PR7]]（Codex） | `useFrame → onFrame`、`hooks → lifecycle`、应用侧 `@pocketjs/*` 别名 |
| 18:44 | PR #9 | `@pocketjs/framework` 首次提出——**太早，未合关闭** |
| 19:10 | [[PR10]]（Claude） | 站点 + 全量改名一次 squash 合入：`feat: pocketjs.dev site + finish removing psp-ui` |
| 20:49 | [[PR11]]（Codex） | 终局：`@pocketjs/framework` + 子路径，resolver 拒绝旧别名 |

中间的冲突时刻值得记录：Claude 的站点分支独立把一切改成了 `@pocketjs/core`，rebase 时发现 main 已经走了 `@pocketjs/*`。agent 没有悄悄挑一个，而是把它亮成显式决策：「main 已经用 @pocketjs/* 方案……跟我这个分支的 @pocketjs/core 冲突。合并前怎么对齐？」作者选择对齐 main 并彻底移除 psp-ui。最终报告：「repo-wide sweep 确认 **zero `psp-ui`** left」——同时保留了 "PSP"（主机名）本身：改的是项目名，不是历史。`@pocketjs/framework/*` 的导入契约今天仍写在仓库 CLAUDE.md 里强制执行。

## 门面：四变体设计面板与 cinematic-bold

凌晨的拟物 PSP 外框失败后（「what」），上午 08:27 作者要求系统性的替代方案——参考 `nexu-io/open-design`，多出几个完整变体，换掉「作为一个 landing page 来说非常简陋」的 ASCII 架构图。agent 开了一个 **4-agent 设计面板**，途中撞上用量上限、3/4 变体一度只返回占位 CSS 被打回重做，最终四个完整变体上线本地预览：linear-precision、cinematic-bold、swiss-vercel、editorial-magazine。18:38 作者拍板：

:::quote 2026-07-04 18:38 · S21
cinematic-bold 改成这个方案 开 PR 完整合并进去，原版的不要留着，彻底地一次性做干净
:::

「Apple/SpaceX 能量——蓝→青大标题、demo 背后的体积光、`60 FPS · 32 MB · 0 kB · 1 core` 数据带」——这套骨架就是今天 pocketjs.dev 首页的前身。

## 脸、组织、最后一个 dreamcart 基因（21:44 → 22:17）

33 分钟里三个收尾 PR：

- **[[PR12]]** 21:44——「bare metal PocketJS brand assets」：SVG favicon、README 标识、三张 1024×1024 头像导出，注明"for GitHub and X/Twitter profile use"。**框架要开社交账号了。**
- **组织迁移只花了 18 分钟**：PR #12 还是从 `doodlewind/pocketjs` 合入的；22:02 的 [[PR13]] 已经在 `pocket-stack/pocketjs` 下——pocket-stack 组织从此成为整个家族（pocket-shell、pocket-youtube、open-strike……）的户口所在地。
- **[[PR14]]** 22:17——`dcpak → pak`。dcpak 的 dc 是 **d**ream**c**art：最后一个前史基因被剪掉。同一个 PR 里还有一处措辞修正：landing 文案原本暗示 PocketJS 只是"长得像" Solid——不对，**它就是 Solid**。

## 22:50 · 三分钟三路扇出

身份落定的当晚，作者在三分钟内开出三个并行 worktree session——项目从"一场对话"变成"一组并行战线"的转折点：

:::quote 2026-07-04 22:50 · S23
ultracode 系统调研一下 PSP 硬件提供的视频解码能力……怎么样去尽可能利用它的原生能力，把这个视频实时地串流播放出来？这个应该实现成一个原生的 Video 组件，然后要有一个最简单的 pocketjs UI Demo，去端到端地测通这个播放的能力
:::

- **[[S23]] Video 组件**：research workflow 摸清 Media Engine 版图（sceMpeg vs scePsmfPlayer；每个解码调用都会阻塞内核线程，60fps 意味着专用解码线程），对抗式 review 在发货前抓住真 bug——一个取反的 `PSMF_PLAYER_CONFIG_LOOP` 会让循环播放永远挂死。次日早晨作者的验收话术成为日后的标准句式：「开 draft PR 出来 然后告诉我你现在测试了哪些东西，实际能跑通什么……离真正实用最大的卡点还有哪些？」→ draft PR #17。
- **[[S25]] 3DS 移植试探**，带着一句典型的架构式随想：「既然我都用 Tailwind，我能不能只定义一套 inline 样式，然后让它在编译期通过一些 token preset，自动去适配很多尺寸的设备？」
- **[[S24]] Gallery 夜班**（`humorous-cough`，19.4MB）：「我想要一个按 L 键和 R 键去整屏整屏切换 Gallery 的那种效果……需要完整把这个端到端都跑通」。这个 session 一直干到 7 月 5 日中午，是下一章大爆炸的第一缕火光。

## 07-05 早晨 · 飞轮闭合

[[S24]] 的尾巴上，三件事完成了身份日的收官：

**自动部署（[[PR28]]）。** 作者问 gallery 合并后会不会自动出现在线上，agent 诚实回答：不会，站点是手动部署的。作者：

:::quote 2026-07-05 10:14 · S24
那你可以把部署配好，只要合并到主干就发布吧。那个 key 什么的，你都有权限配的 弄完你自己合并进去就可以
:::

agent 发现自己的 OAuth 登录无法铸造 API token（403），把能配的都配好后，把唯一需要人手的一步递给作者；10:32 作者：「gh secret set CLOUDFLARE_API_TOKEN --repo pocket-stack/pocketjs 这个我已经搞定了，你再试试」。十分钟后全管线转绿。**从此 merge 到 main 就是发布。**

**`/goal` HUD（[[PR29]]）。** 作者用 `/goal` 斜杠命令（一个 Stop hook，条件不满足 agent 不许收工）钉死验收标准：FPS 和内存占用必须**在画布内**渲染、是 Web host 的一等公民功能。产物是 `host-web/hud.js`，在 `_blit()` 里直接画，外置 FPS 计数器全部删除。

**掌机外壳的正名（PR #30/#31/#35）。** 这次作者明确说「不需要跟 PSP 一模一样」——要的是一台**原创**的方形掌机。11 分钟出第一版；作者甩来一张横版掌机参考照 → 全面重构成两侧握把布局；然后是截图驱动的紧凑化循环，高潮是圆角之战：

:::quote 2026-07-05 12:33 · S24
这个圆角还是不一致啊。你直接就机身是多大圆角，shoulder button 就用多大圆角，完全一致，可不可以？懂不懂？
:::

agent 前两轮靠"同心圆角"猜，第三轮改为**测量 computed style**，找到真凶：`3.6cqw` 在 body 上按视口解析、在肩键上按容器解析——同一份源码，渲染出 50.4px 和 25.3px。钉死为一个共享像素变量后全部对齐 26px，12:46 作者验收：「可以，就用这个部署上去吧」。

同一个早晨还有一条立法：「你要把 PR 名字改成 conventional commit 的格式，然后也更新到你的记忆里面」——今天仓库 CLAUDE.md 里的 Conventional Commits 规则，源头就是这条消息。

## 本章要点

- **身份日是双 agent 作业**：Claude 一场马拉松建站，Codex 一串 20 秒内合并的分支做改名、头像、组织迁移；作者在中间做裁决。
- **命名的每一层都被显式治理**：项目名一句话定死，包路径三步收敛，冲突靠提问不靠默默 rebase，拼写与大小写单独立法。
- **失败是承重结构**：体育场形状的"PSP"、占位 CSS 的设计变体、`cqw` 解析基准 bug——每一个都以测量而非再猜一次收场。
- **到 07-05 12:46，飞轮闭合**：merge → CI 构建 wasm → 自动部署 pocketjs.dev。作者的角色从操作员变成验收官。

**下一章**，7 月 5 日：一天 38 个 commit，8 条战线同时开火。
