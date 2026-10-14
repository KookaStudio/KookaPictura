#include "theme.h"

#include <QtCore/QString>

#include <QtGui/QColor>
#include <QtGui/QFont>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>
#include <QtGui/QPalette>
#include <QtGui/QPixmapCache>
#include <QtWidgets/QAbstractSpinBox>
#include <QtWidgets/QApplication>
#include <QtWidgets/QProxyStyle>
#include <QtWidgets/QStyleOption>
#include <QtWidgets/QStyleFactory>

namespace pictura {

namespace {

// Fusion under the app stylesheet draws a spin box's up/down buttons as dark
// outlines with no visible arrows. Paint the buttons a lighter cell and the
// arrows as small triangles in the button-text colour, so they read on the
// dark ramp.
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
        if (!spin || (!up && !down)) {
            QProxyStyle::drawPrimitive(element, option, painter, widget);
            return;
        }
        // The arrow rect is a few pixels; size the glyph from the constant.
        constexpr qreal half = 3.5;
        const QPointF c = QRectF(option->rect).center();
        const qreal tip = up ? -2.0 : 2.0;
        QPainterPath path;
        path.moveTo(c.x() - half, c.y() - tip);
        path.lineTo(c.x() + half, c.y() - tip);
        path.lineTo(c.x(), c.y() + tip);
        path.closeSubpath();
        const bool enabled = option->state & State_Enabled;
        painter->save();
        painter->setRenderHint(QPainter::Antialiasing);
        painter->setPen(Qt::NoPen);
        painter->setBrush(option->palette.color(enabled ? QPalette::Active : QPalette::Disabled,
                                                QPalette::ButtonText));
        painter->drawPath(path);
        painter->restore();
    }
};

quint64 g_paletteGeneration = 0;
int g_level = Theme::kDefaultLevel;

struct Ramp {
    // Text and accent colours are constant across brightness levels.
    QColor windowText;
    QColor text;
    QColor buttonText;
    QColor highlight;
    QColor highlightedText;
    QColor toolTipBase;
    QColor toolTipText;
    QColor link;
    QColor disabledText;
    // The drop/insertion accent (CS6's #2a7fff), constant across levels.
    QColor accent;
    // Inline warning and info text (a `notice` widget): dark text on yellow,
    // constant across levels so it stands out on every theme.
    QColor noticeBg;
    QColor noticeText;
    // Surfaces. Level 1 holds the explicit CS6 dark swatches; the other levels
    // derive from them by shifting every surface with `shade(level - 1)`.
    QColor window;
    QColor panel;
    QColor activeTab;
    QColor workspace;
    QColor panelHeader;
    QColor separator;
    QColor tableBg;
    QColor border;
    QColor inputBg;
    QColor inputBorder;
    QColor button;
    QColor buttonBorder;
    QColor buttonPressed;
    QColor iconPressed;
    QColor iconHoverBorder;
    QColor hover;
    QColor iconHover;
};

// The default-brightness (level 1) palette. Other levels are derived from it.
const Ramp kLevel1Ramp = {
    QColor(224, 224, 224),    // windowText
    QColor(224, 224, 224),    // text
    QColor(224, 224, 224),    // buttonText
    QColor(61, 111, 153),     // highlight
    QColor(255, 255, 255),    // highlightedText
    QColor(58, 58, 58),       // toolTipBase
    QColor(232, 232, 232),    // toolTipText
    QColor(111, 168, 220),    // link
    QColor(104, 104, 104),    // disabledText
    QColor(0x2a, 0x7f, 0xff), // accent
    QColor(0xf2, 0xd0, 0x4b), // noticeBg
    QColor(0x1e, 0x1e, 0x1e), // noticeText
    QColor(0x36, 0x36, 0x36), // window
    QColor(0x4d, 0x4d, 0x4d), // panel
    QColor(0x4d, 0x4d, 0x4d), // activeTab
    QColor(0x1f, 0x1f, 0x1f), // workspace
    QColor(0x36, 0x36, 0x36), // panelHeader
    QColor(0x40, 0x40, 0x40), // separator
    QColor(0x40, 0x40, 0x40), // tableBg
    QColor(0x2e, 0x2e, 0x2e), // border
    QColor(0x3b, 0x3b, 0x3b), // inputBg
    QColor(0x59, 0x59, 0x59), // inputBorder
    QColor(0x3b, 0x3b, 0x3b), // button
    QColor(0x59, 0x59, 0x59), // buttonBorder
    QColor(0x30, 0x30, 0x30), // buttonPressed
    QColor(0x2e, 0x2e, 0x2e), // iconPressed
    QColor(0x59, 0x59, 0x59), // iconHoverBorder
    QColor(),                 // hover (filled in rampFor)
    QColor(),                 // iconHover (filled in rampFor)
};

