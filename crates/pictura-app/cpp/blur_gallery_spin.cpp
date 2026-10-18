// Spin Blur's half of the Blur Gallery workspace (a post-CS6 effect, ported
// on the owner's request): the spin records and strobe slots of the slot
// list, and the Motion Effects tab's fit to the selected effect. The
// controls follow the owner's CC screenshots; the ranges are unverified.

#include "blur_gallery_dialog.h"

#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSlider>
#include <QtWidgets/QTabWidget>

namespace pictura {

namespace {

// As `pictura_filters::MAX_SPIN_ANGLE` and `MAX_FIELD_PINS`; the radii as
// Iris's.
const double kMaxAngle = 360.0;
const int kMaxSpins = 16;
const double kMinRadius = 0.0001;
const double kMaxRadius = 10.0;
const int kSpinSlots = 9;

} // namespace

void BlurGalleryDialog::appendSpins(QList<double>* out) const
{
    const Effect& effect = effects_[int(BlurGalleryMode::Spin)];
    *out << (effect.enabled ? effect.pins.size() : 0);
    if (!effect.enabled) {
        return;
    }
    // The Blur Angle is the same at any resolution, so it is never scaled.
    for (const BlurGalleryPin& spin : effect.pins) {
        *out << spin.pos.x() << spin.pos.y() << spin.radii.width() << spin.radii.height()
             << spin.angle << spin.feather << spin.blur << spin.pivot.x() << spin.pivot.y();
    }
}

bool BlurGalleryDialog::readSpins(const QList<double>& values, int* at,
                                  QList<BlurGalleryPin>* spins) const
{
    if (*at >= values.size()) {
        return false;
    }
    const double count = values.at(*at);
    if (count < 0 || count > kMaxSpins || count != double(int(count))
        || *at + 1 + kSpinSlots * int(count) > values.size()) {
        return false;
    }
    for (int i = 0; i < int(count); ++i) {
        const auto slot = [&](int k) { return values.at(*at + 1 + kSpinSlots * i + k); };
        BlurGalleryPin spin;
        spin.pos = QPointF(slot(0), slot(1));
        spin.radii = QSizeF(qBound(kMinRadius, slot(2), kMaxRadius),
                            qBound(kMinRadius, slot(3), kMaxRadius));
        spin.angle = slot(4);
        spin.feather = qBound(0.0, slot(5), 1.0);
        spin.blur = qBound(0.0, slot(6), kMaxAngle);
        spin.pivot = QPointF(slot(7), slot(8));
        spins->append(spin);
    }
    *at += 1 + kSpinSlots * int(count);
    return true;
}

void BlurGalleryDialog::appendSpinStrobe(QList<double>* out) const
{
    *out << strobeStrength_.spin->value() << strobeFlashes_.spin->value()
         << strobeDuration_.spin->value();
}

void BlurGalleryDialog::readSpinStrobe(const QList<double>& values, int at)
{
    strobeStrength_.setValue(qBound(0.0, values.at(at), 100.0));
    strobeFlashes_.setValue(qBound(1.0, values.at(at + 1), 100.0));
    setStrobeDuration(values.at(at + 2));
    // The Path Blur preset reads Custom while the shared strobe is on.
    syncPreset();
}

void BlurGalleryDialog::setStrobeDuration(double degrees)
{
    strobeDuration_.setValue(qBound(0.0, degrees, kMaxAngle));
}

void BlurGalleryDialog::syncMotionTab()
{
    if (!effectsTabs_ || !strobeDuration_.label) {
        return;
    }
    // Bokeh does not apply to Path or Spin Blur and the Motion Effects apply
    // only to them, so the tab that does not fit the selected effect is
    // disabled.
    const bool motion = active_ == BlurGalleryMode::Path || active_ == BlurGalleryMode::Spin;
    // Disabling the current tab moves Qt to the next one; land on the tab
    // that took its place instead.
    const bool lost = effectsTabs_->currentIndex() == (motion ? 0 : 1);
    effectsTabs_->setTabEnabled(0, !motion);
    effectsTabs_->setTabEnabled(1, motion);
    if (lost) {
        effectsTabs_->setCurrentIndex(motion ? 1 : 0);
    }
    const bool spin = active_ == BlurGalleryMode::Spin;
    strobeDuration_.label->setVisible(spin);
    strobeDuration_.spin->setVisible(spin);
    strobeDuration_.slider->setVisible(spin);
}

} // namespace pictura
