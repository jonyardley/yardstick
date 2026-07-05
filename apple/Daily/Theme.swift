import SwiftUI

extension Color {
    /// OKLCH → sRGB (Björn Ottosson's reference OKLab constants), so tokens
    /// can be written exactly as the design doc specifies them.
    init(oklch L: Double, _ C: Double, _ hDegrees: Double) {
        let h = hDegrees * .pi / 180
        let a = C * cos(h)
        let b = C * sin(h)

        let l_ = L + 0.3963377774 * a + 0.2158037573 * b
        let m_ = L - 0.1055613458 * a - 0.0638541728 * b
        let s_ = L - 0.0894841775 * a - 1.2914855480 * b

        let l = l_ * l_ * l_
        let m = m_ * m_ * m_
        let s = s_ * s_ * s_

        let rLin = +4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s
        let gLin = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s
        let bLin = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s

        func gamma(_ c: Double) -> Double {
            let x = min(max(c, 0), 1)
            return x <= 0.0031308 ? 12.92 * x : 1.055 * pow(x, 1 / 2.4) - 0.055
        }
        self.init(.sRGB, red: gamma(rLin), green: gamma(gLin), blue: gamma(bLin), opacity: 1)
    }

    /// #RRGGBB hex token.
    init(hex: UInt32) {
        self.init(
            .sRGB,
            red: Double((hex >> 16) & 0xFF) / 255,
            green: Double((hex >> 8) & 0xFF) / 255,
            blue: Double(hex & 0xFF) / 255,
            opacity: 1)
    }
}

/// Design tokens — values verbatim from docs/design/handoff/README.md
/// §Design Tokens, plus the exact-color additions in
/// docs/design/reference/v2-today-view.md §11 that Phase 1 chrome uses.
/// One namespace; no view hardcodes a color/metric (spec §6).
enum Theme {
    // MARK: Accent (blue)
    static let accent = Color(oklch: 0.62, 0.13, 250)          // accent blue
    static let accentEyebrow = Color(oklch: 0.55, 0.13, 250)   // focus-bar eyebrow text
    static let accentText = Color(oklch: 0.50, 0.13, 250)      // link/chip text
    static let accentTextDark = Color(oklch: 0.48, 0.13, 250)  // pill text
    static let focusTintBg = Color(oklch: 0.965, 0.025, 250)
    static let focusTintBorder = Color(oklch: 0.88, 0.05, 250)
    static let chipTintBorder = Color(oklch: 0.85, 0.05, 250)
    static let selectedChipTint = Color(oklch: 0.96, 0.02, 250)
    static let pillTint = Color(oklch: 0.95, 0.04, 250)

    // MARK: Priority
    static let priority1 = Color(oklch: 0.60, 0.16, 25)
    static let priority2 = Color(oklch: 0.70, 0.13, 70)
    static let priority3 = Color(hex: 0xB0B0AE)

    // MARK: Status
    static let statusInProgress = accent
    static let statusWaiting = Color(oklch: 0.70, 0.12, 70)
    static let statusWaitingBg = Color(oklch: 0.96, 0.05, 70)
    static let statusBlocked = Color(oklch: 0.60, 0.16, 25)
    static let statusBlockedBg = Color(oklch: 0.96, 0.04, 25)
    static let statusDone = Color(oklch: 0.62, 0.12, 150)
    static let statusBacklog = Color(hex: 0xB0B0AE)
    static let statusBinned = Color(hex: 0xC4C3C0)

    // MARK: People / projects
    static let personAccent = Color(oklch: 0.58, 0.14, 300)
    static let personTintBg = Color(oklch: 0.96, 0.03, 300)
    static let spaceBadge = Color(oklch: 0.62, 0.16, 25)       // "RB" avatar (§2.1)
    static let projectGreen = Color(oklch: 0.62, 0.12, 150)
    static let amberDot = Color(oklch: 0.70, 0.12, 70)

    // MARK: Neutrals (handoff token list + §11 tiers)
    static let textPrimary = Color(hex: 0x1D1D1F)
    static let textBody = Color(hex: 0x3A3A3C)                  // note body
    static let textSecondary = Color(hex: 0x6E6E73)
    static let textTertiary = Color(hex: 0x86868B)
    static let textQuiet = Color(hex: 0x9A9A98)                 // ghost line, metas
    static let textMuted = Color(hex: 0xA0A09E)                 // section labels, counts
    static let textQuaternary = Color(hex: 0xB0B0AE)            // bullets, chevrons
    static let countEmpty = Color(hex: 0xB8B8B6)                // zero counts (never red)
    static let textDisabled = Color(hex: 0xC0C0BE)              // other-month/weekend days
    static let chipBg = Color(hex: 0xF1F0EE)
    static let blockBg = Color(hex: 0xFAF9F7)
    static let hoverBg = Color(hex: 0xF8F7F5)
    static let sidebarBg = Color(hex: 0xEDECEA)
    static let segmentRemaining = Color(hex: 0xDEDCD8)
    static let calendarCardBg = Color.white.opacity(0.55)
    static let calendarOutline = Color(hex: 0xC9C8C4)           // yesterday's ring (§2.3)

    // MARK: Hairlines (rgba(0,0,0,0.06–0.12) family)
    static let hairline06 = Color.black.opacity(0.06)
    static let hairline08 = Color.black.opacity(0.08)
    static let hairline09 = Color.black.opacity(0.09)           // sidebar right edge
    static let hairline10 = Color.black.opacity(0.10)
    static let hairline12 = Color.black.opacity(0.12)

    // MARK: Metrics (reference §§0–5)
    enum Metrics {
        static let sidebarWidth: CGFloat = 238
        static let contentMaxWidth: CGFloat = 760
        static let noteMaxWidth: CGFloat = 640
        static let sidebarRowHeight: CGFloat = 29
        static let sidebarActiveRowHeight: CGFloat = 30
        static let sidebarRowRadius: CGFloat = 7
        static let calendarCardRadius: CGFloat = 9
        static let calendarCellHeight: CGFloat = 22
        static let calendarDayCircle: CGFloat = 21
        static let cardRadius: CGFloat = 12
        static let rowRadius: CGFloat = 8
        static let buttonRadius: CGFloat = 7
        static let plusButtonSize: CGFloat = 28
        static let contentPaddingH: CGFloat = 28
        static let contentPaddingTop: CGFloat = 22
    }

    // MARK: Typography (handoff §Design Tokens "Type")
    enum Typography {
        static let dateTitle = Font.system(size: 25, weight: .bold)      // -0.02em kerning at use site
        static let sectionHeader = Font.system(size: 16, weight: .bold)
        static let body = Font.system(size: 14)                          // line-height 1.65 at use site
        static let sidebarRow = Font.system(size: 13)
        static let sidebarRowActive = Font.system(size: 13, weight: .medium)
        static let spaceName = Font.system(size: 13, weight: .semibold)
        static let capsLabel = Font.system(size: 11, weight: .bold)      // +0.06em, uppercase at use site
        static let count = Font.system(size: 11)
        static let calendarHeader = Font.system(size: 12.5, weight: .semibold)
        static let calendarWeekday = Font.system(size: 9.5, weight: .semibold)
        static let calendarDay = Font.system(size: 11.5)
        static let meta = Font.system(size: 11.5)
        static let ghost = Font.system(size: 14)
    }
}
