#include "design_system_dialog.h"

#include "icons.h"
#include "panels/color_swatch_widget.h"
#include "panels/numeric_field.h"
#include "panels/panel_group.h"
#include "theme.h"

#include <QtGui/QFont>
#include <QtGui/QIcon>
#include <QtGui/QPalette>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFrame>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QHeaderView>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMenuBar>
#include <QtWidgets/QPlainTextEdit>
#include <QtWidgets/QProgressBar>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QSplitter>
#include <QtWidgets/QStatusBar>
#include <QtWidgets/QStyle>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QTabWidget>
#include <QtWidgets/QToolBar>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QTreeWidget>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QWidget>

#include <functional>

namespace pictura {
namespace {

// The Foundations pages paint the live theme, so read the current family (the
// dialog rebuilds them after a family or level change).
QColor tokenColor(int level, const QString& key)
{
    for (const ThemeToken& token : Theme::tokens(Theme::currentFamily(), level)) {
        if (token.key == key) {
            return QColor(token.value);
        }
    }
    return {};
}

QString tokenValue(int level, const QString& key)
{
    for (const ThemeToken& token : Theme::tokens(Theme::currentFamily(), level)) {
        if (token.key == key) {
            return token.value;
        }
    }
    return {};
}

QLabel* heading(const QString& text)
{
    auto* label = new QLabel(text);
    label->setObjectName(QStringLiteral("sectionTitle"));
    return label;
}

QLabel* note(const QString& text)
{
    auto* label = new QLabel(text);
    label->setWordWrap(true);
    return label;
}

// A captioned colour chip painted from a token (QPalette, so no per-widget
// stylesheet; the colour is data, not chrome).
QWidget* chip(int level, const QString& key)
{
    auto* cellWidget = new QWidget;
    auto* layout = new QVBoxLayout(cellWidget);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(2);
    auto* box = new QFrame;
    box->setObjectName(QStringLiteral("dsChip"));
    box->setFixedHeight(28);
    box->setFrameShape(QFrame::Box);
    box->setAutoFillBackground(true);
    QPalette palette = box->palette();
    palette.setColor(QPalette::Window, tokenColor(level, key));
    box->setPalette(palette);
    layout->addWidget(box);
    auto* caption = new QLabel(key + QLatin1Char(' ') + tokenValue(level, key));
    QFont font = caption->font();
    font.setPointSizeF(font.pointSizeF() * 0.8);
    caption->setFont(font);
    layout->addWidget(caption);
    return cellWidget;
}

QWidget* grid(const QList<QWidget*>& cells, int columns)
{
    auto* host = new QWidget;
    auto* layout = new QGridLayout(host);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(8);
    for (int i = 0; i < cells.size(); ++i) {
        layout->addWidget(cells.at(i), i / columns, i % columns);
    }
    return host;
}

QWidget* row(const QList<QWidget*>& widgets, int spacing = 8)
{
    auto* host = new QWidget;
    auto* layout = new QHBoxLayout(host);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(spacing);
    for (QWidget* child : widgets) {
        layout->addWidget(child);
    }
    layout->addStretch(1);
    return host;
}

QWidget* column(const QList<QWidget*>& widgets, int spacing = 8)
{
    auto* host = new QWidget;
    auto* layout = new QVBoxLayout(host);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(spacing);
    for (QWidget* child : widgets) {
        layout->addWidget(child);
    }
    return host;
}

QWidget* page(const QList<QWidget*>& sections)
{
    auto* host = new QWidget;
    auto* layout = new QVBoxLayout(host);
    layout->setContentsMargins(4, 4, 4, 4);
    layout->setSpacing(10);
    for (QWidget* section : sections) {
        layout->addWidget(section);
    }
    layout->addStretch(1);
    return host;
}

QWidget* cell(const QString& state, QWidget* widget, const QString& objectName = QString())
{
    auto* host = new QWidget;
    auto* layout = new QVBoxLayout(host);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(4);
    if (!objectName.isEmpty()) {
        widget->setObjectName(objectName);
    }
    layout->addWidget(widget, 0, Qt::AlignCenter);
    auto* caption = new QLabel(state);
    caption->setAlignment(Qt::AlignHCenter);
    QFont font = caption->font();
    font.setPointSizeF(font.pointSizeF() * 0.8);
    caption->setFont(font);
    layout->addWidget(caption);
    return host;
}

QWidget* stateCell(const QString& state, const std::function<QWidget*()>& make)
{
    return cell(state, make());
}

// Wrap the state row so the dialog's "Show states" toggle can hide it.
QWidget* states(const QList<QWidget*>& cells)
{
    QWidget* host = row(cells);
    host->setObjectName(QStringLiteral("dsStates"));
    return host;
}

QPushButton* pushButton(const QString& text, const std::function<void(QPushButton*)>& tweak = {})
{
    auto* button = new QPushButton(text);
    if (tweak) {
        tweak(button);
    }
    return button;
}

QToolButton* toolButton(const QString& iconId)
{
    auto* button = new QToolButton;
    button->setIcon(icon(iconId));
    button->setAutoRaise(true);
    return button;
}

QToolButton* toolSlot(const QString& iconId)
{
    auto* button = new QToolButton;
    button->setObjectName(QStringLiteral("toolSlotButton"));
    button->setIcon(icon(iconId));
    button->setFixedSize(Theme::kSlotBaseW, Theme::kSlotBaseH);
    button->setIconSize(QSize(Theme::kSlotIconMaxW, Theme::kSlotIconMaxH));
    return button;
}

QTreeWidget* tree(const QStringList& headers, const QList<QStringList>& rows)
{
    auto* view = new QTreeWidget;
    view->setHeaderLabels(headers);
    view->setRootIsDecorated(false);
    for (const QStringList& values : rows) {
        view->addTopLevelItem(new QTreeWidgetItem(values));
    }
    return view;
}

NumericField* numericField(const QString& label, double value, const QString& suffix)
{
    NumericFieldConfig config;
    config.objectName = QStringLiteral("numericField");
    config.decimals = suffix == QLatin1String("px") || suffix.isEmpty() ? 0 : 2;
    config.suffix = suffix;
    config.popup = true;
    auto* field = new NumericField(label, config);
    field->setValue(value);
    return field;
}

ColorSwatchWidget* swatch(const QColor& color, bool active = false)
{
    auto* widget = new ColorSwatchWidget;
    widget->setColor(color);
    widget->setActive(active);
    return widget;
}

// ---- Foundations ---------------------------------------------------------

QWidget* palettePage()
{
    const int level = Theme::currentLevel();
    QList<QWidget*> sections;
    const auto group = [&](const QString& title, const QStringList& keys) {
        QList<QWidget*> cells;
        for (const QString& key : keys) {
            cells.append(chip(level, key));
        }
        sections.append(column({heading(title), grid(cells, 4)}));
    };
    group(QStringLiteral("Surfaces"),
          {QStringLiteral("surface-recessed"), QStringLiteral("surface"), QStringLiteral("surface-tab-active"),
           QStringLiteral("surface-field"), QStringLiteral("surface-canvas")});
    group(QStringLiteral("Controls"),
          {QStringLiteral("surface-control"), QStringLiteral("border-control"), QStringLiteral("surface-input"),
           QStringLiteral("border-input"), QStringLiteral("border")});
    group(QStringLiteral("Chrome and lines"),
          {QStringLiteral("surface-header"), QStringLiteral("border-subtle"),
           QStringLiteral("border-subtle-alt"), QStringLiteral("border-header"),
           QStringLiteral("surface-scrollbar")});
    group(QStringLiteral("Selection and accent"),
          {QStringLiteral("selection"), QStringLiteral("content-on-selection"),
           QStringLiteral("accent"), QStringLiteral("content-link"), QStringLiteral("surface-hover")});
    group(QStringLiteral("Text and disabled"),
          {QStringLiteral("content-body"), QStringLiteral("content"),
           QStringLiteral("content-disabled"), QStringLiteral("content-disabled-menu"),
           QStringLiteral("surface-control-active")});
    return page(sections);
}

QWidget* brightnessPage()
{
    QList<QWidget*> columns;
    for (int level = 0; level < Theme::kLevelCount; ++level) {
        QList<QWidget*> cells;
        for (const char* key : {"surface-recessed", "surface", "surface-control", "surface-canvas"}) {
            cells.append(chip(level, QLatin1String(key)));
        }
        columns.append(column({heading(QStringLiteral("Level %1").arg(level)), column(cells, 4)}));
    }
    return page({heading(QStringLiteral("Four brightness levels, %1 family")
                            .arg(Theme::familyName(Theme::currentFamily()))),
                 note(QStringLiteral("Each level derives from the default by shifting every surface. "
                                     "The Brightness control at the right previews a level live; the "
                                     "user's level is restored when this dialog closes.")),
                 row(columns, 12)});
}

QWidget* metricsPage()
{
    const int level = Theme::currentLevel();
    QList<QWidget*> items;
    for (const char* key : {"border-width-panel", "border-width-chrome", "divider-width-group",
                            "slot-base-width", "slot-base-height", "slot-icon-max-width",
                            "slot-icon-max-height", "shade-step"}) {
        items.append(new QLabel(QLatin1String(key) + QLatin1String(" = ")
                                + tokenValue(level, QLatin1String(key))));
    }
    return page({heading(QStringLiteral("Shared chrome metrics")), column(items, 4),
                 heading(QStringLiteral("Tool slot")), toolSlot(QStringLiteral("tool.brush"))});
}

QWidget* typographyPage()
{
    auto* base = new QLabel(QStringLiteral("Window text - labels, fields, layers"));
    auto* bold = heading(QStringLiteral("Bold - sectionTitle, dialogTitle, group box titles"));
    auto* disabled = new QLabel(QStringLiteral("Disabled text - disabled controls"));
    disabled->setEnabled(false);
    auto* link = new QLabel(QStringLiteral("<a href=\"#\">Link</a>"));
    link->setTextFormat(Qt::RichText);
    auto* mono = new QLabel(QStringLiteral("0,0 - 1920x1080 - #3b3b3b"));
    QFont monoFont = mono->font();
    monoFont.setFamily(QStringLiteral("monospace"));
    mono->setFont(monoFont);

    auto* tabs = new QTabBar;
    tabs->setObjectName(QStringLiteral("panelTabBar"));
    tabs->addTab(QStringLiteral("Layers"));
    tabs->addTab(QStringLiteral("Channels"));
    tabs->setCurrentIndex(0);

    return page({heading(QStringLiteral("Type scale")), base, bold, disabled, link, mono,
                 heading(QStringLiteral("Panel tab label (base - 2 px)")), row({tabs})});
}

QWidget* statesPage()
{
    const auto forced = [](QPushButton* button, int mode) {
        if (mode == 1) {
            button->setAttribute(Qt::WA_UnderMouse, true);
        } else if (mode == 2) {
            button->setCheckable(true);
            button->setChecked(true);
        } else if (mode == 3) {
            button->setDown(true);
        } else if (mode == 4) {
            button->setEnabled(false);
        }
    };
    return page({heading(QStringLiteral("Interaction states")),
                 note(QStringLiteral("Rest, hover, pressed, checked, and disabled as the global "
                                     "stylesheet defines them. Hover and pressed are forced on real "
                                     "instances (WA_UnderMouse / setDown); they also appear on live "
                                     "interaction.")),
                 states({cell(QStringLiteral("rest"), pushButton(QStringLiteral("OK"))),
                         cell(QStringLiteral("hover"),
                              pushButton(QStringLiteral("OK"), [&](QPushButton* b) { forced(b, 1); })),
                         cell(QStringLiteral("pressed"),
                              pushButton(QStringLiteral("OK"), [&](QPushButton* b) { forced(b, 3); })),
                         cell(QStringLiteral("checked"),
                              pushButton(QStringLiteral("OK"), [&](QPushButton* b) { forced(b, 2); })),
                         cell(QStringLiteral("disabled"),
                              pushButton(QStringLiteral("OK"), [&](QPushButton* b) { forced(b, 4); }))})});
}

// ---- Shell ---------------------------------------------------------------

QWidget* menuBarPage()
{
    auto* bar = new QMenuBar;
    for (const QString& name : {QStringLiteral("File"), QStringLiteral("Edit"),
                                QStringLiteral("Image"), QStringLiteral("Layer"),
                                QStringLiteral("Select"), QStringLiteral("Filter"),
                                QStringLiteral("View"), QStringLiteral("Window"), QStringLiteral("Help")}) {
        QMenu* menu = bar->addMenu(name);
        if (name == QLatin1String("File")) {
            menu->addAction(icon(QStringLiteral("file.new")), QStringLiteral("New..."));
            menu->addAction(icon(QStringLiteral("file.open")), QStringLiteral("Open..."));
            menu->addSeparator();
            menu->addAction(icon(QStringLiteral("file.save")), QStringLiteral("Save"));
        }
    }
    return page({heading(QStringLiteral("QMenuBar")),
                 note(QStringLiteral("On the panel surface; the hovered item takes the selection colour. "
                                     "Clicking a top-level item drops its QMenu.")),
                 row({bar})});
}

QWidget* menuPage()
{
    auto* menu = new QMenu;
    menu->addAction(QStringLiteral("New..."));
    menu->addAction(QStringLiteral("Open..."));
    menu->addAction(QStringLiteral("Open as Smart Object..."));
    menu->addSeparator();
    menu->addAction(QStringLiteral("Save"));
    QAction* disabled = menu->addAction(QStringLiteral("Revert"));
    disabled->setEnabled(false);
    menu->addSeparator();
    QAction* checkable = menu->addAction(QStringLiteral("Visible"));
    checkable->setCheckable(true);
    checkable->setChecked(true);
    // Render the real QMenu inline as a child widget rather than as a popup.
    menu->setWindowFlags(Qt::Widget);
    menu->setSizePolicy(QSizePolicy::Fixed, QSizePolicy::Fixed);
    return page({heading(QStringLiteral("QMenu")),
                 note(QStringLiteral("A real QMenu shown inline: items pad to 22 px, the underline "
                                     "separator is the border-subtle token, and disabled uses the "
                                     "menu-disabled content colour.")),
                 row({menu})});
}

QWidget* optionsBarPage()
{
    auto* bar = new QToolBar;
    bar->setObjectName(QStringLiteral("optionsBar"));
    bar->addWidget(toolButton(QStringLiteral("tool.brush")));
    bar->addSeparator();
    bar->addWidget(new QLabel(QStringLiteral("Size")));
    bar->addWidget(numericField(QStringLiteral("Size:"), 25, QStringLiteral("px")));
    bar->addWidget(new QLabel(QStringLiteral("Opacity")));
    bar->addWidget(numericField(QStringLiteral("Opacity:"), 100, QStringLiteral("%")));
    bar->addSeparator();
    auto* combo = new QComboBox;
    combo->addItems({QStringLiteral("Normal"), QStringLiteral("Dissolve"), QStringLiteral("Multiply")});
    bar->addWidget(combo);
    bar->addSeparator();
    auto* check = new QCheckBox(QStringLiteral("Pressure"));
    check->setChecked(true);
    bar->addWidget(check);
    return page({heading(QStringLiteral("QToolBar#optionsBar")),
                 note(QStringLiteral("A panel strip with a 1 px border underneath and a separator "
                                     "line above; controls group on the chrome separators.")),
                 bar});
}

QWidget* toolsColumnPage()
{
    auto* strip = new QWidget;
    strip->setObjectName(QStringLiteral("toolsColumn"));
    auto* layout = new QVBoxLayout(strip);
    layout->setContentsMargins(3, 3, 3, 3);
    layout->setSpacing(1);
    for (const char* id : {"tool.move", "tool.marquee", "tool.brush", "tool.eraser", "tool.gradient",
                           "tool.horizontaltype", "tool.pen", "tool.crop", "tool.zoom", "tool.hand"}) {
        layout->addWidget(toolSlot(QLatin1String(id)));
    }
    QToolButton* active = toolSlot(QStringLiteral("tool.brush"));
    active->setChecked(true);
    return page({heading(QStringLiteral("QWidget#toolsColumn")),
                 note(QStringLiteral("Tool slots are 36 x 28 with icons capped at 24 x 20. Slots sit "
                                     "flush at rest; only hover and the active (checked) slot differ.")),
                 row({strip})});
}

QWidget* panelTabsPage()
{
    auto* tabs = new QTabWidget;
    tabs->setObjectName(QStringLiteral("panelGroupTabs"));
    for (const QString& name : {QStringLiteral("Layers"), QStringLiteral("Channels"),
                                QStringLiteral("Paths")}) {
        auto* pane = new QWidget;
        auto* paneLayout = new QVBoxLayout(pane);
        paneLayout->addWidget(new QLabel(name + QStringLiteral(" pane content")));
        tabs->addTab(pane, name);
    }
    return page({heading(QStringLiteral("QTabWidget#panelGroupTabs")),
                 note(QStringLiteral("Panel-group tabs on the header surface; the active tab takes "
                                     "the panel surface so it reads continuous with the pane.")),
                 tabs});
}

QWidget* panelGroupPage()
{
    auto* group = new PanelGroup;
    auto* first = new QWidget;
    new QVBoxLayout(first); first->layout()->addWidget(new QLabel(QStringLiteral("Layers pane")));
    auto* second = new QWidget;
    new QVBoxLayout(second); second->layout()->addWidget(new QLabel(QStringLiteral("Channels pane")));
    group->addPanel(first, QStringLiteral("Layers"), QIcon());
    group->addPanel(second, QStringLiteral("Channels"), QIcon());
    group->setFixedWidth(240);
    return page({heading(QStringLiteral("PanelGroup")),
                 note(QStringLiteral("The CS6 panel group: a top-tab QTabWidget whose tabs are the "
                                     "group chrome, with collapse and float controls.")),
                 row({group})});
}

QWidget* documentTabsPage()
{
    auto* tabs = new QTabBar;
    tabs->setObjectName(QStringLiteral("documentTabBar"));
    for (const QString& name : {QStringLiteral("Untitled-1"), QStringLiteral("Untitled-2"),
                                QStringLiteral("poster.psd")}) {
        tabs->addTab(name);
    }
    tabs->setCurrentIndex(2);
    return page({heading(QStringLiteral("QTabBar#documentTabBar")),
                 note(QStringLiteral("Document tabs over the workspace; the active tab takes "
                                     "the active-tab surface. Each carries a close button.")),
                 row({tabs})});
}

QWidget* floatingPanelPage()
{
    auto* panel = new QWidget;
    panel->setObjectName(QStringLiteral("panelFloat"));
    panel->setFixedWidth(220);
    auto* layout = new QVBoxLayout(panel);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(0);
    auto* title = new QLabel(QStringLiteral("History"));
    title->setObjectName(QStringLiteral("sectionTitle"));
    title->setContentsMargins(6, 3, 6, 3);
    layout->addWidget(title);
    auto* tabs = new QTabBar;
    tabs->setObjectName(QStringLiteral("panelTabBar"));
    tabs->addTab(QStringLiteral("History"));
    tabs->addTab(QStringLiteral("Actions"));
    layout->addWidget(tabs);
    layout->addWidget(tree({QStringLiteral("Step")},
                           {{QStringLiteral("Open")}, {QStringLiteral("Brush Tool")},
                            {QStringLiteral("Fill")}}));
    return page({heading(QStringLiteral("QWidget#panelFloat")),
                 note(QStringLiteral("A detached panel: the panel surface with a 1 px border over "
                                     "the workspace.")),
                 row({panel})});
}

QWidget* statusBarPage()
{
    auto* bar = new QStatusBar;
    bar->addWidget(new QLabel(QStringLiteral("100%")));
    bar->addWidget(new QLabel(QStringLiteral("1920 x 1080 px - 72 ppi")));
    bar->addPermanentWidget(new QLabel(QStringLiteral("3.2 MB / 5.1 MB")));
    return page({heading(QStringLiteral("QStatusBar")),
                 note(QStringLiteral("The footer strip on the panel surface; the zoom readout and "
                                     "document info sit left, the size readout right.")),
                 bar});
}

QWidget* splitterPage()
{
    auto* splitter = new QSplitter(Qt::Horizontal);
    auto* left = new QWidget;
    new QVBoxLayout(left); left->layout()->addWidget(new QLabel(QStringLiteral("panel A")));
    auto* right = new QWidget;
    new QVBoxLayout(right); right->layout()->addWidget(new QLabel(QStringLiteral("panel B")));
    splitter->addWidget(left);
    splitter->addWidget(right);
    splitter->setFixedHeight(120);
    return page({heading(QStringLiteral("QSplitter")),
                 note(QStringLiteral("The handle is border-coloured and lightens to the selection colour on "
                                     "hover; drag to resize.")),
                 splitter});
}

// ---- Controls ------------------------------------------------------------

QWidget* pushButtonPage()
{
    return page({
        heading(QStringLiteral("QPushButton")),
        note(QStringLiteral("control surface fill, a 1 px control border, a 3 px radius, and 4 x 10 padding. "
                            "Pressed and checked darken; disabled greys text and border.")),
        row({pushButton(QStringLiteral("OK")), pushButton(QStringLiteral("Cancel")),
             pushButton(QStringLiteral("Checked"), [](QPushButton* b) { b->setCheckable(true); b->setChecked(true); }),
             pushButton(QStringLiteral("Disabled"), [](QPushButton* b) { b->setEnabled(false); })}),
        states({stateCell(QStringLiteral("rest"), [] { return new QPushButton(QStringLiteral("OK")); }),
                stateCell(QStringLiteral("hover"), [] {
                    auto* b = new QPushButton(QStringLiteral("OK"));
                    b->setAttribute(Qt::WA_UnderMouse, true);
                    return b;
                }),
                stateCell(QStringLiteral("pressed"), [] {
                    auto* b = new QPushButton(QStringLiteral("OK"));
                    b->setDown(true);
                    return b;
                }),
                stateCell(QStringLiteral("disabled"), [] {
                    auto* b = new QPushButton(QStringLiteral("OK"));
                    b->setEnabled(false);
                    return b;
                })})});
}

QWidget* toolButtonPage()
{
    return page({
        heading(QStringLiteral("QToolButton")),
        note(QStringLiteral("Transparent at rest. Hover takes the control-hover surface and its border; "
                            "checked/pressed take the active surface.")),
        row({toolButton(QStringLiteral("tool.brush")), toolButton(QStringLiteral("tool.eraser")),
             toolButton(QStringLiteral("tool.gradient")), toolButton(QStringLiteral("tool.zoom"))}),
        states({stateCell(QStringLiteral("rest"), [] { return toolButton(QStringLiteral("tool.brush")); }),
                stateCell(QStringLiteral("checked"), [] {
                    auto* b = toolButton(QStringLiteral("tool.brush"));
                    b->setCheckable(true);
                    b->setChecked(true);
                    return b;
                }),
                stateCell(QStringLiteral("disabled"), [] {
                    auto* b = toolButton(QStringLiteral("tool.brush"));
                    b->setEnabled(false);
                    return b;
                })})});
}

QWidget* lineEditPage()
{
    auto* disabled = new QLineEdit(QStringLiteral("Disabled"));
    disabled->setEnabled(false);
    return page({heading(QStringLiteral("QLineEdit")),
                 note(QStringLiteral("input surface fill, a 1 px input border, a 2 px radius, and 1 x 2 "
                                     "padding. Focus borders take the selection colour; disabled greys text and border.")),
                 column({new QLineEdit(QStringLiteral("Untitled-1")),
                         new QLineEdit, disabled}, 6)});
}

QWidget* plainTextEditPage()
{
    auto* edit = new QPlainTextEdit;
    edit->setPlainText(QStringLiteral("Layer notes:\n- soft light pass\n- sharpen on merge"));
    edit->setFixedHeight(90);
    return page({heading(QStringLiteral("QPlainTextEdit")),
                 note(QStringLiteral("The same input surface as QLineEdit, multi-line.")), edit});
}

QWidget* spinBoxPage()
{
    auto* intSpin = new QSpinBox;
    intSpin->setValue(10);
    auto* doubleSpin = new QDoubleSpinBox;
    doubleSpin->setValue(1.0);
    doubleSpin->setSingleStep(0.1);
    auto* disabled = new QSpinBox;
    disabled->setEnabled(false);
    return page({heading(QStringLiteral("QSpinBox / QDoubleSpinBox")),
                 note(QStringLiteral("The input surface plus up/down arrows painted as small "
                                     "triangles in the button-text colour (the app's SpinArrowStyle).")),
                 row({intSpin, doubleSpin, disabled})});
}

QWidget* comboBoxPage()
{
    auto* combo = new QComboBox;
    combo->addItems({QStringLiteral("Normal"), QStringLiteral("Dissolve"), QStringLiteral("Multiply"),
                     QStringLiteral("Screen")});
    return page({heading(QStringLiteral("QComboBox")),
                 note(QStringLiteral("The input surface plus a 16 px drop-down with an 8 x 5 caret. "
                                     "The popup is an item view on the panel surface with the selection colour "
                                     "selection.")),
                 row({combo})});
}

QWidget* checkBoxPage()
{
    auto* checked = new QCheckBox(QStringLiteral("Channels"));
    checked->setChecked(true);
    auto* partial = new QCheckBox(QStringLiteral("Partial"));
    partial->setTristate(true);
    partial->setCheckState(Qt::PartiallyChecked);
    auto* disabled = new QCheckBox(QStringLiteral("Disabled"));
    disabled->setEnabled(false);
    return page({heading(QStringLiteral("QCheckBox")),
                 note(QStringLiteral("A 13 x 13 indicator, a 2 px radius, input surface fill; checked fills "
                                     "the selection colour, checked-hover fills the link colour.")),
                 column({new QCheckBox(QStringLiteral("Preview")), checked, partial, disabled}, 4)});
}

QWidget* radioButtonPage()
{
    auto* second = new QRadioButton(QStringLiteral("8 bit"));
    auto* third = new QRadioButton(QStringLiteral("32 bit"));
    auto* disabled = new QRadioButton(QStringLiteral("Disabled"));
    disabled->setEnabled(false);
    return page({heading(QStringLiteral("QRadioButton")),
                 note(QStringLiteral("A 13 x 13 circle (7 px radius) with the same fill rules as the "
                                     "check box; one per named group.")),
                 column({new QRadioButton(QStringLiteral("16 bit")), second, third, disabled})});
}

QWidget* sliderPage()
{
    auto* horizontal = new QSlider(Qt::Horizontal);
    horizontal->setRange(0, 100);
    horizontal->setValue(60);
    horizontal->setFixedWidth(220);
    auto* disabled = new QSlider(Qt::Horizontal);
    disabled->setRange(0, 100);
    disabled->setValue(40);
    disabled->setEnabled(false);
    disabled->setFixedWidth(220);
    auto* vertical = new QSlider(Qt::Vertical);
    vertical->setRange(0, 100);
    vertical->setValue(70);
    vertical->setFixedHeight(80);
    return page({heading(QStringLiteral("QSlider")),
                 note(QStringLiteral("A 4 px groove on the border and a 10 px handle on the control text; the "
                                     "handle hovers to the selection colour and greys when disabled.")),
                 column({horizontal, disabled}), row({vertical})});
}

QWidget* progressBarPage()
{
    auto* forty = new QProgressBar;
    forty->setRange(0, 100);
    forty->setValue(40);
    auto* disabled = new QProgressBar;
    disabled->setRange(0, 100);
    disabled->setValue(20);
    disabled->setEnabled(false);
    return page({heading(QStringLiteral("QProgressBar")),
                 note(QStringLiteral("input surface fill, a 1 px input border, a 3 px radius, centred text; "
                                     "the chunk is the selection colour with a 2 px radius.")),
                 column({forty, disabled})});
}

QWidget* groupBoxPage()
{
    auto* units = new QGroupBox(QStringLiteral("Units"));
    auto* unitsLayout = new QVBoxLayout(units);
    unitsLayout->addWidget(new QLabel(QStringLiteral("Rulers & type")));
    auto* disabled = new QGroupBox(QStringLiteral("Units"));
    disabled->setEnabled(false);
    auto* disabledLayout = new QVBoxLayout(disabled);
    disabledLayout->addWidget(new QLabel(QStringLiteral("Disabled group")));
    return page({heading(QStringLiteral("QGroupBox")),
                 note(QStringLiteral("A 1 px separator border, a 3 px radius, with the title "
                                     "overlapping the top edge on the panel surface.")),
                 row({units, disabled})});
}

QWidget* headerViewPage()
{
    auto* view = tree({QStringLiteral("Name"), QStringLiteral("Mode"), QStringLiteral("Opacity")},
                      {{QStringLiteral("Background"), QStringLiteral("Normal"), QStringLiteral("100%")},
                       {QStringLiteral("Layer 1"), QStringLiteral("Multiply"), QStringLiteral("80%")}});
    return page({heading(QStringLiteral("QHeaderView")),
                 note(QStringLiteral("Sections on the header surface with 1 px separator rules on the right "
                                     "and bottom; a section lightens on hover.")),
                 view});
}

QWidget* scrollBarPage()
{
    auto* area = new QScrollArea;
    area->setFixedSize(240, 140);
    area->setWidgetResizable(true);
    auto* content = new QWidget;
    auto* layout = new QVBoxLayout(content);
    for (int i = 0; i < 24; ++i) {
        layout->addWidget(new QLabel(QStringLiteral("Row %1 - scroll to see the thumb").arg(i + 1)));
    }
    area->setWidget(content);
    return page({heading(QStringLiteral("QScrollBar")),
                 note(QStringLiteral("A 12 px track on the scrollbar surface with a 3 px-radius thumb on "
                                     "the thumb surface; the thumb lightens on hover.")),
                 area});
}

QWidget* treeViewPage()
{
    auto* view = tree({QStringLiteral("Name"), QStringLiteral("Mode")},
                      {{QStringLiteral("Effects"), QStringLiteral("")},
                       {QStringLiteral("Layer 1"), QStringLiteral("Normal")},
                       {QStringLiteral("Layer 2"), QStringLiteral("Multiply")},
                       {QStringLiteral("Background"), QStringLiteral("")}});
    view->setFixedWidth(320);
    view->topLevelItem(1)->setSelected(true);
    return page({heading(QStringLiteral("QTreeView / QListView")),
                 note(QStringLiteral("The field surface, a 1 px separator frame, alternating rows on the "
                                     "alternate field surface, and the selection colour for selected rows.")), view});
}

QWidget* tooltipPage()
{
    auto* button = new QPushButton(QStringLiteral("Hover for a tooltip"));
    button->setToolTip(QStringLiteral("A tooltip appears on hover after the style's delay."));
    auto* slot = toolSlot(QStringLiteral("tool.brush"));
    slot->setToolTip(QStringLiteral("Brush Tool (B)"));
    return page({heading(QStringLiteral("QToolTip")),
                 note(QStringLiteral("The tooltip surface at 80 % with the tooltip content colour and a 1 px border. It is a "
                                     "top-level popup, so hover the controls below to see it live.")),
                 row({button, slot})});
}

QWidget* colorSwatchPage()
{
    return page({heading(QStringLiteral("QPushButton#colorSwatch")),
                 note(QStringLiteral("ColorSwatchWidget paints its own colour; the global rule "
                                     "removes the button chrome. Foreground over background.")),
                 row({swatch(QColor(229, 72, 77)), swatch(QColor(59, 130, 246)),
                      swatch(QColor(Qt::white)), swatch(QColor(Qt::black))})});
}

QWidget* numericFieldPage()
{
    return page({heading(QStringLiteral("QWidget#numericField")),
                 note(QStringLiteral("A leading label, a value box with a unit suffix, and an arrow "
                                     "that opens a tracking slider. Drag the label to scrub.")),
                 column({numericField(QStringLiteral("Size:"), 25, QStringLiteral("px")),
                         numericField(QStringLiteral("Opacity:"), 100, QStringLiteral("%")),
                         numericField(QStringLiteral("Gamma:"), 1.0, QString())})});
}

QWidget* panelHeaderBandPage()
{
    auto* band = new QWidget;
    band->setObjectName(QStringLiteral("panelHeaderBand"));
    band->setFixedWidth(280);
    auto* layout = new QHBoxLayout(band);
    layout->setContentsMargins(8, 5, 8, 5);
    auto* title = heading(QStringLiteral("Layers"));
    layout->addWidget(title);
    layout->addStretch(1);
    layout->addWidget(toolButton(QStringLiteral("ellipsis")));
    auto* tabs = new QTabBar;
    tabs->setObjectName(QStringLiteral("panelTabBar"));
    tabs->addTab(QStringLiteral("Layers"));
    tabs->addTab(QStringLiteral("Channels"));
    tabs->setCurrentIndex(0);
    return page({heading(QStringLiteral("QWidget#panelHeaderBand")),
                 note(QStringLiteral("The header-surface strip a group's header, drag grip and corner "
                                     "sit on; it carries no rule of its own.")),
                 column({band, tabs})});
}

// ---- Patterns ------------------------------------------------------------

QWidget* dialogPage()
{
    auto* panel = new QWidget;
    panel->setObjectName(QStringLiteral("panelFloat"));
    panel->setFixedWidth(360);
    auto* layout = new QVBoxLayout(panel);
    layout->setContentsMargins(12, 12, 12, 12);
    layout->setSpacing(10);
    auto* title = heading(QStringLiteral("Levels"));
    layout->addWidget(title);
    auto* group = new QGroupBox(QStringLiteral("Input Levels"));
    auto* groupLayout = new QHBoxLayout(group);
    groupLayout->addWidget(numericField(QString(), 0, QString()));
    groupLayout->addWidget(numericField(QString(), 1.0, QString()));
    groupLayout->addWidget(numericField(QString(), 255, QString()));
    layout->addWidget(group);
    auto* buttons = new QWidget;
    auto* buttonsLayout = new QHBoxLayout(buttons);
    buttonsLayout->setContentsMargins(0, 0, 0, 0);
    buttonsLayout->addStretch(1);
    buttonsLayout->addWidget(new QPushButton(QStringLiteral("Cancel")));
    buttonsLayout->addWidget(new QPushButton(QStringLiteral("OK")));
    layout->addWidget(buttons);
    return page({heading(QStringLiteral("Dialog pattern")),
                 note(QStringLiteral("Composed from the primitives: a bold title, a group box of "
                                     "numeric fields, and a right-aligned button row.")),
                 row({panel})});
}

QWidget* layersPanelPage()
{
    auto* panel = new QWidget;
    panel->setObjectName(QStringLiteral("panelColumnContainer"));
    panel->setFixedWidth(240);
    auto* layout = new QVBoxLayout(panel);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(0);
    auto* header = new QWidget;
    header->setObjectName(QStringLiteral("panelColumnHeader"));
    auto* headerLayout = new QHBoxLayout(header);
    headerLayout->setContentsMargins(6, 3, 6, 3);
    headerLayout->addWidget(heading(QStringLiteral("Layers")));
    headerLayout->addStretch(1);
    headerLayout->addWidget(toolButton(QStringLiteral("ellipsis")));
    layout->addWidget(header);
    auto* combo = new QComboBox;
    combo->addItems({QStringLiteral("Kind"), QStringLiteral("Name"), QStringLiteral("Effect")});
    layout->addWidget(combo);
    auto* view = tree({QStringLiteral("Name"), QStringLiteral("Mode")},
                      {{QStringLiteral("Effects"), QString()},
                       {QStringLiteral("Layer 1"), QStringLiteral("Normal")},
                       {QStringLiteral("Layer 2"), QStringLiteral("Multiply")},
                       {QStringLiteral("Background"), QString()}});
    layout->addWidget(view);
    return page({heading(QStringLiteral("Layers panel pattern")),
                 note(QStringLiteral("A column header, a filter row, and a layer tree with "
                                     "selection, over the panel surface.")),
                 row({panel})});
}

QWidget* colorPanelPage()
{
    auto* panel = new QWidget;
    panel->setObjectName(QStringLiteral("panelColumnContainer"));
    panel->setFixedWidth(220);
    auto* layout = new QVBoxLayout(panel);
    layout->setContentsMargins(10, 10, 10, 10);
    layout->setSpacing(10);
    layout->addWidget(heading(QStringLiteral("Colour")));
    layout->addWidget(row({swatch(QColor(229, 72, 77)), swatch(QColor(Qt::black))}));
    layout->addWidget(column({numericField(QStringLiteral("R:"), 229, QString()),
                              numericField(QStringLiteral("G:"), 72, QString()),
                              numericField(QStringLiteral("B:"), 77, QString())}, 4));
    QList<QWidget*> swatches;
    for (const char* hex : {"#e5484d", "#f97316", "#e5a00d", "#3fb950", "#14b8a6", "#3b82f6",
                            "#8b5cf6", "#ec4899", "#777777", "#111111"}) {
        swatches.append(swatch(QColor(QLatin1String(hex))));
    }
    layout->addWidget(grid(swatches, 5));
    return page({heading(QStringLiteral("Colour panel pattern")),
                 note(QStringLiteral("The foreground/background pair, RGB numeric fields, and a "
                                     "swatch grid.")),
                 row({panel})});
}

QWidget* dockedColumnPage()
{
    auto* strip = new QWidget;
    strip->setObjectName(QStringLiteral("toolsColumn"));
    auto* stripLayout = new QVBoxLayout(strip);
    stripLayout->setContentsMargins(3, 3, 3, 3);
    stripLayout->setSpacing(1);
    for (const char* id : {"tool.move", "tool.brush", "tool.eraser", "tool.gradient", "tool.zoom"}) {
        stripLayout->addWidget(toolSlot(QLatin1String(id)));
    }
    auto* group = new PanelGroup;
    group->setFixedWidth(220);
    auto* pane = new QWidget;
    new QVBoxLayout(pane); pane->layout()->addWidget(new QLabel(QStringLiteral("Layers pane")));
    group->addPanel(pane, QStringLiteral("Layers"), QIcon());
    auto* splitter = new QSplitter(Qt::Horizontal);
    splitter->addWidget(strip);
    splitter->addWidget(group);
    auto* workspace = new QWidget;
    workspace->setMinimumWidth(120);
    new QVBoxLayout(workspace);
    workspace->layout()->addWidget(new QLabel(QStringLiteral("workspace")));
    splitter->addWidget(workspace);
    splitter->setFixedHeight(260);
    return page({heading(QStringLiteral("Docked column pattern")),
                 note(QStringLiteral("A tools column, a panel group, a splitter, and the workspace - "
                                     "the standard arrangement.")),
                 splitter});
}

} // namespace

QVector<DesignSystemStory> designSystemStories()
{
    return {
        // Foundations
        {QStringLiteral("Foundations"), QStringLiteral("Palette"),
         QStringLiteral("Every colour token at the current brightness. selection is the emphasis "
                        "fill; accent is the drop/insertion indicator."),
         {QStringLiteral("surface-recessed"), QStringLiteral("surface"), QStringLiteral("surface-control"),
          QStringLiteral("selection"), QStringLiteral("accent")},
         QString(), palettePage},
        {QStringLiteral("Foundations"), QStringLiteral("Brightness levels"),
         QStringLiteral("The four brightness levels of the one dark family; text and accent stay "
                        "constant across levels."),
         {QStringLiteral("shade-step")}, QString(), brightnessPage},
        {QStringLiteral("Foundations"), QStringLiteral("Metrics"),
         QStringLiteral("Shared chrome metrics from the stylesheet constants."),
         {QStringLiteral("border-width-panel"), QStringLiteral("border-width-chrome"),
          QStringLiteral("divider-width-group"), QStringLiteral("slot-base-width"),
          QStringLiteral("slot-base-height")},
         QString(), metricsPage},
        {QStringLiteral("Foundations"), QStringLiteral("Typography"),
         QStringLiteral("The Qt default UI font and the bold/disabled/link variants; panel tabs run "
                        "2 px under the base."),
         {}, QString(), typographyPage},
        {QStringLiteral("Foundations"), QStringLiteral("Interaction states"),
         QStringLiteral("The shared state vocabulary: hover, pressed/checked, and disabled."),
         {QStringLiteral("surface-hover"), QStringLiteral("surface-pressed"),
          QStringLiteral("content-disabled")},
         QString(), statesPage},

        // Shell
        {QStringLiteral("Shell"), QStringLiteral("Menu bar"),
         QStringLiteral("QMenuBar on the panel surface."), {},
         QStringLiteral("QMenuBar"), menuBarPage},
        {QStringLiteral("Shell"), QStringLiteral("Menu"),
         QStringLiteral("QMenu items, separator, disabled, and a checkable item."), {},
         QStringLiteral("QMenu"), menuPage},
        {QStringLiteral("Shell"), QStringLiteral("Options bar"),
         QStringLiteral("QToolBar#optionsBar with tool buttons, numeric fields, a combo, and a check "
                        "box."),
         {QStringLiteral("surface"), QStringLiteral("border"), QStringLiteral("border-subtle-alt")},
         QStringLiteral("QToolBar#optionsBar"), optionsBarPage},
        {QStringLiteral("Shell"), QStringLiteral("Tools column"),
         QStringLiteral("The tools column with 36 x 28 tool slots."),
         {QStringLiteral("slot-base-width"), QStringLiteral("slot-base-height")},
         QStringLiteral("QWidget#toolsColumn"), toolsColumnPage},
        {QStringLiteral("Shell"), QStringLiteral("Panel group"),
         QStringLiteral("QTabWidget#panelGroupTabs: the CS6 panel-group tabs."),
         {QStringLiteral("surface-header"), QStringLiteral("surface")},
         QStringLiteral("QTabBar#panelTabBar"), panelGroupPage},
        {QStringLiteral("Shell"), QStringLiteral("Panel tabs"),
         QStringLiteral("The panel-group tab widget and its pane."),
         {QStringLiteral("surface-header"), QStringLiteral("surface")},
         QStringLiteral("QTabWidget#panelGroupTabs"), panelTabsPage},
        {QStringLiteral("Shell"), QStringLiteral("Document tabs"),
         QStringLiteral("QTabBar#documentTabBar over the workspace."),
         {QStringLiteral("surface-recessed"), QStringLiteral("surface-tab-active")},
         QStringLiteral("QTabBar#documentTabBar"), documentTabsPage},
        {QStringLiteral("Shell"), QStringLiteral("Floating panel"),
         QStringLiteral("A detached panel (QWidget#panelFloat) over the workspace."),
         {QStringLiteral("surface"), QStringLiteral("border")},
         QStringLiteral("QWidget#panelFloat"), floatingPanelPage},
        {QStringLiteral("Shell"), QStringLiteral("Status bar"),
         QStringLiteral("QStatusBar: the footer strip."), {},
         QStringLiteral("QStatusBar"), statusBarPage},
        {QStringLiteral("Shell"), QStringLiteral("Splitter"),
         QStringLiteral("QSplitter handle with hover highlight."),
         {QStringLiteral("border"), QStringLiteral("selection")},
         QStringLiteral("QSplitter"), splitterPage},

        // Controls
        {QStringLiteral("Controls"), QStringLiteral("Push button"),
         QStringLiteral("QPushButton in rest, hover, pressed, checked, and disabled states."),
         {QStringLiteral("surface-control"), QStringLiteral("border-control"),
          QStringLiteral("surface-control-pressed")},
         QStringLiteral("QPushButton"), pushButtonPage},
        {QStringLiteral("Controls"), QStringLiteral("Tool button"),
         QStringLiteral("QToolButton: transparent at rest, the control-hover surface on hover, the "
                        "active surface when checked."),
         {QStringLiteral("surface-control-hover"), QStringLiteral("surface-control-active")},
         QStringLiteral("QToolButton"), toolButtonPage},
        {QStringLiteral("Controls"), QStringLiteral("Line edit"),
         QStringLiteral("QLineEdit with rest, focus, and disabled states."),
         {QStringLiteral("surface-input"), QStringLiteral("border-input")},
         QStringLiteral("QLineEdit"), lineEditPage},
        {QStringLiteral("Controls"), QStringLiteral("Plain text edit"),
         QStringLiteral("QPlainTextEdit, the multi-line input surface."),
         {QStringLiteral("surface-input"), QStringLiteral("border-input")},
         QStringLiteral("QPlainTextEdit"), plainTextEditPage},
        {QStringLiteral("Controls"), QStringLiteral("Spin box"),
         QStringLiteral("QSpinBox / QDoubleSpinBox with the painted arrows."),
         {QStringLiteral("surface-input"), QStringLiteral("content-control")},
         QStringLiteral("QSpinBox"), spinBoxPage},
        {QStringLiteral("Controls"), QStringLiteral("Combo box"),
         QStringLiteral("QComboBox and its item-view popup."),
         {QStringLiteral("surface-input"), QStringLiteral("surface")},
         QStringLiteral("QComboBox"), comboBoxPage},
        {QStringLiteral("Controls"), QStringLiteral("Check box"),
         QStringLiteral("QCheckBox: the 13 x 13 indicator with checked, partial, and disabled "
                        "states."),
         {QStringLiteral("selection"), QStringLiteral("content-link")},
         QStringLiteral("QCheckBox"), checkBoxPage},
        {QStringLiteral("Controls"), QStringLiteral("Radio button"),
         QStringLiteral("QRadioButton: the 13 x 13 circle."),
         {QStringLiteral("selection"), QStringLiteral("content-link")},
         QStringLiteral("QRadioButton"), radioButtonPage},
        {QStringLiteral("Controls"), QStringLiteral("Slider"),
         QStringLiteral("QSlider with the 4 px groove and 10 px handle, horizontal and vertical."),
         {QStringLiteral("border"), QStringLiteral("content-control")},
         QStringLiteral("QSlider"), sliderPage},
        {QStringLiteral("Controls"), QStringLiteral("Progress bar"),
         QStringLiteral("QProgressBar with the selection chunk."),
         {QStringLiteral("surface-input"), QStringLiteral("selection")},
         QStringLiteral("QProgressBar"), progressBarPage},
        {QStringLiteral("Controls"), QStringLiteral("Group box"),
         QStringLiteral("QGroupBox with its title on the panel surface."),
         {QStringLiteral("border-subtle"), QStringLiteral("surface")},
         QStringLiteral("QGroupBox"), groupBoxPage},
        {QStringLiteral("Controls"), QStringLiteral("Header view"),
         QStringLiteral("QHeaderView sections on the header surface."),
         {QStringLiteral("surface-header"), QStringLiteral("border-subtle")},
         QStringLiteral("QHeaderView"), headerViewPage},
        {QStringLiteral("Controls"), QStringLiteral("Scrollbar"),
         QStringLiteral("QScrollBar track and thumb, hover on the thumb."),
         {QStringLiteral("surface-scrollbar"), QStringLiteral("scrollbar-thumb")},
         QStringLiteral("QScrollBar"), scrollBarPage},
        {QStringLiteral("Controls"), QStringLiteral("Tree / layer view"),
         QStringLiteral("QTreeView with selection and alternating rows."),
         {QStringLiteral("surface-field"), QStringLiteral("selection")},
         QStringLiteral("QTreeView"), treeViewPage},
        {QStringLiteral("Controls"), QStringLiteral("Tooltip"),
         QStringLiteral("QToolTip on the tooltip surface; a top-level popup shown live on hover."),
         {QStringLiteral("surface-tooltip"), QStringLiteral("content-tooltip")},
         QStringLiteral("QToolTip"), tooltipPage},
        {QStringLiteral("Controls"), QStringLiteral("Colour swatch"),
         QStringLiteral("ColorSwatchWidget (QPushButton#colorSwatch) paints its own colour."),
         {QStringLiteral("border")},
         QStringLiteral("QPushButton#colorSwatch"), colorSwatchPage},
        {QStringLiteral("Controls"), QStringLiteral("Numeric field"),
         QStringLiteral("The app's NumericField: label, value, suffix, and slider popup."),
         {QStringLiteral("surface-input"), QStringLiteral("border-input")},
         QStringLiteral("QWidget#numericField"), numericFieldPage},
        {QStringLiteral("Controls"), QStringLiteral("Panel header band"),
         QStringLiteral("QWidget#panelHeaderBand, the header strip."),
         {QStringLiteral("surface-header")},
         QStringLiteral("QWidget#panelHeaderBand"), panelHeaderBandPage},

        // Patterns
        {QStringLiteral("Patterns"), QStringLiteral("Dialog"),
         QStringLiteral("A dialog composed from the primitives."),
         {QStringLiteral("surface"), QStringLiteral("border-subtle")}, QString(), dialogPage},
        {QStringLiteral("Patterns"), QStringLiteral("Layers panel"),
         QStringLiteral("The Layers panel: header, filter row, tree, and selection."),
         {QStringLiteral("surface"), QStringLiteral("surface-header"), QStringLiteral("surface-field")},
         QString(), layersPanelPage},
        {QStringLiteral("Patterns"), QStringLiteral("Colour panel"),
         QStringLiteral("Colour panel: the foreground/background pair, RGB fields, and swatches."),
         {QStringLiteral("surface"), QStringLiteral("surface-input")}, QString(), colorPanelPage},
        {QStringLiteral("Patterns"), QStringLiteral("Docked column"),
         QStringLiteral("Tools column, panel group, splitter, and workspace."),
         {QStringLiteral("surface"), QStringLiteral("border-subtle")}, QString(), dockedColumnPage},
    };
}

} // namespace pictura
