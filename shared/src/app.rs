use crux_core::{
    App, Command,
    macros::effect,
    render::{RenderOperation, render},
};
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::effects::storage::{self, StorageOperation, StorageResult, Task};

#[derive(Facet, Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
pub enum Event {
    Startup,
    CreateTask { title: String },
    TaskSaved(StorageResult),
    TasksLoaded(StorageResult),
}

#[effect(facet_typegen)]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
    Storage(StorageOperation),
}

#[derive(Default)]
pub struct Model {
    pub tasks: Vec<Task>,
    pub error: Option<String>,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct ViewModel {
    pub tasks: Vec<Task>,
    pub count: u64,
    pub error: Option<String>,
}

#[derive(Default)]
pub struct Daily;

impl App for Daily {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match event {
            Event::Startup => storage::list_tasks().then_send(Event::TasksLoaded),
            Event::CreateTask { title } => storage::insert_task(title).then_send(Event::TaskSaved),
            Event::TaskSaved(StorageResult::Task(task)) => {
                model.error = None;
                model.tasks.push(task);
                render()
            }
            Event::TasksLoaded(StorageResult::Tasks(tasks)) => {
                model.error = None;
                model.tasks = tasks;
                render()
            }
            Event::TaskSaved(StorageResult::Error(e))
            | Event::TasksLoaded(StorageResult::Error(e)) => {
                model.error = Some(e);
                render()
            }
            // Mismatched result shapes are handler bugs; keep state, just re-render.
            Event::TaskSaved(_) | Event::TasksLoaded(_) => render(),
        }
    }

    fn view(&self, model: &Model) -> ViewModel {
        ViewModel {
            tasks: model.tasks.clone(),
            count: model.tasks.len() as u64,
            error: model.error.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::storage::{StorageOperation, StorageResult, Task};

    fn task(id: &str, title: &str) -> Task {
        Task {
            id: id.into(),
            title: title.into(),
        }
    }

    #[test]
    fn startup_requests_task_list_then_renders() {
        let app = Daily;
        let mut model = Model::default();

        let mut cmd = app.update(Event::Startup, &mut model);
        let request = cmd.expect_one_effect().expect_storage();
        assert_eq!(request.operation, StorageOperation::ListTasks);
    }

    #[test]
    fn tasks_loaded_updates_model_and_renders() {
        let app = Daily;
        let mut model = Model::default();

        let mut cmd = app.update(
            Event::TasksLoaded(StorageResult::Tasks(vec![task("t1", "Buy milk")])),
            &mut model,
        );
        cmd.expect_one_effect().expect_render();
        let view = app.view(&model);
        assert_eq!(view.count, 1);
        assert_eq!(view.tasks[0].title, "Buy milk");
    }

    #[test]
    fn create_task_requests_insert_then_appends_on_save() {
        let app = Daily;
        let mut model = Model::default();

        let mut cmd = app.update(
            Event::CreateTask {
                title: "Ship it".into(),
            },
            &mut model,
        );
        let request = cmd.expect_one_effect().expect_storage();
        assert_eq!(
            request.operation,
            StorageOperation::InsertTask {
                title: "Ship it".into()
            }
        );

        let mut cmd = app.update(
            Event::TaskSaved(StorageResult::Task(task("t2", "Ship it"))),
            &mut model,
        );
        cmd.expect_one_effect().expect_render();
        assert_eq!(app.view(&model).tasks.len(), 1);
    }

    #[test]
    fn storage_error_is_surfaced_not_fatal() {
        let app = Daily;
        let mut model = Model::default();

        let mut cmd = app.update(
            Event::TasksLoaded(StorageResult::Error("disk full".into())),
            &mut model,
        );
        cmd.expect_one_effect().expect_render();
        assert_eq!(app.view(&model).error.as_deref(), Some("disk full"));
    }
}
