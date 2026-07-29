use crux_core::{
    App, Command,
    macros::effect,
    render::{RenderOperation, render},
};
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::civil::{self, CivilDate};
use crate::effects::storage::{self, StorageOperation, StorageResult, TaskData};
use crate::task::{Bucket, Status};
use crate::view::{
    CalendarVm, DayVm, SidebarVm, TaskListVm, build_calendar, build_day, build_list, build_sidebar,
};

#[derive(Facet, Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
pub enum Event {
    Startup {
        today: String,
    },
    NavigateToDay {
        date: String,
    },
    GoToToday,
    ShiftMonth {
        delta: i32,
    },
    EditDay {
        date: String,
        text: String,
    },
    DayLoaded(StorageResult),
    DaySaved(StorageResult),
    // -- tasks (Phase 2) --
    CaptureTask {
        title: String,
        source: String,
    },
    TriageTask {
        id: String,
        bucket: Bucket,
        priority: u8,
        due: String,
    },
    SetStatus {
        id: String,
        status: Status,
        reason: String,
    },
    ToggleDone {
        id: String,
    },
    EditTaskTitle {
        id: String,
        title: String,
    },
    BulkUpdateTasks {
        ids: Vec<String>,
        bucket: Option<Bucket>,
        priority: Option<u8>,
        status: Option<Status>,
    },
    TaskCreated(StorageResult),
    TaskSaved(StorageResult),
    TasksLoaded(StorageResult),
    // -- routing and the All-actions controls (Phase 2) --
    SelectView {
        kind: String,
    },
    SetGrouping {
        group_by: String,
    },
    SetFilter {
        bucket: String,
        status: String,
    },
}

#[effect(facet_typegen)]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
    Storage(StorageOperation),
}

#[derive(Default)]
pub struct Model {
    pub today: String,
    pub selected_date: String,
    pub calendar_year: i32,
    pub calendar_month: u32,
    pub note_text: String,
    pub editor_version: u64,
    /// Set when `note_text` has been edited since the in-flight `GetDay`
    /// for `selected_date` was issued; cleared whenever a fresh load is
    /// (re-)issued. Lets `DayLoaded` tell a stale same-date snapshot (drop
    /// it — the DB is behind the user's typing) from a genuine fresh-day
    /// load (apply it) without any change to the ViewModel surface.
    pub dirty_since_load: bool,
    pub tasks: Vec<TaskData>,
    /// Which surface the main column shows: "today" | "now" | "next" |
    /// "later" | "waiting" | "inbox" | "all" (plan decision #7).
    pub route: String,
    /// All-actions view state. Empty strings mean "no filter".
    pub group_by: String,
    pub filter_bucket: String,
    pub filter_status: String,
    pub error: Option<String>,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct ViewModel {
    pub sidebar: SidebarVm,
    pub calendar: CalendarVm,
    /// Which surface the main column draws (plan decision #7): both `day` and
    /// `list` are always present, and this tag says which one to show.
    pub route: String,
    pub day: DayVm,
    pub list: TaskListVm,
    pub error: Option<String>,
}

#[derive(Default)]
pub struct Yardstick;

/// A storage result arrived with a shape its handler doesn't expect — a
/// handler bug. Surface it visibly (calm banner), never silently.
fn wrong_shape(model: &mut Model, handler: &str, got: &StorageResult) -> Command<Effect, Event> {
    model.error = Some(format!(
        "internal: unexpected storage result in {handler}: {got:?}"
    ));
    render()
}

/// Find a task by id, or set a visible error. Every task write goes through
/// this — a write against an id the model has never seen is a bug worth
/// surfacing, not a silent no-op.
fn task_mut<'m>(model: &'m mut Model, id: &str, handler: &str) -> Option<&'m mut TaskData> {
    let found = model.tasks.iter().position(|t| t.id == id);
    match found {
        Some(i) => Some(&mut model.tasks[i]),
        None => {
            model.error = Some(format!("internal: {handler} for unknown task {id}"));
            None
        }
    }
}

/// The single place a status changes, so the fields that hang off a status
/// can never drift from it: a reason belongs to Blocked, a done day belongs
/// to Done.
fn apply_status(task: &mut TaskData, status: Status, reason: String, today: &str) {
    task.status = status;
    task.blocked_reason = if status == Status::Blocked {
        reason
    } else {
        String::new()
    };
    task.done_on = if status == Status::Done {
        today.to_owned()
    } else {
        String::new()
    };
}

