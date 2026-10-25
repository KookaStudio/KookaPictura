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

    // UI scale choices in percent: 10 % steps from 80 to 200, plus the 75 %
    // floor the owner asked for. 100 is the default and leaves every metric
    // unchanged.
    static constexpr int kUiScaleMinPercent = 75;
    static constexpr int kUiScaleMaxPercent = 200;
    static constexpr int kUiScaleStepPercent = 10;
    static constexpr int kDefaultUiScalePercent = 100;

    // The palette family: a dark and a light ramp with identical role keys.
    enum class ThemeFamily { Dark, Light };
    static constexpr int kFamilyCount = 2;
    static constexpr ThemeFamily kDefaultFamily = ThemeFamily::Dark;

    static QString familyName(ThemeFamily family);
    static ThemeFamily familyFromName(const QString& name);

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

    // The UI scale choices, current percentage, nearest-choice normalization,
    // and the multiplier applied to chrome pixels.
    static QVector<int> uiScaleChoices();
    static int normalizedUiScalePercent(int percent);
    static int uiScalePercent();
    static void setUiScalePercent(int percent);
    static double uiScale();
    // Scale one chrome pixel value; a positive value never falls below 1.
    static int scaled(int value);

    // The named tokens of `family` at `level`: every colour the stylesheet
    // substitutes (including the derived shades) and every shared metric. The
    // stylesheet builder and the `--dump-theme` JSON both consume this list.
    static QVector<ThemeToken> tokens(ThemeFamily family, int level);
    static QVector<ThemeToken> tokens(int level);

    // CS6-style chrome QSS for `family` at `level`.
    static QString styleSheet(ThemeFamily family, int level);
    static QString styleSheet(int level);

    // Monotonic counter bumped on every apply(); icon engines fold it into
    // their cache key so QIcon re-renders after a brightness change.
    static quint64 paletteGeneration();

    // The brightness level most recently passed to apply(). A temporary preview
    // (the developer design-system dialog) reads it to restore the user's level
    // when it closes.
    static int currentLevel();

    // The family most recently passed to apply().
    static ThemeFamily currentFamily();

    // The empty-workspace surface for `level` (explicit #1f1f1f at the default
    // level). The level form is safe before the QApplication exists (static
    // canvas setup); the no-arg form tracks the last applied level.
    static QColor workspaceColor(int level);
    static QColor workspaceColor();

    // The current panel surface; custom-painted widgets use it to sit flush on
    // the panel/footer chrome.
    static QColor panelColor();

    // The current-level value of a colour token by key, for custom-painted
    // widgets that need a specific token rather than a QPalette role (for
    // example the paint-only graph tokens). Returns an invalid QColor if the
    // key is unknown.
    static QColor tokenColor(const QString& key);

    // Apply the Fusion style and the palette for `family` at `level`. Call once
    // after the QApplication exists, and again whenever the family or level
    // changes. The level-only overload applies the default family.
    static void apply(ThemeFamily family, int level);
    static void apply(int level);
};

} // namespace pictura
