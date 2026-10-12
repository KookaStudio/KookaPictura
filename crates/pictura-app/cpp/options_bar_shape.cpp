// The options bar of the shape tools (Rectangle, Rounded Rectangle, Ellipse,
// Polygon, Line, Custom Shape), laid out as CS6's: Mode, Fill, Stroke (width
// and type), W / link / H, the path operation / alignment / arrangement
// menus, the geometry gear, the tool's own field (Radius, Sides, Weight, or
// the Shape picker), and Align Edges. Part of OptionsBar; split from
// options_bar.cpp along the page seam.
//
// Every tool has its own page, so each page re-reads the shared ShapeOptions
// on `shapeOptionsChanged` (an edit on another page, or the active shape layer
// mirrored by the tool).
//
// ponytail: Fill and Stroke are None or a solid colour (Gradient and Pattern
// are listed but disabled), the stroke is solid only (Dashed / Dotted
// disabled), widths are pixels rather than points, and the path operation /
// alignment / arrangement menus are disabled stubs.

#include "options_bar.h"

#include "color_picker_dialog.h"
#include "icons.h"
#include "panels/numeric_field.h"

#include "pictura_app/src/cxxqt_object/shapes.cxxqt.h"

#include <QtCore/QSignalBlocker>
#include <QtGui/QPainter>
#include <QtGui/QPixmap>
#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QMenu>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QWidgetAction>

#include <memory>