/// Entering the Now bucket starts the age clock; staying in Now keeps it;
/// leaving Now clears it (the age label has no origin outside Now).
fn stamp_now_entry(task: &mut TaskData, bucket: Bucket, today: &str) {
    if bucket == Bucket::Now {
        if task.entered_now_on.is_empty() {
            task.entered_now_on = today.to_owned();
        }
    } else {
        task.entered_now_on = String::new();
    }
    task.bucket = bucket;
}

/// Select a date: update the selection + visible month, clear the editor
/// (bumping its version so the view empties immediately), and request the
/// day's blocks.
fn select_date(model: &mut Model, date: String) -> Command<Effect, Event> {
    if let Some(d) = CivilDate::parse(&date) {
        model.calendar_year = d.year;
        model.calendar_month = d.month;
    }
    model.selected_date = date.clone();
    model.note_text = String::new();
    model.editor_version += 1;
    model.dirty_since_load = false;
    Command::all([render(), storage::get_day(date).then_send(Event::DayLoaded)])
}

impl App for Yardstick {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        match event {
            Event::Startup { today } => {
                // A repeated Startup (Swift re-init/wake) re-issues the same
                // GetDay for the same date. That does NOT make the DB
                // snapshot newer than an edit already applied to
                // note_text — only clear the dirty marker when we're
                // actually landing on a different (unedited) selection.
                if today != model.selected_date {
                    model.dirty_since_load = false;
                }
                model.today = today.clone();
                model.selected_date = today.clone();
                model.route = "today".into();
                model.group_by = "status".into();
                if let Some(d) = CivilDate::parse(&today) {
                    model.calendar_year = d.year;
                    model.calendar_month = d.month;
                }
                Command::all([
                    storage::get_day(today).then_send(Event::DayLoaded),
                    storage::query_tasks().then_send(Event::TasksLoaded),
                ])
            }
            Event::NavigateToDay { date } => select_date(model, date),
            Event::GoToToday => select_date(model, model.today.clone()),
            Event::ShiftMonth { delta } => {
                let (y, m) = if delta < 0 {
                    civil::prev_month(model.calendar_year, model.calendar_month)
                } else {
                    civil::next_month(model.calendar_year, model.calendar_month)
                };
                model.calendar_year = y;
                model.calendar_month = m;
                render()
            }
            Event::EditDay { date, text } => {
                if date == model.selected_date {
                    // Echo of the user's own typing: keep the model in step
                    // WITHOUT bumping editor_version (the editor owns the
                    // caret; see DayVm.editor_version contract).
                    model.note_text = text.clone();
                    // The user's text is now newer than any in-flight load's
                    // DB snapshot: a same-date DayLoaded arriving after this
                    // must be dropped, not applied (I-1).
                    model.dirty_since_load = true;
                }
                let paragraphs: Vec<String> = text.split('\n').map(str::to_owned).collect();
                storage::replace_day_blocks(date, paragraphs).then_send(Event::DaySaved)
            }
            Event::DayLoaded(result) => match result {
                StorageResult::Day(day)
                    if day.date == model.selected_date && model.dirty_since_load =>
                {
                    // The selected day has been edited since this load was
                    // issued: the DB snapshot is stale. Drop it — the
                    // pending debounced save will persist the newer text.
                    Command::done()
                }
                StorageResult::Day(day) if day.date == model.selected_date => {
                    model.note_text = day
                        .blocks
                        .iter()
                        .map(|b| b.text.as_str())
                        .collect::<Vec<_>>()
                        .join("\n");
                    model.editor_version += 1;
                    model.error = None;
                    render()
                }
                // A load for a day we've since navigated away from: drop it.
                StorageResult::Day(_) => Command::done(),
                StorageResult::Error(e) => {
                    model.error = Some(e);
                    render()
                }
                other => wrong_shape(model, "DayLoaded", &other),
            },
            Event::DaySaved(result) => match result {
                StorageResult::DaySaved { .. } => Command::done(),
                StorageResult::Error(e) => {
                    model.error = Some(e);
                    render()
                }
                other => wrong_shape(model, "DaySaved", &other),
            },
            Event::CaptureTask { title, source } => {
                storage::create_task(title, source).then_send(Event::TaskCreated)
            }
            Event::TriageTask {
                id,
                bucket,
                priority,
                due,
            } => {
                let today = model.today.clone();
                let Some(task) = task_mut(model, &id, "TriageTask") else {
                    return render();
                };
                stamp_now_entry(task, bucket, &today);
                task.priority = priority;
                task.due = due;
                storage::save_task(task.clone()).then_send(Event::TaskSaved)
            }
            Event::SetStatus { id, status, reason } => {
                let today = model.today.clone();
                let Some(task) = task_mut(model, &id, "SetStatus") else {
                    return render();
                };
                apply_status(task, status, reason, &today);
                storage::save_task(task.clone()).then_send(Event::TaskSaved)
            }
            Event::ToggleDone { id } => {
                let today = model.today.clone();
                let Some(task) = task_mut(model, &id, "ToggleDone") else {
                    return render();
                };
                if task.status == Status::Done {
                    // Spec §7: unticking restores prev_status, default backlog.
                    // Unlike `apply_status` (used by an explicit SetStatus),
                    // this must not go through `apply_status`'s Blocked-only
                    // reason handling: `blocked_reason` travelled with the
                    // task the whole time it sat in Done, so it is only
                    // cleared here if we are *not* landing back on Blocked.
                    let restored = task.prev_status.take().unwrap_or(Status::Backlog);
                    task.status = restored;
                    task.done_on = String::new();
                    if restored != Status::Blocked {
                        task.blocked_reason = String::new();
                    }
                } else {
                    // Parking a Blocked task in Done must not lose its reason
                    // (found by runtime/tests/tasks_flow.rs's end-to-end
                    // proof — apply_status previously wiped blocked_reason
                    // the moment status left Blocked, even via ToggleDone).
                    let previous = task.status;
                    task.prev_status = Some(previous);
                    task.status = Status::Done;
                    task.done_on = today;
                }
                storage::save_task(task.clone()).then_send(Event::TaskSaved)
            }
            Event::EditTaskTitle { id, title } => {
                let Some(task) = task_mut(model, &id, "EditTaskTitle") else {
                    return render();
                };
                task.title = title;
                storage::save_task(task.clone()).then_send(Event::TaskSaved)
            }
            Event::BulkUpdateTasks {
                ids,
                bucket,
                priority,
                status,
            } => {
                let today = model.today.clone();
                let mut saves = Vec::new();
                for id in ids {
                    let Some(task) = task_mut(model, &id, "BulkUpdateTasks") else {
                        continue;
                    };
                    if let Some(bucket) = bucket {
                        stamp_now_entry(task, bucket, &today);
                    }
                    if let Some(priority) = priority {
                        task.priority = priority;
                    }
                    if let Some(status) = status {
                        apply_status(task, status, String::new(), &today);
                    }
                    saves.push(storage::save_task(task.clone()).then_send(Event::TaskSaved));
                }
                Command::all(saves)
            }
            Event::TaskCreated(result) | Event::TaskSaved(result) => match result {
                StorageResult::Task(_) | StorageResult::TaskSaved { .. } => {
                    model.error = None;
                    // Decision #4: re-query rather than patch the model.
                    Command::all([
                        render(),
                        storage::query_tasks().then_send(Event::TasksLoaded),
                    ])
                }
                StorageResult::Error(e) => {
                    model.error = Some(e);
                    render()
                }
                other => wrong_shape(model, "TaskSaved", &other),
            },
            Event::TasksLoaded(result) => match result {
                StorageResult::Tasks(tasks) => {
                    model.error = None;
                    model.tasks = tasks;
                    render()
                }
                StorageResult::Error(e) => {
                    model.error = Some(e);
                    render()
                }
                other => wrong_shape(model, "TasksLoaded", &other),
            },
            Event::SelectView { kind } => {
                model.route = kind;
                render()
            }
            Event::SetGrouping { group_by } => {
                model.group_by = group_by;
                render()
            }
            Event::SetFilter { bucket, status } => {
                model.filter_bucket = bucket;
                model.filter_status = status;
                render()
            }
        }
    }

    fn view(&self, model: &Model) -> ViewModel {
        // The Today column draws the note plus the Now list, so "today"
        // builds the Now list; every other route builds its own.
        let list_route = if model.route == "today" {
            "now"
        } else {
            &model.route
        };
        ViewModel {
            sidebar: build_sidebar(model),
            calendar: build_calendar(model),
            route: model.route.clone(),
            day: build_day(model),
            list: build_list(
                list_route,
                &model.tasks,
                &model.today,
                &model.group_by,
                &model.filter_bucket,
                &model.filter_status,
            ),
            error: model.error.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::storage::{
        BlockData, Bucket, DayData, Status, StorageOperation, StorageResult, TaskData,
    };

    const TODAY: &str = "2026-07-04";

    fn started() -> (Yardstick, Model) {
        let app = Yardstick;
        let mut model = Model::default();
        let _ = app.update(
            Event::Startup {
                today: TODAY.into(),
            },
            &mut model,
        );
        (app, model)
    }

    fn task_fixture(id: &str, bucket: Bucket, status: Status) -> TaskData {
        TaskData {
            id: id.into(),
            title: "Finalize vendor contract".into(),
            bucket,
            status,
            priority: 0,
            due: String::new(),
            prev_status: None,
            blocked_reason: String::new(),
            source: "quick_add".into(),
            entered_now_on: String::new(),
            done_on: String::new(),
            created_at: 1,
        }
    }

    /// Puts one task into the model the way the runtime does: a query result.
    fn with_task(app: &Yardstick, model: &mut Model, task: TaskData) {
        let _ = app.update(Event::TasksLoaded(StorageResult::Tasks(vec![task])), model);
    }

    /// The storage operations a command carries, ignoring Render. Several
    /// task arms return `Command::all([render(), <storage>])`, so a bare
    /// `expect_storage()` over every effect would panic on the Render.
    fn storage_ops(cmd: &mut Command<Effect, Event>) -> Vec<StorageOperation> {
        cmd.effects()
            .filter(Effect::is_storage)
            .map(|e| e.expect_storage().operation)
            .collect()
    }

    fn day(date: &str, texts: &[&str]) -> StorageResult {
        StorageResult::Day(DayData {
            date: date.into(),
            blocks: texts
                .iter()
                .enumerate()
                .map(|(i, t)| BlockData {
                    id: format!("b{i}"),
                    kind: "paragraph".into(),
                    text: (*t).into(),
                })
                .collect(),
        })
    }

    #[test]
    fn startup_requests_today_and_the_task_list() {
        let app = Yardstick;
        let mut model = Model::default();
        let mut cmd = app.update(
            Event::Startup {
                today: TODAY.into(),
            },
            &mut model,
        );
        let ops: Vec<StorageOperation> = cmd
            .effects()
            .map(|e| e.expect_storage().operation)
            .collect();
        assert_eq!(ops.len(), 2);
        assert!(ops.contains(&StorageOperation::GetDay { date: TODAY.into() }));
        assert!(ops.contains(&StorageOperation::QueryTasks));
        assert_eq!(model.selected_date, TODAY);
    }

    #[test]
    fn day_loaded_joins_blocks_bumps_editor_version_and_renders() {
        let (app, mut model) = started();
        let v0 = app.view(&model).day.editor_version;
        let mut cmd = app.update(
            Event::DayLoaded(day(TODAY, &["Release Meeting", "", "Copy changes?"])),
            &mut model,
        );
        cmd.expect_one_effect().expect_render();
        let view = app.view(&model);
        assert_eq!(view.day.note_text, "Release Meeting\n\nCopy changes?");
        assert_eq!(view.day.title, "Saturday, July 4");
        assert!(view.day.editor_version > v0);
    }

    #[test]
    fn stale_day_load_for_a_departed_date_is_ignored() {
        let (app, mut model) = started();
        let _ = app.update(
            Event::NavigateToDay {
                date: "2026-07-03".into(),
            },
            &mut model,
        );
        // A slow load for the OLD day arrives after navigation:
        let mut cmd = app.update(Event::DayLoaded(day(TODAY, &["old day text"])), &mut model);
        assert_eq!(cmd.effects().count(), 0, "stale load must be dropped");
        assert_eq!(app.view(&model).day.note_text, "");
    }

    #[test]
    fn edit_then_stale_same_date_day_loaded_does_not_clobber_typed_text() {
        let (app, mut model) = started();
        let v0 = app.view(&model).day.editor_version;
        // User types before the initial GetDay resolves.
        let _ = app.update(
            Event::EditDay {
                date: TODAY.into(),
                text: "typed".into(),
            },
            &mut model,
        );
        // The (now-stale) load for the SAME date resolves late, with the
        // empty blocks that were in the DB before the edit was saved.
        let mut cmd = app.update(Event::DayLoaded(day(TODAY, &[])), &mut model);
        assert_eq!(
            cmd.effects().count(),
            0,
            "a dropped stale load must not render"
        );
        let view = app.view(&model);
        assert_eq!(
            view.day.note_text, "typed",
            "stale same-date load must not clobber post-edit text"
        );
        assert_eq!(
            view.day.editor_version, v0,
            "a dropped stale load must not bump editor_version"
        );
    }

    #[test]
    fn repeated_startup_after_edit_does_not_clobber_typed_text() {
        let (app, mut model) = started();
        let _ = app.update(
            Event::EditDay {
                date: TODAY.into(),
                text: "typed".into(),
            },
            &mut model,
        );
        // Swift re-inits/wakes and re-issues Startup for the same day.
        let _ = app.update(
            Event::Startup {
                today: TODAY.into(),
            },
            &mut model,
        );
        // The re-issued GetDay resolves with stale (empty) blocks.
        let _ = app.update(Event::DayLoaded(day(TODAY, &[])), &mut model);
        assert_eq!(
            app.view(&model).day.note_text,
            "typed",
            "repeated Startup's late load must not clobber post-edit text"
        );
    }

    #[test]
    fn navigate_to_day_updates_selection_calendar_and_requests_the_day() {
        let (app, mut model) = started();
        let mut cmd = app.update(
            Event::NavigateToDay {
                date: "2026-06-30".into(),
            },
            &mut model,
        );
        let effects: Vec<Effect> = cmd.effects().collect();
        assert_eq!(effects.len(), 2); // Render (selection highlight) + GetDay
        assert_eq!(model.selected_date, "2026-06-30");
        assert_eq!(app.view(&model).calendar.month_label, "June 2026");
    }

    #[test]
    fn go_to_today_returns_from_elsewhere() {
        let (app, mut model) = started();
        let _ = app.update(
            Event::NavigateToDay {
                date: "2026-06-30".into(),
            },
            &mut model,
        );
        let _ = app.update(Event::GoToToday, &mut model);
        assert_eq!(model.selected_date, TODAY);
        assert_eq!(app.view(&model).calendar.month_label, "July 2026");
    }

    #[test]
    fn shift_month_moves_the_calendar_without_changing_the_selected_day() {
        let (app, mut model) = started();
        let _ = app.update(Event::ShiftMonth { delta: -1 }, &mut model);
        assert_eq!(app.view(&model).calendar.month_label, "June 2026");
        let _ = app.update(Event::ShiftMonth { delta: 1 }, &mut model);
        let _ = app.update(Event::ShiftMonth { delta: 1 }, &mut model);
        assert_eq!(app.view(&model).calendar.month_label, "August 2026");
        assert_eq!(
            model.selected_date, TODAY,
            "paging the calendar is not navigation"
        );
    }

    #[test]
    fn calendar_grid_matches_july_2026() {
        let (app, mut model) = started();
        let cal = app.view(&model).calendar;
        // July 1 2026 is a Wednesday; Monday-first ⇒ two leading blanks (§2.3).
        assert_eq!(cal.cells[0].day, 0);
        assert_eq!(cal.cells[1].day, 0);
        assert_eq!(cal.cells[2].day, 1);
        assert_eq!(cal.cells.len(), 2 + 31);
        let today_cell = cal.cells.iter().find(|c| c.is_today).unwrap();
        assert_eq!(today_cell.day, 4);
        assert!(today_cell.is_selected);
        assert!(today_cell.is_weekend); // 2026-07-04 is a Saturday
        assert!(!cal.cells.iter().find(|c| c.day == 3).unwrap().is_weekend); // Friday
        assert!(cal.cells.iter().find(|c| c.day == 5).unwrap().is_weekend); // Sunday
        let _ = app.update(Event::ShiftMonth { delta: -1 }, &mut model);
        let june = app.view(&model).calendar;
        assert_eq!(
            june.cells[0].day, 1,
            "June 1 2026 is a Monday: no leading blanks"
        );
    }

    #[test]
    fn edit_day_echoes_text_saves_paragraphs_and_leaves_the_editor_alone() {
        let (app, mut model) = started();
        let v0 = app.view(&model).day.editor_version;
        let mut cmd = app.update(
            Event::EditDay {
                date: TODAY.into(),
                text: "line one\n\nline two".into(),
            },
            &mut model,
        );
        let request = cmd.expect_one_effect().expect_storage();
        assert_eq!(
            request.operation,
            StorageOperation::ReplaceDayBlocks {
                date: TODAY.into(),
                paragraphs: vec!["line one".into(), String::new(), "line two".into()],
            }
        );
        let view = app.view(&model);
        assert_eq!(view.day.note_text, "line one\n\nline two");
        assert_eq!(
            view.day.editor_version, v0,
            "own-typing echo must not bump the version"
        );
    }

    #[test]
    fn day_saved_ack_is_silent_and_save_errors_surface_calmly() {
        let (app, mut model) = started();
        let mut cmd = app.update(
            Event::DaySaved(StorageResult::DaySaved { date: TODAY.into() }),
            &mut model,
        );
        assert_eq!(
            cmd.effects().count(),
            0,
            "a successful save changes nothing visible"
        );

        let mut cmd = app.update(
            Event::DaySaved(StorageResult::Error("disk full".into())),
            &mut model,
        );
        cmd.expect_one_effect().expect_render();
        assert_eq!(app.view(&model).error.as_deref(), Some("disk full"));
    }

    #[test]
    fn wrong_shape_results_surface_visibly_not_silently() {
        let (app, mut model) = started();
        // A Tasks result arriving where a Day belongs is a handler bug —
        // it must become a visible error, not a shrug-and-render.
        let mut cmd = app.update(Event::DayLoaded(StorageResult::Tasks(vec![])), &mut model);
        cmd.expect_one_effect().expect_render();
        let err = app.view(&model).error.expect("wrong shape must set error");
        assert!(err.contains("DayLoaded"), "error names the handler: {err}");
    }

    #[test]
    fn tasks_feed_the_inbox_count() {
        let (app, mut model) = started();
        let mut cmd = app.update(
            Event::TasksLoaded(StorageResult::Tasks(vec![task_fixture(
                "t1",
                Bucket::Inbox,
                Status::Backlog,
            )])),
            &mut model,
        );
        cmd.expect_one_effect().expect_render();

        let view = app.view(&model);
        let inbox = view
            .sidebar
            .views
            .iter()
            .find(|v| v.kind == "inbox")
            .unwrap();
        assert_eq!(inbox.count, 1);
        let now = view.sidebar.views.iter().find(|v| v.kind == "now").unwrap();
        assert_eq!(now.count, 0, "buckets do not exist yet — honest zero");
        assert_eq!(view.sidebar.views.len(), 5);
        assert!(view.sidebar.projects.is_empty(), "no fake sidebar data");
        assert_eq!(view.sidebar.space_name, "Red Badger");
        assert_eq!(view.sidebar.today_label, "Jul 4");
    }

    #[test]
    fn capture_creates_an_inbox_task_carrying_its_source() {
        let (app, mut model) = started();
        let mut cmd = app.update(
            Event::CaptureTask {
                title: "Book dentist".into(),
                source: "quick_add".into(),
            },
            &mut model,
        );
        let op = cmd.expect_one_effect().expect_storage().operation;
        assert_eq!(
            op,
            StorageOperation::CreateTask {
                title: "Book dentist".into(),
                source: "quick_add".into(),
            }
        );
    }

    #[test]
    fn a_created_task_triggers_a_requery_not_a_hand_patched_model() {
        let (app, mut model) = started();
        let mut cmd = app.update(
            Event::TaskCreated(StorageResult::Task(task_fixture(
                "t1",
                Bucket::Inbox,
                Status::Backlog,
            ))),
            &mut model,
        );
        let ops = storage_ops(&mut cmd);
        assert!(
            ops.contains(&StorageOperation::QueryTasks),
            "decision #4: writes re-query so the model cannot drift"
        );
        assert!(
            model.tasks.is_empty(),
            "the created task arrives via TasksLoaded, never by hand-patching"
        );
    }

    #[test]
    fn triage_sets_bucket_priority_due_and_stamps_entering_now() {
        let (app, mut model) = started();
        with_task(
            &app,
            &mut model,
            task_fixture("t1", Bucket::Inbox, Status::Backlog),
        );

        let mut cmd = app.update(
            Event::TriageTask {
                id: "t1".into(),
                bucket: Bucket::Now,
                priority: 1,
                due: "2026-07-31".into(),
            },
            &mut model,
        );
        let op = cmd.expect_one_effect().expect_storage().operation;
        let StorageOperation::SaveTask { task } = op else {
            panic!("expected SaveTask, got {op:?}")
        };
        assert_eq!(task.bucket, Bucket::Now);
        assert_eq!(task.priority, 1);
        assert_eq!(task.due, "2026-07-31");
        assert_eq!(
            task.entered_now_on, TODAY,
            "the age label's origin is stamped when the task enters Now"
        );
    }

    #[test]
    fn triage_out_of_now_and_back_does_not_reset_the_age() {
        let (app, mut model) = started();
        let mut aged = task_fixture("t1", Bucket::Now, Status::Backlog);
        aged.entered_now_on = "2026-07-01".into();
        with_task(&app, &mut model, aged);

        let mut cmd = app.update(
            Event::TriageTask {
                id: "t1".into(),
                bucket: Bucket::Now,
                priority: 2,
                due: String::new(),
            },
            &mut model,
        );
        let StorageOperation::SaveTask { task } =
            cmd.expect_one_effect().expect_storage().operation
        else {
            panic!("expected SaveTask")
        };
        assert_eq!(
            task.entered_now_on, "2026-07-01",
            "re-triaging within Now must not restart the age clock"
        );
    }

    #[test]
    fn toggle_done_remembers_the_previous_status_and_stamps_the_day() {
        let (app, mut model) = started();
        with_task(
            &app,
            &mut model,
            task_fixture("t1", Bucket::Now, Status::InProgress),
        );

        let mut cmd = app.update(Event::ToggleDone { id: "t1".into() }, &mut model);
        let StorageOperation::SaveTask { task } =
            cmd.expect_one_effect().expect_storage().operation
        else {
            panic!("expected SaveTask")
        };
        assert_eq!(task.status, Status::Done);
        assert_eq!(task.prev_status, Some(Status::InProgress));
        assert_eq!(task.done_on, TODAY);
    }

    #[test]
    fn unticking_restores_the_previous_status_and_clears_the_day() {
        let (app, mut model) = started();
        let mut done = task_fixture("t1", Bucket::Now, Status::Done);
        done.prev_status = Some(Status::Blocked);
        done.done_on = TODAY.into();
        with_task(&app, &mut model, done);

        let mut cmd = app.update(Event::ToggleDone { id: "t1".into() }, &mut model);
        let StorageOperation::SaveTask { task } =
            cmd.expect_one_effect().expect_storage().operation
        else {
            panic!("expected SaveTask")
        };
        assert_eq!(
            task.status,
            Status::Blocked,
            "spec §7: prev_status restores"
        );
        assert_eq!(task.prev_status, None);
        assert_eq!(task.done_on, "");
    }

    #[test]
    fn unticking_a_task_with_no_remembered_status_falls_back_to_backlog() {
        let (app, mut model) = started();
        let mut done = task_fixture("t1", Bucket::Now, Status::Done);
        done.done_on = TODAY.into();
        with_task(&app, &mut model, done);

        let mut cmd = app.update(Event::ToggleDone { id: "t1".into() }, &mut model);
        let StorageOperation::SaveTask { task } =
            cmd.expect_one_effect().expect_storage().operation
        else {
            panic!("expected SaveTask")
        };
        assert_eq!(task.status, Status::Backlog, "spec §7's stated default");
    }

    #[test]
    fn setting_blocked_keeps_the_reason_and_clearing_the_status_drops_it() {
        let (app, mut model) = started();
        with_task(
            &app,
            &mut model,
            task_fixture("t1", Bucket::Now, Status::Backlog),
        );

        let mut cmd = app.update(
            Event::SetStatus {
                id: "t1".into(),
                status: Status::Blocked,
                reason: "Legal review".into(),
            },
            &mut model,
        );
        let StorageOperation::SaveTask { task } =
            cmd.expect_one_effect().expect_storage().operation
        else {
            panic!("expected SaveTask")
        };
        assert_eq!(task.status, Status::Blocked);
        assert_eq!(task.blocked_reason, "Legal review");

        with_task(&app, &mut model, task);
        let mut cmd = app.update(
            Event::SetStatus {
                id: "t1".into(),
                status: Status::InProgress,
                reason: String::new(),
            },
            &mut model,
        );
        let StorageOperation::SaveTask { task } =
            cmd.expect_one_effect().expect_storage().operation
        else {
            panic!("expected SaveTask")
        };
        assert_eq!(
            task.blocked_reason, "",
            "a reason belongs to Blocked only — it must not linger"
        );
    }

    #[test]
    fn a_bulk_update_saves_every_selected_task_in_one_round_trip() {
        let (app, mut model) = started();
        let _ = app.update(
            Event::TasksLoaded(StorageResult::Tasks(vec![
                task_fixture("t1", Bucket::Inbox, Status::Backlog),
                task_fixture("t2", Bucket::Inbox, Status::Backlog),
                task_fixture("t3", Bucket::Inbox, Status::Backlog),
            ])),
            &mut model,
        );

        let mut cmd = app.update(
            Event::BulkUpdateTasks {
                ids: vec!["t1".into(), "t3".into()],
                bucket: Some(Bucket::Later),
                priority: Some(3),
                status: None,
            },
            &mut model,
        );
        let ops = storage_ops(&mut cmd);
        assert_eq!(ops.len(), 2, "one save per selected task, no re-query yet");
        for op in ops {
            let StorageOperation::SaveTask { task } = op else {
                panic!("expected SaveTask")
            };
            assert!(
                task.id == "t1" || task.id == "t3",
                "unselected must not move"
            );
            assert_eq!(task.bucket, Bucket::Later);
            assert_eq!(task.priority, 3);
            assert_eq!(task.status, Status::Backlog, "None means leave it alone");
        }
    }

    #[test]
    fn editing_an_unknown_task_surfaces_an_error_and_saves_nothing() {
        let (app, mut model) = started();
        let mut cmd = app.update(
            Event::EditTaskTitle {
                id: "ghost".into(),
                title: "nope".into(),
            },
            &mut model,
        );
        cmd.expect_one_effect().expect_render();
        assert!(
            app.view(&model).error.is_some(),
            "a missing task is a visible failure, never a silent no-op"
        );
    }

    #[test]
    fn startup_lands_on_today_and_today_carries_the_now_list() {
        let (app, mut model) = started();
        with_task(&app, &mut model, {
            let mut t = task_fixture("t1", Bucket::Now, Status::Backlog);
            t.title = "Chase COAST support docs response".into();
            t
        });
        let view = app.view(&model);
        assert_eq!(view.route, "today");
        assert_eq!(
            view.day.title, "Saturday, July 4",
            "the note is still there"
        );
        assert_eq!(
            view.list.title, "Now",
            "Today draws the note plus the Now section, not an empty list"
        );
        assert_eq!(view.list.groups[0].rows.len(), 1);
    }

    #[test]
    fn selecting_a_sidebar_view_switches_the_surface_without_touching_the_day() {
        let (app, mut model) = started();
        let before = app.view(&model).day.editor_version;
        let mut cmd = app.update(
            Event::SelectView {
                kind: "inbox".into(),
            },
            &mut model,
        );
        cmd.expect_one_effect().expect_render();
        let view = app.view(&model);
        assert_eq!(view.route, "inbox");
        assert_eq!(view.list.title, "Inbox");
        assert_eq!(
            view.day.editor_version, before,
            "switching surfaces must not disturb the editor (R1 gate)"
        );
    }

    #[test]
    fn grouping_and_filter_changes_only_reshape_the_all_actions_list() {
        let (app, mut model) = started();
        let _ = app.update(Event::SelectView { kind: "all".into() }, &mut model);
        let _ = app.update(
            Event::SetGrouping {
                group_by: "bucket".into(),
            },
            &mut model,
        );
        let labels: Vec<String> = app
            .view(&model)
            .list
            .groups
            .into_iter()
            .map(|g| g.label)
            .collect();
        assert_eq!(labels, vec!["Inbox", "Now", "Next", "Later"]);

        let _ = app.update(
            Event::SetFilter {
                bucket: "now".into(),
                status: String::new(),
            },
            &mut model,
        );
        let view = app.view(&model);
        assert_eq!(
            view.list.filter_bucket, "now",
            "the shell renders the active chip"
        );
        assert_eq!(view.list.group_by, "bucket");
    }

    #[test]
    fn storage_error_on_load_is_surfaced_not_fatal() {
        let (app, mut model) = started();
        let mut cmd = app.update(
            Event::TasksLoaded(StorageResult::Error("disk full".into())),
            &mut model,
        );
        cmd.expect_one_effect().expect_render();
        assert_eq!(app.view(&model).error.as_deref(), Some("disk full"));
    }
}
