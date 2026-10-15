// Path Blur's half of the Blur Gallery workspace (a post-CS6 effect, ported
// on the owner's request): the Path Blur row, the Motion Effects tab, and
// the path records and motion section of the slot list. The controls follow
// the owner's CC screenshots; the ranges are unverified.

#include "blur_gallery_dialog.h"

#include <QtCore/QSignalBlocker>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSlider>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

// As `pictura_filters::MAX_PATH_SPEED`, `MAX_END_POINT_SPEED`,
// `MAX_PATH_POINTS`, and `MAX_FIELD_PINS`.
const double kMaxSpeed = 500.0;
const double kMaxEndSpeed = 500.0;
const int kMaxPoints = 32;
const int kMaxPaths = 16;
const int kCustomPreset = 2;

bool wholeIn(double value, int low, int high)
{
    return value >= low && value <= high && value == double(int(value));
}

} // namespace

void BlurGalleryDialog::buildPathControls(QWidget* box, QVBoxLayout* controls)
{
    pathPreset_ = new QComboBox(box);
    pathPreset_->setObjectName(QStringLiteral("pathBlurPreset"));
    pathPreset_->addItems({QStringLiteral("Basic Blur"), QStringLiteral("Rear Sync Flash")});
    controls->addWidget(pathPreset_);
    pathSpeed_ = addField(controls, box, QStringLiteral("Speed:"), QStringLiteral("pathSpeed"), 0,
                          kMaxSpeed, 50, 0, QStringLiteral("%"));
    pathTaper_ = addField(controls, box, QStringLiteral("Taper:"), QStringLiteral("pathTaper"), 0,
                          100, 0, 0, QStringLiteral("%"));
    pathCentered_ = new QCheckBox(QStringLiteral("Centered Blur"), box);
    pathCentered_->setObjectName(QStringLiteral("pathCentered"));
    pathCentered_->setChecked(true);
    controls->addWidget(pathCentered_);
    pathEndSpeed_ = addField(controls, box, QStringLiteral("End Point Speed:"),
                             QStringLiteral("pathEndSpeed"), 0, kMaxEndSpeed, 0, 0,
                             QStringLiteral(" px"));
    pathEndSpeed_.setEnabled(false);
    // ponytail: each end's blur shape follows the path's tangent; only its
    // length (End Point Speed) is editable, so the shape handles are not
    // drawn.
    pathEditShapes_ = new QCheckBox(QStringLiteral("Edit Blur Shapes"), box);
    pathEditShapes_->setObjectName(QStringLiteral("pathEditShapes"));
    pathEditShapes_->setEnabled(false);
    pathEditShapes_->setToolTip(QStringLiteral("Not implemented yet"));
    controls->addWidget(pathEditShapes_);

    connect(pathPreset_, &QComboBox::currentIndexChanged, this, [this](int index) {
        // The Motion Effects tab, which owns the strobe, is built after this
        // row.
        if (index < 0 || index == kCustomPreset || !strobeStrength_.spin) {
            return;
        }
        // A preset replaces the custom strobe.
        rearSync_ = index == 1;
        strobeStrength_.setValue(0.0);
        syncPreset();
        scheduleRender();
    });
    connect(pathCentered_, &QCheckBox::toggled, this, [this] { scheduleRender(); });
    connect(pathEndSpeed_.spin, &QDoubleSpinBox::valueChanged, this,
            &BlurGalleryDialog::setSelectedEndSpeed);
    connect(pathEndSpeed_.slider, &QSlider::valueChanged, this,
            [this](int value) { setSelectedEndSpeed(value); });
    for (QSlider* slider : {pathSpeed_.slider, pathTaper_.slider, pathEndSpeed_.slider}) {
        connect(slider, &QSlider::sliderReleased, this, &BlurGalleryDialog::scheduleRender);
    }
}

QWidget* BlurGalleryDialog::buildMotionTab()
{
    auto* tab = new QWidget(this);
    auto* layout = new QVBoxLayout(tab);
    layout->setContentsMargins(6, 6, 6, 6);
    layout->setSpacing(3);
    strobeStrength_ = addField(layout, tab, QStringLiteral("Strobe Strength:"),
                               QStringLiteral("strobeStrength"), 0, 100, 0, 0,
                               QStringLiteral("%"));
    strobeFlashes_ = addField(layout, tab, QStringLiteral("Strobe Flashes:"),
                              QStringLiteral("strobeFlashes"), 1, 100, 4, 0, QString());
    auto* hint = new QLabel(QStringLiteral("Motion Effects apply to Path Blur."), tab);
    hint->setWordWrap(true);
    hint->setEnabled(false);
    layout->addWidget(hint);
    layout->addStretch(1);
    connect(strobeStrength_.spin, &QDoubleSpinBox::valueChanged, this, [this] { syncPreset(); });
    connect(strobeStrength_.slider, &QSlider::valueChanged, this, [this] { syncPreset(); });
    for (QSlider* slider : {strobeStrength_.slider, strobeFlashes_.slider}) {
        connect(slider, &QSlider::sliderReleased, this, &BlurGalleryDialog::scheduleRender);
    }
    return tab;
}