namespace pictura {

namespace {

// A Fill / Stroke swatch: the colour, or white with a red slash for None.
QIcon swatchIcon(bool on, const QColor& color)
{
    QPixmap pixmap(22, 16);
    pixmap.fill(on ? color : QColor(Qt::white));
    QPainter painter(&pixmap);
    if (!on) {
        painter.setRenderHint(QPainter::Antialiasing, true);
        painter.setPen(QPen(QColor(0xd0, 0x20, 0x20), 2));
        painter.drawLine(2, 14, 20, 2);
    }
    painter.setPen(QColor(0x40, 0x40, 0x40));
    painter.drawRect(0, 0, 21, 15);
    return QIcon(pixmap);
}

// The three path-menu glyphs, drawn rather than shipped: combined squares,
// aligned bars, and stacked squares.
QIcon pathGlyph(int which)
{
    const QColor ink(0xd0, 0xd0, 0xd0);
    QPixmap pixmap(18, 18);
    pixmap.fill(Qt::transparent);
    QPainter painter(&pixmap);
    painter.setPen(QPen(ink, 1.2));
    if (which == 0) {
        painter.drawRect(2, 2, 9, 9);
        painter.drawRect(7, 7, 9, 9);
    } else if (which == 1) {
        painter.drawLine(2, 2, 2, 16);
        painter.fillRect(4, 4, 10, 3, ink);
        painter.fillRect(4, 10, 6, 3, ink);
    } else {
        painter.drawRect(5, 1, 10, 7);
        painter.drawRect(3, 5, 10, 7);
        painter.drawRect(1, 9, 10, 7);
    }
    return QIcon(pixmap);
}

QToolButton* disabledMenuButton(QWidget* page, int glyph, const QString& name,
                                const QString& what)
{
    auto* button = new QToolButton(page);
    button->setObjectName(name);
    button->setIcon(pathGlyph(glyph));
    button->setEnabled(false);
    button->setToolTip(what + QStringLiteral(": not implemented yet"));
    return button;
}

void addForm(QMenu* menu, QWidget* form)
{
    auto* action = new QWidgetAction(menu);
    action->setDefaultWidget(form);
    menu->addAction(action);
}

// A white silhouette for the dark picker, as CS6 draws its shapes.
QIcon pickerIcon(int index, int size)
{
    QImage image = shape_custom_preview(index, size);
    image.invertPixels(QImage::InvertRgb);
    return QIcon(QPixmap::fromImage(image));
}

} // namespace

// The page: see the file comment.
QWidget* OptionsBar::buildShapePage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));

    const ShapeOptions initial = controller_ ? shape_->get() : ShapeOptions{};
    const ShapeUpdate update = [this](const std::function<void(ShapeOptions&)>& edit) {
        if (controller_) {
            ShapeOptions o = shape_->get();
            edit(o);
            shape_->set(o);
        }
    };
    // Re-reads, run with the page's signals blocked on every shapeOptionsChanged.
    auto syncs = std::make_shared<ShapeSyncs>();

    auto* mode = new QComboBox(page);
    mode->setObjectName(QStringLiteral("optionsShapeMode"));
    mode->setToolTip(QStringLiteral("Pick tool mode"));
    mode->addItems({QStringLiteral("Shape"), QStringLiteral("Path"), QStringLiteral("Pixels")});
    layout->addWidget(mode);
    connect(mode, &QComboBox::currentIndexChanged, this,
            [update](int i) { update([i](ShapeOptions& o) { o.mode = i; }); });
    syncs->push_back([mode](const ShapeOptions& o) { mode->setCurrentIndex(o.mode); });

    buildShapeAppearance(page, layout, update, *syncs);

    // W / link / H: the active shape layer's size; disabled without one.
    auto* width = new NumericField(QStringLiteral("W:"),
                                   numericConfig(0.01, 300000, 1, 2, QStringLiteral(" px"), false,
                                                 QStringLiteral("optionsShapeWidth")),
                                   page);
    auto* link = new QToolButton(page);
    link->setObjectName(QStringLiteral("optionsShapeLinkSize"));
    link->setIcon(icon(QStringLiteral("layers.link")));
    link->setCheckable(true);
    link->setToolTip(QStringLiteral("Link shape width and height"));
    auto* height = new NumericField(QStringLiteral("H:"),
                                    numericConfig(0.01, 300000, 1, 2, QStringLiteral(" px"), false,
                                                  QStringLiteral("optionsShapeHeight")),
                                    page);
    layout->addWidget(width);
    layout->addWidget(link);
    layout->addWidget(height);
    connect(link, &QToolButton::toggled, this,
            [update](bool on) { update([on](ShapeOptions& o) { o.linkSize = on; }); });
    connect(width, &NumericField::valueCommitted, this, [update](double w) {
        update([w](ShapeOptions& o) {
            if (o.linkSize && o.activeWidth > 0.0) {
                o.activeHeight *= w / o.activeWidth;
            }
            o.activeWidth = w;
        });
    });
    connect(height, &NumericField::valueCommitted, this, [update](double h) {
        update([h](ShapeOptions& o) {
            if (o.linkSize && o.activeHeight > 0.0) {
                o.activeWidth *= h / o.activeHeight;
            }
            o.activeHeight = h;
        });
    });
    syncs->push_back([width, height, link](const ShapeOptions& o) {
        const bool active = o.activeWidth > 0.0;
        width->setEnabled(active);
        height->setEnabled(active);
        link->setChecked(o.linkSize);
        width->setValue(active ? o.activeWidth : 0.0);
        height->setValue(active ? o.activeHeight : 0.0);
    });

    layout->addWidget(disabledMenuButton(page, 0, QStringLiteral("optionsShapePathOperations"),
                                         QStringLiteral("Path operations")));
    layout->addWidget(disabledMenuButton(page, 1, QStringLiteral("optionsShapePathAlignment"),
                                         QStringLiteral("Path alignment")));
    layout->addWidget(disabledMenuButton(page, 2, QStringLiteral("optionsShapePathArrangement"),
                                         QStringLiteral("Path arrangement")));

    layout->addWidget(id == ToolId::Line ? buildArrowheadsButton(page, initial, update)
                                         : buildShapeGeometryButton(id, page, update, *syncs));

    if (id == ToolId::RoundedRectangle) {
        auto* radius = new NumericField(
            QStringLiteral("Radius:"),
            numericConfig(0, 1000, 1, 0, QStringLiteral(" px"), true,
                          QStringLiteral("optionsShapeRadius")),
            page);
        radius->setValue(initial.radius);
        layout->addWidget(radius);
        connect(radius, &NumericField::valueChanged, this,
                [update](double v) { update([v](ShapeOptions& o) { o.radius = v; }); });
    } else if (id == ToolId::Polygon) {
        auto* sides = new NumericField(
            QStringLiteral("Sides:"),
            numericConfig(3, 100, 1, 0, QString(), true, QStringLiteral("optionsShapeSides")),
            page);
        sides->setValue(initial.sides);
        layout->addWidget(sides);
        connect(sides, &NumericField::valueChanged, this,
                [update](double v) { update([v](ShapeOptions& o) { o.sides = qRound(v); }); });
    } else if (id == ToolId::Line) {
        auto* weight = new NumericField(
            QStringLiteral("Weight:"),
            numericConfig(1, 1000, 1, 0, QStringLiteral(" px"), true,
                          QStringLiteral("optionsShapeWeight")),
            page);
        weight->setValue(initial.weight);
        layout->addWidget(weight);
        connect(weight, &NumericField::valueChanged, this,
                [update](double v) { update([v](ShapeOptions& o) { o.weight = v; }); });
    } else if (id == ToolId::CustomShape) {
        layout->addWidget(new QLabel(QStringLiteral("Shape:"), page));
        layout->addWidget(buildShapePicker(page, update, *syncs));
    }

    auto* alignEdges = new QCheckBox(QStringLiteral("Align Edges"), page);
    alignEdges->setObjectName(QStringLiteral("optionsShapeAlignEdges"));
    alignEdges->setToolTip(QStringLiteral("Snap straight horizontal and vertical edges to pixels"));
    layout->addWidget(alignEdges);
    connect(alignEdges, &QCheckBox::toggled, this,
            [update](bool on) { update([on](ShapeOptions& o) { o.alignEdges = on; }); });
    syncs->push_back(
        [alignEdges](const ShapeOptions& o) { alignEdges->setChecked(o.alignEdges); });
    layout->addStretch(1);

    const auto syncAll = [this, page, syncs]() {
        const ShapeOptions o = controller_ ? shape_->get() : ShapeOptions{};
        std::vector<std::unique_ptr<QSignalBlocker>> blocks;
        for (QObject* child : page->findChildren<QObject*>()) {
            blocks.push_back(std::make_unique<QSignalBlocker>(child));
        }
        for (const auto& sync : *syncs) {
            sync(o);
        }
    };
    syncAll();
    if (controller_) {
        connect(controller_, &ToolController::shapeOptionsChanged, page, syncAll);
        connect(controller_, &ToolController::colorsChanged, page, syncAll);
    }
    return page;
}

