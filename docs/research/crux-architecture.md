# Crux framework — state of the art (July 2026) for a macOS app with Rust core + SwiftUI shell

Research basis: crux master branch (cloned 2026-07-02, matches published crates), the rewritten docs book (redbadger.github.io/crux/latest_master), crates.io API, GitHub release notes, and Context7 (`/redbadger/crux`). Everything below was verified against actual source in the repo, not just doc summaries.

## Version snapshot (crates.io, verified 2026-07-02)

| Crate | Version | Released | Notes |
|---|---|---|---|
| `crux_core` | **0.19.0** | 2026-06-08 | Command-only API, EffectRouter, BoltFFI |
| `crux_http` | 0.18.0 | 2026-06-08 | Official HTTP capability |
| `crux_kv` | 0.13.0 | 2026-06-08 | Official KV capability (basic) |
| `crux_time` | 0.17.0 | 2026-06-08 | Timers/now |
| `crux_platform` | 0.10.0 | 2026-06-08 | Platform detection |
| `crux_cli` | 0.3.0 | 2026-05-08 | **Removed from workspace in 0.19 — do not adopt** |
| `boltffi` / `boltffi_cli` | 0.27.0 on crates.io; **examples pin `=0.25.2`** | 2026-06-25 | New FFI bindgen (replaces UniFFI) |
| `facet` | examples pin **`=0.44`** (crates.io is at 0.46.5) | — | Exact pin required; crux_core's facet_generate expects a specific facet version |

Toolchain in the templates: Rust edition 2024, `rust-version = "1.90"`, workspace `resolver = "3"`, `cargo-nextest`, `just`, `xcodegen`.

Recent history you need to know:
- **0.16 (Jul 2025)** — Command API became the default; Capabilities API deprecated; facet-based typegen introduced; effect middleware added.
- **0.17 (Mar 2026)** — **Capabilities API removed entirely.** Bridge unified (old `Bridge`/`BridgeWithSerializer` merged). `Core::new_with()` for non-`Default` app/model. Richer Command test API.
- **0.18 (May 2026)** — Generated `EffectTestExt` fluent test assertions (`expect_only_render()`, `resolve_http(...)`, etc., gated behind `crux_core/testing`). C# typegen.
- **0.19 (Jun 2026)** — **UniFFI replaced by BoltFFI** across examples/templates (`crux_core::bindgen` + `uniffi_compat_bindgen` remain only as deprecated migration aids). **EffectRouter** introduced (supersedes/extends middleware). `crux_cli` and rustdoc-based `crux codegen` removed.

---

## 1. Recommended workspace layout + Xcode/SwiftPM integration

The old three-crate layout (`shared` + `shared_types` typegen crate + uniffi scaffolding) is **gone**. Current layout (from `examples/counter`, which is what the Part I docs walk through — and it has first-class **macOS** targets):

```
/
├── Cargo.toml               # workspace: resolver=3, members = ["shared", <rust shells>...]
├── rust-toolchain.toml
├── shared/                  # ONE crate: app + ffi + codegen bin
│   ├── Cargo.toml           # crate-type = ["cdylib", "lib", "staticlib"], [[bin]] codegen
│   ├── boltffi.toml         # BoltFFI packaging config (apple/android/wasm/csharp targets)
│   └── src/
│       ├── lib.rs           # mod app; pub mod ffi; pub use app::*; pub use crux_core::Core;
│       ├── app.rs           # App impl, Event, Model, ViewModel, Effect
│       ├── ffi.rs           # CoreFFI (#[boltffi::export]) wrapping Bridge<App>
│       └── bin/codegen.rs   # facet typegen CLI (clap)
└── apple/
    ├── Justfile             # typegen / package / generate-project / build / dev
    ├── project.yml          # XcodeGen project definition
    ├── generated/           # (gitignored) Swift packages: "Shared" (FFI+staticlib) and "App" (types)
    └── CounterApp/          # SwiftUI sources: CounterApp.swift, ContentView.swift, core.swift
```

**Build tooling: XcodeGen + Justfile + BoltFFI. Not cargo-xcode, not a hand-rolled build phase script.** The Apple docs page says XcodeGen is "the simplest way", and the flow is:

```make
# apple/Justfile (key targets)
typegen:
    cargo run --package shared --bin codegen --features codegen,facet_typegen \
        -- --language swift --output-dir generated
package:
    cd ../shared && boltffi pack apple      # builds staticlib + Swift bindings package
generate-project:
    xcodegen
dev: typegen package generate-project + xcodebuild -scheme CounterApp-macOS ...
```

