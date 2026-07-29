//! `TaskData` → `TaskRowVm`: every display string a row needs, computed
//! once in the core (spec §4). The shell renders; it never formats domain
//! data. The one exception, recorded: wall-clock times of day (Phase 4).

use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::civil::CivilDate;
use crate::effects::storage::TaskData;
use crate::task::{Bucket, Status, age_in_days};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TaskRowVm {
    pub id: String,
    pub title: String,
    /// "open" | "in_progress" | "done" (reference §7.2 checkbox states).
    pub checkbox: String,
    /// 1–3, or 0 for no badge.
    pub priority: u8,
    /// "" when the status is unremarkable; otherwise the pill's text.
    pub status_pill: String,
    /// Which tint the pill takes: "in_progress" | "blocked" | "waiting" | "binned".
    pub status_kind: String,
    /// Project/person chips — empty until Phase 3 (carve-out 1).
    pub chips: Vec<String>,
    /// The 70px right column: age, provenance, or due weekday.
    pub meta: String,
    pub is_done: bool,
    pub blocked_reason: String,
    /// Raw value the triage sheet edits (Task 7): which WHEN bucket the
    /// task is currently in, so the sheet opens on today's value, not a
    /// default.
    pub bucket: Bucket,
    /// Raw 'YYYY-MM-DD' or "" — the triage sheet's DUE field opens on this.
    pub due: String,
}

/// Source tag copy (core-journeys Journey 1A). An unknown source shows its
/// raw value rather than being swallowed — a new capture point should be
/// visible the moment it starts writing, not silently unlabelled.
fn source_label(source: &str) -> String {
    match source {
        "quick_add" => "quick add".into(),
        "note" => "from note".into(),
        "menu_bar" => "menu bar".into(),
        "mcp" => "from an agent".into(),
        other => other.into(),
    }
}

fn age_label(days: i64) -> String {
    if days == 1 {
        "1 day old".into()
    } else {
        format!("{days} days old")
    }
}

/// The 70px meta column, one rule per bucket:
/// - Inbox: always the source tag (nothing else is known yet — Journey 1A).
/// - Now: age once a day has passed, else provenance, else nothing.
/// - Next/Later: the due weekday, else nothing.
///
/// "Provenance" on a Now row means a source worth naming: reference §7.2 row 2
/// shows `from Slack`, row 3 shows an **empty** 70px spacer. `quick_add` is the
/// default in-app path, so it is row 3's case — saying "quick add" beside a
/// task the user typed themselves is noise, exactly as "0 days old" is.
fn meta(task: &TaskData, today: &str) -> String {
    match task.bucket {
        Bucket::Inbox => source_label(&task.source),
        Bucket::Now => match age_in_days(&task.entered_now_on, today) {
            Some(days) if days >= 1 => age_label(days),
            _ if task.source == "quick_add" => String::new(),
            _ => source_label(&task.source),
        },
        Bucket::Next | Bucket::Later => CivilDate::parse(&task.due)
            .map(|d| d.weekday_short())
            .unwrap_or_default(),
    }
}

