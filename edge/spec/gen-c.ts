// edge/spec/gen-c.ts — mirror spec/edge.ts into runtime/edge_gen.h.
//   bun edge/spec/gen-c.ts
import * as S from "./edge.ts";

const lines: string[] = [
  "/* edge_gen.h — GENERATED from edge/spec/edge.ts. Do not edit. */",
  "#ifndef EDGE_GEN_H",
  "#define EDGE_GEN_H",
];

const def = (name: string, v: number): void => {
  lines.push(`#define ${name} ${v}`);
};

for (const [k, v] of Object.entries(S)) {
  if (typeof v === "number") def(`C_${k}`, v);
}
for (const [k, v] of Object.entries(S.OP)) def(`OP_${k}`, v);
for (const [k, v] of Object.entries(S.TW)) def(`TW_${k}`, v);
for (const [k, v] of Object.entries(S.WAITING)) def(`WAITING_${k}`, v);
def("DBG_MAGIC_VAL", S.DBG_MAGIC);
for (const [k, v] of Object.entries(S.DBG)) def(`DBGO_${k}`, v);

lines.push("#endif", "");
await Bun.write(new URL("../runtime/edge_gen.h", import.meta.url).pathname, lines.join("\n"));
console.log("wrote runtime/edge_gen.h");