// Fill and Stroke swatches, the stroke width, and the stroke type pop-up
// (line type and Align).
void OptionsBar::buildShapeAppearance(QWidget* page, QHBoxLayout* layout,
                                      const ShapeUpdate& update, ShapeSyncs& syncs)
{
    const auto swatch = [this, page, layout, update](bool fill) {
        auto* label =
            new QLabel(fill ? QStringLiteral("Fill:") : QStringLiteral("Stroke:"), page);
        auto* button = new QToolButton(page);
        button->setObjectName(fill ? QStringLiteral("optionsShapeFill")
                                   : QStringLiteral("optionsShapeStroke"));
        button->setPopupMode(QToolButton::InstantPopup);
        button->setIconSize(QSize(22, 16));
        auto* menu = new QMenu(button);
        QAction* none = menu->addAction(QStringLiteral("No Color"));
        none->setObjectName(button->objectName() + QStringLiteral("None"));
        QAction* solid = menu->addAction(QStringLiteral("Solid Color..."));
        solid->setObjectName(button->objectName() + QStringLiteral("Solid"));
        for (const QString& unsupported : {QStringLiteral("Gradient"), QStringLiteral("Pattern")}) {
            QAction* action = menu->addAction(unsupported);
            action->setEnabled(false);
            action->setToolTip(unsupported + QStringLiteral(": not implemented yet"));
        }
        button->setMenu(menu);
        connect(none, &QAction::triggered, this, [update, fill]() {
            update([fill](ShapeOptions& o) { (fill ? o.fillEnabled : o.strokeEnabled) = false; });
        });
        connect(solid, &QAction::triggered, this, [this, update, fill, button]() {
            const ShapeOptions o = controller_ ? shape_->get() : ShapeOptions{};
            QColor start = o.strokeColor;
            if (fill) {
                start = o.fillColor.isValid() || !controller_ ? o.fillColor
                                                               : controller_->foreground();
            }
            const QColor picked = ColorPickerDialog::getColor(
                start, button,
                fill ? QStringLiteral("Fill Color") : QStringLiteral("Stroke Color"));
            if (!picked.isValid()) {
                return;
            }
            update([fill, picked](ShapeOptions& s) {
                if (fill) {
                    s.fillEnabled = true;
                    s.fillColor = picked;
                } else {
                    s.strokeEnabled = true;
                    s.strokeColor = picked;
                }
            });
        });
        layout->addWidget(label);
        layout->addWidget(button);
        return button;
    };
    QToolButton* fill = swatch(true);
    QToolButton* stroke = swatch(false);
    syncs.push_back([this, fill, stroke](const ShapeOptions& o) {
        QColor fillColor = o.fillColor;
        if (!fillColor.isValid()) {
            fillColor = controller_ ? controller_->foreground() : QColor(Qt::black);
        }
        fill->setIcon(swatchIcon(o.fillEnabled, fillColor));
        stroke->setIcon(swatchIcon(o.strokeEnabled, o.strokeColor));
    });

    auto* strokeWidth = new NumericField(
        QString(),
        numericConfig(1, 250, 1, 0, QStringLiteral(" px"), true,
                      QStringLiteral("optionsShapeStrokeWidth")),
        page);
    strokeWidth->setToolTip(QStringLiteral("Set shape stroke width"));
    layout->addWidget(strokeWidth);
    connect(strokeWidth, &NumericField::valueChanged, this,
            [update](double v) { update([v](ShapeOptions& o) { o.strokeWidth = v; }); });

    auto* type = new QToolButton(page);
    type->setObjectName(QStringLiteral("optionsShapeStrokeType"));
    type->setText(QStringLiteral("——"));
    type->setToolTip(QStringLiteral("Set shape stroke type"));
    type->setPopupMode(QToolButton::InstantPopup);
    auto* menu = new QMenu(type);
    auto* form = new QWidget(menu);
    auto* grid = new QFormLayout(form);
    auto* line = new QComboBox(form);
    line->setObjectName(QStringLiteral("optionsShapeStrokeLine"));
    line->addItems({QStringLiteral("Solid"), QStringLiteral("Dashed"), QStringLiteral("Dotted")});
    // Dashed and Dotted are listed, disabled (no dash pattern is rendered).
    for (int i = 1; i < 3; ++i) {
        line->setItemData(i, 0, Qt::UserRole - 1);
        line->setItemData(i, QStringLiteral("Not implemented yet"), Qt::ToolTipRole);
    }
    auto* align = new QComboBox(form);
    align->setObjectName(QStringLiteral("optionsShapeStrokeAlign"));
    align->addItems({QStringLiteral("Inside"), QStringLiteral("Center"), QStringLiteral("Outside")});
    grid->addRow(QStringLiteral("Line:"), line);
    grid->addRow(QStringLiteral("Align:"), align);
    addForm(menu, form);
    type->setMenu(menu);
    layout->addWidget(type);
    connect(align, &QComboBox::currentIndexChanged, this,
            [update](int i) { update([i](ShapeOptions& o) { o.strokeAlign = i; }); });
    syncs.push_back([strokeWidth, align](const ShapeOptions& o) {
        strokeWidth->setValue(o.strokeWidth);
        align->setCurrentIndex(o.strokeAlign);
    });
}

