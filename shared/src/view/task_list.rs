//! One builder for every task surface (plan decision #3): Now, Inbox, the
//! bucket views, Waiting on, and All actions are the same pure function over
//! `Model.tasks` with a different route.

use facet::Facet;
use serde::{Deserialize, Serialize};

use super::task_row::{TaskRowVm, build_row};
use crate::effects::storage::TaskData;
use crate::task::{Bucket, Status, is_open, sort_key};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TaskListVm {
    pub title: String,
    pub subtitle: String,
    pub groups: Vec<TaskGroupVm>,
    /// Only the Now list carries the momentum cue (reference §7.1).
    pub momentum: Option<MomentumVm>,
    /// Journey 5B: Backlog and Binned collapse to counts.
    pub collapsed: Vec<CollapsedGroupVm>,
    pub group_by: String,
    pub filter_bucket: String,
    pub filter_status: String,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct TaskGroupVm {
    pub label: String,
    /// "" for ungrouped; otherwise the status or bucket key, for tinting.
    pub kind: String,
    pub count: u64,
    pub rows: Vec<TaskRowVm>,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct MomentumVm {
    pub done: u64,
    pub remaining: u64,
    pub label: String,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct CollapsedGroupVm {
    pub label: String,
    pub count: u64,
}

fn bucket_key(b: Bucket) -> &'static str {
    match b {
        Bucket::Inbox => "inbox",
        Bucket::Now => "now",
        Bucket::Next => "next",
        Bucket::Later => "later",
    }
}

fn bucket_label(b: Bucket) -> &'static str {
    match b {
        Bucket::Inbox => "Inbox",
        Bucket::Now => "Now",
        Bucket::Next => "Next",
        Bucket::Later => "Later",
    }
}

fn status_key(s: Status) -> &'static str {
    match s {
        Status::Backlog => "backlog",
        Status::InProgress => "in_progress",
        Status::Blocked => "blocked",
        Status::Waiting => "waiting",
        Status::Done => "done",
        Status::Binned => "binned",
    }
}

/// Verbatim from core-journeys Journey 5A.
fn status_label(s: Status) -> &'static str {
    match s {
        Status::Backlog => "Backlog",
        Status::InProgress => "In progress",
        Status::Blocked => "Blocked",
        Status::Waiting => "Waiting",
        Status::Done => "Done",
        Status::Binned => "Binned",
    }
}

fn group(label: &str, kind: &str, tasks: &[&TaskData], today: &str) -> TaskGroupVm {
    let mut sorted: Vec<&TaskData> = tasks.to_vec();
    sorted.sort_by_key(|t| sort_key(t, today));
    TaskGroupVm {
        label: label.into(),
        kind: kind.into(),
        count: sorted.len() as u64,
        rows: sorted.iter().map(|t| build_row(t, today)).collect(),
    }
}

