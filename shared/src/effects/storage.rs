use crux_core::{Command, Request, capability::Operation, command::RequestBuilder};
use facet::Facet;
use serde::{Deserialize, Serialize};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Task {
    pub id: String,
    pub title: String,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum StorageOperation {
    InsertTask { title: String },
    ListTasks,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum StorageResult {
    Task(Task),
    Tasks(Vec<Task>),
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
