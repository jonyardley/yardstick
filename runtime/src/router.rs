use std::sync::{Arc, OnceLock, Weak};

use crux_core::{
    Core,
    bridge::BincodeFfiFormat,
    effects::{EffectRouter, Routes, routes::Serialized},
};
use shared::{Daily, Effect, Event, ViewModel};

use crate::ShellCallback;
use crate::storage_handler::StorageHandler;

/// The route set owned by the [`EffectRouter`]: a single serialized lane for
/// everything the shell handles (Render).
///
/// Storage is deliberately not a route here: [`Routes::new`] has a fixed
/// signature (it only receives the `Weak` router), but [`StorageHandler`]
/// needs a database path and can fail to construct. It is therefore built in
/// [`AppRuntime::new`] once the router `Arc` exists, and installed into a
/// `OnceLock` the routing closure captures.
#[derive(Clone)]
pub struct DailyRoutes {
    serialized: Arc<Serialized<Daily, Self, BincodeFfiFormat>>,
}

impl Routes<Daily> for DailyRoutes {
    fn new(router: Weak<EffectRouter<Daily, Self>>) -> Self {
        Self {
            serialized: Arc::new(Serialized::new(router)),
        }
    }
}

/// Wraps the crux [`Core`] in an [`EffectRouter`]: Storage effects are
/// handled entirely in Rust on a background thread; every other effect is
/// serialized and pushed to the shell via [`ShellCallback`].
pub struct AppRuntime {
    router: Arc<EffectRouter<Daily, DailyRoutes>>,
}

impl AppRuntime {
    /// Build the runtime. `db_path: None` opens an in-memory database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened/migrated or the
    /// storage worker thread cannot be spawned.
    pub fn new(
        db_path: Option<&std::path::Path>,
        shell: Arc<dyn ShellCallback>,
    ) -> anyhow::Result<Arc<Self>> {
        let storage: Arc<OnceLock<StorageHandler>> = Arc::new(OnceLock::new());

        let router = EffectRouter::new(Core::<Daily>::new(), {
            let storage = Arc::clone(&storage);

            move |routes: DailyRoutes| {
                move |effect| match effect {
                    Effect::Storage(request) => {
                        // Handled entirely in Rust — never serialized. The
                        // handler is installed below, before `new` returns,
                        // so it is always present once events can be sent.
                        storage
                            .get()
                            .expect("storage handler installed before any event is sent")
                            .process(request);
                    }
                    other => {
                        let bytes = routes
                            .serialized
                            .serialize(other)
                            .expect("serialized effect request should encode");

                        shell.process_effects(bytes);
                    }
                }
            }
        });

        let handler = StorageHandler::new(db_path, Arc::downgrade(&router))?;
        assert!(
            storage.set(handler).is_ok(),
            "storage handler installed twice"
        );

        Ok(Arc::new(Self { router }))
    }

    pub fn send_event(&self, event: Event) {
        self.router.update(event);
    }

    #[must_use]
    pub fn view(&self) -> ViewModel {
        self.router.view()
    }
}