// The geometry gear: Unconstrained / Square, Circle, or Defined Proportions /
// Fixed Size (W, H) and From Center for the box tools; the star and smoothing
// for the Polygon.
QToolButton* OptionsBar::buildShapeGeometryButton(ToolId id, QWidget* page,
                                                  const ShapeUpdate& update, ShapeSyncs& syncs)
{
    auto* button = new QToolButton(page);
    button->setObjectName(QStringLiteral("optionsShapeGeometry"));
    button->setIcon(icon(QStringLiteral("view.options")));
    button->setToolTip(QStringLiteral("Set additional shape and path options"));
    button->setPopupMode(QToolButton::InstantPopup);
    auto* menu = new QMenu(button);
    auto* form = new QWidget(menu);
    auto* box = new QVBoxLayout(form);
    if (id == ToolId::Polygon) {
        auto* smoothCorners = new QCheckBox(QStringLiteral("Smooth Corners"), form);
        smoothCorners->setObjectName(QStringLiteral("optionsShapeSmoothCorners"));
        auto* star = new QCheckBox(QStringLiteral("Star"), form);
        star->setObjectName(QStringLiteral("optionsShapeStar"));
        auto* indent = new NumericField(
            QStringLiteral("Indent Sides By:"),
            numericConfig(1, 99, 1, 0, QStringLiteral("%"), true,
                          QStringLiteral("optionsShapeIndent")),
            form);
        auto* smoothIndents = new QCheckBox(QStringLiteral("Smooth Indents"), form);
        smoothIndents->setObjectName(QStringLiteral("optionsShapeSmoothIndents"));
        box->addWidget(smoothCorners);
        box->addWidget(star);
        box->addWidget(indent);
        box->addWidget(smoothIndents);
        connect(smoothCorners, &QCheckBox::toggled, this,
                [update](bool on) { update([on](ShapeOptions& o) { o.smoothCorners = on; }); });
        connect(star, &QCheckBox::toggled, this,
                [update](bool on) { update([on](ShapeOptions& o) { o.star = on; }); });
        connect(indent, &NumericField::valueChanged, this,
                [update](double v) { update([v](ShapeOptions& o) { o.indent = v; }); });
        connect(smoothIndents, &QCheckBox::toggled, this,
                [update](bool on) { update([on](ShapeOptions& o) { o.smoothIndents = on; }); });
        syncs.push_back([smoothCorners, star, indent, smoothIndents](const ShapeOptions& o) {
            smoothCorners->setChecked(o.smoothCorners);
            star->setChecked(o.star);
            indent->setValue(o.indent);
            indent->setEnabled(o.star);
            smoothIndents->setChecked(o.smoothIndents);
            smoothIndents->setEnabled(o.star);
        });
    } else {
        QString constrained = QStringLiteral("Square");
        if (id == ToolId::Ellipse) {
            constrained = QStringLiteral("Circle");
        } else if (id == ToolId::CustomShape) {
            constrained = QStringLiteral("Defined Proportions");
        }
        auto* group = new QButtonGroup(form);
        const QStringList labels = {QStringLiteral("Unconstrained"), constrained,
                                    QStringLiteral("Fixed Size")};
        for (int i = 0; i < labels.size(); ++i) {
            auto* radio = new QRadioButton(labels[i], form);
            radio->setObjectName(QStringLiteral("optionsShapeGeometry%1").arg(i));
            group->addButton(radio, i);
            box->addWidget(radio);
        }
        auto* size = new QHBoxLayout;
        auto* fixedW = new NumericField(
            QStringLiteral("W:"),
            numericConfig(0.01, 300000, 1, 2, QStringLiteral(" px"), false,
                          QStringLiteral("optionsShapeFixedWidth")),
            form);
        auto* fixedH = new NumericField(
            QStringLiteral("H:"),
            numericConfig(0.01, 300000, 1, 2, QStringLiteral(" px"), false,
                          QStringLiteral("optionsShapeFixedHeight")),
            form);
        size->addWidget(fixedW);
        size->addWidget(fixedH);
        box->addLayout(size);
        auto* fromCenter = new QCheckBox(QStringLiteral("From Center"), form);
        fromCenter->setObjectName(QStringLiteral("optionsShapeFromCenter"));
        box->addWidget(fromCenter);
        connect(group, &QButtonGroup::idClicked, this,
                [update](int i) { update([i](ShapeOptions& o) { o.geometry = i; }); });
        connect(fixedW, &NumericField::valueChanged, this,
                [update](double v) { update([v](ShapeOptions& o) { o.fixedWidth = v; }); });
        connect(fixedH, &NumericField::valueChanged, this,
                [update](double v) { update([v](ShapeOptions& o) { o.fixedHeight = v; }); });
        connect(fromCenter, &QCheckBox::toggled, this,
                [update](bool on) { update([on](ShapeOptions& o) { o.fromCenter = on; }); });
        syncs.push_back([group, fixedW, fixedH, fromCenter](const ShapeOptions& o) {
            if (QAbstractButton* b = group->button(o.geometry)) {
                b->setChecked(true);
            }
            fixedW->setValue(o.fixedWidth);
            fixedH->setValue(o.fixedHeight);
            fixedW->setEnabled(o.geometry == 2);
            fixedH->setEnabled(o.geometry == 2);
            fromCenter->setChecked(o.fromCenter);
        });
    }
    addForm(menu, form);
    button->setMenu(menu);
    return button;
}

