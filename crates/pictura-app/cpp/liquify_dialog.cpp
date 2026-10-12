#include "liquify_dialog.h"
#include "dialogs.h"
#include "icons.h"
#include "liquify_canvas.h"
#include "panels/jump_slider.h"

#include <QtCore/QSignalBlocker>
#include <QtGui/QGuiApplication>
#include <QtGui/QKeySequence>
#include <QtGui/QPainter>
#include <QtGui/QPainterPath>
#include <QtGui/QScreen>
#include <QtGui/QShortcut>
#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFrame>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <cmath>

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools/liquify.cxxqt.h"

namespace pictura {

namespace {

// The Size slider stops here; the field takes CS6's full 15,000.
constexpr int kSizeSliderMaximum = 1500;
constexpr int kMaxBrush = 15000;

struct ToolInfo {
    LiquifyDialog::Tool tool;
    const char* name;
    Qt::Key key;
    bool advancedOnly;
};

// CS6's column order and shortcuts. Mirror takes X so M can load the last
// mesh, the spec's reading of CS6's conflicting key tables.
const ToolInfo kTools[] = {
    {LiquifyDialog::ForwardWarp, "Forward Warp Tool", Qt::Key_W, false},
    {LiquifyDialog::Reconstruct, "Reconstruct Tool", Qt::Key_R, false},
    {LiquifyDialog::Twirl, "Twirl Clockwise Tool", Qt::Key_C, true},
    {LiquifyDialog::Pucker, "Pucker Tool", Qt::Key_S, false},
    {LiquifyDialog::Bloat, "Bloat Tool", Qt::Key_B, false},
    {LiquifyDialog::PushLeft, "Push Left Tool", Qt::Key_O, false},
    {LiquifyDialog::Mirror, "Mirror Tool", Qt::Key_X, true},
    {LiquifyDialog::Turbulence, "Turbulence Tool", Qt::Key_T, true},
    {LiquifyDialog::Freeze, "Freeze Mask Tool", Qt::Key_F, true},
    {LiquifyDialog::Thaw, "Thaw Mask Tool", Qt::Key_D, true},
    {LiquifyDialog::Hand, "Hand Tool", Qt::Key_H, false},
    {LiquifyDialog::Zoom, "Zoom Tool", Qt::Key_Z, false},
};

const struct {
    const char* name;
    QColor color;
} kMaskColors[] = {
    {"Red", QColor(255, 0, 0)},     {"Yellow", QColor(255, 255, 0)},
    {"Green", QColor(0, 255, 0)},   {"Cyan", QColor(0, 255, 255)},
    {"Blue", QColor(0, 0, 255)},    {"Magenta", QColor(255, 0, 255)},
    {"Gray", QColor(128, 128, 128)},
};

bool actsWhileHeld(int tool)
{
    return tool != LiquifyDialog::ForwardWarp && tool != LiquifyDialog::PushLeft &&
           tool != LiquifyDialog::Mirror && tool < LiquifyDialog::Hand;
}

void arrow(QPainter& p, QPointF from, QPointF to)
{
    p.drawLine(from, to);
    const QPointF d = to - from;
    const double len = std::hypot(d.x(), d.y());
    if (len <= 0.0) {
        return;
    }
    const QPointF u = d / len;
    const QPointF n(-u.y(), u.x());
    p.drawLine(to, to - u * 3.5 + n * 2.5);
    p.drawLine(to, to - u * 3.5 - n * 2.5);
}

// The distortion tools have no glyph in the icon set, so they are drawn in
// `ink`: each shows the motion it gives the pixels.
QIcon toolIcon(int tool, const QColor& ink)
{
    if (tool == LiquifyDialog::Hand) {
        return icon(QStringLiteral("tool.hand"));
    }
    if (tool == LiquifyDialog::Zoom) {
        return icon(QStringLiteral("tool.zoom"));
    }
    QPixmap pixmap(40, 40);
    pixmap.fill(Qt::transparent);
    QPainter p(&pixmap);
    p.setRenderHint(QPainter::Antialiasing, true);
    p.scale(2.0, 2.0);
    p.setPen(QPen(ink, 1.5, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    const QPointF c(10, 10);
    switch (tool) {
    case LiquifyDialog::ForwardWarp: {
        QPainterPath path(QPointF(3, 16));
        path.cubicTo(QPointF(8, 16), QPointF(7, 6), QPointF(15, 5));
        p.drawPath(path);
        arrow(p, QPointF(12, 5.5), QPointF(16.5, 5));
        break;
    }
    case LiquifyDialog::Reconstruct:
        p.drawArc(QRectF(4, 4, 12, 12), 120 * 16, 280 * 16);
        arrow(p, QPointF(6.5, 9), QPointF(7, 5.2));
        break;
    case LiquifyDialog::Twirl: {
        QPainterPath path(c);
        for (int i = 1; i <= 40; ++i) {
            const double t = i / 40.0 * 2.6 * M_PI;
            const double r = 1.0 + t * 0.9;
            path.lineTo(c + QPointF(std::cos(t) * r, std::sin(t) * r));
        }
        p.drawPath(path);
        break;
    }
    case LiquifyDialog::Pucker:
    case LiquifyDialog::Bloat: {
        p.drawEllipse(c, 2.5, 2.5);
        const bool in = tool == LiquifyDialog::Pucker;
        for (const QPointF d : {QPointF(1, 0), QPointF(-1, 0), QPointF(0, 1), QPointF(0, -1)}) {
            in ? arrow(p, c + d * 9, c + d * 4.5) : arrow(p, c + d * 4.5, c + d * 9);
        }
        break;
    }
    case LiquifyDialog::PushLeft:
        p.drawLine(QPointF(15, 3), QPointF(15, 17));
        arrow(p, QPointF(12, 10), QPointF(3, 10));
        break;
    case LiquifyDialog::Mirror: {
        p.setPen(QPen(ink, 1.0, Qt::DashLine));
        p.drawLine(QPointF(10, 2), QPointF(10, 18));
        p.setPen(QPen(ink, 1.3));
        p.setBrush(ink);
        p.drawPolygon(QPolygonF({QPointF(8, 5), QPointF(8, 15), QPointF(3, 15)}));
        p.setBrush(Qt::NoBrush);
        p.drawPolygon(QPolygonF({QPointF(12, 5), QPointF(12, 15), QPointF(17, 15)}));
        break;
    }
    case LiquifyDialog::Turbulence:
        for (double y : {6.0, 10.0, 14.0}) {
            QPainterPath wave(QPointF(2, y));
            wave.cubicTo(QPointF(6, y - 4), QPointF(8, y + 4), QPointF(11, y));
            wave.cubicTo(QPointF(14, y - 4), QPointF(16, y + 4), QPointF(18, y));
            p.drawPath(wave);
        }
        break;
    case LiquifyDialog::Freeze:
        p.setBrush(QColor(0xd0, 0x30, 0x30));
        p.drawEllipse(c, 6.5, 6.5);
        break;
    case LiquifyDialog::Thaw:
        p.drawEllipse(c, 6.5, 6.5);
        p.drawLine(QPointF(5.5, 14.5), QPointF(14.5, 5.5));
        break;
    }
    p.end();
    pixmap.setDevicePixelRatio(2.0);
    return QIcon(pixmap);
}

} // namespace

LiquifyDialog::LiquifyDialog(PictureView* view, const QString& documentName, QWidget* parent)
    : QDialog(parent), view_(view), documentName_(documentName)
{
    setObjectName(QStringLiteral("liquifyDialog"));
    ready_ = view_ && liquify_begin(*view_);

    // The tool column.
    tools_ = new QButtonGroup(this);
    tools_->setExclusive(true);
    auto* column = new QVBoxLayout;
    column->setSpacing(2);
    for (const ToolInfo& info : kTools) {
        if (info.tool == Hand || info.tool == Twirl) {
            column->addSpacing(6); // CS6 rules the column after Reconstruct and Thaw
        }
        auto* button = new QToolButton(this);
        button->setObjectName(QStringLiteral("liquifyTool%1").arg(int(info.tool)));
        button->setCheckable(true);
        button->setAutoRaise(true);
        button->setIcon(toolIcon(info.tool, palette().color(QPalette::WindowText)));
        button->setIconSize(QSize(20, 20));
        button->setToolTip(QStringLiteral("%1 (%2)")
                               .arg(QString::fromLatin1(info.name),
                                    QKeySequence(info.key).toString()));
        tools_->addButton(button, info.tool);
        column->addWidget(button);
        if (info.advancedOnly) {
            advancedOnly_.append(button);
        }
        if (info.tool == Zoom) {
            // CS6 offers its zoom levels on a right click of the Zoom tool.
            button->setContextMenuPolicy(Qt::CustomContextMenu);
            connect(button, &QToolButton::customContextMenuRequested, this,
                    [this, button](const QPoint& at) {
                        canvas_->showZoomMenu(button->mapToGlobal(at));
                    });
        }
        auto* shortcut = new QShortcut(QKeySequence(info.key), this);
        connect(shortcut, &QShortcut::activated, this, [this, tool = info.tool] {
            if (QAbstractButton* b = tools_->button(tool); b && !b->isHidden()) {
                selectTool(tool);
            }
        });
    }
    column->addStretch(1);
    connect(tools_, &QButtonGroup::idToggled, this, [this](int, bool on) {
        if (on) {
            toolChanged();
        }
    });

    // The preview and its zoom row.
    canvas_ = new LiquifyCanvas(this);
    auto* zoomOut = new QToolButton(this);
    zoomOut->setObjectName(QStringLiteral("liquifyZoomOut"));
    zoomOut->setText(QStringLiteral("−"));
    auto* zoomIn = new QToolButton(this);
    zoomIn->setObjectName(QStringLiteral("liquifyZoomIn"));
    zoomIn->setText(QStringLiteral("+"));
    zoomLabel_ = new QLabel(this);
    zoomLabel_->setObjectName(QStringLiteral("liquifyZoom"));
    zoomLabel_->setMinimumWidth(56);
    zoomLabel_->setAlignment(Qt::AlignCenter);
    auto* fit = new QToolButton(this);
    fit->setText(QStringLiteral("Fit in View"));
    connect(zoomOut, &QToolButton::clicked, this, [this] { canvas_->zoomBy(0.5); });
    connect(zoomIn, &QToolButton::clicked, this, [this] { canvas_->zoomBy(2.0); });
    connect(fit, &QToolButton::clicked, canvas_, &LiquifyCanvas::fitInView);
    addZoomShortcuts(this, zoomIn, zoomOut);
    auto* zoomRow = new QHBoxLayout;
    zoomRow->addWidget(zoomOut);
    zoomRow->addWidget(zoomLabel_);
    zoomRow->addWidget(zoomIn);
    zoomRow->addWidget(fit);
    zoomRow->addStretch(1);
    auto* centre = new QVBoxLayout;
    centre->addWidget(canvas_, 1);
    centre->addLayout(zoomRow);
    canvas_->zoomChanged = [this] { updateZoomLabel(); };
    canvas_->beforeStroke = [this](bool reverse) { pushBrush(reverse); };
    canvas_->tabletSeen = [this] { stylus_->setEnabled(true); };

    // The options column.
    auto* ok = new QPushButton(QStringLiteral("OK"), this);
    ok->setDefault(true);
    auto* cancel = new QPushButton(QStringLiteral("Cancel"), this);
    connect(ok, &QPushButton::clicked, this, &QDialog::accept);
    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);
    loadLast_ = new QPushButton(QStringLiteral("Load Last Mesh"), this);
    loadLast_->setObjectName(QStringLiteral("liquifyLoadLastMesh"));
    loadLast_->setEnabled(ready_ && liquify_last_mesh_id() != 0);
    connect(loadLast_, &QPushButton::clicked, this, [this] { loadMesh(liquify_last_mesh_id()); });
    auto* loadLastKey = new QShortcut(QKeySequence(Qt::Key_M), this);
    connect(loadLastKey, &QShortcut::activated, loadLast_, &QPushButton::click);
    advanced_ = new QCheckBox(QStringLiteral("Advanced Mode"), this);
    advanced_->setObjectName(QStringLiteral("liquifyAdvanced"));

    auto* toolBox = new QGroupBox(QStringLiteral("Brush Tool Options"), this);
    toolBox->setObjectName(QStringLiteral("liquifyToolOptions"));
    new QVBoxLayout(toolBox);
    size_ = addSliderRow(toolBox, QStringLiteral("Size:"), QStringLiteral("liquifySize"), 1,
                         kMaxBrush, 100, kSizeSliderMaximum);
    density_ = addSliderRow(toolBox, QStringLiteral("Density:"), QStringLiteral("liquifyDensity"),
                            0, 100, 50);
    pressure_ = addSliderRow(toolBox, QStringLiteral("Pressure:"),
                             QStringLiteral("liquifyPressure"), 0, 100, 100);
    rate_ = addSliderRow(toolBox, QStringLiteral("Rate:"), QStringLiteral("liquifyRate"), 0, 100,
                         80);
    jitter_ = addSliderRow(toolBox, QStringLiteral("Turbulent Jitter:"),
                           QStringLiteral("liquifyJitter"), 0, 100, 50);
    stylus_ = new QCheckBox(QStringLiteral("Stylus Pressure"), toolBox);
    stylus_->setEnabled(false);
    toolBox->layout()->addWidget(stylus_);
    connect(stylus_, &QCheckBox::toggled, canvas_, &LiquifyCanvas::setStylusPressure);
    connect(size_, &QSpinBox::valueChanged, canvas_, &LiquifyCanvas::setBrushSize);
    auto* smaller = new QShortcut(QKeySequence(Qt::Key_BracketLeft), this);
    connect(smaller, &QShortcut::activated, this, [this] { resizeBrush(false); });
    auto* larger = new QShortcut(QKeySequence(Qt::Key_BracketRight), this);
    connect(larger, &QShortcut::activated, this, [this] { resizeBrush(true); });

    auto* reconstructBox = new QGroupBox(QStringLiteral("Reconstruct Options"), this);
    auto* reconstructRow = new QHBoxLayout(reconstructBox);
    reconstruct_ = new QPushButton(QStringLiteral("Reconstruct..."), reconstructBox);
    reconstruct_->setObjectName(QStringLiteral("liquifyReconstruct"));
    auto* restoreAll = new QPushButton(QStringLiteral("Restore All"), reconstructBox);
    restoreAll->setObjectName(QStringLiteral("liquifyRestoreAll"));
    reconstructRow->addWidget(reconstruct_);
    reconstructRow->addWidget(restoreAll);
    connect(reconstruct_, &QPushButton::clicked, this, &LiquifyDialog::reconstructWithAmount);
    connect(restoreAll, &QPushButton::clicked, this, [this] {
        liquify_restore_all();
        canvas_->refreshAll();
    });

    maskBox_ = new QGroupBox(QStringLiteral("Mask Options"), this);
    auto* maskRow = new QHBoxLayout(maskBox_);
    const struct {
        const char* label;
        const char* name;
        int op;
    } maskOps[] = {{"None", "liquifyMaskNone", 0},
                   {"Mask All", "liquifyMaskAll", 1},
                   {"Invert All", "liquifyMaskInvert", 2}};
    for (const auto& m : maskOps) {
        auto* button = new QPushButton(QString::fromLatin1(m.label), maskBox_);
        button->setObjectName(QString::fromLatin1(m.name));
        maskRow->addWidget(button);
        connect(button, &QPushButton::clicked, this, [this, op = m.op] {
            liquify_mask(op);
            canvas_->refreshAll();
        });
    }

    viewBox_ = new QGroupBox(QStringLiteral("View Options"), this);
    // One grid, so every label in the group lines up.
    auto* grid = new QGridLayout(viewBox_);
    grid->setColumnStretch(1, 1);
    auto colorCombo = [this](const QString& name, int initial) {
        auto* combo = new QComboBox(viewBox_);
        combo->setObjectName(name);
        for (const auto& m : kMaskColors) {
            combo->addItem(QString::fromLatin1(m.name), m.color);
        }
        combo->setCurrentIndex(initial);
        return combo;
    };
    auto span = [grid](QWidget* widget) { grid->addWidget(widget, grid->rowCount(), 0, 1, 2); };
    auto spanLayout = [grid](QLayout* layout) {
        grid->addLayout(layout, grid->rowCount(), 0, 1, 2);
    };
    auto rule = [this, span] {
        auto* line = new QFrame(viewBox_);
        line->setFrameShape(QFrame::HLine);
        line->setFrameShadow(QFrame::Sunken);
        span(line);
    };
    auto formRow = [this, grid](const QString& label, QWidget* control) {
        auto* caption = new QLabel(label, viewBox_);
        caption->setBuddy(control);
        const int row = grid->rowCount();
        grid->addWidget(caption, row, 0, Qt::AlignRight);
        grid->addWidget(control, row, 1);
        return caption;
    };

    showImage_ = new QCheckBox(QStringLiteral("Show Image"), viewBox_);
    showImage_->setObjectName(QStringLiteral("liquifyShowImage"));
    showImage_->setChecked(true);
    showMesh_ = new QCheckBox(QStringLiteral("Show Mesh"), viewBox_);
    showMesh_->setObjectName(QStringLiteral("liquifyShowMesh"));
    auto* showRow = new QHBoxLayout;
    showRow->addWidget(showImage_);
    showRow->addWidget(showMesh_);
    showRow->addStretch(1);
    spanLayout(showRow);
    meshSize_ = new QComboBox(viewBox_);
    meshSize_->setObjectName(QStringLiteral("liquifyMeshSize"));
    meshSize_->addItems({QStringLiteral("Small"), QStringLiteral("Medium"), QStringLiteral("Large")});
    meshSize_->setCurrentIndex(1);
    meshColor_ = colorCombo(QStringLiteral("liquifyMeshColor"), 6); // Gray
    meshControls_ << formRow(QStringLiteral("Mesh Size:"), meshSize_) << meshSize_
                  << formRow(QStringLiteral("Mesh Color:"), meshColor_) << meshColor_;
    connect(showImage_, &QCheckBox::toggled, canvas_, &LiquifyCanvas::setShowImage);
    connect(showMesh_, &QCheckBox::toggled, this, &LiquifyDialog::meshChanged);
    connect(meshSize_, &QComboBox::currentIndexChanged, this, &LiquifyDialog::meshChanged);
    connect(meshColor_, &QComboBox::currentIndexChanged, this, &LiquifyDialog::meshChanged);

    rule();
    showMask_ = new QCheckBox(QStringLiteral("Show Mask"), viewBox_);
    showMask_->setObjectName(QStringLiteral("liquifyShowMask"));
    span(showMask_);
    maskColor_ = colorCombo(QStringLiteral("liquifyMaskColor"), 0); // Red
    formRow(QStringLiteral("Mask Color:"), maskColor_);
    connect(showMask_, &QCheckBox::toggled, this, &LiquifyDialog::showMaskChanged);
    connect(maskColor_, &QComboBox::currentIndexChanged, this, &LiquifyDialog::showMaskChanged);

    rule();
    showBackdrop_ = new QCheckBox(QStringLiteral("Show Backdrop"), viewBox_);
    showBackdrop_->setObjectName(QStringLiteral("liquifyShowBackdrop"));
    span(showBackdrop_);
    backdropUse_ = new QComboBox(viewBox_);
    backdropUse_->setObjectName(QStringLiteral("liquifyBackdropUse"));
    backdropUse_->addItem(QStringLiteral("All Layers"), QString());
    if (view_) {
        for (int i = 0; i < view_->layer_row_count(); ++i) {
            const QString kind = view_->layer_row_kind(i);
            if (kind == QStringLiteral("pixel") || kind == QStringLiteral("background")) {
                backdropUse_->addItem(view_->layer_row_name(i), view_->layer_row_path(i));
            }
        }
    }
    backdropMode_ = new QComboBox(viewBox_);
    backdropMode_->setObjectName(QStringLiteral("liquifyBackdropMode"));
    backdropMode_->addItems(
        {QStringLiteral("In Front"), QStringLiteral("Behind"), QStringLiteral("Blend")});
    backdropControls_ << formRow(QStringLiteral("Use:"), backdropUse_) << backdropUse_
                      << formRow(QStringLiteral("Mode:"), backdropMode_) << backdropMode_;
    // addSliderRow appends to a vertical layout, so the row gets its own.
    auto* opacityHost = new QWidget(viewBox_);
    auto* opacityLayout = new QVBoxLayout(opacityHost);
    opacityLayout->setContentsMargins(0, 0, 0, 0);
    backdropOpacity_ = addSliderRow(opacityHost, QStringLiteral("Opacity:"),
                                    QStringLiteral("liquifyBackdropOpacity"), 0, 100, 50);
    span(opacityHost);
    backdropControls_ << opacityHost;
    connect(showBackdrop_, &QCheckBox::toggled, this, [this] { backdropChanged(true); });
    connect(backdropUse_, &QComboBox::currentIndexChanged, this, [this] { backdropChanged(true); });
    connect(backdropMode_, &QComboBox::currentIndexChanged, this,
            [this] { backdropChanged(false); });
    connect(backdropOpacity_, &QSpinBox::valueChanged, this, [this] { backdropChanged(false); });
    meshChanged();
    backdropChanged(false);

    advancedOnly_ << density_->parentWidget() << rate_->parentWidget()
                  << jitter_->parentWidget() << reconstruct_ << maskBox_ << viewBox_;

    auto* side = new QVBoxLayout;
    side->addWidget(ok);
    side->addWidget(cancel);
    side->addSpacing(10);
    side->addWidget(loadLast_);
    side->addWidget(advanced_, 0, Qt::AlignHCenter);
    side->addWidget(toolBox);
    side->addWidget(reconstructBox);
    side->addWidget(maskBox_);
    side->addWidget(viewBox_);
    side->addStretch(1);
    auto* sideWidget = new QWidget(this);
    sideWidget->setLayout(side);
    sideWidget->setFixedWidth(300);

    auto* outer = new QHBoxLayout(this);
    outer->addLayout(column);
    outer->addLayout(centre, 1);
    outer->addWidget(sideWidget);

    connect(advanced_, &QCheckBox::toggled, this, &LiquifyDialog::setAdvanced);
    setAdvanced(false);
    selectTool(ForwardWarp);
    canvas_->setBrushSize(size_->value());

    if (const QScreen* screen = parent ? parent->screen() : QGuiApplication::primaryScreen()) {
        resize(screen->availableGeometry().size() * 0.85);
    }
    canvas_->reload();
    updateZoomLabel();
}

LiquifyDialog::~LiquifyDialog()
{
    finish(false);
}

QSpinBox* LiquifyDialog::addSliderRow(QWidget* box, const QString& label, const QString& name,
                                      int minimum, int maximum, int value, int sliderMaximum)
{
    // Each row is its own widget so Basic mode can hide it whole.
    auto* row = new QWidget(box);
    auto* layout = new QVBoxLayout(row);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(1);
    auto* spin = new QSpinBox(row);
    spin->setObjectName(name);
    spin->setRange(minimum, maximum);
    spin->setValue(value);
    auto* slider = new JumpSlider(Qt::Horizontal, row);
    slider->setRange(minimum, sliderMaximum > 0 ? sliderMaximum : maximum);
    slider->setValue(value);
    auto* caption = new QLabel(label, row);
    caption->setBuddy(spin);
    auto* top = new QHBoxLayout;
    top->addWidget(caption);
    top->addStretch(1);
    top->addWidget(spin);
    layout->addLayout(top);
    layout->addWidget(slider);
    connect(slider, &QSlider::valueChanged, spin, &QSpinBox::setValue);
    connect(spin, &QSpinBox::valueChanged, slider, [slider](int v) {
        QSignalBlocker block(slider);
        slider->setValue(v);
    });
    box->layout()->addWidget(row);
    return spin;
}

bool LiquifyDialog::loadMesh(int id)
{
    if (!ready_ || !liquify_load_mesh(id)) {
        return false;
    }
    canvas_->refreshAll();
    return true;
}

bool LiquifyDialog::advanced() const
{
    return advanced_->isChecked();
}

void LiquifyDialog::setAdvanced(bool on)
{
    if (advanced_->isChecked() != on) {
        QSignalBlocker block(advanced_);
        advanced_->setChecked(on);
    }
    for (QWidget* widget : advancedOnly_) {
        widget->setVisible(on);
    }
    if (auto* box = findChild<QGroupBox*>(QStringLiteral("liquifyToolOptions"))) {
        box->setTitle(on ? QStringLiteral("Tool Options") : QStringLiteral("Brush Tool Options"));
    }
    if (!on) {
        // Basic mode has no View Options, so nothing they add stays on.
        showMask_->setChecked(false);
        showMesh_->setChecked(false);
        showBackdrop_->setChecked(false);
        showImage_->setChecked(true);
    }
    if (QAbstractButton* current = tools_->checkedButton(); current && current->isHidden()) {
        selectTool(ForwardWarp);
    }
}

int LiquifyDialog::tool() const
{
    return tools_->checkedId();
}

void LiquifyDialog::selectTool(int tool)
{
    if (QAbstractButton* button = tools_->button(tool)) {
        button->setChecked(true);
    }
}

void LiquifyDialog::pushBrush(bool reverse)
{
    const int current = tool();
    if (current < 0 || current >= Hand) {
        return;
    }
    liquify_set_brush(current, size_->value(), density_->value(), pressure_->value(),
                      rate_->value(), jitter_->value(), reverse);
}

void LiquifyDialog::toolChanged()
{
    const int current = tool();
    const LiquifyCanvas::Mode mode = current == Hand   ? LiquifyCanvas::Mode::Hand
                                     : current == Zoom ? LiquifyCanvas::Mode::Zoom
                                                       : LiquifyCanvas::Mode::Brush;
    canvas_->setMode(mode, actsWhileHeld(current));
    pushBrush(false);
}

void LiquifyDialog::resizeBrush(bool larger)
{
    const int size = size_->value();
    const int step = std::max(1, static_cast<int>(std::lround(size * 0.1)));
    size_->setValue(larger ? size + step : size - step);
}

void LiquifyDialog::showMaskChanged()
{
    const QColor color = maskColor_->currentData().value<QColor>();
    liquify_set_mask_overlay(showMask_->isChecked(), color.red(), color.green(), color.blue());
    canvas_->refreshAll();
}

void LiquifyDialog::meshChanged()
{
    // Small, Medium and Large cells, in layer pixels.
    constexpr double kSpacing[] = {16.0, 32.0, 64.0};
    const bool on = showMesh_->isChecked();
    for (QWidget* control : meshControls_) {
        control->setEnabled(on);
    }
    canvas_->setMesh(on, kSpacing[std::clamp(meshSize_->currentIndex(), 0, 2)],
                     meshColor_->currentData().value<QColor>());
}

void LiquifyDialog::backdropChanged(bool refetch)
{
    const bool on = showBackdrop_->isChecked();
    for (QWidget* control : backdropControls_) {
        control->setEnabled(on);
    }
    if (!on) {
        canvas_->setBackdrop(QImage(), LiquifyCanvas::BackdropMode::InFront, 0.0);
        return;
    }
    if (refetch || backdrop_.isNull()) {
        backdrop_ = view_ ? liquify_backdrop(*view_, backdropUse_->currentData().toString())
                          : QImage();
    }
    const auto mode = static_cast<LiquifyCanvas::BackdropMode>(backdropMode_->currentIndex());
    canvas_->setBackdrop(backdrop_, mode, backdropOpacity_->value() / 100.0);
}

void LiquifyDialog::reconstructWithAmount()
{
    QDialog prompt(this);
    prompt.setWindowTitle(QStringLiteral("Revert Reconstruction"));
    auto* amount = new QSpinBox(&prompt);
    amount->setObjectName(QStringLiteral("liquifyReconstructAmount"));
    amount->setRange(0, 100);
    amount->setValue(100);
    auto* slider = new JumpSlider(Qt::Horizontal, &prompt);
    slider->setRange(0, 100);
    slider->setValue(100);
    connect(slider, &QSlider::valueChanged, amount, &QSpinBox::setValue);
    connect(amount, &QSpinBox::valueChanged, slider, &QSlider::setValue);
    auto* ok = new QPushButton(QStringLiteral("OK"), &prompt);
    ok->setDefault(true);
    auto* cancel = new QPushButton(QStringLiteral("Cancel"), &prompt);
    connect(ok, &QPushButton::clicked, &prompt, &QDialog::accept);
    connect(cancel, &QPushButton::clicked, &prompt, &QDialog::reject);
    auto* row = new QHBoxLayout;
    row->addWidget(new QLabel(QStringLiteral("Amount:"), &prompt));
    row->addWidget(amount);
    auto* buttons = new QVBoxLayout;
    buttons->addWidget(ok);
    buttons->addWidget(cancel);
    auto* left = new QVBoxLayout;
    left->addLayout(row);
    left->addWidget(slider);
    auto* layout = new QHBoxLayout(&prompt);
    layout->addLayout(left);
    layout->addLayout(buttons);
    if (runDialog(prompt, this) == QDialog::Accepted) {
        liquify_reconstruct(amount->value());
        canvas_->refreshAll();
    }
}

void LiquifyDialog::updateZoomLabel()
{
    const double percent = canvas_->zoom() * 100.0;
    const QString text = percent >= 10.0 ? QString::number(std::lround(percent))
                                         : QString::number(percent, 'f', 2);
    zoomLabel_->setText(text + QLatin1Char('%'));
    setWindowTitle(QStringLiteral("Liquify (%1 @ %2%)").arg(documentName_, text));
}

void LiquifyDialog::finish(bool keep)
{
    if (ready_ && !finished_) {
        meshId_ = liquify_finish(keep);
        finished_ = true;
    }
}

void LiquifyDialog::accept()
{
    finish(true);
    QDialog::accept();
}

void LiquifyDialog::reject()
{
    finish(false);
    QDialog::reject();
}

bool LiquifyDialog::get(PictureView* view, const QString& documentName,
                        const QList<double>& initial, QList<double>* out, QWidget* parent)
{
    LiquifyDialog dialog(view, documentName, parent);
    if (!dialog.ready()) {
        return false;
    }
    if (!initial.isEmpty() && initial.first() > 0.0) {
        dialog.loadMesh(static_cast<int>(initial.first()));
    }
    if (runDialog(dialog, parent) != QDialog::Accepted || dialog.meshId() == 0) {
        return false;
    }
    *out = {static_cast<double>(dialog.meshId())};
    return true;
}

} // namespace pictura