Ramp rampFor(int level)
{
    Ramp ramp = kLevel1Ramp;
    const int steps = level - Theme::kDefaultLevel;
    QColor* surfaces[] = {
        &ramp.window,        &ramp.panel,        &ramp.activeTab,    &ramp.workspace,
        &ramp.panelHeader,   &ramp.separator,    &ramp.tableBg,      &ramp.border,
        &ramp.inputBg,       &ramp.inputBorder,  &ramp.button,       &ramp.buttonBorder,
        &ramp.buttonPressed, &ramp.iconPressed,  &ramp.iconHoverBorder,
    };
    for (QColor* surface : surfaces) {
        *surface = Theme::shade(*surface, steps);
    }
    ramp.hover = Theme::shade(ramp.panel, 1);
    ramp.iconHover = ramp.hover;
    return ramp;
}

QPalette paletteFor(const Ramp& ramp)
{
    QPalette pal;
    pal.setColor(QPalette::Window, ramp.window);
    pal.setColor(QPalette::WindowText, ramp.windowText);
    pal.setColor(QPalette::Base, ramp.tableBg);
    pal.setColor(QPalette::AlternateBase, Theme::shade(ramp.tableBg, 1));
    pal.setColor(QPalette::Text, ramp.text);
    pal.setColor(QPalette::Button, ramp.button);
    pal.setColor(QPalette::ButtonText, ramp.buttonText);
    pal.setColor(QPalette::Mid, ramp.buttonBorder);
    pal.setColor(QPalette::Dark, ramp.border);
    pal.setColor(QPalette::Shadow, ramp.border);
    pal.setColor(QPalette::Highlight, ramp.highlight);
    pal.setColor(QPalette::HighlightedText, ramp.highlightedText);
    pal.setColor(QPalette::ToolTipBase, ramp.toolTipBase);
    pal.setColor(QPalette::ToolTipText, ramp.toolTipText);
    pal.setColor(QPalette::Link, ramp.link);
    pal.setColor(QPalette::Disabled, QPalette::Text, ramp.disabledText);
    pal.setColor(QPalette::Disabled, QPalette::WindowText, ramp.disabledText);
    pal.setColor(QPalette::Disabled, QPalette::ButtonText, ramp.disabledText);
    return pal;
}

