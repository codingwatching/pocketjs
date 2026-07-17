// extract-sessions.ts — distill Claude Code JSONL transcripts into a research index.
// Usage: bun extract-sessions.ts <outDir>
import { readdirSync, mkdirSync, writeFileSync, statSync } from "node:fs";
import { join, basename } from "node:path";

const HOME = process.env.HOME!;
const PROJECTS = join(HOME, ".claude", "projects");
const OUT = process.argv[2] ?? "./index";
mkdirSync(OUT, { recursive: true });

const dirs = readdirSync(PROJECTS).filter(
  (d) => d.includes("pocketjs") || d.includes("dreamcart"),
);

type Msg = { ts: string; text: string };
type SessionInfo = {
  dir: string;
  file: string;
  sessionId: string;
  cwd: string;
  branches: string[];
  firstTs: string;
  lastTs: string;
  summaries: string[];
  humanMessages: Msg[];
  plans: { ts: string; plan: string }[];
  toolCounts: Record<string, number>;
  assistantCount: number;
  slashCommands: string[];
  sizeMB: number;
};

const isNoise = (t: string) =>
  t.startsWith("<command-") ||
  t.startsWith("<local-command") ||
  t.startsWith("Caveat:") ||
  t.startsWith("<system-reminder>") ||
  t.startsWith("[Request interrupted");

const clip = (t: string, n: number) =>
  t.length > n ? t.slice(0, n) + ` …[+${t.length - n} chars]` : t;

const all: SessionInfo[] = [];

for (const d of dirs) {
  const dp = join(PROJECTS, d);
  let files: string[] = [];
  try {
    files = readdirSync(dp).filter((f) => f.endsWith(".jsonl"));
  } catch {
    continue;
  }
  for (const f of files) {
    const fp = join(dp, f);
    const sizeMB = statSync(fp).size / 1024 / 1024;
    const info: SessionInfo = {
      dir: d,
      file: f,
      sessionId: basename(f, ".jsonl"),
      cwd: "",
      branches: [],
      firstTs: "",
      lastTs: "",
      summaries: [],
      humanMessages: [],
      plans: [],
      toolCounts: {},
      assistantCount: 0,
      slashCommands: [],
      sizeMB: Math.round(sizeMB * 10) / 10,
    };
    const text = await Bun.file(fp).text();
    for (const line of text.split("\n")) {
      if (!line) continue;
      let j: any;
      try {
        j = JSON.parse(line);
      } catch {
        continue;
      }
      if (j.type === "summary" && j.summary) {
        if (!info.summaries.includes(j.summary)) info.summaries.push(j.summary);
        continue;
      }
      if (j.isSidechain) continue;
      const ts = j.timestamp;
      if (ts) {
        if (!info.firstTs || ts < info.firstTs) info.firstTs = ts;
        if (!info.lastTs || ts > info.lastTs) info.lastTs = ts;
      }
      if (j.cwd && !info.cwd) info.cwd = j.cwd;
      if (j.gitBranch && !info.branches.includes(j.gitBranch)) info.branches.push(j.gitBranch);

      if (j.type === "user" && j.message) {
        const c = j.message.content;
        let t = "";
        if (typeof c === "string") t = c;
        else if (Array.isArray(c))
          t = c
            .filter((p: any) => p.type === "text" && typeof p.text === "string")
            .map((p: any) => p.text)
            .join("\n");
        t = t.trim();
        if (!t || j.isMeta) continue;
        // slash command invocations
        const cmd = t.match(/^<command-name>(\/[\w:-]+)<\/command-name>/);
        if (cmd) {
          info.slashCommands.push(cmd[1]);
          continue;
        }
        if (isNoise(t)) continue;
        info.humanMessages.push({ ts: ts ?? "", text: clip(t, 2500) });
      } else if (j.type === "assistant" && j.message) {
        info.assistantCount++;
        const c = j.message.content;
        if (Array.isArray(c)) {
          for (const p of c) {
            if (p.type === "tool_use") {
              info.toolCounts[p.name] = (info.toolCounts[p.name] ?? 0) + 1;
              if (p.name === "ExitPlanMode" && p.input?.plan) {
                info.plans.push({ ts: ts ?? "", plan: clip(String(p.input.plan), 4000) });
              }
            }
          }
        }
      }
    }
    all.push(info);
  }
}

all.sort((a, b) => (a.firstTs < b.firstTs ? -1 : 1));
writeFileSync(join(OUT, "sessions-index.json"), JSON.stringify(all, null, 1));

// Human-readable digest: one MD per session + a rollup table.
let rollup = "| # | start | dir | branch | msgs | size | summary |\n|---|---|---|---|---|---|---|\n";
mkdirSync(join(OUT, "sessions"), { recursive: true });
all.forEach((s, i) => {
  const day = s.firstTs.slice(0, 10);
  const short = s.sessionId.slice(0, 8);
  rollup += `| ${i} | ${s.firstTs.slice(0, 16)} | ${s.dir.replace(/-Users-evan-?-?(superset-worktrees-)?/, "")} | ${s.branches.join(",")} | ${s.humanMessages.length} | ${s.sizeMB}M | ${(s.summaries[0] ?? "").slice(0, 80)} |\n`;
  let md = `# Session ${i} — ${day} — ${s.dir}\n\n`;
  md += `- file: ${s.file}\n- cwd: ${s.cwd}\n- branches: ${s.branches.join(", ")}\n- span: ${s.firstTs} → ${s.lastTs}\n- size: ${s.sizeMB} MB, assistant msgs: ${s.assistantCount}\n`;
  md += `- summaries: ${s.summaries.join(" | ")}\n`;
  md += `- slash: ${s.slashCommands.join(" ") || "(none)"}\n`;
  md += `- tools: ${Object.entries(s.toolCounts)
    .sort((a, b) => b[1] - a[1])
    .map(([k, v]) => `${k}:${v}`)
    .join(" ")}\n\n`;
  md += `## Human messages (${s.humanMessages.length})\n\n`;
  for (const m of s.humanMessages) {
    md += `### ${m.ts.slice(5, 16)}\n\n${m.text}\n\n`;
  }
  if (s.plans.length) {
    md += `## ExitPlanMode plans (${s.plans.length})\n\n`;
    for (const p of s.plans) md += `### ${p.ts.slice(5, 16)}\n\n${p.plan}\n\n`;
  }
  writeFileSync(join(OUT, "sessions", `${String(i).padStart(3, "0")}-${day}-${short}.md`), md);
});
writeFileSync(join(OUT, "rollup.md"), rollup);
console.log(`indexed ${all.length} sessions from ${dirs.length} dirs -> ${OUT}`);
