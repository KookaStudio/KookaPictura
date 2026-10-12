#pragma once

#include <QtCore/QString>
#include <QtCore/QVector>
#include <QtGui/QColor>

namespace pictura {

// One named theme token: a stylesheet substitution key with its value. Colours
// are `#rrggbb`; metrics are decimal strings. `Theme::tokens` is the single
// source shared by the stylesheet builder, the `--dump-theme` JSON, and tests.
struct ThemeToken {
    QString key;
    QString value;
    bool color = false;
};

// Single source of truth for the application theme. Applies the Fusion style
// and a dark QPalette for one of four brightness levels (0 darkest .. 3
// lightest). No widget may hard-code a frame colour.
class Theme {
public:
    static constexpr int kLevelCount = 4;
    static constexpr int kDefaultLevel = 1;

    // M44: chosen chrome dimensions (unsourced CS6 metrics), shared so the
    // stylesheet and the splitter agree.
    static constexpr int kPanelBorderWidth = 1;
    static constexpr int kGroupDividerWidth = 4;
    // Shell chrome lines are heavier than in-body lines.
    static constexpr int kChromeBorderWidth = 2;

    // Tool-slot base metrics at 96 DPI; scaled by the owning widget's screen.
    static constexpr int kSlotBaseW = 36;
    static constexpr int kSlotBaseH = 28;
    static constexpr int kSlotIconMaxW = 24;
    static constexpr int kSlotIconMaxH = 20;

    // One background shade increment: positive lightens, negative darkens.
    static constexpr int kShadeStep = 112;
    static QColor shade(QColor color, int steps);

    // Clamp a level into [0, kLevelCount).
    static int clampLevel(int level);

    // The named tokens of `level`: every colour the stylesheet substitutes
    // (including the derived shades) and every shared metric. The stylesheet
    // builder and the `--dump-theme` JSON both consume this list.
    static QVector<ThemeToken> tokens(int level);

    // CS6-style chrome QSS for `level`.
    static QString styleSheet(int level);

    // Monotonic counter bumped on every apply(); icon engines fold it into
    // their cache key so QIcon re-renders after a brightness change.
    static quint64 paletteGeneration();

    // The empty-workspace surface for `level` (explicit #1f1f1f at the default
    // level). The level form is safe before the QApplication exists (static
    // canvas setup); the no-arg form tracks the last applied level.
    static QColor workspaceColor(int level);
    static QColor workspaceColor();

    // The current panel surface; custom-painted widgets use it to sit flush on
    // the panel/footer chrome.
    static QColor panelColor();

    // Apply the Fusion style and the palette for `level`. Call once after the
    // QApplication exists, and again whenever the level changes.
    static void apply(int level);
};

} // namespace pictura
