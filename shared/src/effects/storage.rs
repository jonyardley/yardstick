use crux_core::{Command, Request, capability::Operation, command::RequestBuilder};
use facet::Facet;
use serde::{Deserialize, Serialize};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Task {
    pub id: String,
    pub title: String,
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
    // -- tasks (Phase 0; buckets/status/priority arrive Phase 2) --
    InsertTask {
        title: String,
    },
    ListTasks,
    // -- daily notes (Phase 1) --
    /// Read a day's blocks. Never creates the note (lazy creation is on
    /// first edit — spec §4).
    GetDay {
        date: String,
    },
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
    Task(Task),
    Tasks(Vec<Task>),
    // -- daily notes --
    Day(DayData),
    DaySaved { date: String },
    // -- any operation --
    Error(String),
}

impl Operation for StorageOperation {
    type Output = StorageResult;
}

pub fn insert_task<Effect, Event>(
    title: impl Into<String>,
) -> RequestBuilder<Effect, Event, impl std::future::Future<Output = StorageResult>>
where
    Effect: Send + From<Request<StorageOperation>> + 'static,
    Event: Send + 'static,
{
    Command::request_from_shell(StorageOperation::InsertTask {
        title: title.into(),
    })
}

pub fn list_tasks<Effect, Event>()
-> RequestBuilder<Effect, Event, impl std::future::Future<Output = StorageResult>>
where
    Effect: Send + From<Request<StorageOperation>> + 'static,
    Event: Send + 'static,
{
    Command::request_from_shell(StorageOperation::ListTasks)
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
