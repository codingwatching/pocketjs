//! `pocket3d-script` — the QuickJS/TypeScript **extension boundary** for
//! Pocket3D (OpenStrike DESIGN.md §20).
//!
//! ## Philosophy (DESIGN.md §20)
//!
//! Pocket3D is **Rust-first**. Scripts are a controlled extension layer, *not*
//! the simulation core. The division of labour is:
//!
//! - **Scripts own:** weapon definitions (§18), round constants (§19), bot
//!   behaviour parameters, HUD/debug text, event callbacks, and console
//!   commands.
//! - **Rust owns:** frame-critical movement, physics queries, raycasts,
//!   animation sampling, rendering, and asset loading.
//!
//! The execution model is deliberately narrow:
//!
//! 1. Scripts are loaded at boot ([`ScriptEngine::eval`] / [`ScriptEngine::load_file`]).
//! 2. Scripts *register* data (`defineWeapon`, `defineRoundRules`,
//!    `defineBotConfig`) and *register* callbacks (`on`, `onRoundStart`, …).
//! 3. Rust compiles the script-defined data into compact runtime structs
//!    ([`WeaponConfig`], [`RoundConfig`], [`BotConfig`]) that it reads directly.
//! 4. Rust invokes script callbacks **only on events** ([`ScriptEngine::fire_event`]),
//!    never on the per-frame hot path.
//!
//! This lets designers change weapon damage or round timing in TypeScript and
//! see OpenStrike behaviour change **without touching Rust gameplay code** —
//! the success condition for DESIGN.md Milestone 6.
//!
//! ## Threading
//!
//! The QuickJS [`rquickjs::Runtime`]/[`rquickjs::Context`] are **not `Send`**.
//! [`ScriptEngine`] is therefore single-threaded: it captures shared state in
//! `Rc<RefCell<_>>` and all JS entry happens through `Context::with`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use anyhow::{Context as _, Result};
use rquickjs::{CatchResultExt, Context, Ctx, Function, Object, Persistent, Runtime, Value};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Config structs (DESIGN.md §18 / §19 / §20)
// ---------------------------------------------------------------------------
//
// Every config uses `#[serde(default, rename_all = "camelCase")]`:
//   * `rename_all = "camelCase"` accepts the TypeScript-native key style
//     (`displayName`, `fireIntervalMs`, …) shown in the DESIGN.md §20 example.
//   * `default` (container-level) fills any *missing* field from the struct's
//     `Default`, so a script may specify a partial object and inherit the rest.
// Per-field `alias` also accepts the snake_case form for tolerance.

/// A hitscan weapon definition (DESIGN.md §18).
///
/// The [`Default`] is the **"OS Rifle"**, OpenStrike's v0 weapon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WeaponConfig {
    /// Stable identifier, e.g. `"os_rifle"`.
    pub id: String,
    /// Human-readable name shown in the HUD.
    #[serde(alias = "display_name")]
    pub display_name: String,
    /// Base body-shot damage.
    pub damage: f32,
    /// Cooldown between shots, in milliseconds.
    #[serde(alias = "fire_interval_ms")]
    pub fire_interval_ms: u32,
    /// Rounds per magazine.
    #[serde(alias = "magazine_size")]
    pub magazine_size: u32,
    /// Reload duration, in milliseconds.
    #[serde(alias = "reload_ms")]
    pub reload_ms: u32,
    /// Maximum trace distance, in world units.
    pub range: f32,
    /// Cone half-angle applied to each shot, in degrees.
    #[serde(alias = "spread_degrees")]
    pub spread_degrees: f32,
    /// Damage multiplier applied on head hits.
    #[serde(alias = "headshot_multiplier")]
    pub headshot_multiplier: f32,
}

impl Default for WeaponConfig {
    /// The "OS Rifle" — the initial weapon config from DESIGN.md §18.
    fn default() -> Self {
        Self {
            id: "os_rifle".to_string(),
            display_name: "OS Rifle".to_string(),
            damage: 35.0,
            fire_interval_ms: 120,
            magazine_size: 30,
            reload_ms: 1800,
            range: 4096.0,
            spread_degrees: 0.5,
            headshot_multiplier: 2.0,
        }
    }
}

