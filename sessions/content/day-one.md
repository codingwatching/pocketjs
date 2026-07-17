# Day 1 · 抽取（2026-07-03）

> **舞台** `~/code/pocketjs` 诞生 · **史料** [[S19]] [[S20]] [[S21]] + 两个仓库的 git 历史 · 本章时间为北京时间（UTC+8），引文时间戳保留 UTC

## 出境签证早已写好

抽取不是心血来潮。psp-ui 落进 dreamcart 的那天，DESIGN.md 里就写着自己的出境签证：

> "It lives in `psp-ui/` and deliberately shares no code with the dreamcart game framework — **it will be extracted into its own repository later.**"

7 月 3 日 15:28，`fdfdebc` "Extract PSP UI into PocketJS" 作为**无父 commit** 创建了 `~/code/pocketjs`。四分半钟后，dreamcart 侧的 `17fe56b` 删掉了原目录（94 个文件，−15,186 行）。一出一进，手起刀落。

一个档案学上的趣事：**这个项目最具象征意义的一刻没有任何 Claude 转录。** 那天下午 dreamcart worktree 里唯一的 Claude 活动是 [[S20]]——一个只有一条 `/clear` 的空 session。Day 1 的四个 PR 全部来自 `codex/*` 分支（作者同时在用 Codex CLI 干活），每个 PR 从创建到 merge 只隔十几秒：**PR 在这一天是 changelog，不是评审场**。历史的铰链时刻，未必有旁白。

还有一层错位：仓库叫 `pocketjs`、commit 写着 "into PocketJS"、pocketjs.dev 域名已在 Cloudflare 配好——但箱子里的每个文件都还写着 psp-ui。**Day 1 的仓库名是一张远期支票**，当晚才由 314 处替换兑现。

## 箱子里有什么：129 个文件，16,704 行

这不是种子，是移植。抽取当天的库存：

| 子系统 | 规模 | 内容 |
|---|---|---|
| `core/` | ~4,230 行 Rust | no_std UI 核心：节点树、taffy flexbox、样式、动画、DrawList；单测 1,086 行 |
| `native/` | ~2,100 行 Rust | PSP 侧：QuickJS 桥、sceGu GE 后端、arena 分配器 |
| `wasm/` | ~620 行 Rust | 同一核心的浏览器编译目标：软件光栅器 |
| `compiler/` | ~1,400 行 TS | Tailwind 子集编译器、字体图集烘焙、pak 打包、Solid babel 插件 |
| `src/` | ~1,255 行 TS | JS 运行时：renderer、input、host、anim |
| `spec/` | ~980 行 TS | **单一事实源 op 契约** + Rust 代码生成 |
| `test/` | ~1,830 行 + 45 PNG | wasm byte-exact golden + PPSSPP 截图 golden（锁定模拟器 commit） |
| `demos/` | 7 个应用 | hero、cards、library、music、notifications、settings、stats |

五层语言栈（TSX 应用 → 编译管线 → JS 运行时 → op 契约 → 双后端 Rust 核心），80+ 测试全绿，双宿主 golden 验证——**抽取出来的是一支带弹药的军队**。

当晚三个 Codex PR 快速加固：[[PR1]] demo 打磨与 SVG spinner 烘焙、[[PR2]] 第一个公开 primitives API、[[PR3]] 把巨石 `index.ts` 拆成 `animation/components/hooks/input` 等作用域模块——今天 `@pocketjs/framework/*` 子路径布局的直系祖先。凌晨 01:38，[[PR4]] 带来 overlay/toast/dialog 原语和一个新 demo：`demos/launcher`——几小时后它将成为官网首页的主角。

## 01:43 · S21：「ultracode」与 pocketjs.dev 的诞生

PR #4 合并五分钟后,作者在同一个 worktree（还挂在 `codex/refactor-app-shell` 分支上）开了本仓库的第一个 Claude session，指令直接指向更大的东西：

:::quote 2026-07-03T17:43Z · S21
ultracode 目前这个项目已经初步成型，获得了一个 PSP 上面的完整 UI 框架渲染能力……你可以切到我的 doodlewind@gmail.com 账号。然后用这个账号应该可以操作 pocketjs.dev 这个站点，然后部署 worker 上去 我需要你给这整个站点完成以下工作：
1. 设计好首页并完成完整部署
2. 编写所有的技术文档
3. 提供一个 playground 能够预览已经写好的 demo
关于这些 demo，应该类似于 Tiptap 那样，要配有代码编辑器并支持实时预览，而不是直接把纯代码文本贴上去。
:::

这个 session 跑了 17.5 小时、828 条 assistant 消息，是 agent 拆解风格的标本（详见[《Agent 如何拆解与推进》](/agent/)），骨架是**风险优先**：