#[must_use]
pub fn build_row(task: &TaskData, today: &str) -> TaskRowVm {
    let (pill, kind) = match task.status {
        Status::InProgress => ("In progress", "in_progress"),
        Status::Blocked => ("Blocked", "blocked"),
        Status::Waiting => ("Waiting", "waiting"),
        Status::Binned => ("Binned", "binned"),
        Status::Backlog | Status::Done => ("", ""),
    };
    TaskRowVm {
        id: task.id.clone(),
        title: task.title.clone(),
        checkbox: match task.status {
            Status::Done => "done".into(),
            Status::InProgress => "in_progress".into(),
            _ => "open".into(),
        },
        priority: task.priority,
        status_pill: pill.into(),
        status_kind: kind.into(),
        chips: Vec::new(),
        meta: meta(task, today),
        is_done: task.status == Status::Done,
        blocked_reason: task.blocked_reason.clone(),
        bucket: task.bucket,
        due: task.due.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Bucket, Status};

    fn task(bucket: Bucket, status: Status) -> TaskData {
        TaskData {
            id: "t1".into(),
            title: "Give probation feedback on Thabang to Pieter".into(),
            bucket,
            status,
            priority: 0,
            due: String::new(),
            prev_status: None,
            blocked_reason: String::new(),
            source: "quick_add".into(),
            entered_now_on: String::new(),
            done_on: String::new(),
            created_at: 0,
        }
    }

    const TODAY: &str = "2026-07-04";

    #[test]
    fn checkbox_follows_status_not_bucket() {
        assert_eq!(
            build_row(&task(Bucket::Now, Status::Backlog), TODAY).checkbox,
            "open"
        );
        assert_eq!(
            build_row(&task(Bucket::Now, Status::InProgress), TODAY).checkbox,
            "in_progress",
            "reference §7.2 row 1: blue ring with a soft filled centre"
        );
        assert_eq!(
            build_row(&task(Bucket::Now, Status::Done), TODAY).checkbox,
            "done"
        );
        assert_eq!(
            build_row(&task(Bucket::Now, Status::Blocked), TODAY).checkbox,
            "open",
            "blocked is carried by the pill, not by the checkbox"
        );
    }

    #[test]
    fn done_rows_are_flagged_for_dimming_and_strikethrough() {
        let row = build_row(&task(Bucket::Now, Status::Done), TODAY);
        assert!(row.is_done);
        assert_eq!(
            row.status_pill, "",
            "a struck-through row needs no Done pill"
        );
    }

    #[test]
    fn notable_statuses_get_a_pill_and_ordinary_ones_do_not() {
        assert_eq!(
            build_row(&task(Bucket::Now, Status::Backlog), TODAY).status_pill,
            ""
        );
        assert_eq!(
            build_row(&task(Bucket::Now, Status::InProgress), TODAY).status_pill,
            "In progress"
        );
        assert_eq!(
            build_row(&task(Bucket::Now, Status::Blocked), TODAY).status_pill,
            "Blocked"
        );
        assert_eq!(
            build_row(&task(Bucket::Now, Status::Waiting), TODAY).status_pill,
            "Waiting"
        );
        assert_eq!(
            build_row(&task(Bucket::Now, Status::Binned), TODAY).status_pill,
            "Binned"
        );
    }

    #[test]
    fn a_blocked_row_carries_its_reason_for_the_board_card() {
        let mut blocked = task(Bucket::Next, Status::Blocked);
        blocked.blocked_reason = "Legal review".into();
        assert_eq!(build_row(&blocked, TODAY).blocked_reason, "Legal review");
    }

    #[test]
    fn now_rows_show_age_once_a_day_has_passed_and_never_zero_days() {
        let mut aged = task(Bucket::Now, Status::Backlog);
        aged.entered_now_on = "2026-07-02".into();
        assert_eq!(build_row(&aged, TODAY).meta, "2 days old");

        aged.entered_now_on = "2026-07-03".into();
        assert_eq!(
            build_row(&aged, TODAY).meta,
            "1 day old",
            "singular, not '1 days'"
        );

        aged.entered_now_on = TODAY.into();
        assert_eq!(
            build_row(&aged, TODAY).meta,
            "",
            "'0 days old' is noise; today's arrivals say nothing"
        );
    }

    #[test]
    fn a_now_row_with_no_age_falls_back_to_provenance() {
        let mut fresh = task(Bucket::Now, Status::Backlog);
        fresh.entered_now_on = TODAY.into();
        fresh.source = "note".into();
        assert_eq!(
            build_row(&fresh, TODAY).meta,
            "from note",
            "reference §7.2 row 2 shows provenance where there is no age"
        );
    }

    #[test]
    fn inbox_rows_always_show_their_source_tag() {
        // core-journeys Journey 1A: every capture carries a visible source.
        for (source, expected) in [
            ("quick_add", "quick add"),
            ("note", "from note"),
            ("menu_bar", "menu bar"),
            ("mcp", "from an agent"),
            ("something_new", "something_new"),
        ] {
            let mut t = task(Bucket::Inbox, Status::Backlog);
            t.source = source.into();
            assert_eq!(build_row(&t, TODAY).meta, expected, "source {source}");
        }
    }

    #[test]
    fn dated_rows_outside_now_show_the_due_weekday() {
        let mut t = task(Bucket::Next, Status::Backlog);
        t.due = "2026-07-31".into();
        assert_eq!(
            build_row(&t, TODAY).meta,
            "Fri",
            "Journey 1C: due dates render as an abbreviated weekday in list rows"
        );
    }

    #[test]
    fn rows_carry_the_raw_values_the_triage_sheet_edits() {
        let mut t = task(Bucket::Next, Status::Backlog);
        t.due = "2026-07-31".into();
        t.priority = 2;
        let row = build_row(&t, TODAY);
        assert_eq!(row.bucket, Bucket::Next);
        assert_eq!(row.due, "2026-07-31", "the sheet opens on the current date");
        assert_eq!(row.priority, 2);
    }

    #[test]
    fn chips_are_empty_until_pages_exist() {
        // Phase 2 carve-out 1: no project/person chips until Phase 3 —
        // the field exists so rows render none, rather than faking any.
        assert!(
            build_row(&task(Bucket::Now, Status::Backlog), TODAY)
                .chips
                .is_empty()
        );
    }
}