`boltffi pack apple` (config in `shared/boltffi.toml`, `[targets.apple]` with `include_macos = true`, SPM `layout = "ffi-only"`) produces a **local Swift package `Shared`** containing the Rust static library and the generated FFI bindings. The typegen CLI produces a second **local Swift package `App`** with your `Event`/`Effect`/`ViewModel` value types + bincode serializers. `project.yml` just references both as local packages:

```yaml
packages:
  Shared: { path: ./generated/Shared }
  App:    { path: ./generated/App }
targets:
  CounterApp-macOS:
    templates: [app]
    platform: macOS
    deploymentTarget: "15.0"
    settings: { ENABLE_USER_SCRIPT_SANDBOXING: NO }
```

Install: `cargo install boltffi_cli --version '=0.25.2' --locked` (docs pin this; crates.io has 0.27 — see "in flux" below), `cargo install just`, `brew install xcodegen`.

The larger weather example adds a nested Swift package (`WeatherKit`) holding all views + effect handlers, with the app target containing only the entry point and `LiveBridge` — so SwiftUI previews never load the Rust framework (they use a `FakeBridge`).

## 2. App trait / Command API (crux_core 0.19)

`update()` returns a `Command`; there is no `Capabilities` associated type and no `caps` argument anymore. Actual current code from `examples/counter/shared/src/app.rs`:

```rust
use crux_core::{App, Command, macros::effect, render::{RenderOperation, render}};
use facet::Facet;
use serde::{Deserialize, Serialize};

#[derive(Facet, Serialize, Deserialize, Clone, Debug)]
#[repr(C)]                       // facet needs an enum repr
pub enum Event { Increment, Decrement, Reset }

#[effect(facet_typegen)]         // attribute macro, NOT the old #[derive(Effect)]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
}

impl App for Counter {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;        // new associated type (replaces Capabilities)

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match event { /* mutate model */ }
        render()                 // returns a Command
    }

    fn view(&self, model: &Model) -> ViewModel { ... }
}
```

- **`#[effect]` macro** (`crux_core::macros::effect`) on the Effect enum generates: `From<Request<Op>>` per variant, the FFI companion type, and (with the `testing` feature) an `EffectTestExt` trait with fluent per-variant test helpers. Use `#[effect(facet_typegen)]` when you need Swift typegen.
- **Composition**: capability functions return *command builders* (`RequestBuilder` / `StreamBuilder` / `NotificationBuilder`), finished with `.then_send(Event::...)`. Combinators: `Command::all([a, b])` (concurrent), `.and`, `.map(...)`, `.then_request(...)`, `.then_stream(...)`, `Command::done()` (no-op), `Command::event(...)`. Escape hatch to async: `Command::new(|ctx| async move { let out = ctx.request_from_shell(op).await; ctx.send_event(...); })`; `ctx.spawn`, `.into_future(ctx)`, abort via `cmd.abort_handle()` stored in the model. Commands have **no model access** — results come back as events.
- Real-world example (weather app, parallel fetch on startup):

```rust
let fetch_secret = secret::command::fetch(secret::API_KEY_NAME)
    .then_send(|r| Event::Initializing(InitializingEvent::SecretFetched(r)));
let fetch_favorites = KeyValue::get(FAVORITES_KEY)
    .then_send(|r| Event::Initializing(InitializingEvent::FavoritesLoaded(r)));
Command::all([fetch_secret, fetch_favorites])
```

- **Migration from Capabilities** (docs part-2/effects.md): (1) delete `Capabilities` type + `caps` param, (2) add `type Effect`, (3) return `Command` (start with `Command::done()` and migrate incrementally). Note the Crux async runtime is driven by shell calls — don't run tokio-specific futures *inside* Commands.

## 3. Type generation

**Current recommendation: facet-based typegen via a `codegen` bin in the `shared` crate.** The serde-reflection path (`crux_core::typegen::TypeGen` in a `shared_types` build.rs) still exists behind the legacy `typegen` feature (`crux_core/src/type_generation/serde.rs`) but is not what the docs teach; the rustdoc-based `crux_cli codegen` experiment was **removed in 0.19**.

`shared/src/bin/codegen.rs` (verbatim from counter):

```rust
use crux_core::type_generation::facet::{Config, TypeRegistry};
use shared::Counter;

fn main() -> anyhow::Result<()> {
    let typegen_app = TypeRegistry::new().register_app::<Counter>()?.build()?;
    let config = Config::builder("App", &args.output_dir).build();  // "App" = Swift package name
    typegen_app.swift(&config)?;   // also .kotlin, .typescript, .csharp
    Ok(())
}
```

