use crux_core::{
    App, Command,
    macros::effect,
    render::{RenderOperation, render},
};
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::civil::{self, CivilDate};
use crate::effects::storage::{self, StorageOperation, StorageResult, Task};

#[derive(Facet, Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
pub enum Event {
    Startup { today: String },
    NavigateToDay { date: String },
    GoToToday,
    ShiftMonth { delta: i32 },
    EditDay { date: String, text: String },
    DayLoaded(StorageResult),
    DaySaved(StorageResult),
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
    pub tasks: Vec<Task>,
    pub error: Option<String>,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct ViewModel {
    pub sidebar: SidebarVm,
    pub calendar: CalendarVm,
    pub day: DayVm,
    pub error: Option<String>,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct SidebarVm {
    pub space_name: String,
    pub space_initials: String,
    pub today_label: String,
    pub views: Vec<ViewRowVm>,
    pub projects: Vec<SidebarEntryVm>,
    pub people: Vec<SidebarEntryVm>,
    pub pages: Vec<SidebarEntryVm>,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct ViewRowVm {
    pub kind: String,
    pub label: String,
    pub count: u64,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct SidebarEntryVm {
    pub label: String,
    pub count: u64,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct CalendarVm {
    pub month_label: String,
    pub cells: Vec<CalendarCellVm>,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct CalendarCellVm {
    pub day: u8,
    pub date: String,
    pub is_today: bool,
    pub is_selected: bool,
    pub is_weekend: bool,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct DayVm {
    pub date: String,
    pub title: String,
    pub note_text: String,
    pub editor_version: u64,
}

#[derive(Default)]
pub struct Daily;

/// A storage result arrived with a shape its handler doesn't expect — a
/// handler bug. Surface it visibly (calm banner), never silently.
fn wrong_shape(model: &mut Model, handler: &str, got: &StorageResult) -> Command<Effect, Event> {
    model.error = Some(format!(
        "internal: unexpected storage result in {handler}: {got:?}"
    ));
    render()
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

impl App for Daily {
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
                if let Some(d) = CivilDate::parse(&today) {
                    model.calendar_year = d.year;
                    model.calendar_month = d.month;
                }
                Command::all([
                    storage::get_day(today).then_send(Event::DayLoaded),
                    storage::list_tasks().then_send(Event::TasksLoaded),
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
            Event::CreateTask { title } => storage::insert_task(title).then_send(Event::TaskSaved),
            Event::TaskSaved(result) => match result {
                StorageResult::Task(task) => {
                    model.error = None;
                    model.tasks.push(task);
                    render()
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
        }
    }

    fn view(&self, model: &Model) -> ViewModel {
        ViewModel {
            sidebar: build_sidebar(model),
            calendar: build_calendar(model),
            day: build_day(model),
            error: model.error.clone(),
        }
    }
}

fn build_sidebar(model: &Model) -> SidebarVm {
    let view_row = |kind: &str, label: &str, count: u64| ViewRowVm {
        kind: kind.into(),
        label: label.into(),
        count,
    };
    SidebarVm {
        // Single space until Phase 6 (spec §10); this names the row
        // store::DEFAULT_SPACE_ID seeds. Honest constant, not sample data.
        space_name: "Red Badger".into(),
        space_initials: "RB".into(),
        today_label: CivilDate::parse(&model.today)
            .map(|d| d.short_label())
            .unwrap_or_default(),
        views: vec![
            view_row("now", "Now", 0),
            view_row("next", "Next · This week", 0),
            view_row("later", "Later", 0),
            view_row("waiting", "Waiting on", 0),
            // Every task is inbox until buckets exist (Phase 2).
            view_row("inbox", "Inbox", model.tasks.len() as u64),
        ],
        projects: Vec::new(),
        people: Vec::new(),
        pages: Vec::new(),
    }
}

fn build_calendar(model: &Model) -> CalendarVm {
    let (year, month) = (model.calendar_year, model.calendar_month);
    if !(1..=12).contains(&month) {
        return CalendarVm::default(); // pre-Startup: nothing to draw
    }
    let first = CivilDate {
        year,
        month,
        day: 1,
    };
    let mut cells = Vec::with_capacity(37);
    for _ in 0..first.weekday() {
        cells.push(CalendarCellVm::default()); // leading blanks (day 0)
    }
    for day in 1..=civil::days_in_month(year, month) {
        let date = CivilDate { year, month, day };
        let iso = date.iso();
        cells.push(CalendarCellVm {
            day: day as u8,
            is_today: iso == model.today,
            is_selected: iso == model.selected_date,
            is_weekend: date.weekday() >= 5,
            date: iso,
        });
    }
    CalendarVm {
        month_label: civil::month_label(year, month),
        cells,
    }
}

fn build_day(model: &Model) -> DayVm {
    DayVm {
        date: model.selected_date.clone(),
        title: CivilDate::parse(&model.selected_date)
            .map(|d| d.display_title())
            .unwrap_or_else(|| model.selected_date.clone()),
        note_text: model.note_text.clone(),
        editor_version: model.editor_version,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::storage::{BlockData, DayData, StorageOperation, StorageResult, Task};

    const TODAY: &str = "2026-07-04";

    fn started() -> (Daily, Model) {
        let app = Daily;
        let mut model = Model::default();
        let _ = app.update(
            Event::Startup {
                today: TODAY.into(),
            },
            &mut model,
        );
        (app, model)
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
        let app = Daily;
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
        assert!(ops.contains(&StorageOperation::ListTasks));
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
    fn tasks_feed_the_inbox_count_and_task_flow_still_works() {
        let (app, mut model) = started();
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
            Event::TaskSaved(StorageResult::Task(Task {
                id: "t1".into(),
                title: "Ship it".into(),
            })),
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
