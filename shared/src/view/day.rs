//! The daily note's header and editor state (reference §5).

use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::app::Model;
use crate::civil::CivilDate;

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct DayVm {
    pub date: String,
    pub title: String,
    pub note_text: String,
    pub editor_version: u64,
}

pub fn build_day(model: &Model) -> DayVm {
    DayVm {
        date: model.selected_date.clone(),
        title: CivilDate::parse(&model.selected_date)
            .map(|d| d.display_title())
            .unwrap_or_else(|| model.selected_date.clone()),
        note_text: model.note_text.clone(),
        editor_version: model.editor_version,
    }
}
