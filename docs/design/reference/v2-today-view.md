# Daily — "Today" view: exhaustive implementation reference (from `Todo Note App v2.dc.html`)

Source: `/private/tmp/claude-501/-Users-jonyardley-Dev-Yardstick/769b99c0-250d-4b05-b8a9-bdf8cdaa28ad/scratchpad/design/design_handoff_daily_focus_app/Todo Note App v2.dc.html` (320 lines).
Runtime: `support.js` in the same directory is a **generic** generated "dc-runtime" (React-based renderer for `<x-dc>` documents, streaming shimmer placeholders, postMessage embedding, optional `DCLogic` component classes). This mock contains **no `<script data-dc-script>`**, so there is **zero mock-specific JS behavior** — every state shown (open menu, pulsing dot, done rows) is painted statically in markup. The only animation is a CSS keyframe. Developers do not need support.js at all.

Sibling files (out of scope but present): `Todo Note App.dc.html` (v1) and `Core Journeys.dc.html`.

---

## 0. Document shell & global styles

- Everything is **inline styles on divs/spans** — no classes, no external CSS. The only stylesheet is in a `<helmet>` tag:
  - `* { box-sizing: border-box; }`
  - `html, body { margin: 0; height: 100%; }`
  - body: `font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "SF Pro Display", "Helvetica Neue", Helvetica, sans-serif; background: #c9c8c4; -webkit-font-smoothing: antialiased; color: #1d1d1f;`
  - **`@keyframes focuspulse { 0%,100% { opacity: 1; } 50% { opacity: 0.4; } }`** — used only by the focus-bar dot.
