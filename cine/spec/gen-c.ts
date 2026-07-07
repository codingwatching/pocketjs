// cine/spec/gen-c.ts — mirror spec/cine.ts into runtime/cine_gen.h.
//   bun cine/spec/gen-c.ts
import * as S from "./cine.ts";

const lines: string[] = [
  "/* cine_gen.h — GENERATED from cine/spec/cine.ts. Do not edit. */",
  "#ifndef CINE_GEN_H",
  "#define CINE_GEN_H",
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
await Bun.write(new URL("../runtime/cine_gen.h", import.meta.url).pathname, lines.join("\n"));
console.log("wrote runtime/cine_gen.h");
