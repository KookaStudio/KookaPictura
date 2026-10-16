#include "brush_preset_picker.h"

#include "../paint_tip.h"
#include "../tools.h"
#include "jump_slider.h"

#include "pictura_app/src/cxxqt_object/paint_tools.cxxqt.h"

#include <QtCore/QEvent>
#include <QtCore/QString>
#include <QtGui/QImage>
#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtWidgets/QApplication>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <iterator>

namespace pictura {

namespace brush_size_scale {

namespace {
constexpr int kHalf = kSteps / 2;
constexpr double kKnee = 100.0;
constexpr double kMax = 5000.0;
} // namespace

int sizeAt(int position)
{
    const int p = std::clamp(position, 0, kSteps);
    if (p <= kHalf) {
        return 1 + qRound((kKnee - 1.0) * p / kHalf);
    }
    const double u = double(p - kHalf) / kHalf;
    return qRound(kKnee * std::pow(kMax / kKnee, u));
}

int positionOf(int size)
{
    const double s = std::clamp(size, 1, int(kMax));
    if (s <= kKnee) {
        return qRound((s - 1.0) * kHalf / (kKnee - 1.0));
    }
    return kHalf + qRound(kHalf * std::log(s / kKnee) / std::log(kMax / kKnee));
}

} // namespace brush_size_scale

namespace {

constexpr int kCell = 32;

// One entry of the default set: size, hardness, roundness %, angle°, scatter %,
// count, size / angle° / roundness jitter, spacing %.
struct Preset {
    const char* name;
    int size;
    int hardness;
    int roundness;
    int angle;
    int scatter;
    int count;
    int sizeJitter;
    int angleJitter;
    int roundnessJitter;
    int spacing;
};

// CS6's default brush set in its order, one family per run. The numbers under
// CS6's thumbnails are the diameters; the sampled tips are approximated.
// Ported from photorust's shell/src/panels/BrushPresetPicker.cpp.
const Preset kPresets[] = {
    {"Soft Round 5", 5, 0, 100, 0, 0, 1, 0, 0, 0, 25},
    {"Soft Round 9", 9, 0, 100, 0, 0, 1, 0, 0, 0, 25},
    {"Soft Round 13", 13, 0, 100, 0, 0, 1, 0, 0, 0, 25},
    {"Soft Round 17", 17, 0, 100, 0, 0, 1, 0, 0, 0, 25},
    {"Soft Round 21", 21, 0, 100, 0, 0, 1, 0, 0, 0, 25},
    {"Soft Round 27", 27, 0, 100, 0, 0, 1, 0, 0, 0, 25},
    {"Hard Round 1", 1, 100, 100, 0, 0, 1, 0, 0, 0, 25},
    {"Hard Round 3", 3, 100, 100, 0, 0, 1, 0, 0, 0, 25},
    {"Hard Round 5", 5, 100, 100, 0, 0, 1, 0, 0, 0, 25},
    {"Hard Round 9", 9, 100, 100, 0, 0, 1, 0, 0, 0, 25},
    {"Hard Round 13", 13, 100, 100, 0, 0, 1, 0, 0, 0, 25},
    {"Hard Round 19", 19, 100, 100, 0, 0, 1, 0, 0, 0, 25},
    {"Flat 14", 14, 90, 20, 0, 0, 1, 0, 0, 0, 15},
    {"Flat Angled 25", 25, 90, 22, 45, 0, 1, 0, 0, 0, 15},
    {"Flat Angled 50", 50, 90, 20, 135, 0, 1, 0, 0, 0, 15},
    {"Chisel 25", 25, 85, 30, -45, 0, 1, 0, 0, 0, 15},
    {"Chisel 36", 36, 85, 25, 20, 0, 1, 0, 0, 0, 15},
    {"Chisel Hard 30", 30, 100, 18, -60, 0, 1, 0, 0, 0, 12},
    {"Charcoal 9", 9, 70, 60, 40, 25, 2, 40, 25, 25, 20},
    {"Chalk 23", 23, 60, 55, 20, 30, 2, 45, 30, 30, 22},
    {"Chalk 36", 36, 55, 50, -30, 35, 3, 50, 35, 30, 25},
    {"Charcoal 46", 46, 50, 45, 15, 40, 3, 55, 40, 35, 25},
    {"Chalk 59", 59, 45, 50, -160, 40, 3, 55, 45, 35, 28},
    {"Charcoal 60", 60, 40, 40, 100, 45, 4, 60, 50, 40, 30},
    {"Spatter 14", 14, 85, 100, 0, 140, 6, 60, 0, 0, 40},
    {"Spatter 24", 24, 80, 100, 0, 150, 7, 65, 0, 0, 45},
    {"Spatter 27", 27, 80, 85, 0, 160, 8, 70, 90, 20, 45},
    {"Spatter 39", 39, 75, 100, 0, 170, 9, 70, 0, 0, 50},
    {"Spatter 45", 45, 70, 90, 0, 180, 10, 75, 90, 25, 55},
    {"Spatter 59", 59, 70, 100, 0, 190, 11, 75, 0, 0, 60},
    {"Star 33", 33, 90, 22, 0, 60, 8, 45, 180, 30, 40},
    {"Star 74", 74, 85, 20, 0, 70, 10, 50, 180, 35, 45},
    {"Rough Round 42", 42, 60, 70, 0, 70, 5, 60, 90, 40, 35},
    {"Rough Round 55", 55, 55, 65, 0, 80, 6, 65, 90, 45, 38},
    {"Grass 63", 63, 85, 25, 90, 100, 8, 70, 45, 30, 45},
    {"Dune Grass 112", 112, 80, 22, 90, 120, 10, 75, 50, 35, 50},
    {"Dune Grass 134", 134, 80, 20, 90, 130, 11, 80, 55, 35, 55},
    {"Scattered Leaves 95", 95, 70, 55, 0, 110, 9, 70, 180, 40, 50},
    {"Soft Round 48", 48, 10, 100, 0, 0, 1, 0, 0, 0, 20},
    {"Soft Round 66", 66, 5, 100, 0, 0, 1, 0, 0, 0, 20},
    {"Soft Round 90", 90, 0, 100, 0, 0, 1, 0, 0, 0, 20},
    {"Hard Round 100", 100, 100, 100, 0, 0, 1, 0, 0, 0, 25},
    {"Soft Round 200", 200, 0, 100, 0, 0, 1, 0, 0, 0, 25},
    {"Soft Round 300", 300, 0, 100, 0, 0, 1, 0, 0, 0, 25},
};

constexpr int kPresetCount = int(std::size(kPresets));

PaintTip tipOf(const Preset& p)
{
    return PaintTip{p.size,    p.hardness, p.roundness,  p.angle,       p.spacing,
                    p.scatter, p.count,    p.sizeJitter, p.angleJitter, p.roundnessJitter};
}

} // namespace

QPixmap BrushPresetPicker::tipIcon(const PaintTip& tip, int edge)
{
    const ::rust::Vec<std::uint8_t> rgba = brush_dab_preview(tip, edge);
    if (rgba.size() != static_cast<size_t>(edge) * edge * 4) {
        return QPixmap();
    }
    const QImage image(rgba.data(), edge, edge, edge * 4, QImage::Format_RGBA8888);
    return QPixmap::fromImage(image.copy());
}

BrushPresetPicker::BrushPresetPicker(ToolController* controller, QWidget* parent)
    : QWidget(parent)
    , controller_(controller)
    , paint_(controller ? controller->optionsFor<PaintOptions>(ToolId::Brush) : nullptr)
{
    setObjectName(QStringLiteral("brushPresetPicker"));
    // An in-window overlay, not a Qt::Popup: a popup grabs the pointer, which
    // freezes the canvas brush ring and swallows the first press. The frame
    // styles this widget (QWidget#brushPresetPicker) as a floating panel.
    setAttribute(Qt::WA_StyledBackground, true);
    // Dismiss on an outside press or Escape while visible; installed for the
    // widget's lifetime, so `eventFilter` early-returns when hidden.
    qApp->installEventFilter(this);
    auto* outer = new QVBoxLayout(this);
    outer->setContentsMargins(8, 8, 8, 8);
    outer->setSpacing(8);

    auto* top = new QGridLayout;
    top->setHorizontalSpacing(10);
    top->setVerticalSpacing(4);
    preview_ = new QLabel(this);
    preview_->setObjectName(QStringLiteral("brushPickerPreview"));
    preview_->setFixedSize(64, 64);
    preview_->setAlignment(Qt::AlignCenter);
    top->addWidget(preview_, 0, 0, 4, 1);

    const auto addRow = [&](int row, const QString& label, const QString& name, int max,
                            const QString& suffix, QSpinBox** value, QSlider** slider) {
        top->addWidget(new QLabel(label, this), row, 1);
        *value = new QSpinBox(this);
        (*value)->setObjectName(name);
        (*value)->setRange(name == QStringLiteral("brushPickerSize") ? 1 : 0, max);
        (*value)->setSuffix(suffix);
        (*value)->setFixedWidth(80);
        top->addWidget(*value, row, 2);
        *slider = new JumpSlider(Qt::Horizontal, this);
        (*slider)->setObjectName(name + QStringLiteral("Slider"));
        top->addWidget(*slider, row + 1, 1, 1, 2);
    };
    addRow(0, tr("Size:"), QStringLiteral("brushPickerSize"), 5000, QStringLiteral(" px"), &size_,
           &sizeSlider_);
    addRow(2, tr("Hardness:"), QStringLiteral("brushPickerHardness"), 100, QStringLiteral("%"),
           &hardness_, &hardnessSlider_);
    sizeSlider_->setRange(0, brush_size_scale::kSteps);
    hardnessSlider_->setRange(0, 100);
    top->setColumnStretch(1, 1);
    outer->addLayout(top);

    current_ = new QLabel(this);
    current_->setObjectName(QStringLiteral("brushPickerCurrent"));
    outer->addWidget(current_);

    grid_ = new QListWidget(this);
    grid_->setObjectName(QStringLiteral("brushPickerGrid"));
    grid_->setViewMode(QListView::IconMode);
    grid_->setMovement(QListView::Static);
    grid_->setResizeMode(QListView::Adjust);
    grid_->setUniformItemSizes(true);
    grid_->setIconSize(QSize(kCell, kCell));
    grid_->setGridSize(QSize(kCell + 6, kCell + 6));
    grid_->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    grid_->setFixedHeight(7 * (kCell + 6) + 8);
    grid_->setMinimumWidth(7 * (kCell + 6) + 24);
    buildGrid();
    outer->addWidget(grid_);

    const auto setSize = [this](int size) {
        if (!updating_ && paint_) {
            paint_->setSize(size);
        }
    };
    const auto setHardness = [this](int hardness) {
        if (!updating_ && paint_) {
            paint_->setHardness(hardness);
        }
    };
    connect(size_, &QSpinBox::valueChanged, this, setSize);
    connect(sizeSlider_, &QSlider::valueChanged, this,
            [setSize](int p) { setSize(brush_size_scale::sizeAt(p)); });
    connect(hardness_, &QSpinBox::valueChanged, this, setHardness);
    connect(hardnessSlider_, &QSlider::valueChanged, this, setHardness);
    connect(grid_, &QListWidget::itemClicked, this,
            [this](QListWidgetItem* item) { applyPreset(item->data(Qt::UserRole).toInt()); });
    if (controller_) {
        connect(controller_, &ToolController::brushSizeChanged, this, [this](int) { sync(); });
        connect(controller_, &ToolController::brushTipChanged, this, [this] { sync(); });
    }
    sync();
}

int BrushPresetPicker::presetCount() const { return kPresetCount; }

void BrushPresetPicker::choosePresetForTest(int index) { applyPreset(index); }

void BrushPresetPicker::buildGrid()
{
    for (int i = 0; i < kPresetCount; ++i) {
        const Preset& preset = kPresets[i];
        QPixmap cell(kCell, kCell);
        cell.fill(Qt::transparent);
        {
            QPainter painter(&cell);
            painter.drawPixmap(4, 0, tipIcon(tipOf(preset), kCell - 8));
            QFont small = painter.font();
            small.setPixelSize(8);
            painter.setFont(small);
            painter.setPen(QColor(0xc0, 0xc0, 0xc0));
            painter.drawText(QRect(0, kCell - 10, kCell, 10), Qt::AlignHCenter | Qt::AlignBottom,
                             QString::number(preset.size));
        }
        auto* item = new QListWidgetItem(QIcon(cell), QString(), grid_);
        item->setToolTip(QString::fromLatin1(preset.name));
        item->setData(Qt::UserRole, i);
    }
}

void BrushPresetPicker::applyPreset(int index)
{
    if (!paint_ || index < 0 || index >= kPresetCount) {
        return;
    }
    const Preset& p = kPresets[index];
    paint_->setSize(p.size);
    paint_->setHardness(p.hardness);
    paint_->setRoundness(p.roundness);
    paint_->setTipAngle(p.angle);
    paint_->setSpacing(p.spacing);
    paint_->setFlip(false, false);
    paint_->setDynamics(
        BrushDynamics{p.scatter, p.count, p.sizeJitter, p.angleJitter, p.roundnessJitter});
    current_->setText(QString::fromLatin1(p.name));
    grid_->setCurrentRow(index);
}

void BrushPresetPicker::sync()
{
    if (!paint_) {
        return;
    }
    updating_ = true;
    const int size = paint_->size();
    size_->setValue(size);
    // Several positions share a size in the lower half; keep the one dragged to.
    if (brush_size_scale::sizeAt(sizeSlider_->value()) != size) {
        sizeSlider_->setValue(brush_size_scale::positionOf(size));
    }
    hardness_->setValue(paint_->hardness());
    hardnessSlider_->setValue(paint_->hardness());
    updating_ = false;

    // The tip at a size that fits the preview, crossed at its centre as CS6's is.
    const int edge = preview_->width();
    QPixmap pm(edge, edge);
    pm.fill(Qt::transparent);
    {
        QPainter painter(&pm);
        painter.drawPixmap(0, 0, tipIcon(paintTip(*paint_), edge));
        painter.setPen(QPen(QColor(0x80, 0x80, 0x80), 1));
        painter.drawLine(edge / 2, 2, edge / 2, edge - 2);
        painter.drawLine(2, edge / 2, edge - 2, edge / 2);
    }
    preview_->setPixmap(pm);
}

void BrushPresetPicker::popUpUnder(QWidget* anchor)
{
    sync();
    adjustSize();
    const QPoint where = anchor ? anchor->mapToGlobal(QPoint(0, anchor->height()))
                                : mapToGlobal(QPoint(0, 0));
    moveWithinParent(where);
    raise();
    show();
}

void BrushPresetPicker::popUpAt(const QPoint& globalPos)
{
    sync();
    adjustSize();
    // A small offset keeps the pointer off the overlay, so the canvas brush ring
    // stays visible at once.
    moveWithinParent(globalPos + QPoint(12, 12));
    raise();
    show();
}

void BrushPresetPicker::moveWithinParent(const QPoint& globalTopLeft)
{
    QWidget* host = parentWidget();
    if (!host) {
        move(globalTopLeft);
        return;
    }
    const QPoint local = host->mapFromGlobal(globalTopLeft);
    move(std::clamp(local.x(), 0, std::max(0, host->width() - width())),
         std::clamp(local.y(), 0, std::max(0, host->height() - height())));
}

bool BrushPresetPicker::eventFilter(QObject* watched, QEvent* event)
{
    if (!isVisible()) {
        return QWidget::eventFilter(watched, event);
    }
    switch (event->type()) {
    case QEvent::MouseButtonPress:
    case QEvent::NonClientAreaMouseButtonPress: {
        const auto* mouse = static_cast<QMouseEvent*>(event);
        const QRect frame(mapToGlobal(QPoint(0, 0)), size());
        if (!frame.contains(mouse->globalPosition().toPoint())) {
            // Hide without consuming: the press reaches the widget under the
            // pointer, so a press on the canvas starts the stroke.
            hide();
        }
        break;
    }
    case QEvent::KeyPress:
        if (static_cast<QKeyEvent*>(event)->key() == Qt::Key_Escape) {
            hide();
            return true;
        }
        break;
    default:
        break;
    }
    return QWidget::eventFilter(watched, event);
}

} // namespace pictura