/// Round-flow constants (DESIGN.md §19).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RoundConfig {
    /// `PreRound` warm-up duration before `Live`, in milliseconds.
    #[serde(alias = "pre_round_ms")]
    pub pre_round_ms: u32,
    /// How long a win/loss result is shown before restarting, in milliseconds.
    #[serde(alias = "intermission_ms")]
    pub intermission_ms: u32,
    /// Player starting health.
    #[serde(alias = "player_health")]
    pub player_health: i32,
    /// Bot starting health.
    #[serde(alias = "bot_health")]
    pub bot_health: i32,
}

impl Default for RoundConfig {
    fn default() -> Self {
        Self {
            pre_round_ms: 1000,
            intermission_ms: 3000,
            player_health: 100,
            bot_health: 100,
        }
    }
}

/// Bot behaviour parameters (DESIGN.md §20 "bot behaviour parameters").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BotConfig {
    /// Movement speed, in world units per second.
    #[serde(alias = "move_speed")]
    pub move_speed: f32,
    /// Distance at which a bot acquires the player, in world units.
    #[serde(alias = "aggro_range")]
    pub aggro_range: f32,
    /// Cooldown between bot shots, in milliseconds.
    #[serde(alias = "fire_interval_ms")]
    pub fire_interval_ms: u32,
}

impl Default for BotConfig {
    fn default() -> Self {
        // Sensible starting values for a walk-and-shoot melee/ranged bot.
        Self {
            move_speed: 250.0,
            aggro_range: 1024.0,
            fire_interval_ms: 500,
        }
    }
}

// ---------------------------------------------------------------------------
// Registry — the Rust-side store populated by the JS `define*`/`on*` globals
// ---------------------------------------------------------------------------

/// Shared, script-populated state. Registered JS closures capture an
/// `Rc<RefCell<Registry>>` and write into it synchronously (DESIGN.md §20:
/// "small synchronous mutation surface into Rust").
///
/// Config objects are stored as [`serde_json::Value`] and deserialized on read;
/// this keeps the hot JS-closure path trivial (just a JS→JSON walk) and lets a
/// malformed object fall back to [`Default`] at read time rather than throwing.
#[derive(Default)]
struct Registry {
    weapon: Option<serde_json::Value>,
    round: Option<serde_json::Value>,
    bot: Option<serde_json::Value>,
    /// Event callbacks, keyed by event name. Stored as [`Persistent`] so the
    /// `Function` survives outside the `Context::with` scope in which it was
    /// registered.
    callbacks: HashMap<String, Persistent<Function<'static>>>,
}

/// The recognised event-callback names (DESIGN.md §20 "Allowed callbacks").
/// Each is exposed as a dedicated global (e.g. `onRoundStart(fn)`) in addition
/// to the generic `on(name, fn)` form.
const CALLBACK_EVENTS: &[&str] = &[
    "onRoundStart",
    "onRoundEnd",
    "onBotKilled",
    "onPlayerKilled",
    "onTriggerEnter",
    "onConsoleCommand",
];

// ---------------------------------------------------------------------------
// ScriptEngine
// ---------------------------------------------------------------------------

/// A single-threaded QuickJS engine that hosts OpenStrike gameplay scripts.
///
/// See the crate-level docs and DESIGN.md §20 for the execution model.
pub struct ScriptEngine {
    // `Context` holds a strong ref to the runtime internally, but we keep the
    // `Runtime` alive explicitly for clarity and deterministic teardown.
    _runtime: Runtime,
    context: Context,
    registry: Rc<RefCell<Registry>>,
}

impl ScriptEngine {
    /// Boot a fresh engine and install the `define*` / `on*` globals.
    pub fn new() -> Result<Self> {
        let runtime = Runtime::new().context("create QuickJS runtime")?;
        let context = Context::full(&runtime).context("create QuickJS context")?;
        let registry = Rc::new(RefCell::new(Registry::default()));

        Self::install_globals(&context, &registry)?;

        Ok(Self {
            _runtime: runtime,
            context,
            registry,
        })
    }

