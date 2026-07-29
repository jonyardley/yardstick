//! The mini month calendar (reference §2.3).

use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::app::Model;
use crate::civil::{self, CivilDate};

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

pub fn build_calendar(model: &Model) -> CalendarVm {
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