// The Custom Shape picker: a button showing the chosen shape that opens a grid
// of the shapes beside the picker's gear menu (Reset Shapes; loading, saving,
// and replacing shape sets are listed, disabled).
QToolButton* OptionsBar::buildShapePicker(QWidget* page, const ShapeUpdate& update,
                                          ShapeSyncs& syncs)
{
    auto* button = new QToolButton(page);
    button->setObjectName(QStringLiteral("optionsShapeCustom"));
    button->setToolTip(QStringLiteral("Click to open Custom Shape picker"));
    button->setIconSize(QSize(24, 24));
    button->setPopupMode(QToolButton::InstantPopup);
    auto* menu = new QMenu(button);
    auto* panel = new QWidget(menu);
    auto* grid = new QGridLayout(panel);
    auto* list = new QListWidget(panel);
    list->setObjectName(QStringLiteral("optionsShapeCustomGrid"));
    list->setViewMode(QListView::IconMode);
    list->setIconSize(QSize(32, 32));
    list->setGridSize(QSize(40, 40));
    list->setResizeMode(QListView::Adjust);
    list->setMovement(QListView::Static);
    list->setMinimumSize(250, 130);
    for (int i = 0; i < shape_custom_count(); ++i) {
        auto* item = new QListWidgetItem(pickerIcon(i, 64), QString(), list);
        item->setToolTip(shape_custom_name(i));
    }
    auto* gear = new QToolButton(panel);
    gear->setObjectName(QStringLiteral("optionsShapeCustomGear"));
    gear->setIcon(icon(QStringLiteral("view.options")));
    gear->setPopupMode(QToolButton::InstantPopup);
    auto* gearMenu = new QMenu(gear);
    QAction* reset = gearMenu->addAction(QStringLiteral("Reset Shapes..."));
    reset->setObjectName(QStringLiteral("optionsShapeCustomReset"));
    for (const QString& unsupported : {QStringLiteral("Load Shapes..."),
                                       QStringLiteral("Save Shapes..."),
                                       QStringLiteral("Replace Shapes...")}) {
        gearMenu->addAction(unsupported)->setEnabled(false);
    }
    gear->setMenu(gearMenu);
    grid->addWidget(list, 0, 0);
    grid->addWidget(gear, 0, 1, Qt::AlignTop);
    addForm(menu, panel);
    button->setMenu(menu);
    connect(list, &QListWidget::currentRowChanged, this, [update, menu](int row) {
        if (row >= 0) {
            update([row](ShapeOptions& o) { o.custom = row; });
            menu->close();
        }
    });
    connect(reset, &QAction::triggered, this,
            [update]() { update([](ShapeOptions& o) { o.custom = 0; }); });
    syncs.push_back([button, list](const ShapeOptions& o) {
        list->setCurrentRow(o.custom);
        button->setIcon(pickerIcon(o.custom, 48));
    });
    return button;
}