    /// Register the JS globals scripts call at boot. Each closure captures a
    /// clone of the shared [`Registry`] `Rc` and mutates it synchronously.
    fn install_globals(context: &Context, registry: &Rc<RefCell<Registry>>) -> Result<()> {
        context
            .with(|ctx| -> rquickjs::Result<()> {
                let globals = ctx.globals();

                // --- data registration: define*(obj) ------------------------
                // The object is walked into a serde_json::Value and stored.
                macro_rules! define_data {
                    ($name:literal, $field:ident) => {{
                        let reg = registry.clone();
                        let f = Function::new(
                            ctx.clone(),
                            move |obj: Object<'_>| -> rquickjs::Result<()> {
                                let json = js_to_json(obj.as_value())?;
                                reg.borrow_mut().$field = Some(json);
                                Ok(())
                            },
                        )?;
                        globals.set($name, f)?;
                    }};
                }
                define_data!("defineWeapon", weapon);
                define_data!("defineRoundRules", round);
                define_data!("defineBotConfig", bot);

                // --- generic callback registration: on(name, fn) ------------
                {
                    let reg = registry.clone();
                    let f = Function::new(
                        ctx.clone(),
                        move |name: String, cb: Function<'_>| -> rquickjs::Result<()> {
                            // Derive the Ctx from the callback itself so both
                            // share one `'js` lifetime (avoids a mixed-lifetime
                            // closure signature).
                            let cctx = cb.ctx().clone();
                            reg.borrow_mut()
                                .callbacks
                                .insert(name, Persistent::save(&cctx, cb));
                            Ok(())
                        },
                    )?;
                    globals.set("on", f)?;
                }

                // --- named callback shorthands: onRoundStart(fn), … ---------
                for &event in CALLBACK_EVENTS {
                    let reg = registry.clone();
                    let key = event.to_string();
                    let f = Function::new(
                        ctx.clone(),
                        move |cb: Function<'_>| -> rquickjs::Result<()> {
                            let cctx = cb.ctx().clone();
                            reg.borrow_mut()
                                .callbacks
                                .insert(key.clone(), Persistent::save(&cctx, cb));
                            Ok(())
                        },
                    )?;
                    globals.set(event, f)?;
                }

                Ok(())
            })
            .context("install script globals")
    }

    /// Evaluate a script source string. `define*` / `on*` calls made by the
    /// script populate the internal registry.
    pub fn eval(&self, src: &str) -> Result<()> {
        self.context.with(|ctx| -> Result<()> {
            // Evaluate as an untyped Value so any completion value is accepted,
            // then discard it. `catch` surfaces JS exceptions as readable text.
            ctx.eval::<Value, _>(src)
                .catch(&ctx)
                .map_err(|e| anyhow::anyhow!("script eval error: {e}"))?;
            Ok(())
        })
    }

    /// Load and evaluate a script from a file path.
    pub fn load_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let src = std::fs::read_to_string(path)
            .with_context(|| format!("read script file {}", path.display()))?;
        self.eval(&src)
            .with_context(|| format!("evaluate script file {}", path.display()))
    }

    /// The registered [`WeaponConfig`], or the "OS Rifle" [`Default`] if no
    /// script called `defineWeapon` (or the object failed to deserialize).
    pub fn weapon(&self) -> WeaponConfig {
        read_config(&self.registry.borrow().weapon)
    }

    /// The registered [`RoundConfig`], or [`Default`] (DESIGN.md §19).
    pub fn round(&self) -> RoundConfig {
        read_config(&self.registry.borrow().round)
    }

    /// The registered [`BotConfig`], or [`Default`].
    pub fn bot(&self) -> BotConfig {
        read_config(&self.registry.borrow().bot)
    }

    /// Invoke a registered JS callback by event name, passing `json_arg` as its
    /// single argument. No-op if no callback was registered for `name`
    /// (DESIGN.md §20: Rust calls callbacks only on events).
    pub fn fire_event(&self, name: &str, json_arg: &serde_json::Value) -> Result<()> {
        // Clone the Persistent handle out so we don't hold the RefCell borrow
        // across the JS call (the callback could re-enter and register more).
        let callback = self.registry.borrow().callbacks.get(name).cloned();
        let Some(callback) = callback else {
            return Ok(());
        };

        self.context.with(|ctx| -> Result<()> {
            let func = callback
                .restore(&ctx)
                .with_context(|| format!("restore callback '{name}'"))?;
            let arg = json_to_js(&ctx, json_arg)
                .with_context(|| format!("marshal argument for '{name}'"))?;
            func.call::<_, ()>((arg,))
                .catch(&ctx)
                .map_err(|e| anyhow::anyhow!("callback '{name}' threw: {e}"))?;
            Ok(())
        })
    }
}

