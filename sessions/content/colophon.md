# 方法论 · 本站如何生成

这座 wiki 的一手史料是 Claude Code 的 session 存档：`~/.claude/projects/` 下按项目目录存放的 JSONL 转录，每行一个事件——用户消息、assistant 消息、工具调用、时间戳、git 分支。PocketJS 相关的目录共 37 个（主仓库 + 各 worktree + 前身 dreamcart），69 个 session，原始体积约 540 MB。

## 抽取管线

1. 一个 Bun 脚本扫描全部 JSONL，抽出每个 session 的：起止时间、工作目录、分支、**全部人类消息原文**（过滤掉斜杠命令与命令回显）、agent 的 ExitPlanMode 计划书、各工具调用次数。长消息在 2500 字符处截断。
2. `gh pr list` 与两个仓库的 `git log` 提供 PR / commit / tag 的时间骨架。
3. 八个并行研究 agent 各领一段时期或一个主题（前史、抽取日、命名日、大爆炸、首个 release、steering 模式、agent 行为模式、后记），深读对应的 session 与 git 历史，产出带原文引用的研究笔记。
4. 章节由这些笔记综合写成；[时间线](/timeline/)、[档案馆](/sessions/)、[数字全景](/numbers/)三页由构建脚本直接从数据生成——`sessions/build.ts` 每次构建时重新计算所有数字。

## 编辑原则

- **引文一律原文**，包括语气词和错别字；只做截断（以 … 标注）。中文原话不翻译。
- 叙事里的每个事实——时间、PR 号、行数、帧率——都能回溯到 session 原文或 git 对象；写不进引用的推测会被明确标成推测。
- Session 档案页展示的是**作者侧**的指令流与 agent 的计划书。agent 的逐条执行过程（数十万次工具调用）不逐页展示，只在[剖析](/agent/)章以样本呈现。
- 时间戳均为 UTC。作者在东八区，所以 UTC 20:00–22:00 的 session 其实是清晨——"凌晨两点还在改 XMB 封面"这类判断已按时区换算。

## 局限

- 抽取日（7 月 3 日）本身没有 Claude 转录——那几个 PR 走的是 `codex/*` 分支，只能靠 git 侧证据重建。
- 转录截断意味着极长的粘贴（日志、报错）不完整；子 agent（sidechain）的内部对话未计入人类消息。
- 这里只覆盖 PocketJS 主仓库与 dreamcart 前史；后来独立出去的 open-strike、pocket-youtube、pocket-shell 等仓库的 session 未收录，只在[后记](/epilogue/)里带过。

## 自指

本站自己也是档案的一部分：[S68](/sessions/068/) 收录的唯一一条指令，就是委托建造这座 wiki 的那句话。研究、抽取、写作、建站、预览，都发生在那个 session 里——你现在读到的一切，是那条指令的输出。