void BlurGalleryDialog::syncPreset()
{
    if (!pathPreset_ || !strobeStrength_.spin) {
        return;
    }
    const QSignalBlocker block(pathPreset_);
    const bool custom = strobeStrength_.spin->value() > 0.0;
    if (custom && pathPreset_->count() == kCustomPreset) {
        pathPreset_->addItem(QStringLiteral("Custom"));
    }
    pathPreset_->setCurrentIndex(custom ? kCustomPreset : (rearSync_ ? 1 : 0));
    if (!custom && pathPreset_->count() > kCustomPreset) {
        pathPreset_->removeItem(kCustomPreset);
    }
}

void BlurGalleryDialog::syncPathControls()
{
    if (!pathEndSpeed_.spin) {
        return;
    }
    // End Point Speed edits the selected path's selected end point.
    const Effect& effect = effects_[int(BlurGalleryMode::Path)];
    const bool active = active_ == BlurGalleryMode::Path;
    const int index = active ? canvas_->selected() : effect.selected;
    const int point = active ? canvas_->selectedPoint() : -1;
    int end = -1;
    if (index >= 0 && index < effect.pins.size() && point >= 0) {
        const int last = int(effect.pins.at(index).points.size()) - 1;
        end = point == 0 ? 0 : (point == last ? 1 : -1);
    }
    pathEndSpeed_.setEnabled(end >= 0);
    if (end >= 0) {
        pathEndSpeed_.setValue(effect.pins.at(index).speeds[end]);
    }
}

void BlurGalleryDialog::setSelectedEndSpeed(double speed)
{
    if (active_ != BlurGalleryMode::Path) {
        return;
    }
    const int index = canvas_->selected();
    const int point = canvas_->selectedPoint();
    if (index < 0 || index >= pins().size() || point < 0) {
        return;
    }
    BlurGalleryPin& path = pins()[index];
    const int last = int(path.points.size()) - 1;
    if (point != 0 && point != last) {
        return;
    }
    path.speeds[point == 0 ? 0 : 1] = qBound(0.0, speed, kMaxEndSpeed);
    scheduleRender();
}

void BlurGalleryDialog::deleteSelectedPathPart()
{
    // The selected point while its path keeps two, else the selected path
    // while another remains.
    const int index = canvas_->selected();
    const int point = canvas_->selectedPoint();
    if (index < 0 || index >= pins().size()) {
        return;
    }
    QList<QPointF>& points = pins()[index].points;
    if (point >= 0 && point < points.size() && points.size() > 2) {
        points.removeAt(point);
    } else if (pins().size() > 1) {
        pins().removeAt(index);
        canvas_->setSelected(qMin(index, int(pins().size()) - 1));
    } else {
        return;
    }
    canvas_->setSelectedPoint(-1);
    syncControls();
    canvas_->update();
    scheduleRender();
}

void BlurGalleryDialog::appendPaths(QList<double>* out, double scale) const
{
    const Effect& effect = effects_[int(BlurGalleryMode::Path)];
    *out << (effect.enabled ? effect.pins.size() : 0);
    if (!effect.enabled) {
        return;
    }
    for (const BlurGalleryPin& path : effect.pins) {
        *out << path.points.size();
        for (const QPointF& point : path.points) {
            *out << point.x() << point.y();
        }
        *out << path.speeds[0] * scale << path.speeds[1] * scale;
    }
}

void BlurGalleryDialog::appendMotion(QList<double>* out) const
{
    *out << pathSpeed_.spin->value() << pathTaper_.spin->value()
         << (pathCentered_->isChecked() ? 1.0 : 0.0) << (rearSync_ ? 1.0 : 0.0)
         << strobeStrength_.spin->value() << strobeFlashes_.spin->value();
}

bool BlurGalleryDialog::readPaths(const QList<double>& values, int* at,
                                  QList<BlurGalleryPin>* paths) const
{
    if (*at >= values.size() || !wholeIn(values.at(*at), 0, kMaxPaths)) {
        return false;
    }
    const int count = int(values.at(*at));
    int cursor = *at + 1;
    for (int i = 0; i < count; ++i) {
        if (cursor >= values.size() || !wholeIn(values.at(cursor), 2, kMaxPoints)) {
            return false;
        }
        const int points = int(values.at(cursor));
        if (cursor + 1 + 2 * points + 2 > values.size()) {
            return false;
        }
        BlurGalleryPin path;
        for (int k = 0; k < points; ++k) {
            path.points << QPointF(values.at(cursor + 1 + 2 * k), values.at(cursor + 2 + 2 * k));
        }
        for (int end = 0; end < 2; ++end) {
            path.speeds[end] = qBound(0.0, values.at(cursor + 1 + 2 * points + end), kMaxEndSpeed);
        }
        paths->append(path);
        cursor += 1 + 2 * points + 2;
    }
    *at = cursor;
    return true;
}

void BlurGalleryDialog::readMotion(const QList<double>& values, int at)
{
    {
        const QSignalBlocker blockCentered(pathCentered_);
        pathSpeed_.setValue(qBound(0.0, values.at(at), kMaxSpeed));
        pathTaper_.setValue(qBound(0.0, values.at(at + 1), 100.0));
        pathCentered_->setChecked(values.at(at + 2) >= 0.5);
        rearSync_ = values.at(at + 3) >= 0.5;
        strobeStrength_.setValue(qBound(0.0, values.at(at + 4), 100.0));
        strobeFlashes_.setValue(qBound(1.0, values.at(at + 5), 100.0));
    }
    syncPreset();
}

} // namespace pictura
