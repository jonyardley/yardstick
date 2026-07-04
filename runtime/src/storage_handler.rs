use std::path::Path;
use std::sync::Weak;
use std::sync::mpsc::{Sender, channel};
use std::thread;

use crux_core::Request;
use crux_core::effects::ResolveSink;
use shared::StorageOperation;

/// One background thread owns the rusqlite `Connection` (`Send`, not `Sync`).
/// Requests arrive on an mpsc queue; results resolve back into the core
/// through the router (as a [`ResolveSink`]), which routes any follow-up
/// effects under the same policy.
pub struct StorageHandler {
    jobs: Sender<Request<StorageOperation>>,
}

impl StorageHandler {
    /// Open the database (`None` = in-memory) and start the worker thread.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened/migrated or the
    /// thread cannot be spawned.
    pub fn new<S>(db_path: Option<&Path>, sink: Weak<S>) -> anyhow::Result<Self>
    where
        S: ResolveSink<StorageOperation> + Send + Sync + 'static,
    {
        let conn = match db_path {
            Some(path) => store::open(path)?,
            None => store::open_in_memory()?,
        };

        let (jobs, jobs_rx) = channel::<Request<StorageOperation>>();

        thread::Builder::new()
            .name("daily-storage".into())
            .spawn(move || {
                while let Ok(mut request) = jobs_rx.recv() {
                    // store::execute maps database errors into
                    // StorageResult::Error, so this never panics.
                    let output = store::execute(&conn, &request.operation);

                    let Some(sink) = sink.upgrade() else {
                        // Router dropped: the app is shutting down.
                        return;
                    };
                    // A resolve error means the core no longer expects this
                    // request (e.g. it was dropped mid-shutdown); discarding
                    // the result is the panic-free option.
                    let _ = sink.resolve_request(&mut request, output);
                }
            })?;

        Ok(Self { jobs })
    }

    pub fn process(&self, request: Request<StorageOperation>) {
        // The receiver only drops when the worker exits at shutdown; a send
        // error then is moot.
        let _ = self.jobs.send(request);
    }
}
