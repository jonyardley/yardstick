//! Per-surface ViewModel builders. `app.rs` owns the model and `update()`;
//! everything that turns model state into display data lives here.

pub mod sidebar;
pub mod task_list;
pub mod task_row;

mod calendar;
mod day;

pub use calendar::{CalendarCellVm, CalendarVm, build_calendar};
pub use day::{DayVm, build_day};
pub use sidebar::{SidebarEntryVm, SidebarVm, ViewRowVm, build_sidebar};
pub use task_list::{CollapsedGroupVm, MomentumVm, TaskGroupVm, TaskListVm, build_list};
pub use task_row::TaskRowVm;
