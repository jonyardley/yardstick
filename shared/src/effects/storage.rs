use crux_core::{Command, Request, capability::Operation, command::RequestBuilder};
use facet::Facet;
use serde::{Deserialize, Serialize};

/// When a task should happen. Orthogonal to [`Status`] (handoff §Task).
#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum Bucket {
    Inbox,
    Now,
    Next,
    Later,
}

/// What state a task is in. Orthogonal to [`Bucket`]; six states, one at a
/// time (core-journeys Journey 5).
#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum Status {
    Backlog,
    InProgress,
    Blocked,
    Waiting,
    Done,
    Binned,
}

/// One task, whole. Absent values are `""` / `0` across the FFI boundary and
/// NULL in the database (plan Task 1 interfaces).
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TaskData {
    pub id: String,
    pub title: String,
    pub bucket: Bucket,
    pub status: Status,
    /// 1, 2, 3 — or 0 for "no priority" (priority is optional).
    pub priority: u8,
    /// 'YYYY-MM-DD' or "".
    pub due: String,
    /// Restored when a done checkbox is unticked (spec §7).
    pub prev_status: Option<Status>,
    pub blocked_reason: String,
    /// Capture provenance: 'quick_add' | 'note' | 'menu_bar' | 'mcp'.
    pub source: String,
    /// Civil date the task entered the Now bucket — the age label's origin.
    pub entered_now_on: String,
    pub done_on: String,
    /// The store's clock; ordering only, never displayed.
    pub created_at: i64,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BlockData {
    pub id: String,
    pub kind: String,
    pub text: String,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DayData {
    pub date: String,
    pub blocks: Vec<BlockData>,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum StorageOperation {
    // -- tasks (Phase 2) --
    /// Capture: the store generates the id and the timestamps; everything
    /// else takes its untriaged default (bucket=inbox, status=backlog).
    CreateTask { title: String, source: String },
    /// Upsert every mutable column from the core's own copy (decision #2).
    /// Errors if the id is unknown — a save is never a disguised insert.
    SaveTask { task: TaskData },
    /// Every non-deleted task in the space, oldest first (decision #3).
    QueryTasks,
    // -- daily notes (Phase 1) --
    /// Read a day's blocks. Never creates the note (lazy creation is on
    /// first edit — spec §4).
    GetDay { date: String },
    /// Rewrite the day's blocks from paragraphs, creating the note row if
    /// needed. One transaction including the FTS index.
    ReplaceDayBlocks {
        date: String,
        paragraphs: Vec<String>,
    },
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum StorageResult {
    // -- tasks --
    Task(TaskData),
    Tasks(Vec<TaskData>),
    TaskSaved { id: String },
    // -- daily notes --
    Day(DayData),
    DaySaved { date: String },
    // -- any operation --
    Error(String),
}

impl Operation for StorageOperation {
    type Output = StorageResult;
}

pub fn create_task<Effect, Event>(
    title: impl Into<String>,
    source: impl Into<String>,
) -> RequestBuilder<Effect, Event, impl std::future::Future<Output = StorageResult>>
where
    Effect: Send + From<Request<StorageOperation>> + 'static,
    Event: Send + 'static,
{
    Command::request_from_shell(StorageOperation::CreateTask {
        title: title.into(),
        source: source.into(),
    })
}

pub fn save_task<Effect, Event>(
    task: TaskData,
) -> RequestBuilder<Effect, Event, impl std::future::Future<Output = StorageResult>>
where
    Effect: Send + From<Request<StorageOperation>> + 'static,
    Event: Send + 'static,
{
    Command::request_from_shell(StorageOperation::SaveTask { task })
}

pub fn query_tasks<Effect, Event>()
-> RequestBuilder<Effect, Event, impl std::future::Future<Output = StorageResult>>
where
    Effect: Send + From<Request<StorageOperation>> + 'static,
    Event: Send + 'static,
{
    Command::request_from_shell(StorageOperation::QueryTasks)
}

pub fn get_day<Effect, Event>(
    date: impl Into<String>,
) -> RequestBuilder<Effect, Event, impl std::future::Future<Output = StorageResult>>
where
    Effect: Send + From<Request<StorageOperation>> + 'static,
    Event: Send + 'static,
{
    Command::request_from_shell(StorageOperation::GetDay { date: date.into() })
}

pub fn replace_day_blocks<Effect, Event>(
    date: impl Into<String>,
    paragraphs: Vec<String>,
) -> RequestBuilder<Effect, Event, impl std::future::Future<Output = StorageResult>>
where
    Effect: Send + From<Request<StorageOperation>> + 'static,
    Event: Send + 'static,
{
    Command::request_from_shell(StorageOperation::ReplaceDayBlocks {
        date: date.into(),
        paragraphs,
    })
}
