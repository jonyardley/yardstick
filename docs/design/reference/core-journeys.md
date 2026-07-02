# Behavioral Spec Extraction — "Core Journeys" Design Storyboard

**Source:** `/private/tmp/claude-501/-Users-jonyardley-Dev-Yardstick/769b99c0-250d-4b05-b8a9-bdf8cdaa28ad/scratchpad/design/design_handoff_daily_focus_app/Core Journeys.dc.html` (622 lines; `support.js` is only a rendering shim, contains no spec content).

---

## 0. Document-level framing

**Eyebrow label (verbatim):** "Daily · Core journeys"
**Title (verbatim):** "How the app works, journey by journey"
**Intro (verbatim):** "Five flows that cover every feature — each shown as its key moments with notes on the mechanics. Same macOS shell throughout; the persistent focus bar from **1a** is the home base."

Behavioral implications:

- Every frame is rendered inside the same macOS window shell (traffic-light buttons red/yellow/green, titled title bar with gradient). The app is a native-style macOS app.
- The **focus bar is persistent app-wide** and is described as "the home base." (Note: the intro says "from 1a" but the focus bar actually appears in Journey 3 — a labeling inconsistency in the doc; see Open Questions.)
- These five journeys are declared to "cover every feature" — anything not shown here is out of scope.

---

## Journey 1 — Triage: "from Inbox to the right bucket"

**Journey subtitle (verbatim):** "Anything you capture lands in Inbox unsorted. Triage assigns four things: **when**, **priority**, **project/person**, **due date**."

### Frame 1A — Inbox

**Window title bar:** "Inbox" plus a **red count badge "4"** (white text on red pill) directly in the window chrome — the Inbox count is surfaced in the title bar.

**Section header microcopy (verbatim):** "Captured today · unsorted" — implies Inbox groups/labels items by capture day and explicitly signals no ordering/metadata.

**Contents — 4 untriaged tasks**, each rendered as: empty circular checkbox (gray 1.5px ring) + task title + a right-aligned **source tag** (small gray pill):

| Task title | Source tag |
|---|---|
| Finalize vendor contract | `⌘Space` |
| Reply to Sarah re: offsite | `from note` |
| Book dentist | `⌘Space` |
| Research competitors | `menu bar` |

The first row ("Finalize vendor contract") is shown **selected/highlighted** (light blue tint background + blue border) — the item about to be triaged.

**Annotation 1 (verbatim):** "Every capture — global **⌘Space**, a note, the menu-bar window — drops here with a source tag. No decisions required at capture time."

Behavioral requirements extracted:
1. **Three capture entry points:** (a) global hotkey **⌘Space** (quick-capture from anywhere in macOS), (b) capture from within a note, (c) a **menu-bar window** (standalone menu-bar extra).
2. All captures land in Inbox with **zero required metadata** — no when/priority/project/due at capture time.
3. Every Inbox item persists a **provenance/source tag** (`⌘Space`, `from note`, `menu bar`) displayed on the row.
4. Inbox displays an unread/untriaged **count badge**.
5. Inbox items are checkbox-bearing tasks even before triage (they can presumably be completed directly, though not shown).

### Frame 1B — Triage sheet

**Window title bar (verbatim):** "Triage · Finalize vendor contract" — the sheet is titled with the task name.

