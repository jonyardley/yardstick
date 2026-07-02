# macOS SwiftUI Research: Hard UI Problems for a Calm Todo + Daily-Notes App

Scope: native macOS app, macOS 15 (Sequoia) → macOS 26 (Tahoe) era, SwiftUI-first with AppKit escape hatches, Crux (Rust core) driving an externally-updated view model. Researched July 2026 against Apple docs, WWDC 25/26 material, and community sources.

---

## 1. Rich-text daily-note editor with inline tokens

### The landscape (2026)

**Option A — SwiftUI `TextEditor` + `AttributedString`.**
As of macOS 26 / iOS 26, `TextEditor` finally accepts an `AttributedString` binding, with `AttributedTextSelection`, `transformAttributes(in:)`, typing attributes, Markdown parsing, and an `AttributedTextFormattingDefinition` system (with value constraints) for restricting/normalizing attributes ([WWDC25 session 280](https://developer.apple.com/videos/play/wwdc2025/280/), [Apple sample](https://developer.apple.com/documentation/swiftui/building-rich-swiftui-text-experiences), [Create with Swift walkthrough](https://www.createwithswift.com/using-rich-text-in-the-texteditor-with-swiftui/)). You can define custom `AttributedStringKey`s (e.g. a `mentionID` attribute) and style runs with color/background.

Hard limits for your feature set:
- **macOS 26 only.** On macOS 15 `TextEditor` is plain-string. If 15 is in your support matrix, Option A is out on its own.
- **No inline view attachments.** SwiftUI text still cannot embed interactive views inside the text flow — `NSTextView`/`UITextView` can, `TextEditor` can't ([Fatbobman's deep dive](https://fatbobman.com/en/posts/a-deep-dive-into-swiftui-rich-text-layout/)). Chips can be *colored runs*, but not tappable widgets; there is no per-range tap/hover API, no caret-rect API for anchoring a mention picker, and no way to swap a line for a "task row widget."

**Option B — `NSTextView` + TextKit 2 via `NSViewRepresentable`.** Full control: view-based attachments, caret geometry, paragraph-level custom layout/drawing, delegate interception of edits. This is what real native note apps ship: Bear, NotePlan, Agenda and Ulysses are all customized `NSTextView`/TextKit stacks (single text view, markdown-ish source with styled/hidden syntax), not SwiftUI editors; Craft uses its own block engine. TextKit 2 is the default engine in macOS Ventura+ text controls, though `NSTextView` created in code must opt in ([WWDC22: What's new in TextKit](https://developer.apple.com/videos/play/wwdc2022/10090/)). Apple is actively investing here: [WWDC26 "Elevate your app's text experience with TextKit"](https://developer.apple.com/videos/play/wwdc2026/370/) added attachment-view reuse policies (`onEditingInlineParagraphs`, `onScrollingOutOfViewport`) precisely so interactive inline views survive typing and scrolling — a strong signal this is the sanctioned path for token/chip UIs ([Kyle Howells' iOS 27 UIKit roundup](https://ikyle.me/blog/2026/whats-new-in-uikit-ios-27)).

**Option C — a list of per-block editors** (one `TextEditor`/text view per paragraph, Notion/Craft-style). Viable only if your document model is block-first from day one. You must hand-roll: cross-block selection, copy/paste spanning blocks, arrow-key navigation across block boundaries, focus transfer on Enter/Backspace-at-start, undo coalescing. Community consensus (e.g. [Apple dev forums on Notes-style editors](https://developer.apple.com/forums/thread/737709), the [Qt block-editor writeup](https://rubymamistvalove.com/block-editor)) is that this is a large sustained engineering tax. SwiftUI's `FocusState` + `List` makes it worse (focus loss on row reuse).

### Recommendation: **Option B — one `NSTextView` (TextKit 2) wrapped in `NSViewRepresentable`**

It is the only option that delivers all three behaviors (interactive chips, caret-anchored live picker, line→task-row conversion) on both macOS 15 and 26, and it matches what every comparable shipping app does. Consider [STTextView](https://github.com/krzyzanowskim/STTextView) as a TextKit2-native base if you want line numbers/modern internals without `NSTextView` legacy quirks — but plain `NSTextView` with TextKit 2 opt-in is fine and better documented.

### Implementation shape

**Source of truth** lives in the Crux core as plain text with explicit token markup, e.g. `@[Anna](person:123)` and `#[Yardstick](project:9)` and `- [ ] task` lines. The Swift shell owns a parser that maps model text → `NSAttributedString` with custom attributes, and an inverse serializer. Never persist `NSAttributedString`.

```swift
struct NoteEditor: NSViewRepresentable {
    @Bindable var doc: NoteDocModel          // shell-side mirror of core view model
    let onEdit: (String) -> Void             // debounced → core event

    func makeNSView(context: Context) -> NSScrollView {
        let textView = NSTextView(usingTextLayoutManager: true) // TextKit 2 opt-in
        textView.delegate = context.coordinator
        textView.textStorage?.delegate = context.coordinator
        textView.allowsUndo = true
        // scroll view wrapping elided
    }
    func makeCoordinator() -> Coordinator { Coordinator(self) }
}
```

**Chips (mentions/projects):** two mechanisms, pick per token:
- *Atomic tokens (recommended for @person/#project):* replace the typed text with an `NSTextAttachment` whose `NSTextAttachmentViewProvider` returns a small `NSHostingView` containing your SwiftUI chip (colored capsule, hover, click → `sendPrompt` to open the person/project). TextKit 2 routes events to the attachment view directly ([NSTextAttachmentViewProvider](https://developer.apple.com/documentation/uikit/nstextattachmentviewprovider), [WWDC22](https://developer.apple.com/videos/play/wwdc2022/10090/)). The token deletes as one character — exactly the chip UX users expect. Keep the semantic payload (`person:123`) on the attachment subclass and in a parallel custom attribute for serialization.
- *Lightweight styled runs:* keep `@Anna` as text, apply a custom attribute + background color, and make it atomic by implementing `textView(_:shouldChangeTextIn:replacementString:)` to expand partial deletions to the whole run. Cheaper, fully editable, but hit-testing clicks means resolving the character index under the mouse (`characterIndexForInsertion(at:)`) yourself.

**Live picker while typing @/#:** the classic, well-trodden pattern ([NCRAutocompleteTextView](https://github.com/danjonweb/NCRAutocompleteTextView), [Swift port](https://gist.github.com/martinpi/5e5ca6f0df035145402bf2f288055dfd)):
1. In `textDidChange`, scan back from the insertion point for an unterminated trigger (`@` + partial query).
2. Anchor: `let rect = textView.firstRect(forCharacterRange: triggerRange, actualRange: nil)` (screen coords → convert via `window.convertFromScreen`), then show an `NSPopover` (`behavior = .transient`) or a borderless child window at that rect. Popover is easiest; a child `NSWindow` avoids popover chrome for a Spotlight-y look.
3. While the picker is up, intercept ↑ ↓ ↩ ⎋ in `textView(_:doCommandBy:)` (`moveUp:`, `moveDown:`, `insertNewline:`, `cancelOperation:`) and route them to the picker instead of the editor.
4. On commit, replace the trigger range with the chip attachment in one undo group.

**`[ ]` at line start → task row:** intercept in `shouldChangeTextIn`/`didChangeText`: when a line begins `[ ] ` (or `- [ ] `), rewrite that paragraph — set a custom paragraph-level attribute (`taskID`), insert a checkbox attachment (view provider → SwiftUI `Toggle`-style button that sends `ToggleTask(id)` to the core), and style the paragraph. For a *full widget* look (hover affordances, drag handle, due-date pill), subclass `NSTextLayoutFragment` and return it from the `NSTextLayoutManagerDelegate` for task paragraphs — this is exactly the paragraph-decoration technique from [WWDC21 "Meet TextKit 2"](https://developer.apple.com/videos/play/wwdc2021/10061/) (the comment-bubble sample). Start with the checkbox-attachment version; graduate to custom layout fragments only if design demands it.

**Where Option A still helps:** use the new rich `TextEditor` for *small* rich fields (task notes, quick-capture body) on macOS 26 builds — zero AppKit for the easy cases — while the daily-note canvas stays TextKit.

---

## 2. Global quick capture (hotkey + floating panel + menu bar)

### Hotkey — do not ship Cmd+Space

Cmd+Space is Spotlight's system default; users cannot be asked to give it up, and shortcut recorders will (correctly) flag it. Precedents: **Things 3 uses Ctrl+Space** for Quick Entry ([Cultured Code docs](https://culturedcode.com/things/support/articles/2249437/)); Alfred/Raycast converts use **Option+Space**. Caveat: Ctrl+Space is also the default "Select previous input source" shortcut (active only for users with multiple input sources) — another reason it must be user-configurable.

**Recommendation:** default **Option+Space** (or Ctrl+Space), configurable via [sindresorhus/KeyboardShortcuts](https://github.com/sindresorhus/KeyboardShortcuts) (actively maintained, v2.x in 2025-26 — [releases](https://github.com/sindresorhus/KeyboardShortcuts/releases)). It uses the Carbon hotkey API under the hood, so **no Accessibility permission** and App Store/sandbox safe, and its SwiftUI `KeyboardShortcuts.Recorder` persists to `UserDefaults` and warns when a chosen shortcut collides with system or menu shortcuts:

```swift
extension KeyboardShortcuts.Name {
    static let quickCapture = Self("quickCapture", default: .init(.space, modifiers: [.option]))
}
// at app start
KeyboardShortcuts.onKeyUp(for: .quickCapture) { CapturePanelController.shared.toggle() }
// in Settings
KeyboardShortcuts.Recorder("Quick capture:", name: .quickCapture)
```

### The floating capture panel — `NSPanel`, not a SwiftUI window

SwiftUI `Window`/`openWindow` activates your app and steals focus from whatever the user was doing — wrong for capture. The established recipe ([Cindori's floating panel](https://cindori.com/developer/floating-panel), [Markus Bodner's Spotlight-like window](https://www.markusbodner.com/til/2021/02/08/create-a-spotlight/alfred-like-window-on-macos-with-swiftui/), [Fazm's NSPanel patterns](https://fazm.ai/blog/swiftui-floating-panel)):

```swift
final class CapturePanel: NSPanel {
    init(hosting root: some View) {
        super.init(contentRect: .init(x: 0, y: 0, width: 560, height: 120),
                   styleMask: [.nonactivatingPanel, .titled, .fullSizeContentView],
                   backing: .buffered, defer: false)
        isFloatingPanel = true
        level = .floating
        collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary] // follow user across Spaces/fullscreen
        titleVisibility = .hidden; titlebarAppearsTransparent = true
        isMovableByWindowBackground = true
        hidesOnDeactivate = false
        becomesKeyOnlyIfNeeded = true
        animationBehavior = .utilityWindow
        contentView = NSHostingView(rootView: root)
    }
    override var canBecomeKey: Bool { true }   // required so the TextField gets focus
    override func cancelOperation(_ sender: Any?) { close() }   // Esc dismisses
    override func resignKey() { super.resignKey(); close() }    // click-away dismisses
}
```

Key points: `.nonactivatingPanel` means the panel takes *key* status (keyboard goes to your text field) without *activating* the app — the user's current app stays frontmost and gets focus back instantly on dismiss. Center it with `panel.center()` + offset toward the top third, `makeKeyAndOrderFront(nil)`. On save, fire a `CaptureTodo(text)` event into the Crux core and close.

### MenuBarExtra — capabilities and limits

Use SwiftUI `MenuBarExtra` for the status item ([Apple docs](https://developer.apple.com/documentation/SwiftUI/MenuBarExtra), [Nil Coalescing guide](https://nilcoalescing.com/blog/BuildAMacOSMenuBarUtilityInSwiftUI/)). Set `LSUIElement = YES` if you want a dock-less agent; include an explicit Quit button.

- **Timer in the title:** the label closure supports `Text` + template `Image`, and the title re-renders reactively on state change. Two approaches: (1) `Text(timerInterval:)` / `Text(_, style: .timer)`, which self-update with **zero** view invalidation — try this first; (2) if the self-updating text misbehaves in the status item on your target OS, tick an `@Observable` string at 1 Hz from a `Timer` only while a timer is running. Either way use `.monospacedDigit()`-style formatting (or a fixed-width string) so the menu bar item doesn't jitter in width. Keep it short ("25:00"); menu bar real estate is contested.
- **`.menu` style limits:** the body is not re-rendered when the menu opens ([FB13683957](https://github.com/feedback-assistant/reports/issues/477)), the runloop blocks while open, and content is restricted to text/buttons/dividers (images and custom styles ignored).
- **`.window` style** gives a popover-like anchored window that can host arbitrary SwiftUI — right choice for a mini dashboard (today's tasks, running timer, capture field). But it is *anchored to the status item and activating* — it is **not** a substitute for the hotkey capture panel. Use both: `MenuBarExtra(.window)` for click-on-icon UI, the `NSPanel` above for the global hotkey.
- **No first-party API** for programmatic open/close, `NSStatusItem` access, or presentation state; use [orchetect/MenuBarExtraAccess](https://github.com/orchetect/MenuBarExtraAccess) if you need `isPresented` control. Opening Settings from a menu-bar agent has known activation quirks — see [Steipete's writeup](https://steipete.me/posts/2025/showing-settings-from-macos-menu-bar-items) (activate the app before `openSettings`).

---

## 3. SwiftUI macOS shell structure

### NavigationSplitView with a custom-designed sidebar

Baseline shape: `NavigationSplitView { List(selection:) {...}.listStyle(.sidebar) } detail: {...}` with `navigationSplitViewColumnWidth(min:ideal:max:)`.

Custom background: the sidebar hosts an `NSVisualEffectView` material behind the `List`. To tint it:

```swift
List(selection: $selection) { ... }
    .scrollContentBackground(.hidden)          // remove List's own background
    .background(Theme.sidebarBackground)       // your color/material
```

This works on macOS 13+, but community experience is that it's the fussiest part of macOS SwiftUI ([swiftui-introspect discussion](https://github.com/siteline/swiftui-introspect/discussions/378), [HWS forum thread](https://www.hackingwithswift.com/forums/macos/navigationsplitview-on-macos/24237)); some teams introspect the underlying `NSVisualEffectView` and set `state = .inactive`/custom material for a fully opaque sidebar. **On macOS 26**, the sidebar becomes a floating Liquid Glass pane and `backgroundExtensionEffect` lets detail content extend under it ([WWDC25 "Build a SwiftUI app with the new design"](https://developer.apple.com/videos/play/wwdc2025/323/), [Doran Gao's Tahoe walkthrough](https://medium.com/@dorangao/build-a-macos-swiftui-app-with-a-tahoe-style-liquid-glass-ui-fecb8029b2d8)). Advice for a *calm* app: tint gently and keep the system material rather than fighting for a fully custom opaque sidebar — you get Tahoe's look for free and avoid an introspection maintenance tax. Custom section headers are easy and safe: `Section { rows } header: { CustomHeader("Projects") }` with `.listSectionSeparator(.hidden)`.

### Toolbar search field

`.searchable(text: $query, placement: .toolbar)` on the content/detail column renders the standard trailing toolbar search field on macOS; scope it to the column whose data it filters (placement `.sidebar` puts it in the sidebar on macOS 26-era layouts). Route the query string into the core as an event (`SearchQueryChanged`), debounced ~150 ms.

### Cheap per-second timer updates

Rule: **never** re-render the window at 1 Hz from a `Timer` in your model.
- Best: `Text(timerInterval: start...distantFuture)` / `Text(start, style: .timer)` — the text updates itself out-of-band with no SwiftUI invalidation at all.
- When you need custom formatting: wrap *only the label* in `TimelineView(.periodic(from: .now, by: 1)) { ctx in Text(format(elapsed(at: ctx.date))) }`. Invalidation is scoped to that leaf view.
- Keep ticking out of the Crux core entirely: the core stores `timerStartedAt` (wall clock) in the view model once; the shell derives display time locally. No FFI traffic per second.

### Hosting a Crux-style externally-driven view model

Crux's contract: shell sends `Event` → core `update` returns effects; a `Render` effect means "re-fetch the view model" via `core.view()` ([Crux docs](https://redbadger.github.io/crux/guide/message_interface.html), [shared core guide](https://redbadger.github.io/crux/getting_started/core.html)). Modern Swift shape — one `@Observable` bridge, mutated only on the MainActor:

```swift
@Observable @MainActor
final class CoreBridge {
    private(set) var view: ViewModel = .init()   // generated (serde) type

    func send(_ event: Event) {
        let effects = try! [Request].bincodeDeserialize(
            input: [UInt8](processEvent(Data(try! event.bincodeSerialize()))))
        for effect in effects { process(effect) }
    }

    private func process(_ request: Request) {
        switch request.effect {
        case .render:
            view = try! ViewModel.bincodeDeserialize(input: [UInt8](Yardstick.view()))
        case .keyValue(let op):
            Task {                                    // async side effect off-main
                let result = await KeyValueStore.run(op)
                for follow in resolve(request, with: result) { process(follow) }
            }
        // timer, http, etc.
        }
    }
}
```

Inject with `.environment(bridge)`; views read `bridge.view.todaysTasks` etc. Because `@Observable` tracks per-property access, replacing the whole `view` struct invalidates only views that read fields whose values changed is *not* true — it invalidates readers of `view` — so if profiling shows over-invalidation, split the bridge into a few `@Observable` sub-stores (tasks, note, timer) and diff on assignment. Practical guidance: keep `ViewModel` cheap to deserialize (it crosses FFI on every render), and coalesce bursts of `Render` effects into one fetch per runloop turn.

---

## 4. Persisting a timer across restarts; monotonic vs wall clock

### Clock facts (Darwin)

| Clock | Ticks during sleep? | Survives reboot? | Affected by user/NTP clock changes? |
|---|---|---|---|
| `Date()` (wall) | yes | yes | **yes — can jump backward** |
| `mach_absolute_time` / **`SuspendingClock`** | **no** | no | no |
| `mach_continuous_time` / **`ContinuousClock`** | yes | no (resets at boot) | no |

Sources: [mach_absolute_time](https://developer.apple.com/documentation/kernel/1462446-mach_absolute_time) ("does not increment while the system is asleep"), [mach_continuous_time](https://developer.apple.com/documentation/kernel/1646199-mach_continuous_time), [SE-0329 Clock/Instant/Duration](https://github.com/swiftlang/swift-evolution/blob/main/proposals/0329-clock-instant-duration.md), [ContinuousClock](https://developer.apple.com/documentation/swift/continuousclock) / [SuspendingClock](https://developer.apple.com/documentation/swift/suspendingclock).

Consequences: a monotonic instant is meaningless after reboot, so **the only cross-restart anchor is wall-clock time** — but wall clock can jump (user changes date, NTC correction). And the naive `Timer`-accumulator approach silently loses time across sleep. Design accordingly:

### Recommended design

Persist *events*, not elapsed counters:

```swift
struct PersistedTimer: Codable {
    var accumulated: TimeInterval      // sum of completed run segments
    var runningSince: Date?            // wall clock; nil when paused
    var lastCheckpoint: Date           // written every ~30 s while running
}

func elapsed(_ t: PersistedTimer, now: Date = .now) -> TimeInterval {
    guard let start = t.runningSince else { return t.accumulated }
    let delta = now.timeIntervalSince(start)
    // Harden against wall-clock jumps:
    if delta < 0 {                     // clock moved backward past start
        return t.accumulated + t.lastCheckpoint.timeIntervalSince(start).clamped(min: 0)
    }
    return t.accumulated + delta
}
```

Robustness layers:
1. **Cross-restart / cross-sleep correctness for free:** since elapsed = `now - runningSince`, quitting the app, sleeping the Mac, or rebooting all "just work" — this is why you never accumulate with a ticking `Timer`.
2. **Clock-jump detection:** observe `Notification.Name.NSSystemClockDidChange`; on fire (and on every checkpoint), if `now < lastCheckpoint`, the clock went backward — freeze the timer at the checkpointed elapsed and re-anchor `runningSince = now - checkpointedElapsed`. The 30 s checkpoint bounds the damage of any jump.
3. **In-session display:** while the app is running, you may additionally anchor a `ContinuousClock.Instant` at launch and render `wallAnchorElapsed + continuousClock.now - launchInstant` — immune to wall jumps between checkpoints and it ticks through system sleep (a "time since I started this task" timer usually *should* include sleep; if your product wants to auto-pause on sleep, compare `SuspendingClock` vs `ContinuousClock` deltas at wake, or observe `NSWorkspace.willSleepNotification`/`didWakeNotification` and pause/resume explicitly — likely the calmer product behavior).
4. **Ownership:** the Crux core owns `PersistedTimer` and the pause/resume/checkpoint logic (pure, testable); the shell supplies `now` via a Time capability and renders elapsed locally per §3.

---

## Summary of recommendations

1. **Editor:** single `NSTextView` + TextKit 2 in `NSViewRepresentable`; chips as `NSTextAttachment` + `NSTextAttachmentViewProvider` hosting SwiftUI capsules; picker as `NSPopover` anchored at `firstRect(forCharacterRange:)` with `doCommandBy` key routing; `[ ]` conversion via delegate interception + checkbox attachment (custom `NSTextLayoutFragment` later if needed). Plain text + token markup as the persisted model in the Rust core. Use macOS 26's rich `TextEditor` only for small secondary fields.
2. **Quick capture:** KeyboardShortcuts library, default Option+Space (Things-style Ctrl+Space acceptable), never Cmd+Space; non-activating `NSPanel` (`.nonactivatingPanel`, `canBecomeKey = true`, `.floating`, all-Spaces) hosting SwiftUI; `MenuBarExtra(.window)` for the status item with the timer in its `Text` label (self-updating timer text first, 1 Hz observable fallback), plus MenuBarExtraAccess if you need programmatic presentation.
3. **Shell:** `NavigationSplitView` + `List(.sidebar)` with `scrollContentBackground(.hidden)` + background tint (lean into Tahoe's glass rather than fully overriding); `.searchable(placement: .toolbar)`; per-second UI via self-updating `Text(timerInterval:)`/scoped `TimelineView` leaves; one `@MainActor @Observable` CoreBridge that re-fetches the Crux view model on `Render` effects, with ticking kept entirely shell-side.
4. **Timer:** persist `runningSince: Date` + `accumulated`, compute elapsed on read; checkpoint every 30 s; handle `NSSystemClockDidChange` and negative deltas; use `ContinuousClock` only for in-session deltas (it includes sleep; it does not survive reboot); decide sleep semantics explicitly via `NSWorkspace` sleep/wake notifications.

## Sources

- [WWDC25 280: Cook up a rich text experience in SwiftUI with AttributedString](https://developer.apple.com/videos/play/wwdc2025/280/) · [Apple: Building rich SwiftUI text experiences](https://developer.apple.com/documentation/swiftui/building-rich-swiftui-text-experiences) · [Create with Swift: rich text TextEditor](https://www.createwithswift.com/using-rich-text-in-the-texteditor-with-swiftui/) · [Fatbobman: SwiftUI rich text layout deep dive](https://fatbobman.com/en/posts/a-deep-dive-into-swiftui-rich-text-layout/)
- [WWDC21: Meet TextKit 2](https://developer.apple.com/videos/play/wwdc2021/10061/) · [WWDC22: What's new in TextKit and text views](https://developer.apple.com/videos/play/wwdc2022/10090/) · [WWDC26 370: Elevate your app's text experience with TextKit](https://developer.apple.com/videos/play/wwdc2026/370/) · [NSTextAttachmentViewProvider](https://developer.apple.com/documentation/uikit/nstextattachmentviewprovider) · [Kyle Howells: What's new in UIKit iOS 27](https://ikyle.me/blog/2026/whats-new-in-uikit-ios-27) · [STTextView](https://github.com/krzyzanowskim/STTextView) · [NCRAutocompleteTextView](https://github.com/danjonweb/NCRAutocompleteTextView) · [Swift autocomplete popover gist](https://gist.github.com/martinpi/5e5ca6f0df035145402bf2f288055dfd)
- [sindresorhus/KeyboardShortcuts](https://github.com/sindresorhus/KeyboardShortcuts) ([releases](https://github.com/sindresorhus/KeyboardShortcuts/releases)) · [Things Quick Entry docs](https://culturedcode.com/things/support/articles/2249437/) · [Cindori: floating panel in SwiftUI](https://cindori.com/developer/floating-panel) · [Markus Bodner: Spotlight-like window](https://www.markusbodner.com/til/2021/02/08/create-a-spotlight/alfred-like-window-on-macos-with-swiftui/) · [Fazm: NSPanel patterns](https://fazm.ai/blog/swiftui-floating-panel)
- [Apple: MenuBarExtra](https://developer.apple.com/documentation/SwiftUI/MenuBarExtra) · [Nil Coalescing: macOS menu bar utility](https://nilcoalescing.com/blog/BuildAMacOSMenuBarUtilityInSwiftUI/) · [MenuBarExtraAccess](https://github.com/orchetect/MenuBarExtraAccess) · [FB13683957 menu re-render issue](https://github.com/feedback-assistant/reports/issues/477) · [Steipete: Settings from menu bar items](https://steipete.me/posts/2025/showing-settings-from-macos-menu-bar-items)
- [WWDC25 323: Build a SwiftUI app with the new design](https://developer.apple.com/videos/play/wwdc2025/323/) · [Tahoe Liquid Glass macOS walkthrough](https://medium.com/@dorangao/build-a-macos-swiftui-app-with-a-tahoe-style-liquid-glass-ui-fecb8029b2d8) · [swiftui-introspect sidebar background discussion](https://github.com/siteline/swiftui-introspect/discussions/378) · [HWS forum: NavigationSplitView on macOS](https://www.hackingwithswift.com/forums/macos/navigationsplitview-on-macos/24237)
- [Crux docs](https://redbadger.github.io/crux/) · [Crux core/shell interface](https://redbadger.github.io/crux/guide/message_interface.html) · [redbadger/crux](https://github.com/redbadger/crux)
- [mach_absolute_time](https://developer.apple.com/documentation/kernel/1462446-mach_absolute_time) · [mach_continuous_time](https://developer.apple.com/documentation/kernel/1646199-mach_continuous_time) · [SE-0329 Clock/Instant/Duration](https://github.com/swiftlang/swift-evolution/blob/main/proposals/0329-clock-instant-duration.md) · [ContinuousClock](https://developer.apple.com/documentation/swift/continuousclock) · [SuspendingClock](https://developer.apple.com/documentation/swift/suspendingclock)