// The stylesheet only re-colours surfaces; the CS6 dark swatches are explicit
// in `kLevel1Ramp` and shifted per brightness level by `rampFor`.
QString styleSheetFor(const QVector<ThemeToken>& tokens)
{
    QString qss = QStringLiteral(R"(
QMainWindow { background: ${panel}; }
QMainWindow::separator { background: ${border}; width: 3px; height: 3px; }

QMenuBar { background: ${panel}; color: ${windowText}; }
QMenuBar::item { background: transparent; color: ${windowText}; padding: 4px 5px; }
QMenuBar::item:selected { background: ${highlight}; color: ${highlightedText}; }
QMenuBar::item:pressed { background: ${pressed}; color: ${buttonText}; }

QMenu { background: ${panel}; color: ${text}; border: 1px solid ${border}; }
QMenu::item { background: transparent; padding: 4px 22px; }
QMenu::item:selected { background: ${highlight}; color: ${highlightedText}; }
QMenu::item:disabled { color: ${menuDisabledText}; }
QMenu::separator { background: ${separator}; height: 1px; margin: 4px 6px; }

QToolBar { background: ${panel}; color: ${windowText}; border: 0; spacing: 2px; padding: 2px; }
QToolBar::separator { background: ${border}; width: ${chromeBorderWidth}px; margin: 3px 2px; }
QToolBar#optionsBar { border-bottom: ${panelBorderWidth}px solid ${border}; border-top: 1px solid ${separator}; padding-right: 8px; }
QFrame#optionsToolSeparator { background: ${separatorLight}; border: 0; }

QToolButton { background: transparent; color: ${buttonText}; border: 1px solid transparent; border-radius: 3px; padding: 3px; }
QToolButton:hover { background: ${iconHover}; border-color: ${iconHoverBorder}; }
QToolButton:pressed { background: ${iconPressed}; border-color: ${iconHoverBorder}; }
QToolButton:checked { background: ${iconPressed}; border-color: ${iconHoverBorder}; color: ${buttonText}; }
QToolButton:disabled { color: ${disabledText}; }
QToolButton:focus { border-color: ${highlight}; }
/* Tool slots sit flush on the toolbar at rest; only hover/press/active differ. */
QToolButton#toolSlotButton { background: transparent; border: 1px solid transparent; border-radius: 3px; }
QToolButton#toolSlotButton:hover { background: ${iconHover}; border-color: ${iconHoverBorder}; }
QToolButton#toolSlotButton:pressed, QToolButton#toolSlotButton:checked { background: ${iconPressed}; border-color: ${iconHoverBorder}; }
QToolButton#statusOptionsButton { background: transparent; border: 0; }
QToolButton#statusOptionsButton::menu-indicator { image: none; width: 0; }

QDockWidget { color: ${windowText}; }
QDockWidget::title { background: ${window}; color: ${windowText}; padding: 3px 6px; border-bottom: 1px solid ${border}; }
QDockWidget::close-button, QDockWidget::float-button { background: transparent; border: 0; }

QTabBar::tab { background: ${window}; color: ${windowText}; border: 1px solid ${border}; border-bottom: 0; padding: 4px 10px; margin-right: 1px; }
QTabBar::tab:hover { background: ${hover}; }
QTabBar::tab:selected { background: ${activeTab}; color: ${windowText}; }
QTabBar::tab:disabled { color: ${disabledText}; }

/* M44: the active panel tab takes the panel surface (`${panel}`) so it reads as
   continuous with the body; inactive tabs recede to the darker `${panelHeader}`.
   The panel pane is `${panel}` too, so active == pane and inactive != pane. The
   header strip behind the tabs is the inactive-tab shade, so it is no lighter
   than the tabs. The document tab bar keeps the unscoped `QTabBar::tab` rules
   above. */
QTabBar#panelTabBar { background: ${panelHeader}; }
QTabBar#panelTabBar::tab { background: ${panelHeader}; color: ${windowText}; border: ${panelBorderWidth}px solid ${border}; border-bottom: 0; padding: 4px 8px; margin-right: 1px; font-size: ${tabFontSize}px; font-weight: 700; }
QTabBar#panelTabBar::tab:hover { background: ${hover}; }
QTabBar#panelTabBar::tab:selected { background: ${panel}; color: ${windowText}; }

QSplitter#panelColumnSplitter::handle { background: ${border}; }
QFrame#panelIconDivider { background: transparent; border: 0; }
QWidget#panelIconGroup { background: ${panel}; border: 0; border-bottom: 1px solid ${separator}; }
QWidget#panelIconGroupGrip { background: transparent; color: ${disabledText}; }
/* M47: a collapsed group's iconic row reuses the docked strip's
   `panelIconGroup` box, so the float and the strip read alike. The header band,
   the reserved corner grip, corner container, and its `▾` button share the
   header strip; the band spans the whole group width behind the tabs and corner
   so no vertical slice of the header is left unpainted. */
QWidget#panelHeaderBand { background: ${panelHeader}; }
QWidget#panelGroupDragGrip { background: ${panelHeader}; }
QWidget#panelWidgetCorner { background: ${panelHeader}; }
QWidget#panelWidgetCorner QToolButton { background: ${panelHeader}; color: ${buttonText}; border: 0; border-radius: 0; padding: 0; }
QWidget#panelWidgetCorner QToolButton::menu-indicator { image: none; width: 0; }
QWidget#panelWidgetCorner QToolButton:hover { background: ${hover}; }
QWidget#panelWidgetCorner QToolButton:pressed { background: ${pressed}; }

QTabWidget::pane { border: ${panelBorderWidth}px solid ${border}; background: ${workspace}; }
QTabWidget#panelGroupTabs { border: ${panelBorderWidth}px solid ${border}; }
QTabWidget#panelGroupTabs::pane { border: 0; background: ${panel}; }
QTabWidget#panelGroupTabs QStackedWidget { background: ${panel}; }
QTabWidget#documentTabs::pane { border: 0; }
QTabWidget#documentTabs QStackedWidget { background: ${workspace}; }
QTabBar#documentTabBar { border: 0; }
QTabBar#documentTabBar { background: ${panelHeader}; }
QTabBar#documentTabBar::tab { font-size: ${tabFontSize}px; font-weight: 700; padding-right: 4px; border: 0; }
QTabBar#documentTabBar::close-button { image: url(:/icons/panel.close.light.png); background: transparent; border: 0; margin-left: 2px; margin-right: 8px; }
QTabBar#documentTabBar::close-button:hover { background: ${hover}; }
QTabBar#documentTabBar::close-button:pressed { background: ${hover}; }
/* Columns keep their side rules only; the top and bottom edges are open, and
   the header carries no rule of its own. */
QWidget#panelColumnContainer { border: 0; border-left: ${panelBorderWidth}px solid ${separator}; border-right: ${panelBorderWidth}px solid ${separator}; border-bottom: ${panelBorderWidth}px solid ${separator}; background: ${panel}; }
QWidget#toolsColumn { border: 0; border-left: ${panelBorderWidth}px solid ${separator}; border-right: ${panelBorderWidth}px solid ${separator}; border-bottom: ${panelBorderWidth}px solid ${separator}; background: ${panel}; }
/* The central band's 1 px inner frame line; the outer 2 px is the window
   `${panel}`. The outermost column's frame-side border is suppressed so the
   band edge reads as exactly 2 px panel + 1 px border. */
QWidget#workspaceFrameLine { background: ${border}; }
QWidget#panelColumnContainer[frameEdge="left"], QWidget#toolsColumn[frameEdge="left"] { border-left: 0; }
QWidget#panelColumnContainer[frameEdge="right"], QWidget#toolsColumn[frameEdge="right"] { border-right: 0; }
QWidget#panelColumnHeader { background: ${panelHeader}; border: 0; border-bottom: 1px solid ${panelHeaderBorder}; }
/* The icon strip sits inside the bordered column container, so it must not
   draw its own side rule (that would double the column border). */
QWidget#panelColumnIconStrip { border: 0; background: ${panel}; }
QWidget#toolsPanel { background: ${panel}; border: 0; }
QWidget#panelIconFlyout { background: ${panel}; border: ${panelBorderWidth}px solid ${border}; }
QWidget#panelFloat { background: ${panel}; border: ${panelBorderWidth}px solid ${border}; }
/* Frameless dialog chrome (dialog_chrome.cpp): a header-shade title strip and
   a 1 px outline drawn over the dialog's edge. */
QWidget#dialogTitleBar { background: ${panelHeader}; border: 0; border-bottom: 1px solid ${panelHeaderBorder}; }
QLabel#dialogTitleLabel { background: transparent; color: ${windowText}; font-weight: 700; }
QToolButton#dialogCloseButton { background: transparent; border: 0; padding: 2px; }
QToolButton#dialogCloseButton:hover { background: ${hover}; }
QToolButton#dialogCloseButton:pressed { background: ${pressed}; }
QWidget#dialogFrame { background: transparent; border: 1px solid ${border}; }
/* Inline warning / info text: a label flagged `notice="true"` reads
   dark-on-yellow; its icon and any buttons beside it keep their own look. */
QLabel[notice="true"] { background: ${noticeBg}; color: ${noticeText}; border: 0; border-radius: 2px; padding: 3px 6px; }

/* Info panel readout grid: only the inner cross of the 2x2 table is drawn, in
   the frame shade; the outer edge is the panel pane itself. The blocks flag
   their inner sides with the `gridRight`/`gridBottom` dynamic properties. */
QWidget#infoBlock[gridRight="true"] { border-right: 1px solid ${separator}; }
QWidget#infoBlock[gridBottom="true"] { border-bottom: 1px solid ${separator}; }
QWidget#infoRule { background: ${separator}; }
/* The readout icons are menu affordances, not buttons: bare icon at rest. */
QWidget#infoBlock QToolButton { background: transparent; border: 0; padding: 2px; }
QWidget#infoBlock QToolButton:hover { background: ${hover}; }
QWidget#infoBlock QToolButton:pressed { background: ${pressed}; }


QStatusBar { background: ${panel}; color: ${windowText}; }
QStatusBar::item { border: 0; }
QFrame#statusSeparator { background: ${separator}; border: 0; }

QScrollBar:vertical { background: ${scrollbar}; width: 12px; border: 0; margin: 0; }
QScrollBar::handle:vertical { background: ${scrollbarThumb}; min-height: 24px; border-radius: 3px; margin: 2px; }
QScrollBar::handle:vertical:hover { background: ${scrollbarThumbHover}; }
QScrollBar:horizontal { background: ${scrollbar}; height: 12px; border: 0; margin: 0; }
QScrollBar::handle:horizontal { background: ${scrollbarThumb}; min-width: 24px; border-radius: 3px; margin: 2px; }
QScrollBar::handle:horizontal:hover { background: ${scrollbarThumbHover}; }
QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical,
QScrollBar::add-line:horizontal, QScrollBar::sub-line:horizontal { background: transparent; border: 0; width: 0; height: 0; }
QScrollBar::add-page, QScrollBar::sub-page { background: transparent; }
/* The empty cell where the canvas's two scrollbars meet takes the scrollbar
   track colour instead of the workspace showing through. */
QWidget#canvasScrollCorner { background: ${scrollbar}; }
QWidget#canvasRulerCorner { background: ${panel}; }

QToolTip { background: ${toolTipBase}; color: ${toolTipText}; border: 1px solid ${border}; padding: 2px; }

QListView, QTreeView, QTableView { background: ${tableBg}; color: ${text}; border: 1px solid ${separator}; alternate-background-color: ${alternateTableBg}; selection-background-color: ${highlight}; selection-color: ${highlightedText}; }
QLineEdit, QSpinBox, QDoubleSpinBox, QComboBox, QPlainTextEdit { background: ${inputBg}; color: ${text}; border: 1px solid ${inputBorder}; border-radius: 2px; padding: 1px 2px; }
QLineEdit:hover, QSpinBox:hover, QDoubleSpinBox:hover, QComboBox:hover, QPlainTextEdit:hover { border-color: ${inputBorder}; }
QLineEdit:focus, QSpinBox:focus, QDoubleSpinBox:focus, QComboBox:focus, QPlainTextEdit:focus { border-color: ${highlight}; }
QLineEdit:disabled, QSpinBox:disabled, QDoubleSpinBox:disabled, QComboBox:disabled, QPlainTextEdit:disabled { color: ${disabledText}; border-color: ${border}; }
QComboBox::drop-down { background: transparent; border: 0; width: 16px; }
QComboBox::down-arrow { image: url(:/ui/combo-caret.svg); width: 8px; height: 5px; }
QComboBox::down-arrow:disabled { image: url(:/ui/combo-caret-disabled.svg); }
QComboBox QAbstractItemView { background: ${panel}; color: ${text}; border: 1px solid ${border}; selection-background-color: ${highlight}; selection-color: ${highlightedText}; }

QWidget#numericField, QWidget#percentField QLineEdit { background: ${inputBg}; color: ${text}; border: 1px solid ${inputBorder}; border-radius: 2px; padding: 1px 2px; }
QWidget#numericField, QWidget#percentField QLineEdit:disabled { color: ${disabledText}; }
QWidget#numericField, QWidget#percentField QToolButton { background: transparent; color: ${buttonText}; border: 0; border-left: 1px solid ${border}; border-radius: 0; padding: 0 2px; }
QWidget#numericField, QWidget#percentField QToolButton:hover { background: ${hover}; }
QWidget#numericField, QWidget#percentField QToolButton:disabled { color: ${disabledText}; }
QTabWidget::tab-bar { alignment: left; }

QWidget#numericField, QWidget#percentField QSlider::groove:horizontal { height: 4px; background: ${border}; border-radius: 2px; }
QWidget#numericField, QWidget#percentField QSlider::handle:horizontal { width: 10px; margin: -4px 0; background: ${buttonText}; border-radius: 3px; }

QWidget#layersFilterBar { background: ${panel}; border-bottom: 1px solid ${separator}; }
QWidget#layersFilterBar QComboBox, QWidget#layersFilterBar QLineEdit { background: ${inputBg}; color: ${text}; border: 1px solid ${inputBorder}; border-radius: 2px; padding: 1px 2px; }
QWidget#layersFilterBar QComboBox:disabled, QWidget#layersFilterBar QLineEdit:disabled { color: ${disabledText}; }
QWidget#layersFilterBar QToolButton { background: transparent; color: ${buttonText}; border: 1px solid transparent; border-radius: 3px; padding: 2px 4px; }
QWidget#layersFilterBar QToolButton:hover { background: ${iconHover}; border-color: ${iconHoverBorder}; }
QWidget#layersFilterBar QToolButton:checked { background: ${iconPressed}; border-color: ${iconHoverBorder}; }
QWidget#layersFilterBar QToolButton#layersFilterToggle:checked { background: ${highlight}; color: ${highlightedText}; }

QPushButton { background: ${button}; color: ${buttonText}; border: 1px solid ${buttonBorder}; border-radius: 3px; padding: 4px 10px; }
QPushButton:hover { background: ${button}; border-color: ${buttonBorder}; }
QPushButton:pressed { background: ${buttonPressed}; }
QPushButton:checked { background: ${buttonPressed}; border-color: ${inputBorder}; }
QPushButton:focus { border-color: ${highlight}; }
QPushButton:disabled { color: ${disabledText}; border-color: ${border}; }

/* Design-system primitives: one global definition per standard control, every
   value from a token. Object names scope the controls that need their own
   surface; the rest are type rules. */
QCheckBox { color: ${text}; spacing: 5px; }
QCheckBox:disabled { color: ${disabledText}; }
QCheckBox::indicator { width: 13px; height: 13px; border: 1px solid ${inputBorder}; border-radius: 2px; background: ${inputBg}; }
QCheckBox::indicator:hover { border-color: ${highlight}; }
QCheckBox::indicator:checked { background: ${highlight}; border-color: ${highlight}; }
QCheckBox::indicator:checked:hover { background: ${link}; border-color: ${link}; }
QCheckBox::indicator:disabled { border-color: ${border}; }
QCheckBox::indicator:checked:disabled { background: ${disabledText}; border-color: ${disabledText}; }

QSlider::groove:horizontal { height: 4px; background: ${border}; border-radius: 2px; }
QSlider::handle:horizontal { width: 10px; margin: -4px 0; background: ${buttonText}; border-radius: 3px; }
QSlider::handle:horizontal:hover { background: ${highlight}; }
QSlider::groove:vertical { width: 4px; background: ${border}; border-radius: 2px; }
QSlider::handle:vertical { height: 10px; margin: 0 -4px; background: ${buttonText}; border-radius: 3px; }
QSlider::handle:vertical:hover { background: ${highlight}; }
QSlider::handle:horizontal:disabled, QSlider::handle:vertical:disabled { background: ${disabledText}; }

/* A colour swatch paints its data colour itself; the global rule only removes
   the button chrome and the focus ring comes from the paint. */
QPushButton#colorSwatch { background: transparent; border: 0; padding: 0; }

/* Static surfaces that used to carry per-widget stylesheets. */
QToolButton#optionsBrushTip:!hover:!pressed { background: transparent; border: 0; padding: 0; }
QToolButton#gradientMapPresets::menu-indicator { image: none; width: 0; }
QWidget#panelDropIndicator, QWidget#panelStripDropIndicator, QWidget#panelFloatDropIndicator,
QWidget#panelColumnEdgeIndicator { background: ${accent}; }
QWidget#panelOutlineIndicator { background: transparent; border: 2px solid ${accent}; border-radius: 6px; }
QLabel#imageSizePreview { background: ${inputBg}; border: 1px solid ${inputBorder}; }
QLabel#brushTipPreview { background: ${inputBg}; }
QLabel#replaceColorMaskPreview { background: ${workspace}; border: 1px solid ${inputBorder}; }
QListWidget#galleryEffects::indicator { width: 0; height: 0; }
QMenu#toolFlyoutMenu::item { padding: 3px 18px 3px 24px; }
QLabel#dialogTitle, QLabel#sectionTitle { font-weight: 700; }
QLabel#sectionTabLabel { font-weight: 700; padding: 3px 0; }
)");

    for (const ThemeToken& token : tokens) {
        qss.replace(QStringLiteral("${") + token.key + QLatin1Char('}'), token.value);
    }
    // Tab labels run 2 px under the app default; a point-sized default font
    // (pixelSize() < 0) falls back to 12 px (~9 pt) at 96 DPI.
    int baseFontPx = 12;
    if (QApplication::instance() != nullptr && QApplication::font().pixelSize() > 0) {
        baseFontPx = QApplication::font().pixelSize();
    }
    qss.replace(QStringLiteral("${tabFontSize}"), QString::number(baseFontPx - 2));
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
    return rampFor(clampLevel(level)).workspace;
}