impl Drop for ScriptEngine {
    fn drop(&mut self) {
        // Registered callbacks are held as `Persistent<Function>`. Because the
        // native `define*`/`on*` closures stored in the runtime capture an `Rc`
        // to the `Registry`, that registry (and its persisted functions) stays
        // alive until the runtime frees those closures — i.e. *during*
        // teardown. QuickJS asserts its GC object list is empty at
        // `JS_FreeRuntime`, so we must release the persisted JS functions here,
        // while the context/runtime are still alive.
        self.registry.borrow_mut().callbacks.clear();
    }
}

/// Deserialize a stored config JSON value into `T`, falling back to `Default`
/// if it is absent or malformed.
fn read_config<T: Default + for<'de> Deserialize<'de>>(stored: &Option<serde_json::Value>) -> T {
    stored
        .as_ref()
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// JS <-> serde_json marshalling (manual walk — no `rquickjs/serde` feature)
// ---------------------------------------------------------------------------

/// Recursively convert an rquickjs [`Value`] into a [`serde_json::Value`].
///
/// Functions and other non-data values become `null`. Integers are preserved
/// as integers so `magazineSize: 30` round-trips to a JSON integer.
fn js_to_json(v: &Value<'_>) -> rquickjs::Result<serde_json::Value> {
    use serde_json::Value as J;

    if v.is_null() || v.is_undefined() {
        return Ok(J::Null);
    }
    if let Some(b) = v.as_bool() {
        return Ok(J::Bool(b));
    }
    // Prefer an exact integer, then fall back to float.
    if let Some(i) = v.as_int() {
        return Ok(J::from(i));
    }
    if let Some(f) = v.as_number() {
        return Ok(serde_json::Number::from_f64(f)
            .map(J::Number)
            .unwrap_or(J::Null));
    }
    if let Some(s) = v.as_string() {
        return Ok(J::String(s.to_string()?));
    }
    // Arrays are objects too, so test array before the generic object branch.
    if let Some(arr) = v.as_array() {
        let mut out = Vec::with_capacity(arr.len());
        for idx in 0..arr.len() {
            let item: Value = arr.get(idx)?;
            out.push(js_to_json(&item)?);
        }
        return Ok(J::Array(out));
    }
    if v.is_function() {
        // Callbacks are registered separately; not representable as JSON data.
        return Ok(J::Null);
    }
    if let Some(obj) = v.as_object() {
        let mut map = serde_json::Map::new();
        for key in obj.keys::<String>() {
            let key = key?;
            let val: Value = obj.get(&key)?;
            map.insert(key, js_to_json(&val)?);
        }
        return Ok(J::Object(map));
    }
    Ok(J::Null)
}

/// Recursively convert a [`serde_json::Value`] into an rquickjs [`Value`] for
/// use as a callback argument.
fn json_to_js<'js>(ctx: &Ctx<'js>, v: &serde_json::Value) -> rquickjs::Result<Value<'js>> {
    use serde_json::Value as J;

    let out = match v {
        J::Null => Value::new_null(ctx.clone()),
        J::Bool(b) => Value::new_bool(ctx.clone(), *b),
        J::Number(n) => {
            if let Some(i) = n.as_i64().and_then(|i| i32::try_from(i).ok()) {
                Value::new_int(ctx.clone(), i)
            } else {
                Value::new_float(ctx.clone(), n.as_f64().unwrap_or(0.0))
            }
        }
        J::String(s) => rquickjs::String::from_str(ctx.clone(), s)?.into_value(),
        J::Array(items) => {
            let arr = rquickjs::Array::new(ctx.clone())?;
            for (idx, item) in items.iter().enumerate() {
                arr.set(idx, json_to_js(ctx, item)?)?;
            }
            arr.into_value()
        }
        J::Object(fields) => {
            let obj = Object::new(ctx.clone())?;
            for (key, val) in fields {
                obj.set(key.as_str(), json_to_js(ctx, val)?)?;
            }
            obj.into_value()
        }
    };
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_os_rifle_and_round_rules() {
        // Sanity-check the DESIGN.md §18/§19 defaults.
        let w = WeaponConfig::default();
        assert_eq!(w.id, "os_rifle");
        assert_eq!(w.display_name, "OS Rifle");
        assert_eq!(w.damage, 35.0);
        assert_eq!(w.fire_interval_ms, 120);
        assert_eq!(w.magazine_size, 30);
        assert_eq!(w.reload_ms, 1800);
        assert_eq!(w.range, 4096.0);
        assert_eq!(w.spread_degrees, 0.5);
        assert_eq!(w.headshot_multiplier, 2.0);

        let r = RoundConfig::default();
        assert_eq!(r.pre_round_ms, 1000);
        assert_eq!(r.intermission_ms, 3000);
        assert_eq!(r.player_health, 100);
        assert_eq!(r.bot_health, 100);
    }

    #[test]
    fn unregistered_configs_fall_back_to_default() {
        let engine = ScriptEngine::new().unwrap();
        assert_eq!(engine.weapon(), WeaponConfig::default());
        assert_eq!(engine.round(), RoundConfig::default());
        assert_eq!(engine.bot(), BotConfig::default());
    }

    /// DESIGN.md §20 example: `defineWeapon({...})` with camelCase keys.
    #[test]
    fn define_weapon_populates_config() {
        let engine = ScriptEngine::new().unwrap();
        engine
            .eval(
                r#"
                defineWeapon({
                    id: "os_rifle",
                    displayName: "OS Rifle",
                    damage: 35,
                    fireIntervalMs: 120,
                    magazineSize: 30,
                    reloadMs: 1800,
                    range: 4096,
                    spreadDegrees: 0.5,
                    headshotMultiplier: 2.0
                });
                "#,
            )
            .unwrap();

        let w = engine.weapon();
        assert_eq!(w.damage, 35.0);
        assert_eq!(w.display_name, "OS Rifle");
        assert_eq!(w.fire_interval_ms, 120);
        assert_eq!(w.magazine_size, 30);
    }

    /// DESIGN.md §20 example: `defineRoundRules({...})`.
    #[test]
    fn define_round_rules_populates_config() {
        let engine = ScriptEngine::new().unwrap();
        engine
            .eval(
                r#"
                defineRoundRules({
                    preRoundMs: 1000,
                    intermissionMs: 3000,
                    playerHealth: 100,
                    botHealth: 100
                });
                "#,
            )
            .unwrap();

        assert_eq!(engine.round().intermission_ms, 3000);
    }

    /// DESIGN.md Milestone 6 success condition: changing a value in the script
    /// changes engine behaviour with no Rust changes.
    #[test]
    fn script_driven_damage_change_takes_effect() {
        let engine = ScriptEngine::new().unwrap();
        engine
            .eval(r#"defineWeapon({ id: "os_rifle", damage: 50 });"#)
            .unwrap();
        assert_eq!(engine.weapon().damage, 50.0);

        // A subsequent script can override it again.
        engine
            .eval(r#"defineWeapon({ id: "os_rifle", damage: 75 });"#)
            .unwrap();
        assert_eq!(engine.weapon().damage, 75.0);
    }

    #[test]
    fn define_bot_config_and_partial_merge() {
        let engine = ScriptEngine::new().unwrap();
        engine
            .eval(r#"defineBotConfig({ moveSpeed: 300.0, aggroRange: 2048.0 });"#)
            .unwrap();
        let b = engine.bot();
        assert_eq!(b.move_speed, 300.0);
        assert_eq!(b.aggro_range, 2048.0);
        // fire_interval_ms was not specified -> inherits Default.
        assert_eq!(b.fire_interval_ms, BotConfig::default().fire_interval_ms);
    }

    #[test]
    fn callbacks_fire_on_events_and_receive_json_args() {
        let engine = ScriptEngine::new().unwrap();
        engine
            .eval(
                r#"
                globalThis.__log = [];
                onBotKilled(function (ev) { globalThis.__log.push("bot:" + ev.botId); });
                on("onConsoleCommand", function (ev) { globalThis.__log.push("cmd:" + ev.name); });
                "#,
            )
            .unwrap();

        // Unregistered event is a silent no-op.
        engine
            .fire_event("onRoundStart", &serde_json::json!({}))
            .unwrap();

        engine
            .fire_event("onBotKilled", &serde_json::json!({ "botId": 7 }))
            .unwrap();
        engine
            .fire_event("onConsoleCommand", &serde_json::json!({ "name": "give_ammo" }))
            .unwrap();

        // Read the JS-side log back out to confirm the callbacks ran in order.
        let log: Vec<String> = engine
            .context
            .with(|ctx| ctx.eval::<Vec<String>, _>("globalThis.__log"))
            .unwrap();
        assert_eq!(log, vec!["bot:7".to_string(), "cmd:give_ammo".to_string()]);
    }
}
