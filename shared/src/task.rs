//! Pure task-domain helpers. No I/O, no clocks: "today" is always a
//! parameter, supplied by the model from `Event::Startup`.

use crate::civil::CivilDate;
use crate::effects::storage::{Status, TaskData};

/// Open = not terminal. Done and Binned are the two terminal states
/// (core-journeys Journey 5: Binned is "dropped", distinct from Done).
#[must_use]
pub fn is_open(status: Status) -> bool {
    !matches!(status, Status::Done | Status::Binned)
}

/// Whole days between entering the Now bucket and `today`. `None` when the
/// task never entered Now, when the stored date is unparseable, or when it
/// is in the future — all three render as "no age label", never as a
/// negative or a zero that lies.
#[must_use]
pub fn age_in_days(entered_now_on: &str, today: &str) -> Option<i64> {
    let from = CivilDate::parse(entered_now_on)?;
    let to = CivilDate::parse(today)?;
    let days = to.days_since_epoch() - from.days_since_epoch();
    (days >= 0).then_some(days)
}

/// Ordering for every open list: done rows last, then priority 1→3 with
/// "no priority" after 3, then oldest first (spec §7).
#[must_use]
pub fn sort_key(task: &TaskData, today: &str) -> (bool, u8, i64) {
    let done_last = task.status == Status::Done;
    let priority_rank = if task.priority == 0 { 4 } else { task.priority };
    let age = age_in_days(&task.entered_now_on, today).unwrap_or(0);
    (done_last, priority_rank, -age)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::storage::{Bucket, Status, TaskData};

    fn task(priority: u8, entered: &str, status: Status) -> TaskData {
        TaskData {
            id: "id".into(),
            title: "t".into(),
            bucket: Bucket::Now,
            status,
            priority,
            due: String::new(),
            prev_status: None,
            blocked_reason: String::new(),
            source: "quick_add".into(),
            entered_now_on: entered.into(),
            done_on: String::new(),
            created_at: 0,
        }
    }

    #[test]
    fn open_covers_everything_except_done_and_binned() {
        assert!(is_open(Status::Backlog));
        assert!(is_open(Status::InProgress));
        assert!(is_open(Status::Blocked));
        assert!(is_open(Status::Waiting));
        assert!(!is_open(Status::Done));
        assert!(!is_open(Status::Binned));
    }

    #[test]
    fn age_counts_whole_days_from_entering_now() {
        assert_eq!(age_in_days("2026-07-02", "2026-07-04"), Some(2));
        assert_eq!(age_in_days("2026-07-04", "2026-07-04"), Some(0));
        assert_eq!(age_in_days("", "2026-07-04"), None, "never entered Now");
        assert_eq!(age_in_days("garbage", "2026-07-04"), None);
        assert_eq!(
            age_in_days("2026-07-05", "2026-07-04"),
            None,
            "a future origin is not negative age, it is no age"
        );
    }

    #[test]
    fn sort_is_priority_first_then_oldest_then_done_last() {
        // Spec §7: "strict P1-first, then age". Priority 0 (none) sorts after
        // 3, and done rows sink below every open row (reference §7.2 row 4).
        let mut rows = vec![
            task(0, "2026-07-01", Status::Backlog),
            task(2, "2026-07-03", Status::Backlog),
            task(1, "2026-07-03", Status::Backlog),
            task(2, "2026-07-01", Status::Backlog),
            task(1, "2026-07-02", Status::Done),
        ];
        rows.sort_by_key(|t| sort_key(t, "2026-07-04"));
        let shape: Vec<(u8, &str, bool)> = rows
            .iter()
            .map(|t| {
                (
                    t.priority,
                    t.entered_now_on.as_str(),
                    t.status == Status::Done,
                )
            })
            .collect();
        assert_eq!(
            shape,
            vec![
                (1, "2026-07-03", false),
                (2, "2026-07-01", false), // older beats newer at equal priority
                (2, "2026-07-03", false),
                (0, "2026-07-01", false), // no priority sorts last among open
                (1, "2026-07-02", true),  // done sinks regardless of priority
            ]
        );
    }
}