#[must_use]
pub fn build_list(
    route: &str,
    tasks: &[TaskData],
    today: &str,
    group_by: &str,
    filter_bucket: &str,
    filter_status: &str,
) -> TaskListVm {
    let base = TaskListVm {
        group_by: group_by.into(),
        filter_bucket: filter_bucket.into(),
        filter_status: filter_status.into(),
        ..TaskListVm::default()
    };

    // A single bucket or status view: one group, no collapsing.
    let simple = |title: &str, subtitle: &str, rows: Vec<&TaskData>| TaskListVm {
        title: title.into(),
        subtitle: subtitle.into(),
        groups: vec![group("", "", &rows, today)],
        ..base.clone()
    };

    match route {
        "now" => {
            // Principle 6: done rows stay for the rest of the day, and drive
            // the momentum cue.
            let rows: Vec<&TaskData> = tasks
                .iter()
                .filter(|t| t.bucket == Bucket::Now && t.status != Status::Binned)
                .collect();
            let done = rows.iter().filter(|t| t.status == Status::Done).count() as u64;
            let remaining = rows.len() as u64 - done;
            let mut list = simple("Now", "Today", rows);
            list.momentum = Some(MomentumVm {
                done,
                remaining,
                label: format!("{done} done · {remaining} to go"),
            });
            list
        }
        "next" => simple(
            "Next",
            "This week",
            tasks
                .iter()
                .filter(|t| t.bucket == Bucket::Next && is_open(t.status))
                .collect(),
        ),
        "later" => simple(
            "Later",
            "",
            tasks
                .iter()
                .filter(|t| t.bucket == Bucket::Later && is_open(t.status))
                .collect(),
        ),
        "inbox" => simple(
            "Inbox",
            "Captured today · unsorted",
            tasks
                .iter()
                .filter(|t| t.bucket == Bucket::Inbox && is_open(t.status))
                .collect(),
        ),
        "waiting" => simple(
            "Waiting on",
            "",
            tasks
                .iter()
                .filter(|t| t.status == Status::Waiting)
                .collect(),
        ),
        "all" => {
            let kept: Vec<&TaskData> = tasks
                .iter()
                .filter(|t| filter_bucket.is_empty() || bucket_key(t.bucket) == filter_bucket)
                .filter(|t| filter_status.is_empty() || status_key(t.status) == filter_status)
                .collect();
            let mut list = TaskListVm {
                title: "All actions".into(),
                subtitle: String::new(),
                ..base
            };
            match group_by {
                "bucket" => {
                    for b in [Bucket::Inbox, Bucket::Now, Bucket::Next, Bucket::Later] {
                        let rows: Vec<&TaskData> =
                            kept.iter().copied().filter(|t| t.bucket == b).collect();
                        list.groups
                            .push(group(bucket_label(b), bucket_key(b), &rows, today));
                    }
                }
                "none" => {
                    list.groups.push(group("", "", &kept, today));
                }
                _ => {
                    // Journey 5B's four columns, then the two collapsed counts.
                    for s in [
                        Status::InProgress,
                        Status::Blocked,
                        Status::Waiting,
                        Status::Done,
                    ] {
                        let rows: Vec<&TaskData> =
                            kept.iter().copied().filter(|t| t.status == s).collect();
                        list.groups
                            .push(group(status_label(s), status_key(s), &rows, today));
                    }
                    for s in [Status::Backlog, Status::Binned] {
                        list.collapsed.push(CollapsedGroupVm {
                            label: status_label(s).into(),
                            count: kept.iter().filter(|t| t.status == s).count() as u64,
                        });
                    }
                }
            }
            list
        }
        // "today" and anything unrecognised: the Today column draws the note
        // plus the Now list, which it fetches under the "now" route.
        _ => TaskListVm { ..base },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::storage::TaskData;
    use crate::task::{Bucket, Status};

    const TODAY: &str = "2026-07-04";

    fn t(id: &str, bucket: Bucket, status: Status, priority: u8) -> TaskData {
        TaskData {
            id: id.into(),
            title: format!("task {id}"),
            bucket,
            status,
            priority,
            due: String::new(),
            prev_status: None,
            blocked_reason: String::new(),
            source: "quick_add".into(),
            entered_now_on: String::new(),
            done_on: String::new(),
            created_at: 0,
        }
    }

    fn ids(group: &TaskGroupVm) -> Vec<&str> {
        group.rows.iter().map(|r| r.id.as_str()).collect()
    }

    #[test]
    fn the_now_list_is_titled_from_the_reference_and_orders_by_priority_then_age() {
        let tasks = vec![
            t("none", Bucket::Now, Status::Backlog, 0),
            t("p2", Bucket::Now, Status::Backlog, 2),
            t("p1", Bucket::Now, Status::Backlog, 1),
            t("elsewhere", Bucket::Next, Status::Backlog, 1),
        ];
        let list = build_list("now", &tasks, TODAY, "status", "", "");
        assert_eq!(list.title, "Now");
        assert_eq!(list.subtitle, "Today", "reference §7.1");
        assert_eq!(list.groups.len(), 1);
        assert_eq!(ids(&list.groups[0]), vec!["p1", "p2", "none"]);
    }

    #[test]
    fn done_now_tasks_stay_visible_at_the_bottom_and_feed_the_momentum_cue() {
        // Principle 6: done stays visible for the rest of the day.
        let tasks = vec![
            t("open1", Bucket::Now, Status::Backlog, 0),
            t("done1", Bucket::Now, Status::Done, 1),
            t("open2", Bucket::Now, Status::Backlog, 0),
            t("open3", Bucket::Now, Status::Backlog, 0),
        ];
        let list = build_list("now", &tasks, TODAY, "status", "", "");
        assert_eq!(ids(&list.groups[0]).last(), Some(&"done1"));
        let momentum = list.momentum.expect("the Now list has a momentum cue");
        assert_eq!(momentum.done, 1);
        assert_eq!(momentum.remaining, 3);
        assert_eq!(
            momentum.label, "1 done · 3 to go",
            "reference §7.1 verbatim"
        );
    }

    #[test]
    fn only_the_now_list_has_a_momentum_cue() {
        let tasks = vec![t("a", Bucket::Inbox, Status::Backlog, 0)];
        assert!(
            build_list("inbox", &tasks, TODAY, "status", "", "")
                .momentum
                .is_none()
        );
        assert!(
            build_list("all", &tasks, TODAY, "status", "", "")
                .momentum
                .is_none()
        );
    }

    #[test]
    fn the_inbox_says_captured_today_unsorted_and_holds_only_untriaged_tasks() {
        let tasks = vec![
            t("in1", Bucket::Inbox, Status::Backlog, 0),
            t("now1", Bucket::Now, Status::Backlog, 0),
        ];
        let list = build_list("inbox", &tasks, TODAY, "status", "", "");
        assert_eq!(list.title, "Inbox");
        assert_eq!(
            list.subtitle, "Captured today · unsorted",
            "Journey 1A verbatim"
        );
        assert_eq!(ids(&list.groups[0]), vec!["in1"]);
    }

    #[test]
    fn binned_and_done_tasks_never_appear_in_a_bucket_list() {
        let tasks = vec![
            t("open", Bucket::Next, Status::Backlog, 0),
            t("binned", Bucket::Next, Status::Binned, 0),
            t("done", Bucket::Next, Status::Done, 0),
        ];
        let list = build_list("next", &tasks, TODAY, "status", "", "");
        assert_eq!(
            ids(&list.groups[0]),
            vec!["open"],
            "only the Now list keeps done rows visible (principle 6 is about today)"
        );
        assert_eq!(list.subtitle, "This week", "Journey 1C verbatim");
    }

    #[test]
    fn the_waiting_view_is_a_status_query_across_every_bucket() {
        let tasks = vec![
            t("w1", Bucket::Now, Status::Waiting, 0),
            t("w2", Bucket::Later, Status::Waiting, 0),
            t("other", Bucket::Now, Status::Blocked, 0),
        ];
        let list = build_list("waiting", &tasks, TODAY, "status", "", "");
        assert_eq!(list.title, "Waiting on");
        let mut found = ids(&list.groups[0]);
        found.sort_unstable();
        assert_eq!(found, vec!["w1", "w2"]);
    }

    #[test]
    fn all_actions_groups_by_status_with_backlog_and_binned_collapsed() {
        // Journey 5B: four columns; Backlog and Binned collapse to counts.
        let tasks = vec![
            t("ip", Bucket::Now, Status::InProgress, 0),
            t("bl", Bucket::Next, Status::Blocked, 0),
            t("wa", Bucket::Later, Status::Waiting, 0),
            t("dn", Bucket::Now, Status::Done, 0),
            t("bk1", Bucket::Inbox, Status::Backlog, 0),
            t("bk2", Bucket::Inbox, Status::Backlog, 0),
            t("bn", Bucket::Later, Status::Binned, 0),
        ];
        let list = build_list("all", &tasks, TODAY, "status", "", "");
        assert_eq!(list.title, "All actions");
        let labels: Vec<&str> = list.groups.iter().map(|g| g.label.as_str()).collect();
        assert_eq!(labels, vec!["In progress", "Blocked", "Waiting", "Done"]);
        let collapsed: Vec<(&str, u64)> = list
            .collapsed
            .iter()
            .map(|c| (c.label.as_str(), c.count))
            .collect();
        assert_eq!(collapsed, vec![("Backlog", 2), ("Binned", 1)]);
    }

    #[test]
    fn all_actions_can_group_by_bucket_instead() {
        let tasks = vec![
            t("n", Bucket::Now, Status::Backlog, 0),
            t("e", Bucket::Next, Status::Backlog, 0),
            t("i", Bucket::Inbox, Status::Backlog, 0),
        ];
        let list = build_list("all", &tasks, TODAY, "bucket", "", "");
        let labels: Vec<&str> = list.groups.iter().map(|g| g.label.as_str()).collect();
        assert_eq!(labels, vec!["Inbox", "Now", "Next", "Later"]);
        assert!(
            list.collapsed.is_empty(),
            "collapsing is a status-grouping concept only"
        );
        assert_eq!(
            list.groups[3].count, 0,
            "an empty bucket group still reports zero"
        );
    }

    #[test]
    fn all_actions_filters_narrow_the_same_grouping() {
        let tasks = vec![
            t("a", Bucket::Now, Status::Blocked, 0),
            t("b", Bucket::Next, Status::Blocked, 0),
            t("c", Bucket::Now, Status::Waiting, 0),
        ];
        let by_bucket = build_list("all", &tasks, TODAY, "status", "now", "");
        let rows: Vec<&str> = by_bucket.groups.iter().flat_map(ids).collect();
        assert_eq!(rows, vec!["a", "c"], "bucket filter");

        let by_status = build_list("all", &tasks, TODAY, "status", "", "blocked");
        let rows: Vec<&str> = by_status.groups.iter().flat_map(ids).collect();
        assert_eq!(rows, vec!["a", "b"], "status filter");

        let both = build_list("all", &tasks, TODAY, "status", "now", "blocked");
        let rows: Vec<&str> = both.groups.iter().flat_map(ids).collect();
        assert_eq!(rows, vec!["a"], "filters compose");
    }

    #[test]
    fn all_actions_with_no_grouping_is_one_flat_list() {
        let tasks = vec![
            t("p1", Bucket::Now, Status::Backlog, 1),
            t("p3", Bucket::Later, Status::Backlog, 3),
        ];
        let list = build_list("all", &tasks, TODAY, "none", "", "");
        assert_eq!(list.groups.len(), 1);
        assert_eq!(list.groups[0].label, "");
        assert_eq!(
            ids(&list.groups[0]),
            vec!["p1", "p3"],
            "priority order holds"
        );
    }

    #[test]
    fn an_unknown_route_is_an_empty_list_not_a_panic() {
        let list = build_list("nonsense", &[], TODAY, "status", "", "");
        assert!(list.groups.iter().all(|g| g.rows.is_empty()));
    }
}