- Desktop wrapper: `height: 100vh; padding: 22px; display: flex;` on warm-grey `#c9c8c4` background (fake macOS desktop).
- **Window card**: `flex: 1; min-height: 0; background: #fff; border-radius: 12px; box-shadow: 0 32px 80px rgba(0,0,0,0.22), 0 2px 8px rgba(0,0,0,0.08); border: 0.5px solid rgba(0,0,0,0.12); overflow: hidden; display: flex; flex-direction: column;`
- Layout skeleton: window = column of [title bar (52px, flex:none)] + [body row (flex:1, min-height:0)] where body row = [sidebar 238px] + [main column flex:1 min-width:0 bg #fff].
- Hairline borders throughout are **0.5px** (macOS-style), except the focus bar which uses a full **1px** border.
- **No hover styles exist anywhere** (inline styles can't express them). All `<button>` elements carry `cursor: pointer`. Hover/active affordances must be invented; the mock only *paints* one hover state (the highlighted row inside the Combine menu).

---

## 1. Title bar

Container: `height: 52px; flex: none; background: linear-gradient(#f7f6f4, #efeeeb); border-bottom: 0.5px solid rgba(0,0,0,0.1); display: flex; align-items: center; padding: 0 16px; gap: 16px;`

Left → right:
1. **Traffic lights**: flex `gap: 8px`; three 12×12px circles, each with `border: 0.5px solid rgba(0,0,0,0.12)`; fills `#ff5f57`, `#febc2e`, `#28c840`.
2. **Vertical divider**: `width: 0.5px; height: 24px; background: rgba(0,0,0,0.1)`.
3. **Window title**: text `Today` — 14px / 600 / `#1d1d1f`.
4. Spacer `flex: 1`.
5. **Search field (fake)**: `display:flex; align-items:center; gap:7px; background: rgba(0,0,0,0.05); border-radius: 7px; padding: 5px 10px; width: 200px;` (fixed width). Icon is just an **11×11px circle** (`border: 1.5px solid #a0a09e; border-radius: 50%`) — a magnifier *without a handle*; easy to miss that it isn't a real icon. Placeholder text `Search` — 12.5px `#9a9a98`.
6. **New-item button**: `<button>` with text `+` — `border:none; background: oklch(0.62 0.13 250); color:#fff; width:28px; height:28px; border-radius:7px; font-size:18px; line-height:1; font-weight:400;`.

---

## 2. Sidebar (238px)

Container: `width: 238px; flex: none; background: #edecea; border-right: 0.5px solid rgba(0,0,0,0.09); display:flex; flex-direction:column; padding: 12px 10px; overflow-y: auto;`

### 2.1 Space switcher
Row: `display:flex; align-items:center; gap:8px; padding: 5px 8px 12px;`
- Avatar: 22×22px, `border-radius: 6px`, background **`oklch(0.62 0.16 25)`** (red — note: slightly *different* from the priority-1 badge red `oklch(0.6 0.16 25)`), white text `RB` at 10px/700, centered.
- Name `Red Badger` — 13px / 600 / `flex: 1`.
- Chevron character `⌄` — 10px `#b0b0ae`.

### 2.2 Active "Today" item
`display:flex; align-items:center; gap:9px; height: 30px; padding: 0 9px; border-radius: 7px; background: oklch(0.62 0.13 250); color:#fff; font-size:13px; font-weight:500;`
- Icon: 9×9px white square, `border-radius: 2px`.
- Label `Today` (`flex:1`), then date `Jul 2` — 11px, `opacity: 0.85`.
- Note: active row is **30px tall**; all other sidebar rows are **29px**.

### 2.3 Mini calendar
Card: `margin: 10px 4px 12px; padding: 9px; background: rgba(255,255,255,0.55); border-radius: 9px;`
- Header row (`padding: 0 2px 7px`, space-between): `July 2026` — 12.5px/600/`#1d1d1f`; nav arrows are HTML entities `‹ ›` (`&#8249; &#8250;`) in a flex span `gap: 11px`, color `#b0b0ae`, 11px.
- Grid: `display:grid; grid-template-columns: repeat(7, 1fr); gap: 1px;`
- Weekday header cells: `M T W T F S S` (**Monday-first**), 9.5px / 600 / `#b0b0ae`, centered; first cell has `padding-bottom: 3px` (others don't — minor inconsistency in the mock).
- Day cells: `height: 22px`, centered flex, 11.5px.
- **Two empty leading cells** (July 1 2026 is a Wednesday).
- Day states:
  - **Day 1 (yesterday)**: outlined circle — inner span 21×21px, `border-radius: 50%; border: 1px solid #c9c8c4; color: #3a3a3c;`
  - **Day 2 (today, selected)**: filled circle — 21×21px, background `oklch(0.62 0.13 250)`, white, 11.5px **600**.
  - Weekdays (3, 6–10, 13–17, 20–24, 27–31): plain text `#3a3a3c` (no circle span at all).
  - Weekends (4, 5, 11, 12, 18, 19, 25, 26): plain text `#c0c0be`.

### 2.4 Section headers (Views / Projects / People / Pages)
All: `font-size: 11px; font-weight: 700; letter-spacing: 0.06em; color: #9a9a98; text-transform: uppercase;`
- "Views" header padding: `4px 8px 6px`; the other three: `12px 8px 6px` (extra top spacing).

### 2.5 Sidebar row anatomy (shared)
`display:flex; align-items:center; gap: 9px; height: 29px; padding: 0 9px; border-radius: 7px; font-size: 13px; color: #1d1d1f;` — icon + label (`flex:1`) + optional count (11px `#a0a09e`).

### 2.6 Views (icons differ per row — all hand-drawn with CSS)
| Label | Icon | Count |
|---|---|---|
| `Now` | 8px filled circle `oklch(0.62 0.13 250)` (blue) | `3` |
| `Next · This week` | 8px filled circle `oklch(0.7 0.12 70)` (amber) — **note middot in label** | `4` |
| `Later` | 8px filled circle `#c0c0be` (grey) | `11` |
| `Waiting on` | 8px **hollow ring**: `border: 1.5px solid oklch(0.7 0.12 70); background: transparent` | `3` |
| `Inbox` | tray shape: `width:11px; height:8px; border-radius:2px; border:1.5px solid #a0a09e; border-top:none` | `0` — count color **`#b8b8b6`** (lighter than other counts' `#a0a09e`, signalling empty) |

### 2.7 Projects (squares, 11×11px, `border-radius: 3px`)
- `COAST` — `oklch(0.62 0.13 250)` (blue), count `12`
- `Line Management` — `oklch(0.58 0.14 300)` (purple), count `6`
- `Red Badger` — `oklch(0.62 0.12 150)` (green), **no count element at all**

### 2.8 People (avatar circles 17×17px, white initial 9px/700)
- `G` `oklch(0.65 0.12 230)` — `Gary Wilson`
- `M` `oklch(0.7 0.1 330)` — `Marisa`
- `T` `oklch(0.62 0.12 150)` — `Tomash`
- `H` `oklch(0.7 0.13 70)` — `Hopo`
No counts on people rows.

### 2.9 Pages (document icon: `width:10px; height:12px; border-radius:2px; border:1.5px solid #a0a09e`)
- `Feature Flagging`
- `COAST Architecture`

---

## 3. Main column — persistent focus bar

**Structurally outside the scroll container** (main column = flex column of [focus bar, `flex:none`] + [scroll div `flex:1; overflow-y:auto`]) — it stays pinned while content scrolls.

Container: `margin: 16px 28px 0; background: oklch(0.965 0.025 250); border: 1px solid oklch(0.88 0.05 250); border-radius: 12px; padding: 13px 16px; display:flex; align-items:center; gap: 14px;` (only **1px** border in the whole mock).

Left → right:
1. **Pulsing dot**: 9×9px circle `oklch(0.62 0.13 250)`, `animation: focuspulse 2.4s ease-in-out infinite; flex:none`.
2. **Text stack** (`flex-direction:column; gap:1px; min-width:0; flex:1`):
   - Eyebrow `Focus right now` — 10.5px / 700 / `letter-spacing: 0.08em` / uppercase / `oklch(0.55 0.13 250)`.
   - Task `Chase COAST support docs response for Akshay` — 15.5px / 600 / `#1d1d1f`, single-line ellipsis (`white-space:nowrap; overflow:hidden; text-overflow:ellipsis`).
3. **Project chip**: inline-flex, gap 6px, 11.5px `oklch(0.5 0.13 250)`, `background:#fff; padding: 3px 9px; border-radius: 7px; border: 0.5px solid oklch(0.85 0.05 250); white-space:nowrap; flex:none;` containing an 8×8px `border-radius:3px` swatch in `oklch(0.62 0.13 250)` + text `COAST`.
4. **Timer**: `18:42` — `font-variant-numeric: tabular-nums; font-size: 20px; font-weight: 600; color: oklch(0.5 0.13 250); letter-spacing: 0.01em;`
5. `<button>` **Pause** — primary: `border:none; background: oklch(0.62 0.13 250); color:#fff; 13px/500; padding: 7px 14px; border-radius: 8px;`
6. `<button>` **Switch** — secondary: `border: 0.5px solid rgba(0,0,0,0.15); background:#fff; color:#3a3a3c; 13px/500; padding: 7px 13px; border-radius: 8px;` (note asymmetric padding vs Pause: 13px vs 14px horizontal).

---

## 4. Scroll container

`flex: 1; overflow-y: auto; padding: 22px 28px 40px;`
Content width caps: daily-note body `max-width: 640px`; **everything else** (briefing, section headers, lists, cards) `max-width: 760px`.

---

## 5. Daily note

Wrapper: `margin-bottom: 26px`.
- Eyebrow `Daily note` — 11px / 700 / `letter-spacing: 0.06em` / uppercase / `#a0a09e` / `margin-bottom: 3px`.
- `<h2>` `Thursday, July 2` — `margin: 0 0 12px; font-size: 25px; font-weight: 700; letter-spacing: -0.02em; color: #1d1d1f;` (one of only two real headings; the other two are `<h3>`).
- Body: 14px / `line-height: 1.65` / `#3a3a3c` / max-width 640px.
  - Sub-heading line `Release Meeting` — 600, `margin-bottom: 4px` (plain div, not a heading element).
  - Bullets are flex rows `gap: 9px; margin-bottom: 4px` with a literal `•` (`&bull;`) span colored `#b0b0ae`:
    1. `Feature flags — toggles after 16 Jul need a separate CAB exception, check with @Ollie` — the mention `@Ollie` is an inline span `color: oklch(0.5 0.13 250); font-weight: 500;` (no pill/background, unlike tag chips elsewhere).
    2. `Copy changes?`
    3. Ghost/placeholder bullet: `Type to keep writing…` — text colored `#9a9a98` (bullet dot still `#b0b0ae`; no margin-bottom on last row).

---

## 6. "Actions from yesterday" briefing block

Card: `margin-bottom: 28px; max-width: 760px; background: #faf9f7; border: 0.5px solid rgba(0,0,0,0.08); border-radius: 12px; position: relative;`

### 6.1 Header row
`display:flex; align-items:center; gap:10px; padding: 11px 16px; border-bottom: 0.5px solid rgba(0,0,0,0.06);`
- 8px amber dot `oklch(0.7 0.13 70)`.
- `Actions from yesterday` — 13px / 600.
- `1 of 6 left to sort` — 12px / `#9a9a98`. **Sample-data nuance**: only 2 of the 6 items are rendered (1 sorted + 1 unsorted); the other 4 sorted items are implied but not shown.
- Spacer `flex:1`.
- Link-style text `Full brief · Wed, Jul 1 ↗` — 12px / `oklch(0.5 0.13 250)` / 500 (plain span, not an anchor; `↗` is a literal char).

### 6.2 Body
`padding: 12px 16px 14px;` containing two item cards.

### 6.3 Item card — SORTED state (variant)
`display:flex; align-items:center; gap:11px; padding: 8px 8px; border-radius: 8px; background:#fff; border: 0.5px solid rgba(0,0,0,0.07); margin-bottom: 6px;` **entire card at `opacity: 0.6`**.
- Checkbox: 16×16px filled circle `oklch(0.62 0.13 250)` containing an inline **SVG checkmark**: `width=9 height=9 viewBox="0 0 12 12"`, path `M2 6.5 L4.8 9 L10 3`, `stroke:#fff; stroke-width:1.9; stroke-linecap/linejoin: round; fill:none`.
- Title `Decide whether to strip complexity from the COAST architecture` — 13.5px / `#3a3a3c`.
- Provenance line (11px `#9a9a98`, `margin-top:1px`): `from MB & LF meeting ↗ · Wed 13:00` — the source link `MB & LF meeting ↗` is `oklch(0.5 0.13 250)`.
- Right pill `Sorted → Now` — 11.5px, `color: oklch(0.48 0.13 250); background: oklch(0.95 0.04 250); padding: 3px 10px; border-radius: 20px; white-space:nowrap; flex:none;` (literal `→` char).

### 6.4 Item card — UNSORTED state with open Combine menu
Same card chrome as above but full opacity, `position: relative`, no margin-bottom.
- Checkbox: 16×16px **empty** circle, `border: 1.5px solid #c4c3c0`.
- Title `Speak to Tomash — Danielle check-in, does he want to line manage?` — 13.5px / `#1d1d1f` (darker than the sorted card's title).
- Meta row (`display:flex; align-items:center; gap:6px; margin-top:3px; font-size:11px; color:#9a9a98`):
  - `from Jon / Marisa ↗ · Wed 14:45` (link part blue `oklch(0.5 0.13 250)`).
  - **Removable tag chips** (auto-suggested tags with delete affordance): inline-flex, `gap:4px; padding: 1px 7px; border-radius: 5px;` (note: radius 5 *rectangle-ish* chips here, unlike the radius-20 pills in task rows):
    - `@Tomash ✕` — person tag, purple: text `oklch(0.5 0.14 300)` on `oklch(0.96 0.03 300)`; the `✕` is `oklch(0.7 0.08 300)` at 10px.
    - `Line Management ✕` — project tag, grey: text `#6e6e73` on `#f1f0ee`; `✕` is `#b0b0ae` at 10px.
- **Triage button group** (right, `display:flex; gap:5px; flex:none; align-items:center`):
  - Three `<button>`s `Now` / `Next` / `Later` — `border: 0.5px solid rgba(0,0,0,0.1); background:#fff; color:#6e6e73; font-size:11.5px; font-weight:500; padding: 4px 10px; border-radius: 7px;`
  - Vertical divider span: `width:0.5px; height:16px; background: rgba(0,0,0,0.1)`.
  - `<button>` `Combine…` — accent-tinted: `border: 0.5px solid oklch(0.85 0.05 250); background: oklch(0.96 0.02 250); color: oklch(0.5 0.13 250); font-weight: 600;` (same size/padding as siblings). Literal ellipsis char `…`.

### 6.5 Combine popover (rendered OPEN — a captured state, not JS-driven)
Anchored to the unsorted card: `position:absolute; top: calc(100% + 6px); right: 0; width: 262px; background:#fff; border-radius: 10px; box-shadow: 0 16px 40px rgba(0,0,0,0.18), 0 0 0 0.5px rgba(0,0,0,0.1); padding: 5px; z-index: 30;` — the second shadow is a 0.5px **ring** substituting for a border. It visually overlaps the "Now" section below.
- Menu caption: `Combine with an existing task` — 10.5px / 700 / `letter-spacing: 0.05em` / uppercase / `#a0a09e` / padding `6px 9px 4px`.
- Options column (`gap: 1px`), each option `padding: 7px 9px; border-radius: 7px` with a 12.5px title + 10.5px `#9a9a98` subtitle:
  1. `Merge into one` (weight 500) / `combine copy, links & history`
  2. **`Add as subtask…`** (weight **600**) / `nest under a task you pick — e.g. Tomasz: pay & development` — this row has `background: oklch(0.96 0.02 250)` = the painted **hover/highlight state**. ⚠️ Copy inconsistency to be aware of: spelled `Tomasz` here vs `Tomash` everywhere else.
  3. `Make this the parent…` (weight 500) / `an existing task nests under it`

---

## 7. "Now" section

### 7.1 Section header
`display:flex; align-items:baseline; gap:10px; padding-bottom:4px; border-bottom: 0.5px solid rgba(0,0,0,0.08); margin-bottom:4px; max-width:760px;`
- `<h3>` `Now` — 16px / 700 / `#1d1d1f`, margin 0.
- Subtitle `Today` — 13px / `#86868b`.
- Spacer, then **progress pips + count**: 12px `#86868b` text `1 done · 3 to go` preceded by 4 bars in a `gap:3px` flex: each `width:16px; height:5px; border-radius:3px`; bar 1 = green `oklch(0.62 0.12 150)` (done), bars 2–4 = `#dedcd8`.

### 7.2 Task rows (container `max-width: 760px`)
Row anatomy: `display:flex; align-items:center; gap:11px; padding: 9px 6px; border-radius: 8px;` — checkbox (17×17px, flex:none) + title (14px, flex:1) + optional priority badge + chips + fixed **70px right-aligned meta column**.

**Priority badge** spec: 19×19px, `border-radius: 5px`, white 11px/700 digit, centered, flex:none. Colors: `2` = amber `oklch(0.7 0.13 70)`; `1` = red `oklch(0.6 0.16 25)`.
**Chip** spec (task rows): 11.5px, `padding: 3px 9px; border-radius: 20px; white-space:nowrap` — grey variant `#6e6e73` on `#f1f0ee`.

Row 1 — **focusing state**: 
- Checkbox: ring `border: 1.5px solid oklch(0.62 0.13 250)` with `position:relative` and inner span `inset: 3px; border-radius: 50%; background: oklch(0.62 0.13 250); opacity: 0.25` (soft filled center = in-progress).
- Title `Chase COAST support docs response for Akshay` (matches the focus bar task).
- Priority badge `2` (amber).
- **Focusing pill**: `color: oklch(0.48 0.13 250); background: oklch(0.95 0.04 250); padding: 3px 9px; border-radius: 20px;` with 6×6px blue dot `oklch(0.62 0.13 250)` + text `Focusing`.
- Grey chip `COAST`.
- Meta `2 days old` — 11.5px `#9a9a98`, `width:70px; text-align:right`, **with `title="Quietly waiting since Jun 30"`** (native tooltip — the only `title` attribute in the file).

Row 2 — plain open task:
- Empty checkbox `border: 1.5px solid #c4c3c0`.
- Title `Give probation feedback on Thabang to Pieter`.
- Priority badge `1` (red `oklch(0.6 0.16 25)`).
- Grey chip `Line Management`.
- Meta `from Slack` (provenance instead of age).

Row 3 — plain open task:
- Empty checkbox; title `Hopo — structure the job-description conversation`.
- Priority badge `2` (amber); grey chip `@Hopo` (person chip uses same grey style as project chips in task rows).
- Meta column is an **empty 70px spacer span** (`<span style="width: 70px;"></span>`) to preserve alignment — easy to miss.

Row 4 — **done state**: entire row `opacity: 0.55`.
- Checkbox: filled **green** circle `oklch(0.62 0.12 150)` with SVG check `width=10 height=10 viewBox="0 0 12 12"`, same path `M2 6.5 L4.8 9 L10 3`, `stroke-width: 1.8` (vs 1.9 at 9px in the briefing check).
- Title `Go back to Hopo — confirm his ask has been heard` — color `#86868b` + `text-decoration: line-through`.
- No priority badge. Grey chip `@Hopo`. Meta = completion time `10:15` in `#86868b` (darker than open-row meta `#9a9a98`).

---

## 8. "Waiting on" section

Header (same style as Now header but `margin: 24px 0 4px`): `<h3>` `Waiting on` + subtitle `3 people`. No progress pips.

Rows: `display:flex; align-items:center; gap:11px; padding: 8px 6px;` — note **8px vertical padding vs 9px in Now rows** (tighter), and no border-radius. Anatomy: avatar (17×17px circle, white 9px/700 initial, flex:none) + text (13.5px `#3a3a3c`, flex:1 — smaller/greyer than Now titles) + age (11.5px `#9a9a98`) + Nudge button.

| Avatar | Color | Text (verbatim) | Age |
|---|---|---|---|
| `G` | `oklch(0.65 0.12 230)` (matches Gary in sidebar) | `Gary — email Michael Bennett, set up the September planning group` | `since Wed` |
| `M` | `oklch(0.7 0.1 330)` (matches Marisa) | `Marisa — Jamie supporting Tomash; Shelley on the Dave conversation` | `since Wed` |
| `A` | `oklch(0.62 0.12 150)` (green — Alex/Sinni is not in the sidebar People list; reuses Tomash/Red Badger green) | `Alex / Sinni — action the BGC codes for the listed RJF users` | `since today` |

`Nudge` buttons (3×, identical): `border: 0.5px solid rgba(0,0,0,0.12); background:#fff; color:#6e6e73; font-size:11.5px; padding: 3px 10px; border-radius: 7px;` (no font-weight set — normal 400, unlike triage buttons at 500).

---

## 9. Resurfaced card (amber "Later" resurface)

`display:flex; align-items:center; gap:11px; margin: 24px 0 0; max-width:760px; padding: 11px 13px; background: oklch(0.985 0.012 70); border: 0.5px solid oklch(0.91 0.035 70); border-radius: 10px;` — very faint warm-amber tint; radius 10 (vs 12 for briefing/focus bar).
- 8px amber dot `oklch(0.7 0.12 70)` (note: `0.12` chroma here matching the Next view dot, vs `0.13` on priority badges/briefing dot).
- Text stack (`flex:1; min-width:0`):
  - Title `Confirm the Flagsmith session with Jinu Vijay is arranged` — 13.5px `#1d1d1f`.
  - Meta `resurfaced from Later — parked 49 days · COAST · still relevant?` — 11px `#9a9a98`, margin-top 1px.
- Three `<button>`s, all `font-size: 11.5px; padding: 4px 11px; border-radius: 7px;`:
  1. `This week` — accent-tinted primary of the trio: `border: 0.5px solid oklch(0.85 0.05 250); background: oklch(0.96 0.02 250); color: oklch(0.5 0.13 250); font-weight: 600;`
  2. `Keep parked` — `border: 0.5px solid rgba(0,0,0,0.12); background:#fff; color:#6e6e73;`
  3. `Bin` — same chrome as Keep parked but **lighter text `#9a9a98`** (de-emphasized destructive).

---

## 10. Collapsed "Next" / "Later" rows

Both: `display:flex; align-items:center; gap:10px; max-width:760px; padding: 10px 12px; background: #faf9f7; border-radius: 9px;` — Next row `margin: 24px 0 0`; Later row `margin: 8px 0 0`.
- Label — 13px / 600 / `#6e6e73`.
- Summary — 12.5px / `#9a9a98` / flex:1.
- Chevron `⌄` — 11px `#b0b0ae` (same glyph as space switcher).

Verbatim copy:
- **Next**: `4 for this week — James PPR (blocked), architecture sessions, Tomash pay planning…`
- **Later**: `11 parked — each resurfaces gently after ~30 days or when a due date nears`

---

## 11. Cross-cutting details, gotchas & data consistency

**Semantic elements**: only real `<button>`s (13 total: `+`, Pause, Switch, Now/Next/Later triage, Combine…, 3× Nudge, This week, Keep parked, Bin), one `<h2>`, two `<h3>`, two inline `<svg>` checkmarks. Everything else is div/span. All "links" (`Full brief ↗`, `MB & LF meeting ↗`, `Jon / Marisa ↗`) are plain styled spans with a literal `↗` character.

**State variants catalog** (all statically painted):
- Checkbox: empty (1.5px `#c4c3c0` ring, 16px in briefing / 17px in task list), in-focus (blue ring + 25%-opacity blue inner dot), done-blue (briefing, filled blue + white SVG check), done-green (task list, filled green + check, row at 0.55 opacity + strikethrough).
- Card: sorted (0.6 opacity + pill `Sorted → Now`), unsorted (full opacity + triage buttons + removable tag chips).
- Menu option: default (weight 500) vs highlighted (weight 600 + `oklch(0.96 0.02 250)` bg).
- Button hierarchy: solid accent (Pause, `+`) > accent-tinted (Combine…, This week) > white-bordered neutral (Switch, Now/Next/Later, Nudge, Keep parked) > white-bordered muted-text (Bin).

**Two chip systems** (don't conflate): task-row chips are pill radius 20 / padding 3px 9px; briefing tag chips are radius 5 / padding 1px 7px with `✕` remove glyphs; focus-bar project chip is radius 7 with a color swatch.

**Fixed-width alignment tricks**: search box `width: 200px`; task-row meta column `width: 70px; text-align: right` (with an *empty* 70px spacer on the row lacking meta).

**Exact color additions beyond the README token list** (as used here): desktop `#c9c8c4`; titlebar gradient `#f7f6f4 → #efeeeb`; traffic lights `#ff5f57 / #febc2e / #28c840`; sidebar `#edecea`; calendar card `rgba(255,255,255,0.55)`; muted text tiers `#3a3a3c / #6e6e73 / #86868b / #9a9a98 / #a0a09e / #b0b0ae / #b8b8b6 / #c0c0be / #c4c3c0 / #c9c8c4 / #dedcd8`; soft surfaces `#faf9f7 / #f1f0ee`; accent-tint family `oklch(0.965 0.025 250) / 0.96 0.02 250 / 0.95 0.04 250 / 0.88 0.05 250 / 0.85 0.05 250`; accent-text family `oklch(0.62 0.13 250) / 0.55 0.13 250 / 0.5 0.13 250 / 0.48 0.13 250`; amber family `oklch(0.7 0.13 70) / 0.7 0.12 70 / 0.985 0.012 70 / 0.91 0.035 70`; purple `oklch(0.58 0.14 300) / 0.5 0.14 300 / 0.96 0.03 300 / 0.7 0.08 300`; green `oklch(0.62 0.12 150)`; reds `oklch(0.62 0.16 25)` (space avatar) vs `oklch(0.6 0.16 25)` (priority 1); people avatars `oklch(0.65 0.12 230) / 0.7 0.1 330 / 0.7 0.13 70`.

**Data consistency checks a dev can rely on**: sidebar counts (Now 3, Next 4, Later 11, Waiting on 3, Inbox 0) match section contents (Now shows 3 open + 1 done = "1 done · 3 to go"; Next collapsed says "4 for this week"; Later says "11 parked"; Waiting on shows 3 rows). The focus-bar task is the same task as Now row 1 (`Focusing` pill + amber `2` + COAST chip). The briefing header count ("1 of 6 left to sort") intentionally exceeds the 2 rendered items.

**Known copy quirks**: `Tomasz` vs `Tomash` spelling inconsistency in the Combine menu example; typographic characters used literally throughout: `—` em dash, `·` middot, `…` ellipsis, `⌄` chevron, `‹ ›` calendar arrows, `↗` external-link, `→` arrow, `✕` remove, `•` bullet (`&bull;`).

**Interaction hints present in markup** (no JS): `cursor: pointer` on every button; `focuspulse` 2.4s opacity pulse on the focus dot; `title` tooltip on "2 days old"; the open Combine popover with a highlighted second option models the menu's hover state; z-index 30 on the popover (only stacking context declared).