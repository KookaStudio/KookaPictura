#include "theme.h"

#include <QtCore/QString>

#include <QtGui/QColor>
#include <QtGui/QFont>
#include <QtGui/QPainter>
#include <QtGui/QPalette>
#include <QtGui/QPixmapCache>
#include <QtWidgets/QAbstractSpinBox>
#include <QtWidgets/QApplication>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QProxyStyle>
#include <QtWidgets/QStyleOption>
#include <QtWidgets/QStyleFactory>
#include <QtWidgets/QToolButton>

#include "icons.h"

namespace pictura {

namespace {

// Fusion under the app stylesheet draws a spin box's up/down buttons as dark
// outlines with no visible arrows. Paint the button cells and draw the arrow as
// a small Lucide chevron in the widget's foreground colour, tinted at render
// time (the stylesheet engine cannot tint an SVG). The spin-box and menu-button
// arrows go through here; the select's chevron is a per-family stylesheet image,
// but the combo branch keeps a combo tinted if the stylesheet supplies none.
class SpinArrowStyle : public QProxyStyle {
public:
    using QProxyStyle::QProxyStyle;

    void drawPrimitive(PrimitiveElement element, const QStyleOption* option, QPainter* painter,
                       const QWidget* widget) const override
    {
        const bool spin = qobject_cast<const QAbstractSpinBox*>(widget) != nullptr;
        if (spin && element == PE_PanelButtonBevel) {
            const bool sunken = option->state & (State_Sunken | State_On);
            painter->fillRect(option->rect, option->palette.color(QPalette::Button)
                                                .lighter(sunken ? 100 : 135));
            return;
        }
        const bool up = element == PE_IndicatorSpinUp || element == PE_IndicatorArrowUp;
        const bool down = element == PE_IndicatorSpinDown || element == PE_IndicatorArrowDown;
        const bool combo = qobject_cast<const QComboBox*>(widget) != nullptr;
        const bool menuButton = qobject_cast<const QToolButton*>(widget) != nullptr;
        if ((!spin && !combo && !menuButton) || (!up && !down)) {
            QProxyStyle::drawPrimitive(element, option, painter, widget);
            return;
        }
        const bool enabled = option->state & State_Enabled;
        const QColor color = option->palette.color(enabled ? QPalette::Active : QPalette::Disabled,
                                                   QPalette::ButtonText);
        constexpr int kGlyph = 10;
        QRect box(0, 0, kGlyph, kGlyph);
        box.moveCenter(option->rect.center());
        painter->save();
        painter->setRenderHint(QPainter::Antialiasing);
        icon(up ? QStringLiteral("chevron.up") : QStringLiteral("chevron.down"), color)
            .paint(painter, box);
        painter->restore();
    }
};

quint64 g_paletteGeneration = 0;
int g_level = Theme::kDefaultLevel;
Theme::ThemeFamily g_family = Theme::kDefaultFamily;

struct Ramp {
    // Text and accent colours are constant across brightness levels.
    QColor content;
    QColor contentBody;
    QColor contentControl;
    QColor selection;
    QColor contentOnSelection;
    QColor surfaceTooltip;
    QColor contentTooltip;
    QColor contentLink;
    QColor contentDisabled;
    // The drop/insertion accent (CS6's #2a7fff), constant across levels.
    QColor accent;
    // Inline warning and info text (a `notice` widget): dark text on yellow,
    // constant across levels so it stands out on every theme.
    QColor surfaceNotice;
    QColor contentNotice;
    // Surfaces. Level 1 holds the explicit CS6 dark swatches; the other levels
    // derive from them by shifting every surface with `shade(level - 1)`.
    QColor surfaceRecessed;
    QColor surface;
    QColor surfaceTabActive;
    QColor surfaceCanvas;
    QColor surfaceHeader;
    QColor borderSubtle;
    QColor surfaceField;
    QColor border;
    QColor surfaceInput;
    QColor borderInput;
    QColor surfaceControl;
    QColor borderControl;
    QColor surfaceControlPressed;
    QColor surfaceControlActive;
    QColor borderControlHover;
    QColor surfaceHover;
    QColor surfaceControlHover;
    // Custom-painted graph surfaces (curve editor, spectrum, histogram,
    // navigator) so their greys track the ramp instead of being literals.
    QColor graphFill;
    QColor graphAxis;
};

// The default-brightness (level 1) palette. Other levels are derived from it.
const Ramp kLevel1Ramp = {
    QColor(224, 224, 224),    // content
    QColor(224, 224, 224),    // contentBody
    QColor(224, 224, 224),    // contentControl
    QColor(61, 111, 153),     // selection
    QColor(255, 255, 255),    // contentOnSelection
    QColor(58, 58, 58),       // surfaceTooltip
    QColor(232, 232, 232),    // contentTooltip
    QColor(111, 168, 220),    // contentLink
    QColor(104, 104, 104),    // contentDisabled
    QColor(0x2a, 0x7f, 0xff), // accent
    QColor(0xf2, 0xd0, 0x4b), // surfaceNotice
    QColor(0x1e, 0x1e, 0x1e), // contentNotice
    QColor(0x36, 0x36, 0x36), // surfaceRecessed
    QColor(0x4d, 0x4d, 0x4d), // surface
    QColor(0x4d, 0x4d, 0x4d), // surfaceTabActive
    QColor(0x1f, 0x1f, 0x1f), // surfaceCanvas
    QColor(0x36, 0x36, 0x36), // surfaceHeader
    QColor(0x40, 0x40, 0x40), // borderSubtle
    QColor(0x40, 0x40, 0x40), // surfaceField
    QColor(0x2e, 0x2e, 0x2e), // border
    QColor(0x3b, 0x3b, 0x3b), // surfaceInput
    QColor(0x59, 0x59, 0x59), // borderInput
    QColor(0x3b, 0x3b, 0x3b), // surfaceControl
    QColor(0x59, 0x59, 0x59), // borderControl
    QColor(0x30, 0x30, 0x30), // surfaceControlPressed
    QColor(0x2e, 0x2e, 0x2e), // surfaceControlActive
    QColor(0x59, 0x59, 0x59), // borderControlHover
    QColor(),                 // surfaceHover (filled in rampFor)
    QColor(),                 // surfaceControlHover (filled in rampFor)
    QColor(0x55, 0x55, 0x55), // graphFill
    QColor(0x80, 0x80, 0x80), // graphAxis
};

// The light family's default-brightness (level 1) palette: the dark ramp's role
// keys with light surfaces and dark text/icons. Derived levels and shades use
// the per-family direction in `rampFor`/`tokens`.
const Ramp kLight1Ramp = {
    QColor(43, 43, 43),       // content
    QColor(43, 43, 43),       // contentBody
    QColor(43, 43, 43),       // contentControl
    QColor(61, 111, 153),     // selection
    QColor(255, 255, 255),    // contentOnSelection
    QColor(0xf0, 0xf0, 0xf0), // surfaceTooltip
    QColor(43, 43, 43),       // contentTooltip
    QColor(0x1a, 0x5f, 0xa8), // contentLink
    QColor(0x9a, 0x9a, 0x9a), // contentDisabled
    QColor(0x2a, 0x7f, 0xff), // accent
    QColor(0xf2, 0xd0, 0x4b), // surfaceNotice
    QColor(0x1e, 0x1e, 0x1e), // contentNotice
    QColor(0xd9, 0xd9, 0xd9), // surfaceRecessed
    QColor(0xe4, 0xe4, 0xe4), // surface
    QColor(0xf0, 0xf0, 0xf0), // surfaceTabActive
    QColor(0xf0, 0xf0, 0xf0), // surfaceCanvas
    QColor(0xd0, 0xd0, 0xd0), // surfaceHeader
    QColor(0xc0, 0xc0, 0xc0), // borderSubtle
    QColor(0xff, 0xff, 0xff), // surfaceField
    QColor(0xb0, 0xb0, 0xb0), // border
    QColor(0xff, 0xff, 0xff), // surfaceInput
    QColor(0xa0, 0xa0, 0xa0), // borderInput
    QColor(0xed, 0xed, 0xed), // surfaceControl
    QColor(0xa0, 0xa0, 0xa0), // borderControl
    QColor(0xd0, 0xd0, 0xd0), // surfaceControlPressed
    QColor(0xd0, 0xd0, 0xd0), // surfaceControlActive
    QColor(0xa0, 0xa0, 0xa0), // borderControlHover
    QColor(),                 // surfaceHover (filled in rampFor)
    QColor(),                 // surfaceControlHover (filled in rampFor)
    QColor(0xf5, 0xf5, 0xf5), // graphFill
    QColor(0x80, 0x80, 0x80), // graphAxis
};

Ramp rampFor(Theme::ThemeFamily family, int level)
{
    Ramp ramp = family == Theme::ThemeFamily::Light ? kLight1Ramp : kLevel1Ramp;
    const int steps = level - Theme::kDefaultLevel;
    QColor* surfaces[] = {
        &ramp.surfaceRecessed,       &ramp.surface,              &ramp.surfaceTabActive,
        &ramp.surfaceCanvas,         &ramp.surfaceHeader,        &ramp.borderSubtle,
        &ramp.surfaceField,          &ramp.border,               &ramp.surfaceInput,
        &ramp.borderInput,           &ramp.surfaceControl,       &ramp.borderControl,
        &ramp.surfaceControlPressed, &ramp.surfaceControlActive, &ramp.borderControlHover,
        &ramp.graphFill,             &ramp.graphAxis,
    };
    for (QColor* surface : surfaces) {
        *surface = Theme::shade(*surface, steps);
    }
    // Hover reads as a lift on the dark ramp and as a shade-down on the light
    // ramp, where a lighter hover would wash out against the surface.
    const int hoverDirection = family == Theme::ThemeFamily::Light ? -1 : 1;
    ramp.surfaceHover = Theme::shade(ramp.surface, hoverDirection);
    ramp.surfaceControlHover = ramp.surfaceHover;
    return ramp;
}

QPalette paletteFor(Theme::ThemeFamily family, const Ramp& ramp)
{
    // Alternate rows shade toward the family's direction, matching the token
    // layer; a light `#ffffff` table would otherwise stay white.
    const int dir = family == Theme::ThemeFamily::Light ? -1 : 1;
    QPalette pal;
    pal.setColor(QPalette::Window, ramp.surfaceRecessed);
    pal.setColor(QPalette::WindowText, ramp.content);
    pal.setColor(QPalette::Base, ramp.surfaceField);
    pal.setColor(QPalette::AlternateBase, Theme::shade(ramp.surfaceField, dir));
    pal.setColor(QPalette::Text, ramp.contentBody);
    pal.setColor(QPalette::Button, ramp.surfaceControl);
    pal.setColor(QPalette::ButtonText, ramp.contentControl);
    pal.setColor(QPalette::Mid, ramp.borderControl);
    pal.setColor(QPalette::Dark, ramp.border);
    pal.setColor(QPalette::Shadow, ramp.border);
    pal.setColor(QPalette::Highlight, ramp.selection);
    pal.setColor(QPalette::HighlightedText, ramp.contentOnSelection);
    pal.setColor(QPalette::ToolTipBase, ramp.surfaceTooltip);
    pal.setColor(QPalette::ToolTipText, ramp.contentTooltip);
    pal.setColor(QPalette::Link, ramp.contentLink);
    pal.setColor(QPalette::Disabled, QPalette::Text, ramp.contentDisabled);
    pal.setColor(QPalette::Disabled, QPalette::WindowText, ramp.contentDisabled);
    pal.setColor(QPalette::Disabled, QPalette::ButtonText, ramp.contentDisabled);
    return pal;
}

// The stylesheet only re-colours surfaces; the CS6 dark swatches are explicit
// in `kLevel1Ramp` and shifted per brightness level by `rampFor`.
QString styleSheetFor(const QVector<ThemeToken>& tokens)
{
    QString qss = QStringLiteral(R"(
QMainWindow { background: ${surface}; }
QMainWindow::separator { background: ${border}; width: 3px; height: 3px; }

QMenuBar { background: ${surface}; color: ${content}; }
QMenuBar::item { background: transparent; color: ${content}; padding: 4px 5px; }
QMenuBar::item:selected { background: ${selection}; color: ${content-on-selection}; }
QMenuBar::item:pressed { background: ${surface-pressed}; color: ${content-control}; }

QMenu { background: ${surface}; color: ${content-body}; border: 1px solid ${border}; }
QMenu::item { background: transparent; padding: 4px 22px; }
QMenu::item:selected { background: ${selection}; color: ${content-on-selection}; }
QMenu::item:disabled { color: ${content-disabled-menu}; }
QMenu::separator { background: ${border-subtle}; height: 1px; margin: 4px 6px; }

QToolBar { background: ${surface}; color: ${content}; border: 0; spacing: 2px; padding: 2px; }
QToolBar::separator { background: ${border}; width: ${border-width-chrome}px; margin: 3px 2px; }
QToolBar#optionsBar { border-bottom: ${border-width-panel}px solid ${border}; border-top: 1px solid ${border-subtle}; padding-right: 8px; }
QFrame#optionsToolSeparator { background: ${border-subtle-alt}; border: 0; }

QToolButton { background: transparent; color: ${content-control}; border: 1px solid transparent; border-radius: 3px; padding: 3px; }
QToolButton:hover { background: ${surface-control-hover}; border-color: ${border-control-hover}; }
QToolButton:pressed { background: ${surface-control-active}; border-color: ${border-control-hover}; }
QToolButton:checked { background: ${surface-control-active}; border-color: ${border-control-hover}; color: ${content-control}; }
QToolButton:disabled { color: ${content-disabled}; }
QToolButton:focus { border-color: ${selection}; }
/* Tool slots sit flush on the toolbar at rest; only hover/press/active differ. */
QToolButton#toolSlotButton { background: transparent; border: 1px solid transparent; border-radius: 3px; }
QToolButton#toolSlotButton:hover { background: ${surface-control-hover}; border-color: ${border-control-hover}; }
QToolButton#toolSlotButton:pressed, QToolButton#toolSlotButton:checked { background: ${surface-control-active}; border-color: ${border-control-hover}; }
QToolButton#statusOptionsButton { background: transparent; border: 0; }
QToolButton#statusOptionsButton::menu-indicator { image: none; width: 0; }

QDockWidget { color: ${content}; }
QDockWidget::title { background: ${surface-recessed}; color: ${content}; padding: 3px 6px; border-bottom: 1px solid ${border}; }
QDockWidget::close-button, QDockWidget::float-button { background: transparent; border: 0; }

QTabBar::tab { background: ${surface-recessed}; color: ${content}; border: 1px solid ${border}; border-bottom: 0; padding: 4px 10px; margin-right: 1px; }
QTabBar::tab:hover { background: ${surface-hover}; }
QTabBar::tab:selected { background: ${surface-tab-active}; color: ${content}; }
QTabBar::tab:disabled { color: ${content-disabled}; }

/* M44: the active panel tab takes the panel surface (`${surface}`) so it reads as
   continuous with the body; inactive tabs recede to the darker `${surface-header}`.
   The panel pane is `${surface}` too, so active == pane and inactive != pane. The
   header strip behind the tabs is the inactive-tab shade, so it is no lighter
   than the tabs. The document tab bar keeps the unscoped `QTabBar::tab` rules
   above. */
QTabBar#panelTabBar { background: ${surface-header}; }
QTabBar#panelTabBar::tab { background: ${surface-header}; color: ${content}; border: ${border-width-panel}px solid ${border}; border-bottom: 0; padding: 4px 8px; margin-right: 1px; font-size: ${font-size-tab}px; font-weight: 700; }
QTabBar#panelTabBar::tab:hover { background: ${surface-hover}; }
QTabBar#panelTabBar::tab:selected { background: ${surface}; color: ${content}; }

QSplitter#panelColumnSplitter::handle { background: ${border}; }
QFrame#panelIconDivider { background: transparent; border: 0; }
QWidget#panelIconGroup { background: ${surface}; border: 0; border-bottom: 1px solid ${border-subtle}; }
QWidget#panelIconGroupGrip { background: transparent; color: ${content-disabled}; }
/* M47: a collapsed group's iconic row reuses the docked strip's
   `panelIconGroup` box, so the float and the strip read alike. The header band,
   the reserved corner grip, corner container, and its `▾` button share the
   header strip; the band spans the whole group width behind the tabs and corner
   so no vertical slice of the header is left unpainted. */
QWidget#panelHeaderBand { background: ${surface-header}; }
QWidget#panelGroupDragGrip { background: ${surface-header}; }
QWidget#panelWidgetCorner { background: ${surface-header}; }
QWidget#panelWidgetCorner QToolButton { background: ${surface-header}; color: ${content-control}; border: 0; border-radius: 0; padding: 0; }
QWidget#panelWidgetCorner QToolButton::menu-indicator { image: none; width: 0; }
QWidget#panelWidgetCorner QToolButton:hover { background: ${surface-hover}; }
QWidget#panelWidgetCorner QToolButton:pressed { background: ${surface-pressed}; }

QTabWidget::pane { border: ${border-width-panel}px solid ${border}; background: ${surface-canvas}; }
QTabWidget#panelGroupTabs { border: ${border-width-panel}px solid ${border}; }
QTabWidget#panelGroupTabs::pane { border: 0; background: ${surface}; }
QTabWidget#panelGroupTabs QStackedWidget { background: ${surface}; }
QTabWidget#documentTabs::pane { border: 0; }
QTabWidget#documentTabs QStackedWidget { background: ${surface-canvas}; }
QTabBar#documentTabBar { border: 0; }
QTabBar#documentTabBar { background: ${surface-header}; }
QTabBar#documentTabBar::tab { font-size: ${font-size-tab}px; font-weight: 700; padding-right: 4px; border: 0; }
QToolButton#tabCloseButton { border: 0; background: transparent; }
QToolButton#tabCloseButton:hover, QToolButton#tabCloseButton:pressed { background: ${surface-hover}; }
/* Floating document windows and tile panes carry the document tab's header. */
QWidget#documentFloatWindow { background: ${surface}; border: ${border-width-panel}px solid ${border}; }
QWidget#documentFloatHeader, QWidget#documentTileHeader { background: ${surface-header}; border: 0; border-bottom: 1px solid ${border-header}; }
QLabel#documentFloatTitle, QLabel#documentTileTitle { font-size: ${font-size-tab}px; font-weight: 700; padding-left: 6px; }
QWidget#documentFloatBody, QWidget#documentTilePane { background: ${surface-canvas}; }
QToolButton#documentFloatClose, QToolButton#documentTileClose { border: 0; background: transparent; }
QToolButton#documentFloatClose:hover, QToolButton#documentFloatClose:pressed,
QToolButton#documentTileClose:hover, QToolButton#documentTileClose:pressed { background: ${surface-hover}; }
/* Columns keep their side rules only; the top and bottom edges are open, and
   the header carries no rule of its own. */
QWidget#panelColumnContainer { border: 0; border-left: ${border-width-panel}px solid ${border-subtle}; border-right: ${border-width-panel}px solid ${border-subtle}; border-bottom: ${border-width-panel}px solid ${border-subtle}; background: ${surface}; }
QWidget#toolsColumn { border: 0; border-left: ${border-width-panel}px solid ${border-subtle}; border-right: ${border-width-panel}px solid ${border-subtle}; border-bottom: ${border-width-panel}px solid ${border-subtle}; background: ${surface}; }
/* The central band's 1 px inner frame line; the outer 2 px is the window
   `${surface}`. The outermost column's frame-side border is suppressed so the
   band edge reads as exactly 2 px panel + 1 px border. */
QWidget#workspaceFrameLine { background: ${border}; }
QWidget#panelColumnContainer[frameEdge="left"], QWidget#toolsColumn[frameEdge="left"] { border-left: 0; }
QWidget#panelColumnContainer[frameEdge="right"], QWidget#toolsColumn[frameEdge="right"] { border-right: 0; }
QWidget#panelColumnHeader { background: ${surface-header}; border: 0; border-bottom: 1px solid ${border-header}; }
QWidget#panelFloatHeader { background: ${surface-header}; border: 0; }
/* The icon strip sits inside the bordered column container, so it must not
   draw its own side rule (that would double the column border). */
QWidget#panelColumnIconStrip { border: 0; background: ${surface}; }
QWidget#toolsPanel { background: ${surface}; border: 0; }
QWidget#panelIconFlyout { background: ${surface}; border: ${border-width-panel}px solid ${border}; }
QWidget#panelFloat { background: ${surface}; border: ${border-width-panel}px solid ${border}; }
/* The Brush Preset picker: a non-grabbing in-window overlay, styled like a
   floating panel so it reads as a popup. */
QWidget#brushPresetPicker { background: ${surface}; border: ${border-width-panel}px solid ${border}; }
/* Help > Search (Shift+Space): the same floating-panel look; the result list
   sits flush on it, and a row that cannot run is greyed. */
QFrame#searchBar { background: ${surface}; border: ${border-width-panel}px solid ${border}; }
QListWidget#searchResults { background: ${surface}; border: 0; }
QListWidget#searchResults::item { padding: 3px 6px; }
QListWidget#searchResults::item:disabled { color: ${content-disabled}; }
QLabel#searchEmpty, QLabel#searchFooter { color: ${content-disabled}; }
/* The reveal outline: a short accent pulse over the found widget. */
QWidget#searchFlash { background: transparent; border: 2px solid ${accent}; }
/* Frameless dialog chrome (dialog_chrome.cpp): a header-shade title strip and
   a 1 px outline drawn over the dialog's edge. */
QWidget#dialogTitleBar { background: ${surface-header}; border: 0; border-bottom: 1px solid ${border-header}; }
QLabel#dialogTitleLabel { background: transparent; color: ${content}; font-weight: 700; }
QToolButton#dialogCloseButton { background: transparent; border: 0; padding: 2px; }
QToolButton#dialogCloseButton:hover { background: ${surface-hover}; }
QToolButton#dialogCloseButton:pressed { background: ${surface-pressed}; }
QWidget#dialogFrame { background: transparent; border: 1px solid ${border}; }
/* Inline warning / info text: a label flagged `notice="true"` reads
   dark-on-yellow; its icon and any buttons beside it keep their own look. */
QLabel[notice="true"] { background: ${surface-notice}; color: ${content-notice}; border: 0; border-radius: 2px; padding: 3px 6px; }
/* Info panel readout grid: only the inner cross of the 2x2 table is drawn, in
   the frame shade; the outer edge is the panel pane itself. The blocks flag
   their inner sides with the `gridRight`/`gridBottom` dynamic properties. */
QWidget#infoBlock[gridRight="true"] { border-right: 1px solid ${border-subtle}; }
QWidget#infoBlock[gridBottom="true"] { border-bottom: 1px solid ${border-subtle}; }
QWidget#infoRule { background: ${border-subtle}; }
/* The readout icons are menu affordances, not buttons: bare icon at rest. */
QWidget#infoBlock QToolButton { background: transparent; border: 0; padding: 2px; }
QWidget#infoBlock QToolButton:hover { background: ${surface-hover}; }
QWidget#infoBlock QToolButton:pressed { background: ${surface-pressed}; }

)") + QStringLiteral(R"(
QStatusBar { background: ${surface}; color: ${content}; }
QStatusBar::item { border: 0; }
QFrame#statusSeparator { background: ${border-subtle}; border: 0; }

QScrollBar:vertical { background: ${surface-scrollbar}; width: 12px; border: 0; margin: 0; }
QScrollBar::handle:vertical { background: ${scrollbar-thumb}; min-height: 24px; border-radius: 3px; margin: 2px; }
QScrollBar::handle:vertical:hover { background: ${scrollbar-thumb-hover}; }
QScrollBar:horizontal { background: ${surface-scrollbar}; height: 12px; border: 0; margin: 0; }
QScrollBar::handle:horizontal { background: ${scrollbar-thumb}; min-width: 24px; border-radius: 3px; margin: 2px; }
QScrollBar::handle:horizontal:hover { background: ${scrollbar-thumb-hover}; }
QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical,
QScrollBar::add-line:horizontal, QScrollBar::sub-line:horizontal { background: transparent; border: 0; width: 0; height: 0; }
QScrollBar::add-page, QScrollBar::sub-page { background: transparent; }
/* The empty cell where the canvas's two scrollbars meet takes the scrollbar
   track colour instead of the workspace showing through. */
QWidget#canvasScrollCorner { background: ${surface-scrollbar}; }
QWidget#canvasRulerCorner { background: ${surface}; }

QToolTip { background: ${surface-tooltip}; color: ${content-tooltip}; border: 1px solid ${border}; padding: 2px; }

QListView, QTreeView, QTableView { background: ${surface-field}; color: ${content-body}; border: 1px solid ${border-subtle}; alternate-background-color: ${surface-field-alternate}; selection-background-color: ${selection}; selection-color: ${content-on-selection}; }
QLineEdit, QSpinBox, QDoubleSpinBox, QComboBox, QPlainTextEdit, QTextEdit { background: ${surface-input}; color: ${content-body}; border: 1px solid ${border-input}; border-radius: 2px; padding: 1px 2px; font-size: ${font-size-input}px; }
/* Spin boxes stand this tall (their buttons set the floor); lift the other
   inputs to match, so a select and a text field are the same height. */
QLineEdit, QComboBox, QPlainTextEdit, QTextEdit { min-height: 20px; }
QLineEdit:hover, QSpinBox:hover, QDoubleSpinBox:hover, QComboBox:hover, QPlainTextEdit:hover, QTextEdit:hover { border-color: ${border-input}; }
QLineEdit:focus, QSpinBox:focus, QDoubleSpinBox:focus, QComboBox:focus, QPlainTextEdit:focus, QTextEdit:focus { border-color: ${selection}; }
QLineEdit:disabled, QSpinBox:disabled, QDoubleSpinBox:disabled, QComboBox:disabled, QPlainTextEdit:disabled, QTextEdit:disabled { color: ${content-disabled}; border-color: ${border}; }
/* An editable combo's line edit is the field itself: no second frame. */
QComboBox QLineEdit { background: transparent; border: 0; border-radius: 0; padding: 0 2px; font-size: ${font-size-input}px; }
/* A flat drop-down cell reserves the value's right space; the chevron is a
   baked per-family asset because the stylesheet engine cannot tint an SVG. */
QComboBox::drop-down { background: transparent; border: 0; width: 18px; }
QComboBox::down-arrow { image: url(${select-chevron}); width: 10px; height: 10px; }
QComboBox::down-arrow:disabled { image: url(:/ui/select-chevron-disabled.svg); }
QComboBox QAbstractItemView { background: ${surface}; color: ${content-body}; border: 1px solid ${border}; selection-background-color: ${selection}; selection-color: ${content-on-selection}; }
/* The workspace switcher reads as a select on the same input surface. */
QToolButton#workspaceSwitcher { background: ${surface-input}; color: ${content-body}; border: 1px solid ${border-input}; border-radius: 2px; padding: 1px 18px 1px 4px; font-size: ${font-size-input}px; }
QToolButton#workspaceSwitcher:hover { border-color: ${border-input}; }
QToolButton#workspaceSwitcher:focus, QToolButton#workspaceSwitcher:open { border-color: ${selection}; }
QToolButton#workspaceSwitcher:disabled { color: ${content-disabled}; border-color: ${border}; }
QToolButton#workspaceSwitcher::menu-button { background: transparent; border: 0; width: 18px; }
QToolButton#workspaceSwitcher::menu-indicator { subcontrol-origin: padding; subcontrol-position: center right; image: url(${select-chevron}); width: 10px; height: 10px; }

/* The numeric-field arrow sits inside the box, separated by a hairline; the
   button darkens on hover and while its slider popup is open. */
QLineEdit QToolButton { background: transparent; color: ${content-control}; border: 0; border-left: 1px solid ${border}; border-radius: 0; padding: 0 2px; }
QLineEdit QToolButton:hover { background: ${surface-hover}; }
QLineEdit QToolButton:pressed, QLineEdit QToolButton:checked { background: ${surface-pressed}; }
QLineEdit QToolButton:disabled { color: ${content-disabled}; }
QTabWidget::tab-bar { alignment: left; }

QWidget#percentField QSlider::groove:horizontal { height: 4px; background: ${border}; border-radius: 2px; }
QWidget#percentField QSlider::handle:horizontal { width: 10px; margin: -4px 0; background: ${content-control}; border-radius: 3px; }

QWidget#layersFilterBar { background: ${surface}; border-bottom: 1px solid ${border-subtle}; }
QWidget#layersFilterBar QComboBox, QWidget#layersFilterBar QLineEdit { background: ${surface-input}; color: ${content-body}; border: 1px solid ${border-input}; border-radius: 2px; padding: 1px 2px; }
QWidget#layersFilterBar QComboBox:disabled, QWidget#layersFilterBar QLineEdit:disabled { color: ${content-disabled}; }
QWidget#layersFilterBar QToolButton { background: transparent; color: ${content-control}; border: 1px solid transparent; border-radius: 3px; padding: 2px 4px; }
QWidget#layersFilterBar QToolButton:hover { background: ${surface-control-hover}; border-color: ${border-control-hover}; }
QWidget#layersFilterBar QToolButton:checked { background: ${surface-control-active}; border-color: ${border-control-hover}; }
QWidget#layersFilterBar QToolButton#layersFilterToggle:checked { background: ${selection}; color: ${content-on-selection}; }

QPushButton { background: ${surface-control}; color: ${content-control}; border: 1px solid ${border-control}; border-radius: 3px; padding: 4px 10px; }
QPushButton:hover { background: ${surface-control}; border-color: ${border-control}; }
QPushButton:pressed { background: ${surface-control-pressed}; }
QPushButton:checked { background: ${surface-control-pressed}; border-color: ${border-input}; }
QPushButton:focus { border-color: ${selection}; }
QPushButton:disabled { color: ${content-disabled}; border-color: ${border}; }

/* Design-system primitives: one global definition per standard control, every
   value from a token. Object names scope the controls that need their own
   surface; the rest are type rules. */
QCheckBox { color: ${content-body}; spacing: 5px; }
QCheckBox:disabled { color: ${content-disabled}; }
QCheckBox::indicator { width: 13px; height: 13px; border: 1px solid ${border-input}; border-radius: 2px; background: ${surface-input}; }
QCheckBox::indicator:hover { border-color: ${selection}; }
QCheckBox::indicator:checked { background: ${selection}; border-color: ${selection}; image: url(:/ui/checkbox-check.svg); }
QCheckBox::indicator:checked:hover { background: ${content-link}; border-color: ${content-link}; }
QCheckBox::indicator:disabled { border-color: ${border}; }
QCheckBox::indicator:checked:disabled { background: ${content-disabled}; border-color: ${content-disabled}; }

QSlider::groove:horizontal { height: 4px; background: ${border}; border-radius: 2px; }
QSlider::handle:horizontal { width: 10px; margin: -4px 0; background: ${content-control}; border-radius: 3px; }
QSlider::handle:horizontal:hover { background: ${selection}; }
QSlider::groove:vertical { width: 4px; background: ${border}; border-radius: 2px; }
QSlider::handle:vertical { height: 10px; margin: 0 -4px; background: ${content-control}; border-radius: 3px; }
QSlider::handle:vertical:hover { background: ${selection}; }
QSlider::handle:horizontal:disabled, QSlider::handle:vertical:disabled { background: ${content-disabled}; }

QGroupBox { color: ${content-body}; border: 1px solid ${border-subtle}; border-radius: 3px; margin-top: 8px; padding-top: 6px; }
QGroupBox::title { subcontrol-origin: margin; subcontrol-position: top left; left: 8px; padding: 0 3px; color: ${content}; font-weight: 700; }
QGroupBox:disabled, QGroupBox::title:disabled { color: ${content-disabled}; }

QRadioButton { color: ${content-body}; spacing: 5px; }
QRadioButton:disabled { color: ${content-disabled}; }
QRadioButton::indicator { width: 13px; height: 13px; border: 1px solid ${border-input}; border-radius: 7px; background: ${surface-input}; }
QRadioButton::indicator:hover { border-color: ${selection}; }
QRadioButton::indicator:checked { background: ${selection}; border-color: ${selection}; }
QRadioButton::indicator:checked:hover { background: ${content-link}; border-color: ${content-link}; }
QRadioButton::indicator:disabled { border-color: ${border}; }
QRadioButton::indicator:checked:disabled { background: ${content-disabled}; border-color: ${content-disabled}; }

QProgressBar { background: ${surface-input}; border: 1px solid ${border-input}; border-radius: 3px; text-align: center; color: ${content-body}; }
QProgressBar::chunk { background: ${selection}; border-radius: 2px; }
QProgressBar:disabled { color: ${content-disabled}; border-color: ${border}; }
QFrame#loadingCard { background: ${surface}; border: 1px solid ${border}; border-radius: 8px; }
QFrame#loadingCard QLabel#loadingCardName { color: ${content}; }
QFrame#loadingCard QLabel#loadingCardPhase, QFrame#loadingCard QLabel#loadingCardPercent { color: ${content-body}; }

QHeaderView { background: ${surface-header}; border: 0; }
QHeaderView::section { background: ${surface-header}; color: ${content-body}; border: 0; border-right: 1px solid ${border-subtle}; border-bottom: 1px solid ${border-subtle}; padding: 3px 6px; }
QHeaderView::section:hover { background: ${surface-hover}; }
QHeaderView::section:disabled { color: ${content-disabled}; }

/* The panel-column splitter keeps its object-scoped rule (below); this is the
   general handle so any other splitter is tokenised too. */
QSplitter::handle { background: ${border}; }
QSplitter::handle:hover { background: ${selection}; }

/* A colour swatch paints its data colour itself; the global rule only removes
   the button chrome and the focus ring comes from the paint. */
QPushButton#colorSwatch { background: transparent; border: 0; padding: 0; }

/* Static surfaces that used to carry per-widget stylesheets. */
QToolButton#optionsBrushTip:!hover:!pressed { background: transparent; border: 0; padding: 0; }
QToolButton#gradientMapPresets::menu-indicator { image: none; width: 0; }
QWidget#panelDropIndicator, QWidget#panelStripDropIndicator, QWidget#panelFloatDropIndicator,
QWidget#panelColumnEdgeIndicator { background: ${accent}; }
QWidget#panelOutlineIndicator { background: transparent; border: 2px solid ${accent}; border-radius: 6px; }
QLabel#imageSizePreview { background: ${surface-input}; border: 1px solid ${border-input}; }
QLabel#brushTipPreview { background: ${surface-input}; }
QLabel#replaceColorMaskPreview { background: ${surface-canvas}; border: 1px solid ${border-input}; }
QListWidget#galleryEffects::indicator { width: 0; height: 0; }
QMenu#toolFlyoutMenu::item { padding: 3px 18px 3px 24px; }
QMenu#optionsCropSettingsMenu { padding: 3px 0; }
QMenu#optionsCropSettingsMenu::item { padding: 3px 18px 3px 24px; }
/* The grid menu's icons start at the Settings menu's checkbox inset, clear of the edge. */
QMenu#optionsCropGridMenu { padding: 4px 0; }
/* Qt places menu icons by ::icon's offset; the item's left padding moves only the text. */
QMenu#optionsCropGridMenu::item { padding: 4px 18px 4px 14px; }
QMenu#optionsCropGridMenu::icon { left: 10px; }
QWidget#optionsCropShieldPanel { background: transparent; }
QLabel#dialogTitle, QLabel#sectionTitle { font-weight: 700; }
QLabel#sectionTabLabel { font-weight: 700; padding: 3px 0; }
)");

    for (const ThemeToken& token : tokens) {
        qss.replace(QStringLiteral("${") + token.key + QLatin1Char('}'), token.value);
    }
    // Tab labels run 2 px under the app default and input/select text 1 px
    // under it; a point-sized default font (pixelSize() < 0) falls back to
    // 12 px (~9 pt) at 96 DPI.
    int baseFontPx = 12;
    if (QApplication::instance() != nullptr && QApplication::font().pixelSize() > 0) {
        baseFontPx = QApplication::font().pixelSize();
    }
    qss.replace(QStringLiteral("${font-size-tab}"), QString::number(baseFontPx - 2));
    qss.replace(QStringLiteral("${font-size-input}"), QString::number(baseFontPx - 1));
    return qss;
}

} // namespace

QColor Theme::shade(QColor color, int steps)
{
    for (int i = 0; i < steps; ++i) {
        color = color.lighter(kShadeStep);
    }
    for (int i = 0; i < -steps; ++i) {
        color = color.darker(kShadeStep);
    }
    // `lighter`/`darker` return an HSV-spec colour, which compares unequal to
    // an equal-valued RGB-spec QColor; normalise so callers and tests can
    // compare shades directly.
    return color.toRgb();
}

int Theme::clampLevel(int level)
{
    if (level < 0) {
        return 0;
    }
    if (level >= kLevelCount) {
        return kLevelCount - 1;
    }
    return level;
}

QColor Theme::workspaceColor(int level)
{
    return rampFor(Theme::kDefaultFamily, clampLevel(level)).surfaceCanvas;
}

QColor Theme::workspaceColor()
{
    return rampFor(g_family, clampLevel(g_level)).surfaceCanvas;
}

QColor Theme::panelColor()
{
    return rampFor(g_family, clampLevel(g_level)).surface;
}

QColor Theme::tokenColor(const QString& key)
{
    for (const ThemeToken& token : tokens(g_family, g_level)) {
        if (token.color && token.key == key) {
            return QColor(token.value);
        }
    }
    return {};
}

QString Theme::styleSheet(Theme::ThemeFamily family, int level)
{
    QString qss = styleSheetFor(tokens(family, level));
    // The select chevron is a baked asset (QSS cannot tint an SVG); pick the
    // family's foreground-coloured file.
    qss.replace(QStringLiteral("${select-chevron}"),
                family == Theme::ThemeFamily::Light
                    ? QStringLiteral(":/ui/select-chevron-light.svg")
                    : QStringLiteral(":/ui/select-chevron.svg"));
    return qss;
}

QString Theme::styleSheet(int level)
{
    return styleSheet(Theme::kDefaultFamily, level);
}

QVector<ThemeToken> Theme::tokens(Theme::ThemeFamily family, int level)
{
    const Ramp ramp = rampFor(family, clampLevel(level));
    // Derived shades go toward the family's lighter/darker direction so light
    // surfaces (surface-field #ffffff, surface-header #d0d0d0) do not wash toward white.
    const int dir = family == Theme::ThemeFamily::Light ? -1 : 1;
    const auto color = [](const char* key, const QColor& value) {
        return ThemeToken{QString::fromLatin1(key), value.name(QColor::HexRgb), true};
    };
    const auto metric = [](const char* key, int value) {
        return ThemeToken{QString::fromLatin1(key), QString::number(value), false};
    };
    return {
        color("surface-recessed", ramp.surfaceRecessed),
        color("content", ramp.content),
        color("content-body", ramp.contentBody),
        color("surface-control", ramp.surfaceControl),
        color("content-control", ramp.contentControl),
        color("selection", ramp.selection),
        color("content-on-selection", ramp.contentOnSelection),
        color("surface-tooltip", ramp.surfaceTooltip),
        color("content-tooltip", ramp.contentTooltip),
        color("content-link", ramp.contentLink),
        color("content-disabled", ramp.contentDisabled),
        color("accent", ramp.accent),
        color("surface-notice", ramp.surfaceNotice),
        color("content-notice", ramp.contentNotice),
        color("content-disabled-menu", shade(ramp.contentDisabled, dir)),
        color("border", ramp.border),
        color("border-subtle", ramp.borderSubtle),
        color("border-subtle-alt", shade(ramp.borderSubtle, dir)),
        color("surface-field", ramp.surfaceField),
        color("surface-field-alternate", shade(ramp.surfaceField, dir)),
        color("surface-input", ramp.surfaceInput),
        color("border-input", ramp.borderInput),
        color("border-control", ramp.borderControl),
        color("surface-control-pressed", ramp.surfaceControlPressed),
        color("surface", ramp.surface),
        color("surface-tab-active", ramp.surfaceTabActive),
        color("surface-canvas", ramp.surfaceCanvas),
        color("surface-header", ramp.surfaceHeader),
        color("surface-hover", ramp.surfaceHover),
        color("surface-pressed", ramp.surfaceControlActive),
        color("surface-control-hover", ramp.surfaceControlHover),
        color("border-control-hover", ramp.borderControlHover),
        color("surface-control-active", ramp.surfaceControlActive),
        color("border-header", shade(ramp.surfaceHeader, -1)),
        color("surface-scrollbar", ramp.surfaceHeader),
        color("scrollbar-thumb", ramp.borderControl),
        color("scrollbar-thumb-hover", shade(ramp.borderControl, dir)),
        color("graph-fill", ramp.graphFill),
        color("graph-axis", ramp.graphAxis),
        metric("border-width-panel", kPanelBorderWidth),
        metric("border-width-chrome", kChromeBorderWidth),
        metric("divider-width-group", kGroupDividerWidth),
        metric("slot-base-width", kSlotBaseW),
        metric("slot-base-height", kSlotBaseH),
        metric("slot-icon-max-width", kSlotIconMaxW),
        metric("slot-icon-max-height", kSlotIconMaxH),
        metric("shade-step", kShadeStep),
    };
}

QVector<ThemeToken> Theme::tokens(int level)
{
    return tokens(Theme::kDefaultFamily, level);
}

QString Theme::familyName(Theme::ThemeFamily family)
{
    return family == Theme::ThemeFamily::Light ? QStringLiteral("light")
                                               : QStringLiteral("dark");
}

Theme::ThemeFamily Theme::familyFromName(const QString& name)
{
    return name.compare(QStringLiteral("light"), Qt::CaseInsensitive) == 0
        ? Theme::ThemeFamily::Light
        : Theme::ThemeFamily::Dark;
}

quint64 Theme::paletteGeneration()
{
    return g_paletteGeneration;
}

int Theme::currentLevel()
{
    return g_level;
}

Theme::ThemeFamily Theme::currentFamily()
{
    return g_family;
}

void Theme::apply(Theme::ThemeFamily family, int level)
{
    const int clamped = clampLevel(level);
    g_family = family;
    g_level = clamped;
    ++g_paletteGeneration;
    QPixmapCache::clear();
    QApplication::setStyle(new SpinArrowStyle(QStyleFactory::create(QStringLiteral("Fusion"))));
    qApp->setPalette(paletteFor(family, rampFor(family, clamped)));
    qApp->setStyleSheet(styleSheet(family, clamped));
}

void Theme::apply(int level)
{
    apply(Theme::kDefaultFamily, level);
}

} // namespace pictura