QColor Theme::workspaceColor()
{
    return workspaceColor(g_level);
}

QColor Theme::panelColor()
{
    return rampFor(clampLevel(g_level)).panel;
}

QString Theme::styleSheet(int level)
{
    return styleSheetFor(tokens(level));
}

QVector<ThemeToken> Theme::tokens(int level)
{
    const Ramp ramp = rampFor(clampLevel(level));
    const auto color = [](const char* key, const QColor& value) {
        return ThemeToken{QString::fromLatin1(key), value.name(QColor::HexRgb), true};
    };
    const auto metric = [](const char* key, int value) {
        return ThemeToken{QString::fromLatin1(key), QString::number(value), false};
    };
    return {
        color("window", ramp.window),
        color("windowText", ramp.windowText),
        color("text", ramp.text),
        color("button", ramp.button),
        color("buttonText", ramp.buttonText),
        color("highlight", ramp.highlight),
        color("highlightedText", ramp.highlightedText),
        color("toolTipBase", ramp.toolTipBase),
        color("toolTipText", ramp.toolTipText),
        color("link", ramp.link),
        color("disabledText", ramp.disabledText),
        color("accent", ramp.accent),
        color("noticeBg", ramp.noticeBg),
        color("noticeText", ramp.noticeText),
        color("menuDisabledText", shade(ramp.disabledText, 1)),
        color("border", ramp.border),
        color("separator", ramp.separator),
        color("separatorLight", shade(ramp.separator, 1)),
        color("tableBg", ramp.tableBg),
        color("alternateTableBg", shade(ramp.tableBg, 1)),
        color("inputBg", ramp.inputBg),
        color("inputBorder", ramp.inputBorder),
        color("buttonBorder", ramp.buttonBorder),
        color("buttonPressed", ramp.buttonPressed),
        color("panel", ramp.panel),
        color("activeTab", ramp.activeTab),
        color("workspace", ramp.workspace),
        color("panelHeader", ramp.panelHeader),
        color("hover", ramp.hover),
        color("pressed", ramp.iconPressed),
        color("iconHover", ramp.iconHover),
        color("iconHoverBorder", ramp.iconHoverBorder),
        color("iconPressed", ramp.iconPressed),
        color("panelHeaderBorder", shade(ramp.panelHeader, -1)),
        color("scrollbar", ramp.panelHeader),
        color("scrollbarThumb", ramp.buttonBorder),
        color("scrollbarThumbHover", shade(ramp.buttonBorder, 1)),
        metric("panelBorderWidth", kPanelBorderWidth),
        metric("chromeBorderWidth", kChromeBorderWidth),
        metric("groupDividerWidth", kGroupDividerWidth),
        metric("slotBaseW", kSlotBaseW),
        metric("slotBaseH", kSlotBaseH),
        metric("slotIconMaxW", kSlotIconMaxW),
        metric("slotIconMaxH", kSlotIconMaxH),
        metric("shadeStep", kShadeStep),
    };
}

quint64 Theme::paletteGeneration()
{
    return g_paletteGeneration;
}

void Theme::apply(int level)
{
    const int clamped = clampLevel(level);
    g_level = clamped;
    ++g_paletteGeneration;
    QPixmapCache::clear();
    QApplication::setStyle(new SpinArrowStyle(QStyleFactory::create(QStringLiteral("Fusion"))));
    qApp->setPalette(paletteFor(rampFor(clamped)));
    qApp->setStyleSheet(styleSheet(clamped));
}

} // namespace pictura
