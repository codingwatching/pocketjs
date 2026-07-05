// OpenStrike gameplay config (DESIGN.md §20).
//
// Scripts are a Rust-first *extension* layer: they register data and event
// callbacks at boot; Rust compiles the data into compact runtime structs and
// calls back only on events. Changing values here changes behavior with no
// Rust edits (DESIGN.md Milestone 6). QuickJS runs this JS directly; a .ts
// source can be transpiled to this .js via `bun build` / `tsc`.

defineWeapon({
  id: "os_rifle",
  displayName: "OS Rifle",
  damage: 35,
  fireIntervalMs: 120,
  magazineSize: 30,
  reloadMs: 1800,
  range: 4096,
  spreadDegrees: 0.5,
  headshotMultiplier: 2.0,
});

defineRoundRules({
  preRoundMs: 1000,
  intermissionMs: 3000,
  playerHealth: 100,
  botHealth: 100,
});

defineBotConfig({
  moveSpeed: 160,
  aggroRange: 1200,
  fireIntervalMs: 900,
});

// Example event hooks (optional). These fire from Rust on real events.
onRoundStart(function (arg) {
  print && print("[script] round start " + JSON.stringify(arg));
});
onBotKilled(function (arg) {
  print && print("[script] bot killed");
});