// The Line's arrowheads: Start / End and their Width, Length (both % of the
// weight), and Concavity, under the Line's geometry gear.
QToolButton* OptionsBar::buildArrowheadsButton(QWidget* page, const ShapeOptions& initial,
                                               const ShapeUpdate& update)
{
    auto* button = new QToolButton(page);
    button->setObjectName(QStringLiteral("optionsShapeArrowheads"));
    button->setIcon(icon(QStringLiteral("view.options")));
    button->setToolTip(QStringLiteral("Arrowheads"));
    button->setPopupMode(QToolButton::InstantPopup);
    auto* menu = new QMenu(button);
    auto* form = new QWidget(menu);
    auto* layout = new QFormLayout(form);
    layout->addRow(new QLabel(QStringLiteral("Arrowheads"), form));
    auto* start = new QCheckBox(QStringLiteral("Start"), form);
    start->setObjectName(QStringLiteral("optionsShapeArrowStart"));
    start->setChecked(initial.arrowStart);
    auto* end = new QCheckBox(QStringLiteral("End"), form);
    end->setObjectName(QStringLiteral("optionsShapeArrowEnd"));
    end->setChecked(initial.arrowEnd);
    layout->addRow(start);
    layout->addRow(end);
    connect(start, &QCheckBox::toggled, this,
            [update](bool on) { update([on](ShapeOptions& o) { o.arrowStart = on; }); });
    connect(end, &QCheckBox::toggled, this,
            [update](bool on) { update([on](ShapeOptions& o) { o.arrowEnd = on; }); });
    const auto field = [&](const QString& label, double lo, double hi, double value,
                           const QString& name, double ShapeOptions::*member) {
        auto* f = new NumericField(
            label, numericConfig(lo, hi, 1, 0, QStringLiteral("%"), true, name), form);
        f->setValue(value);
        layout->addRow(f);
        connect(f, &NumericField::valueChanged, this, [update, member](double v) {
            update([v, member](ShapeOptions& o) { o.*member = v; });
        });
    };
    field(QStringLiteral("Width:"), 10, 1000, initial.arrowWidth,
          QStringLiteral("optionsShapeArrowWidth"), &ShapeOptions::arrowWidth);
    field(QStringLiteral("Length:"), 10, 5000, initial.arrowLength,
          QStringLiteral("optionsShapeArrowLength"), &ShapeOptions::arrowLength);
    field(QStringLiteral("Concavity:"), -50, 50, initial.arrowConcavity,
          QStringLiteral("optionsShapeArrowConcavity"), &ShapeOptions::arrowConcavity);
    addForm(menu, form);
    button->setMenu(menu);
    return button;
}

} // namespace pictura
