//! The sidebar: space identity, the Views rows and their live counts.

use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::app::Model;
use crate::civil::CivilDate;
use crate::task::{Bucket, Status, is_open};

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

pub fn build_sidebar(model: &Model) -> SidebarVm {
    let view_row = |kind: &str, label: &str, count: u64| ViewRowVm {
        kind: kind.into(),
        label: label.into(),
        count,
    };
    // Counts are outstanding work: done and binned tasks are finished with,
    // so they are not something the row is asking the user to look at.
    let open_in = |bucket: Bucket| {
        model
            .tasks
            .iter()
            .filter(|t| t.bucket == bucket && is_open(t.status))
            .count() as u64
    };
    let waiting = model
        .tasks
        .iter()
        .filter(|t| t.status == Status::Waiting)
        .count() as u64;
    SidebarVm {
        // Single space until Phase 6 (spec §10); this names the row
        // store::DEFAULT_SPACE_ID seeds. Honest constant, not sample data.
        space_name: "Red Badger".into(),
        space_initials: "RB".into(),
        today_label: CivilDate::parse(&model.today)
            .map(|d| d.short_label())
            .unwrap_or_default(),
        views: vec![
            view_row("now", "Now", open_in(Bucket::Now)),
            view_row("next", "Next · This week", open_in(Bucket::Next)),
            view_row("later", "Later", open_in(Bucket::Later)),
            view_row("waiting", "Waiting on", waiting),
            view_row("inbox", "Inbox", open_in(Bucket::Inbox)),
            // Every open task in the space, whatever its bucket — the row
            // that reaches the All-actions surface (plan Task 9 Step 4).
            view_row(
                "all",
                "All actions",
                model.tasks.iter().filter(|t| is_open(t.status)).count() as u64,
            ),
        ],
        projects: Vec::new(),
        people: Vec::new(),
        pages: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::storage::TaskData;
    use crate::task::{Bucket, Status};

    fn t(bucket: Bucket, status: Status) -> TaskData {
        TaskData {
            id: "x".into(),
            title: "x".into(),
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

    fn counts(tasks: Vec<TaskData>) -> Vec<(String, u64)> {
        let model = Model {
            today: "2026-07-04".into(),
            tasks,
            ..Model::default()
        };
        build_sidebar(&model)
            .views
            .into_iter()
            .map(|r| (r.kind, r.count))
            .collect()
    }

    #[test]
    fn views_rows_count_open_work_per_bucket_and_status() {
        let rows = counts(vec![
            t(Bucket::Now, Status::Backlog),
            t(Bucket::Now, Status::Done),
            t(Bucket::Next, Status::Backlog),
            t(Bucket::Later, Status::Backlog),
            t(Bucket::Later, Status::Binned),
            t(Bucket::Inbox, Status::Backlog),
            t(Bucket::Next, Status::Waiting),
        ]);
        assert_eq!(
            rows,
            vec![
                ("now".to_string(), 1),
                ("next".to_string(), 2),
                ("later".to_string(), 1),
                ("waiting".to_string(), 1),
                ("inbox".to_string(), 1),
                ("all".to_string(), 5),
            ],
            "counts are open work: done and binned are not outstanding, and \
             Waiting counts in both its bucket and the Waiting row; All \
             actions counts every open task across every bucket"
        );
    }

    #[test]
    fn an_empty_database_shows_zeroes_and_no_projects_people_or_pages() {
        let rows = counts(vec![]);
        assert!(rows.iter().all(|(_, count)| *count == 0));
        let model = Model::default();
        let sidebar = build_sidebar(&model);
        assert!(sidebar.projects.is_empty(), "pages arrive in Phase 3");
        assert!(sidebar.people.is_empty());
        assert!(sidebar.pages.is_empty());
    }
}