Requirements: derive `Facet` on every boundary type (`Event`, `ViewModel`, operation types), give enums a `#[repr(C)]`/`#[repr(u8)]`, use `#[effect(facet_typegen)]`, and gate with features `facet_typegen = ["crux_core/facet_typegen"]` + a `codegen` feature. `register_app::<Counter>()` walks the App's associated types via facet reflection — no manual type registration. Fields that shouldn't cross the FFI can be `#[facet(skip)]`/`#[facet(opaque)]` (+ `#[serde(skip)]`).

Swift side: output is a complete Swift package (`apple/generated/App`) with value types + `bincodeSerialize()/bincodeDeserialize()` (Serde-compatible bincode). Added to Xcode as a local SwiftPM dependency via project.yml. Wire format is bincode by default (`BincodeFfiFormat`; `JsonFfiFormat` also exists).

## 4. Shell-side effect loop (Bridge + Swift structure)

Rust side, `shared/src/ffi.rs`: a `CoreFFI` struct wraps `crux_core::bridge::Bridge<Counter>` and is exported with `#[boltffi::export]`. Three byte-oriented methods: `update(data) -> Vec<u8>`, `resolve(id: u32, data) -> Vec<u8>`, `view() -> Vec<u8>`. The Bridge assigns each in-flight effect an `EffectId` and serializes `Requests` (a `Vec<Request<EffectFfi>>`).

Recommended Swift structure (weather example, the production-shaped one):

```swift
// bridge.swift — protocol so previews can fake the core
public protocol CoreBridge {
    func processEvent(_ event: Event) -> [Request]
    func resolve(requestId: UInt32, responseBytes: [UInt8]) -> [Request]
    func currentView() -> ViewModel
}

// LiveBridge.swift (app target) — bincode boundary around generated CoreFFI
struct LiveBridge: CoreBridge {
    private let ffi = CoreFFI()
    func processEvent(_ event: Event) -> [Request] {
        let effects = [UInt8](ffi.update(data: Data(try! event.bincodeSerialize())))
        return try! Requests.bincodeDeserialize(input: effects).value
    }
    func resolve(requestId: UInt32, responseBytes: [UInt8]) -> [Request] { ... }
    func currentView() -> ViewModel { try! .bincodeDeserialize(input: [UInt8](ffi.view())) }
}

// core.swift — @Observable core wrapper (yes, the new Observation framework)
@Observable @MainActor
public class Core {
    public var view: ViewModel
    private let bridge: CoreBridge

    public func update(_ event: Event) {
        for request in bridge.processEvent(event) { processEffect(request) }
    }
    func processEffect(_ request: Request) {
        switch request.effect {
        case .render:                    view = bridge.currentView()
        case let .http(req):             resolveHttp(request: req, requestId: request.id)
        case let .keyValue(req):         resolveKeyValue(request: req, requestId: request.id)
        // ... one case per Effect variant
        }
    }
    func resolve(requestId: UInt32, serialize: () throws -> [UInt8]) {
        let requests = bridge.resolve(requestId: requestId, responseBytes: try! serialize())
        for request in requests { processEffect(request) }   // recurse until quiescent
    }
}
```

Effect handlers live as `extension Core` per capability (http.swift with URLSession, keyValue.swift, secret.swift with Keychain, location.swift with CoreLocation, time.swift). Events are dispatched from views via a tiny `@Observable` `CoreUpdater` injected through `@Environment`. The simpler counter example uses `ObservableObject` + `@Published var view` instead — both patterns are current; `@Observable` is what the flagship example uses.

## 5. Custom effects/capabilities; storage story