1. **先侦察后承诺**——读完设计文档与构建管线后，agent 自己点出命门："live editing 的关键在于两段式构建（Babel → Tailwind 编译 → 字体烘焙 → pak 打包 → bundle）能否整个跑进浏览器。"
2. **决策门**——动手前用一个问题锁定品牌：「站点和文档用什么品牌名？(仓库是 psp-ui，域名是 pocketjs.dev)」。作者拍板 **PocketJS**，并顺势加码：psp-ui 要从仓库里完全移除。
3. **先拆雷再施工**——真实 `@babel/core` + `babel-preset-solid` 的浏览器打包 spike 失败两次（CJS/ESM 边界），精确诊断后用 shim 修通："**Critical risk retired.**" 然后才建 6 项任务清单：rename → playground 引擎 → 首页 → 文档 → 站点生成器 → 部署。
4. **rename 用可评审的脚本而不是一堆 sed**——65 个文件 314 处替换，随后 `bun run test`（80 绿）+ tsc + wasm 重建三重验证。
5. **13 个并行 agent 写文档**，每个都从真实源码取材——13 页文档约 4 分钟产出，中途还自己纠正了 `spring()` 的签名。
6. **验证不靠感觉**——agent 写了 `site/verify.ts`：headless Chrome 加载每一页、捕获 console 错误、**探测 canvas 是否有非空白像素**、截图。首轮结果："浏览器内编译成功（23 styles · 4 atlases · 85ms），wasm 循环 60fps，canvas 100% 非空白，0 错误。"
7. **分级部署**——dry-run → workers.dev 冒烟 → 挂 pocketjs.dev 域名 → 线上复验。
8. **诚实的收尾**——报告里明说：没有提交任何代码（"你没让我提交"）；EBOOT 原生路径无法在此环境编译验证，**真机复验是未偿债务**。

## 03:00 · 定名之夜：「Bare Metal Modern Web」

站点上线后，作者花了 90 分钟做定位——一场公开进行的第一性原理拷问：

:::quote 2026-07-03T18:55Z → 19:10Z · S21
Pocket 这种设备本质上是什么？为什么以前 Modern Web 进不来呢？这个我好像还没有从根源上理解。……实际上，这一切是要从 PSP 这个级别的东西开始，也就是从 32MB 内存开始，才能享受 Modern Web。在那之前其实也有很多像 Game Boy、GBA 这种设备，它其实是没法 Modern Web 的。这个边界在哪里？…我需要你从第一性原理出发想一想，然后给我一些你的建议。
:::

六个候选口号被逐一否决（"Too small for browser, not too small"——「要绕一点才能理解到，还差一点」），然后：

:::quote 2026-07-03T19:13Z · S21
Bare Metal Modern Web 这个如何
:::

两条消息内定案，随即立法拼写：「中间不要连字符」「Bare Metal Modern Web 我需要的是这么拼写」。今天还挂在 pocketjs.dev 上的口号，锁定于第一晚的凌晨三点。同一口气里还有行文审查：「那个 to the GE，这个 GE 的缩写没人看得懂……然后最后也不是 build for PSP 啊，这个能不能格局稍微大一点？」

## 04:00 · 「what」

当晚的 boss 战是首页的 PSP 机身外框。作者要求真实的 PSP 轮廓（「这台机器不是圆角矩形,它的左右是两个完全的弧边……左上角、右上角各自有一个缺口」），agent 专门跑了一个 workflow 去画。作者对结果的完整评审，附两张截图：

:::quote 2026-07-03T20:29Z · S21
what
:::

几小时后作者给出务实的撤退令：放弃拟物外壳，改用模拟器式虚拟按键覆盖层。（这场外壳之战余波两周——直到 [[PR116]]/[[PR117]] 才真正把 L/R 肩键"坐"进机身剪影。）session 一路滚进 Day 2：并行 landing 变体、触发限流后一个「继续」、最后一锤定音——「cinematic-bold 改成这个方案 开 PR 完整合并进去，原版的不要留着，彻底地一次性做干净」。

期间 main 在分支底下移动了（PR #7 抢先改了 `@pocketjs/*` 命名），agent 没有悄悄 rebase，而是把冲突亮成一个明确的问题请作者裁决——作者选择「对齐 main，psp-ui 彻底移除」。

## Day 1 收盘：家底与债务

**已验证**：7 个 demo + launcher 在 PPSSPP 与 wasm 双侧 golden 全绿；80 测试通过；浏览器内实时重编译 playground（真编译器，85ms）；pocketjs.dev 上线，13 页从源码生成的文档。

**未兑现**：改名后的 EBOOT 原生路径尚未真机验证（「连上了”属于旧名字，新名字还没挣到它）；npm 包、版本号、CI——都还不存在。这些债，会在接下来四天里逐一清偿。

**下一章**，Day 2：三次包名更迭、品牌资产、pocket-stack 组织迁移——一个框架获得身份的一天。