**Sheet contents, top to bottom:**
1. Task title repeated as heading: "Finalize vendor contract".
2. **WHEN** (uppercase field label) — a 3-way segmented control: `Now | Next | Later`. **"Next" is selected** (white text on blue segment); Now and Later unselected (gray text on light gray track).
3. **PRIORITY** (uppercase label) — three square toggle buttons `1 | 2 | 3`. **"1" is selected**, rendered **red** (priority-1 = red); 2 and 3 unselected gray.
4. **DUE** (uppercase label) — a date field showing "**Fri, Jul 4**" (format: abbreviated weekday, abbreviated month, day number).
5. **PROJECT / PERSON** (uppercase label) — a token field containing a selected **project chip "Product Planning"** (blue text on light-blue pill, with a small blue square color swatch), followed by a ghost hint "**#…**" indicating you can keep typing `#` to add more links. Field accepts both projects (#) and people (@).

**Annotation 2 (verbatim):** "One lightweight sheet sets all four attributes. Keyboard-driven — **N/E/L** for when, **1/2/3** for priority, **#** to link."

Behavioral requirements:
1. Triage is a **single lightweight sheet/popover** covering all four attributes for one task.
2. **Keyboard shortcuts inside triage:** `N` = Now, `E` = Next, `L` = Later; `1`/`2`/`3` = priority; `#` = open project/person linker. (Note `E` for Next — presumably because N is taken by Now.)
3. Priority is a 3-level scale (1 highest). Color coding: 1 = red, 2 = amber/orange, 3 = gray (colors confirmed across Journeys 1C/3A/3C).
4. Due date is a fourth, independent attribute (distinct from the Now/Next/Later "when" bucket).
5. Project/person linking from triage produces the same link/backlink as an inline `#`/`@` mention (per Frame 1C annotation and Journey 4).
6. Multiple links per task are possible (the "#…" affordance).

### Frame 1C — Result: task lands in "Next"

**Window title bar:** "Today" — the main list view is called Today.

**Section header:** bold "**Next**" with the sublabel "**This week**" (baseline-aligned gray caption). (Journey 3A shows the companion: "**Now**" with sublabel "**Today**". So the buckets carry timeframe semantics: **Now = Today, Next = This week**, Later = beyond, by extension.)

**Row 1 (the just-triaged task, highlighted with a green tint + green border — a "just landed here" success highlight):**
- Empty circular checkbox
- Title: "Finalize vendor contract"
- **Priority badge:** red rounded-square "1"
- **Project chip:** gray pill "Product Planning"
- **Due chip:** plain gray text "**Fri**" (due dates render as abbreviated weekday in list rows)

**Row 2 (pre-existing Next task, unhighlighted):**
- "1:1 prep with Marcus" · gray priority badge "3" · **person chip "@Marcus Reed"** (tasks can be linked to a person instead of a project; person chips render with `@` prefix) · due "Wed"

**Annotation 3 (verbatim):** "It leaves the Inbox and lands under **Next** with its chips. Because it&rsquo;s linked, it also now appears on the Product Planning page (Journey 4)."

Behavioral requirements:
1. Completing triage **removes the item from Inbox** (badge count decrements) and files it under the chosen When bucket in the Today view.
2. All assigned attributes render as **chips on the row**: priority badge, project/person pill, weekday due chip.
3. Linking during triage causes the task to **simultaneously appear on the linked project's page** — one task, two live surfaces.
4. A newly-arrived task gets a transient green highlight in its destination list (visual confirmation of where it went).

---

## Journey 2 — Daily note: "write freely, tasks fall out of the text"

**Journey subtitle (verbatim):** "Type a note like any day. **[ ]** makes a task inline, **@** and **#** link people and projects, and the calendar walks you across days."

### Frame 2A — @-mention autocomplete mid-sentence

**Window title bar:** "Tuesday, July 1". **Note heading (H3 in the document body):** "Tuesday, July 1" — daily notes are titled by full weekday + date; the window title mirrors the note title.

**Editor state:** prose paragraph "Standup went fine. Need to loop in **@sa**" with a text cursor (rendered as a muted `|`) immediately after the partial token. The typed token `@sa` is already styled as a mention-in-progress (blue, medium weight).

**Autocomplete popover** anchored directly below the `@sa` token (appears inline in the text, not in a toolbar):
- Item 1 (**highlighted/pre-selected**, blue tint row): circular avatar with initial "S" (pink/magenta) + "**Sarah Chen**" (medium weight)
- Item 2 (unselected): circular avatar "S" (red) + "Sam Ortiz"

The picker is **filtered by the typed substring** ("sa" matches Sarah Chen and Sam Ortiz); the first result is pre-highlighted so Enter/Return would presumably accept it. People have colored initial avatars.

**Following text** "on the launch checklist before Friday." is rendered dimmed/gray — the rest of the sentence de-emphasized while the picker is open.

**Annotation 1 (verbatim):** "Typing **@** or **#** mid-sentence opens an inline picker. The note stays prose — mentions become live links, not form fields."

Behavioral requirements:
1. `@` triggers a people picker; `#` triggers a project picker — both **inline, mid-sentence**, anywhere in note text.
2. Picker filters as-you-type; first match highlighted for keyboard acceptance.
3. Accepted mentions become **live links inside prose** (no structural change to the note; no form fields).

### Frame 2B — `[ ]` inline task conversion

**Same window/note ("Tuesday, July 1").**

**Editor state:**
- Line 1: the resolved sentence — "Standup went fine. Need to loop in **@Sarah Chen** on the launch checklist." The mention now shows the **full resolved name**, styled blue/medium (link styling).
- Line 2: an **inline task block** rendered in place inside the note: blue-tinted rounded card containing:
  - circular checkbox with a **blue** ring (not gray — visually distinct from plain inbox checkboxes)
  - task text: "Draft the launch checklist **#Product Planning**" (project mention rendered as a blue live link inside the task text)
  - right-aligned **green pill "→ Now"** — a destination chip showing which When-bucket the task went to.

**Annotation 2 (verbatim):** "Start a line with **[ ]** and it becomes a real task in place — defaulting to **Now**, and linked to **#Product Planning** so it shows on that page too."

Behavioral requirements:
1. Typing `[ ]` at the **start of a line** converts that line into a real task object, rendered inline in the note (the note remains its home surface).
2. Tasks created this way **default to the "Now" bucket** (they do NOT go to Inbox — contrast with Journey 1's "from note" inbox item; see Open Questions #2).
3. A `#Project` mention inside the task line **links the task to that project**, so it also appears on the project page (Journey 4).
4. The row displays a "→ Now" chip confirming the auto-filing destination.
5. This exact task ("Draft the launch checklist", #Product Planning) reappears in Journey 4B and 5B with status "In progress" — the inline note task is the same object everywhere (live, not a copy).

### Frame 2C — Sidebar calendar / day navigation

**Layout:** split window — a **150px left sidebar** containing a mini month calendar, and the note pane on the right.

**Calendar sidebar details:**
- Header: "**July 2026**" with `‹` `›` previous/next month arrows.
- Day-of-week header row: S M T W T F S (Sunday-first week).
- **July 1** rendered as a **solid blue filled circle** (today / the current note's day).
- **July 8** rendered as a **dashed blue circle outline** (the future day being jumped to — dashed = selected-but-not-today, and/or a future note that doesn't exist yet).
- Other days plain text.

**Right pane (the jumped-to day):**
- Eyebrow label (verbatim): "**Jump to a day**"
- Heading: "**Tue, Jul 8**"
- Body copy (verbatim): "Future note — pre-write agenda items, drop tasks with a due date, and they surface when the day arrives."

**Annotation 3 (verbatim):** "The sidebar calendar navigates days — review a past note or pre-seed a future one. Today is always one click away."

Behavioral requirements:
1. Every day has (or can have) a daily note; the calendar navigates to **past notes (review)** and **future notes (pre-seed)** — future notes are creatable/editable before their date.
2. Tasks dropped into a future note **with a due date** "surface when the day arrives" — i.e., they are scheduled/deferred and appear (in Today view / the daily note) on that date, not before.
3. There is an always-available one-click **return-to-Today** affordance.
4. Visual language: solid filled circle = today; dashed circle = navigated-to non-today day.

---

## Journey 3 — Focus session: "one thing, right now"

**Journey subtitle (verbatim):** "Promote a task to the focus bar, run a timer, and when it&rsquo;s done the bar hands you the next one. The ADHD anchor."

### Frame 3A — Picking a focus task from Now

**Window title bar:** "Now · Today". **Section header:** bold "**Now**" + sublabel "**Today**".

**Row 1 (hover state, blue-tinted with blue border):**
- Empty checkbox + "Write the Q3 planning doc"
- A blue "**Focus**" button (small white dot glyph + label) revealed on the right — this button is the hover affordance replacing the usual chips.

**Row 2 (resting state):**
- "Reply to Sarah re: offsite" (title in muted gray — de-emphasized relative to the hovered row) + **amber priority badge "2"** (confirms priority-2 = amber/orange). No Focus button visible when not hovered.

**Annotation 1 (verbatim):** "Hover any task and hit **Focus** (or press **F**). Only one task can be the focus at a time — that&rsquo;s the point."

Behavioral requirements:
1. Focus is initiated per-task via a **hover-revealed Focus button** or the keyboard shortcut **F** (on the selected/hovered task).
2. **Singleton constraint:** exactly one task may be the focus at any time. (Implies starting focus on task B while A is focused must replace/stop A — mechanism unspecified.)
3. "Hover **any** task" — focusing is not restricted to the Now bucket, even though the flow starts there.

### Frame 3B — Focus bar running

**Window title bar:** "Today".

**Focus bar** (blue-tinted rounded card pinned at top of content):
- A **pulsing blue dot** — animated via `focuspulse`: opacity 1 → 0.4 → 1 over 2.4s, ease-in-out, infinite (an explicit "session live" indicator with defined animation timing).
- Eyebrow label (verbatim, uppercase): "**Focus right now**"
- Task title: "Write the Q3 planning doc" — styled `white-space: nowrap; overflow: hidden; text-overflow: ellipsis` → **long titles truncate with an ellipsis**, the bar never wraps/grows.
- **Timer: "12:34"** — tabular numerals, blue, MM:SS. Given Frame 3C reports "41 min focused," this is a **count-up elapsed timer**, not a countdown.
- A blue "**Pause**" button — sessions are pausable (resume behavior implied but not shown).

**In-frame caption below the bar (verbatim):** "Everything below dims while a session runs — the rest of the day is still there, just quieter."

**Annotation 2 (verbatim):** "The task jumps into the persistent bar and a timer starts. It stays pinned across every view, so you never lose the thread."

Behavioral requirements:
1. Starting focus **moves the task into the persistent bar** and **auto-starts the timer** (no separate start step).
2. The bar is **pinned across every view/navigation** in the app.
3. While a session runs, all content beneath the bar is **dimmed** (visible but de-emphasized), app-wide.
4. Timer is elapsed-time (count-up), MM:SS, tabular-figure rendering; Pause available in the bar.

### Frame 3C — Completion → next-up handoff

**Window title bar:** "Today".

**Completion card** (green tint replacing the blue focus bar):
- Green filled circle with a white checkmark (SVG check).
- Message (verbatim): "**Done — 41 min focused. Next up?**" — completing **logs the focused time** and reports it rounded to minutes.
- Below, **two suggested next tasks**, each a full-width white clickable card:
  1. Empty checkbox + "Reply to Sarah re: offsite" + amber priority badge "2" + blue link-styled label "**Focus →**"
  2. Empty checkbox + "Review PR #482" + red priority badge "1" + "**Focus →**"

**Annotation 3 (verbatim):** "Marking done logs the time and immediately offers the next Now task — sorted by priority — so momentum carries into the next thing."

Behavioral requirements:
1. Marking the focused task done: (a) records the elapsed focus time against the task, (b) immediately replaces the focus bar with a **"next up" prompt** — no dead state.
2. Suggestions are drawn **from the Now bucket**, annotation says **sorted by priority** (⚠ but the mock renders P2 above P1 — see Open Questions #3).
3. Each suggestion has a one-click "Focus →" that starts a new session directly — sessions chain without returning to the list.
4. Implied: the completion card is dismissible/decline-able (the user can choose neither), though no dismiss affordance is drawn.
5. "Review PR #482" is a P1 task visible here but absent from earlier Now frames — the Now list is larger than any one frame shows.

---

## Journey 4 — Projects, People & Pages: "mentions become backlinks"

**Journey subtitle (verbatim):** "A **#Project** or **@Person** written anywhere auto-links. That page then aggregates every action that references it — no manual filing."

### Frame 4A — Mentions accumulate from "Various places"

**Window title bar:** "Various places". Three captioned snippets, each caption in small gray, each snippet in a light-gray rounded block with blue live-link mentions:

1. Caption: "In today&rsquo;s note" → "Draft the launch checklist **#Product Planning**"
2. Caption: "In a task from Inbox" → "Finalize vendor contract **#Product Planning**"
3. Caption: "In a 1:1 page" → "Get sign-off from **@Sarah Chen** on **#Product Planning**"

**Annotation 1 (verbatim):** "The same project gets mentioned across notes, tasks, and other pages throughout the week — each mention is a live backlink."

Behavioral requirements:
1. Mentions work identically in **at least three contexts**: daily notes, tasks (including ones triaged from Inbox), and other pages.
2. **"1:1 page"** is named as an existing page type — pages beyond projects and people exist (the journey title says "Projects, People & **Pages**").
3. A single task line can carry **both** an `@person` and a `#project` mention; it then backlinks to both pages.
4. Every mention is a **live backlink** — the referenced page aggregates it automatically, no manual filing.

### Frame 4B — Project page: Product Planning

**Window title bar:** "Product Planning".

**Page header:** small blue rounded-square **color swatch** + page title "Product Planning" (projects have an identity color, matching the swatch in the triage chip from 1B).

**Section 1 — "Notes"** (uppercase label): a free-text notes block (verbatim content): "Q3 launch targets the first week of August. Legal is the current bottleneck; Sarah owns the offsite where we lock scope." → project pages have their **own editable notes area**.

**Section 2 — "Actions · pulled automatically"** (uppercase label, verbatim) with a right-aligned counter (verbatim): "**3 open · 1 done**".

Four aggregated action rows (each originally created elsewhere):

| Task | Checkbox state | Chips | Status pill |
|---|---|---|---|
| Finalize vendor contract | empty gray ring | red priority badge "1" | **Blocked** (red pill, red dot) |
| Draft the launch checklist | **blue ring** (in-progress checkbox styling) | — (no priority badge) | **In progress** (blue pill, blue dot) |
| Get sign-off from Sarah | empty gray ring | person chip "@Sarah Chen" | **Waiting** (amber pill, amber dot) |
| Circulate Q2 retro | **green filled circle + white check** | — | — (row at 50% opacity, title struck through) |

Note the title normalization: the source text "Get sign-off from @Sarah Chen on #Product Planning" displays here as "Get sign-off from Sarah" + an @Sarah Chen chip — mentions are extracted into chips / cleaned from the aggregated title.

**Annotation 2 (verbatim):** "The project page has its own **Notes**, and an <b>Actions</b> list that gathers every referencing task — with live status — wherever it was created."

Behavioral requirements:
1. Project page = **Notes (freeform) + Actions (auto-aggregated)**.
2. Actions list includes every task referencing the project **regardless of where it was created** (note, inbox triage, another page).
3. Status is **live** on this page (same object; status changes elsewhere reflect here).
4. Open/done counts summarized in the section header.
5. **Done tasks stay visible** on the project page — dimmed to 50% opacity, strikethrough, green check — not removed.
6. Checkbox ring color encodes status: gray = not started/other, blue = in progress, green-filled = done.
7. Priority badges only render when a priority is set ("Draft the launch checklist" — created inline from a note without triage — has none → **priority is optional**).

### Frame 4C — Person page: Sarah Chen

**Window title bar:** "Sarah Chen".

**Page header:** circular avatar "S" (same pink color as her picker avatar in 2A — person colors are stable) + name "Sarah Chen" + role subtitle "**Design lead**" (people have a role/subtitle field).

**Section 1 — "Waiting on Sarah"** (uppercase label; note the header interpolates the person's first name):
- Row: amber status dot + "Sign-off on launch checklist" (the Waiting task from 4B, viewed from the person side).

**Section 2 — "Assigned / shared"** (uppercase label):
- Row: gray status dot + "Offsite logistics".

**Annotation 3 (verbatim):** "People pages work the same way, and split actions into **Waiting on them** vs <b>shared</b> — perfect for prepping a 1:1."

Behavioral requirements:
1. Person pages aggregate exactly like project pages (mentions → backlinks → auto-pulled actions).
2. Actions on a person page are **partitioned into two groups**: "Waiting on {FirstName}" (tasks with Waiting status tied to them) vs "Assigned / shared" (everything else referencing them).
3. Rows here render with a **status-colored dot** (amber = waiting, gray = backlog/none) rather than full pills.
4. Stated use case: 1:1 preparation.

---

## Journey 5 — Task lifecycle: "one status, six states"

**Journey subtitle (verbatim):** "Every task carries a status. Set it from the task itself, and see the whole pipeline grouped in one board."

### Frame 5A — Status menu

**Window title bar:** "Set status". Task heading: "Finalize vendor contract". Below, a dropdown menu (rounded, shadowed) listing all six statuses. Each row: **colored dot + status name + right-aligned description**; the current status row is tinted, bold, and carries a checkmark.

Verbatim status list (order as designed):

| # | Status | Dot color | Description (verbatim) | State in mock |
|---|---|---|---|---|
| 1 | Backlog | gray (#b0b0ae) | "Someday / unstarted" | |
| 2 | In progress | blue | "Actively on it" | |
| 3 | **Blocked** | red | "Can&rsquo;t proceed" | **selected** — red tinted row, bold red label, red checkmark |
| 4 | Waiting | amber | "On someone else" | |
| 5 | Done | green | "Complete" | |
| 6 | Binned | light gray (#c4c3c0), label itself grayed | "Dropped" | |

**Annotation 1 (verbatim):** "Status is one click from the task. **Blocked** and **Waiting** are first-class — they keep stuck work visible instead of hidden."

Behavioral requirements:
1. Exactly **six statuses**, one status per task, changeable **directly from the task** (one click).
2. Semantics: Blocked = can't proceed (an impediment); Waiting = pending on another person (pairs with person pages' "Waiting on them" grouping); Binned = dropped/abandoned (soft-delete distinct from Done).
3. Blocked and Waiting are first-class states, deliberately kept visible (see 5B and project page pills).
4. The menu always shows current selection (tint + check).

### Frame 5B — All-tasks status board

**Window title bar:** "All tasks · by status".

**Layout:** a 4-column board for the "active" statuses; each column header = colored dot + status name + gray count.

- **In progress · 2**: card "Draft launch checklist" (with red priority-1 badge on the card); card "Review PR #482". Cards = neutral light-gray rounded tiles.
- **Blocked · 1**: card "Finalize vendor contract" — this card alone gets a **red-tinted background + red border**, and a red sub-line "**Legal review**" → **blocked tasks carry a visible blocked-reason string**, and blocked cards are visually escalated.
- **Waiting · 1**: card "Sign-off from Sarah" with gray sub-line "**@Sarah Chen**" → waiting tasks display **who** they're waiting on.
- **Done · 2**: cards "Push staging build" and "Circulate Q2 retro" — gray text, **strikethrough**.

**Footer row (below the grid, collapsed groups as inline legends, verbatim):** "Backlog · 6" and "Binned · 2" — these two statuses do **not** get columns; they collapse to count-only chips at the bottom.

**Annotation 2 (verbatim):** "The same tasks — regardless of Now/Next/Later or which project they belong to — roll up into one status board. Backlog and Binned collapse out of the way."

Behavioral requirements:
1. A global board view aggregates **all tasks across all When-buckets and all projects/people**, grouped by status.
2. Column set: In progress, Blocked, Waiting, Done. **Backlog and Binned are collapsed by default** with visible counts (implying they're expandable).
3. Cards surface contextual metadata: priority badge (if set), blocked reason (Blocked), waiting-on person (Waiting); Done cards struck through.
4. Cross-referencing this board with earlier frames confirms tasks are single live objects: "Finalize vendor contract" (Inbox → triaged Next/P1/Fri/#Product Planning → Blocked "Legal review") and "Draft the launch checklist" (born via `[ ]` in a note → Now → In progress) appear consistently everywhere.

---

## Cross-cutting system rules (derived from all frames)

### Data model
- **Task attributes:** title · when-bucket (Now/Next/Later) · priority (1/2/3, optional) · due date (optional) · project/person links (0..n) · status (one of 6) · source tag (capture provenance) · logged focus time · blocked reason (when Blocked) · waiting-on person (when Waiting).
- **When-bucket semantics:** Now = "Today", Next = "This week", Later = (unlabeled, someday-ish). Independent of status and of due date.
- **Entities/pages:** Daily notes (one per calendar day), Projects (colored swatch + notes + auto actions), People (avatar, name, role, waiting/shared split), and other Pages (e.g. "1:1 page").
- Tasks are **single live objects** rendered on multiple surfaces simultaneously (daily note, Today list, project page, person page, status board, focus bar); status/state changes propagate everywhere.

### Keyboard shortcuts (complete set shown)
| Shortcut | Context | Action |
|---|---|---|
| ⌘Space | global (system-wide) | Quick capture to Inbox |
| N / E / L | triage sheet | Set When = Now / Next / Later |
| 1 / 2 / 3 | triage sheet | Set priority |
| # | triage sheet | Open project/person link picker |
| @ / # | note editor, mid-sentence | Open inline person / project picker |
| `[ ]` at line start | note editor | Convert line to a real task (defaults to Now) |
| F | task list (hovered/selected task) | Start focus session on task |

### Color/status semantics (consistent across all journeys)
- Blue = accent, In progress, selection/hover, focus session, project links/mentions.
- Red = priority 1, Blocked, Inbox count badge.
- Amber = priority 2, Waiting.
- Gray = priority 3, Backlog; lighter gray = Binned.
- Green = Done, success/confirmation highlights (task landing row, completion card, "→ Now" chip).
- Checkbox states: gray ring = open; blue ring = in progress; green fill + white check = done (with strikethrough + ~50% opacity on the row).

### Microcopy inventory (verbatim strings to ship)
"Captured today · unsorted" · "Triage · {task}" · When/Priority/Due/"Project / Person" field labels · "Next" + "This week" · "Now" + "Today" · "Focus" · "Focus right now" · "Pause" · "Done — {n} min focused. Next up?" · "Focus →" · "Jump to a day" · "Future note — pre-write agenda items, drop tasks with a due date, and they surface when the day arrives." · "Notes" · "Actions · pulled automatically" · "{n} open · {n} done" · "Waiting on {FirstName}" · "Assigned / shared" · "Set status" · status names+descriptions (Backlog "Someday / unstarted", In progress "Actively on it", Blocked "Can't proceed", Waiting "On someone else", Done "Complete", Binned "Dropped") · "All tasks · by status" · "Backlog · {n}" / "Binned · {n}" · source tags "⌘Space" / "from note" / "menu bar".

### Edge cases implied by the mocks
1. **Focus bar title truncation:** explicit ellipsis styling — long task titles must truncate, never wrap or resize the bar.
2. **Singleton focus:** starting Focus while a session runs must handle replace/stop of the current session.
3. **Pause state:** the bar has Pause; a paused state (and resume) exists but is never drawn — needs definition.
4. **Timer/logging:** count-up MM:SS display; completion message rounds to whole minutes ("41 min").
5. **Empty Inbox / zero badge:** badge shows a number; zero/empty states undrawn — needs definition.
6. **Done items on project pages** remain listed (dimmed/struck), and the header counts them separately ("3 open · 1 done").
7. **Future daily notes** can exist and hold content/tasks before their date; due-dated tasks in them stay dormant until the date arrives.
8. **Backlog/Binned expansion** on the board: collapsed-with-counts implies an expand interaction (undrawn).
9. **Blocked reason entry:** "Legal review" appears on the board card, but no UI for entering the reason is shown — needs a capture point (likely on setting status to Blocked).
10. **Priority is optional** (note-born tasks show no badge until triaged/edited).
11. **Mention title normalization:** aggregated views strip inline mention tokens from titles and re-render them as chips.
12. **Dimming during focus** applies app-wide ("across every view"), not just the Today view.

### Open questions / inconsistencies to resolve before writing acceptance criteria
1. **Intro says "the persistent focus bar from 1a"** but the focus bar first appears in Journey 3 — frame-reference error in the doc; confirm the bar is global chrome (as "home base" implies).
2. **Capture-from-note vs `[ ]` conversion conflict:** Journey 1A shows an Inbox item tagged "from note" ("Reply to Sarah re: offsite"), yet Journey 2B says `[ ]` tasks skip Inbox and default to Now. Either there are two distinct note-capture mechanisms (a send-to-inbox capture action vs `[ ]` inline conversion), or this is a contradiction — must be resolved.
3. **Next-up ordering:** Journey 3C's annotation says suggestions are "sorted by priority," but the mock lists the P2 task above the P1 task. Decide: strict priority sort (mock is wrong) or another ordering (e.g., Now-list order).
4. **"F" shortcut targeting:** "press F" needs a defined target (hovered row? keyboard-selected row?).
5. **Journey 5 board omits "In progress" chip parity:** "Review PR #482" card has no priority badge while it showed P1 in 3C — likely just mock sloppiness, but confirm badges always render when priority is set.
6. **Person-page "Assigned / shared" semantics** ("Offsite logistics", gray dot): what statuses/relationships route a task here vs "Waiting on them"? Only Waiting-status tasks are shown in the Waiting group; everything else referencing the person presumably falls to shared — confirm.
7. **Checkbox vs status redundancy:** ticking a checkbox presumably sets status = Done (and vice versa); Binned via checkbox is impossible — confirm checkbox ↔ status mapping.