A capability in 2026 is just: **an `Operation` protocol type + free functions returning command builders**. Complete minimal example (weather app's Location, verbatim pattern to copy for a SQLite capability):

```rust
// protocol
#[derive(Facet, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum SqlOperation {                    // your SQLite version of LocationOperation
    Execute { sql: String, params: Vec<Value> },
    Query   { sql: String, params: Vec<Value> },
}
#[derive(Facet, Clone, Serialize, Deserialize, Debug, PartialEq)]
#[repr(C)]
pub enum SqlResult { RowsAffected(u64), Rows(Vec<Row>), Error(String) }

impl crux_core::capability::Operation for SqlOperation {
    type Output = SqlResult;
}

// developer API — generic over Effect/Event so it's reusable in any app
pub fn query<Effect, Event>(sql: impl Into<String>)
    -> RequestBuilder<Effect, Event, impl Future<Output = SqlResult>>
where
    Effect: Send + From<Request<SqlOperation>> + 'static,
    Event: Send + 'static,
{
    Command::request_from_shell(SqlOperation::Query { sql: sql.into(), params: vec![] })
}
```

Add `Sql(SqlOperation)` as a variant of your `#[effect(facet_typegen)]` enum, add a `case .sql(let req): resolveSql(...)` arm in the Swift `processEffect`, and implement it with GRDB/SQLite.swift/raw sqlite3 — resolve via `core.resolve(requestId:...)`. Streams (`stream_from_shell`) are available for subscription-style effects.

**Alternative for SQLite specifically**: since 0.19 you can keep SQLite in Rust (rusqlite) *without* polluting the pure core, using **middleware** (`EffectMiddleware` + `.handle_effects_using(...)` + `.map_effect::<FfiEffect>()` to hide the variant from the shell) or the new **EffectRouter** (typed per-effect dispatch; lanes: `Serialized` to the shell, plus `Parked`/`Buffer` for hard-to-serialize effects). `examples/counter-routing/shared/src/ffi.rs` shows a full router: `Random` effects handled by a Rust `RngHandler`, everything else serialized and pushed to the shell via a `#[boltffi::export] trait CruxShell { fn process_effects(&self, bytes: Vec<u8>); }` callback. This is the natural home for a Rust SQLite handler on macOS.

**Official capabilities**: `crux_kv` (0.13.0) is the official KV store — get/set/delete/exists/list_keys(prefix, cursor), values are `Vec<u8>`; actively maintained and used by the flagship weather example (Swift handler backed by Core Data in the example). It is deliberately basic ("A basic Key-Value store"). **There is no official SQL/SQLite capability** — zero references in the repo; the docs explicitly describe SQL storage as an app-defined capability decision. Also official: `crux_http`, `crux_time`, `crux_platform`.

## 6. Testing story

**`AppTester` is deprecated** (still present in `crux_core::testing` for migration only). Current approach: call `app.update(event, &mut model)` directly and assert on the returned `Command`.

- Enable in `[dev-dependencies]`: `crux_core = { workspace = true, features = ["testing"] }`.
- The `#[effect]` macro generates an **`EffectTestExt`** trait (per your Effect enum) with fluent helpers per variant: `expect_only_render()`, `expect_only_location_with(|op| ...)`, `resolve_http(|op| HttpResult::Ok(...))`, plus generic `Command` helpers (`effects()`, `events()`, `expect_one_effect()`, `take_effects()`, `expect_done()`, `is_done()`).

```rust
#[test]
fn weather_fetched_stores_data() {
    let (local, mut cmd) = drive_to_fetching_weather();     // resolve earlier effects with canned data

    let event = cmd
        .resolve_http(|_op| HttpResult::Ok(HttpResponse::ok().body(json).build()))
        .expect_event();                                    // command emits the follow-up event

    let (local, _cmd) = local.update(event, &api_key()).expect_continue().into_parts();
    assert!(matches!(local, LocalWeather::Fetched(..)));
}
```

You drive the full event → update → effect → resolve → event loop by hand, deterministically; the weather app's 57 tests run in ~20 ms. The docs also sketch deterministic simulation testing (random event fuzzing against fake effect implementations) as a recommended pattern.

## 7. Multiple shells / headless shells

- **Multiple shells per core codebase: yes, this is the whole point and heavily exemplified.** The counter example alone ships SwiftUI (iOS + macOS), Kotlin/Compose, **Ratatui TUI (headless-adjacent Rust shell)**, Tauri, Leptos, Yew, Dioxus, Next.js, React Router, and Windows C#/WinUI shells — all against one `shared` crate. Each shell instantiates **its own `Core`** in its own process.
- **Rust shells skip the bridge entirely**: `let core: Core<Counter> = Core::new(); core.process_event(event) -> Vec<Effect>; core.resolve(&mut request, output)` with typed effects — see `examples/counter/tui/src/main.rs` and the Ratatui/Tauri platform docs (part-3). This is exactly what a headless MCP-server shell would use: link `shared` directly, run your own loop, execute effects with tokio (tokio is fine in the *shell*; avoid tokio-specific futures *inside* Commands).
- **Same process, one core, two frontends (e.g. SwiftUI GUI + MCP server)**: no official example, but it is architecturally supported: `Core<A>` uses `&self` methods with interior locking (`RwLock<Model>` + `Mutex<Command>`), so it can sit behind an `Arc` and be driven from multiple threads. The ffi.rs docs also note the shell may create *several* app instances if it wants. The missing piece is fan-out: Crux has no built-in subscriber/broadcast for effects or view updates — with the synchronous Bridge, effects are returned to whichever caller made the call. The **EffectRouter + `CruxShell.process_effects` callback pattern** (counter-routing) inverts this into push delivery, which is the primitive you would use to notify both frontends (your callback fans out `Render` to GUI and MCP sides). Budget for writing this glue yourself.
- **Different processes**: separate core instances with app-level state sync. The official pattern is the **notes example**: two instances collaborating via a custom `PubSub` capability + Automerge CRDT document, with `crux_kv` persistence — the recommended shape for GUI app + separate headless daemon sharing state.

## Things that are ambiguous or in flux (flagged)

1. **BoltFFI migration is one month old.** Docs/examples pin `boltffi = "=0.25.2"` while crates.io is already at 0.27.0 — expect exact-version pinning and churn. The deprecated `uniffi_compat_bindgen` feature exists as a bridge; the counter example even still contains a leftover `uniffi.toml`. BoltFFI is a real crate ("up to 1000x faster than UniFFI", boltffi.dev) but young.
2. **EffectRouter is RFC-stage** (`docs/src/rfcs/effect-router.md` has open questions; `examples/counter-routing/README.md` contains a literal `**FIXME**`). Middleware docs likewise warn "the API may change more than the rest of Crux". Fine for a custom Rust-side SQLite handler, but expect breaking changes.
3. **Two typegen systems coexist** in crux_core: legacy `typegen` feature (serde-generate, build.rs style) and current `facet_typegen`. Several READMEs (e.g. `crux_kv`'s) still describe the old `shared_types` crate layout — stale; follow the book (part-1/shell.md, part-4/typegen.md) and the counter/weather examples instead.
4. **Facet pinning**: `facet = "=0.44"` exact pin is required to match crux_core's `facet_generate`; crates.io facet is at 0.46.5/0.50-rc. Don't float this.
5. **Old doc URLs still live**: `redbadger.github.io/crux/getting_started/...` pages reflect the pre-0.17 Capabilities world. Use `redbadger.github.io/crux/latest_master/` (the rebuilt book: Part I counter, Part II weather, Part III platforms/middleware, Part IV runtime/typegen).
6. **`AppTester`** deprecated-but-present; new code should never use it.
7. The book hints at future work: `difficient` (view-model diffing over the wire) and shell-side observability patterns are explicitly "still exploring".

## Practical recipe for a Yardstick macOS app

1. Workspace with `shared` crate (crate-type `["lib","staticlib","cdylib"]`), `crux_core = "0.19"`, `boltffi = "=0.25.2"`, `facet = "=0.44"`, plus `[[bin]] codegen` behind `codegen`/`facet_typegen` features.
2. App: `impl App` with `type Effect`, `update -> Command<Effect, Event>`; effects via `#[effect(facet_typegen)]`; use `crux_kv` now, define a custom `SqlOperation` capability (or an EffectRouter-handled rusqlite handler) for SQLite.
3. Apple shell: `boltffi pack apple` + typegen CLI → two local Swift packages → XcodeGen `project.yml` with a macOS target (deployment 15.0) → `@Observable @MainActor Core` wrapper + `CoreBridge` protocol + per-capability `extension Core` handlers; `FakeBridge` for previews.
4. Headless/MCP shell: a second Rust binary in the workspace linking `shared` and using `Core<App>` directly (Ratatui/TUI example is the template). Same-process GUI+MCP sharing one core is DIY via EffectRouter callbacks; separate processes should use two cores + explicit sync (notes example pattern).
5. Test with `cargo nextest`, `features = ["testing"]`, `EffectTestExt` fluent assertions.

Sources:
- [crux repo (master, cloned)](https://github.com/redbadger/crux) — docs/src, examples/counter, examples/weather, examples/counter-routing, crux_core source
- [crux_core 0.19.0 release notes](https://github.com/redbadger/crux/releases/tag/crux_core-v0.19.0) and [releases list](https://github.com/redbadger/crux/releases)
- [Crux docs book (latest master)](https://redbadger.github.io/crux/latest_master/)
- [crux_core on crates.io](https://crates.io/crates/crux_core), [crux_kv](https://crates.io/crates/crux_kv), [boltffi](https://crates.io/crates/boltffi)
- Context7 `/redbadger/crux` (docs snippets for effects, typegen, iOS shell)