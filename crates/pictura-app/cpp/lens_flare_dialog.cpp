#include "lens_flare_dialog.h"
#include "dialogs.h"
#include "panels/jump_slider.h"

#include <QtCore/QSignalBlocker>
#include <QtCore/QTimer>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPen>
#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

#include <functional>

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"

namespace pictura {

namespace {

const int kPadSize = 250;

// The spec's slots, in order: Brightness, Flare Center (x, y), Lens Type.
enum Param { kBrightness, kCenter, kLens };

} // namespace

// The preview frame: the whole picture fitted inside, letterboxed on a dark
// ground, under CS6's gapped crosshair. Click or drag moves the centre, which
// is a fraction of the picture, not of the frame.
class FlareCenterPad : public QWidget {
public:
    explicit FlareCenterPad(QWidget* parent) : QWidget(parent)
    {
        setObjectName(QStringLiteral("lensFlarePad"));
        setFixedSize(kPadSize, kPadSize);
        setCursor(Qt::CrossCursor);
        setToolTip(QStringLiteral("Click or drag to move the centre of the flare"));
    }

    std::function<void()> moved;

    QPointF center() const { return center_; }

    void setCenter(const QPointF& center)
    {
        center_ = QPointF(qBound(0.0, center.x(), 1.0), qBound(0.0, center.y(), 1.0));
        update();
    }

    QImage image() const { return image_; }

    void setImage(const QImage& image)
    {
        image_ = image;
        update();
    }

    // Where the picture sits in the frame, keeping its shape so the centre
    // points at the same place it will land.
    QRectF imageRect() const
    {
        if (aspect_.isEmpty()) {
            return QRectF(rect());
        }
        const QSizeF fitted = QSizeF(aspect_).scaled(QSizeF(size()), Qt::KeepAspectRatio);
        return QRectF(
            QPointF((width() - fitted.width()) / 2.0, (height() - fitted.height()) / 2.0), fitted);
    }

    void setAspect(const QSize& aspect) { aspect_ = aspect; }

protected:
    void paintEvent(QPaintEvent*) override
    {
        QPainter painter(this);
        painter.fillRect(rect(), QColor(0x2b, 0x2b, 0x2b));
        const QRectF area = imageRect();
        if (!image_.isNull()) {
            painter.setRenderHint(QPainter::SmoothPixmapTransform, true);
            painter.drawImage(area, image_);
        }
        // Black under white so the cross reads on a blown-out flare as well as
        // on a dark picture; the gap keeps what is under it visible.
        const QPointF at(area.x() + center_.x() * area.width(),
                         area.y() + center_.y() * area.height());
        constexpr double kArm = 9.0;
        constexpr double kGap = 3.0;
        const QLineF cross[] = {{at + QPointF(-kArm, 0), at + QPointF(-kGap, 0)},
                                {at + QPointF(kGap, 0), at + QPointF(kArm, 0)},
                                {at + QPointF(0, -kArm), at + QPointF(0, -kGap)},
                                {at + QPointF(0, kGap), at + QPointF(0, kArm)}};
        for (const auto& [color, width] :
             {std::pair{QColor(0, 0, 0, 180), 3.0}, std::pair{QColor(255, 255, 255), 1.0}}) {
            painter.setPen(QPen(color, width));
            for (const QLineF& line : cross) {
                painter.drawLine(line);
            }
        }
        painter.setPen(QPen(QColor(0x14, 0x14, 0x14), 1));
        painter.drawRect(rect().adjusted(0, 0, -1, -1));
    }

    void mousePressEvent(QMouseEvent* event) override { moveTo(event->position()); }

    void mouseMoveEvent(QMouseEvent* event) override
    {
        if (event->buttons() & Qt::LeftButton) {
            moveTo(event->position());
        }
    }

private:
    void moveTo(const QPointF& pos)
    {
        const QRectF area = imageRect();
        if (area.width() <= 0.0 || area.height() <= 0.0) {
            return;
        }
        setCenter(QPointF((pos.x() - area.x()) / area.width(),
                          (pos.y() - area.y()) / area.height()));
        if (moved) {
            moved();
        }
    }

    QImage image_;
    QSize aspect_;
    QPointF center_{0.5, 0.5};
};

LensFlareDialog::LensFlareDialog(PictureView* view, const FilterCommandSpec& spec,
                                 QWidget* parent)
    : QDialog(parent), view_(view), spec_(spec)
{
    setWindowTitle(spec.label);
    const FilterParamSpec& brightness = spec_.params.at(kBrightness);
    const FilterParamSpec& center = spec_.params.at(kCenter);
    const FilterParamSpec& lens = spec_.params.at(kLens);

    pad_ = new FlareCenterPad(this);
    if (view_ && view_->has_document()) {
        const QImage image = documentPreviewImage(view_, kPadSize);
        if (!image.isNull()) {
            proxy_ = image.scaled(kPadSize, kPadSize, Qt::KeepAspectRatio, Qt::SmoothTransformation)
                         .convertToFormat(QImage::Format_RGBA8888);
            for (int y = 0; y < proxy_.height(); ++y) {
                proxyRgba_.append(reinterpret_cast<const char*>(proxy_.constScanLine(y)),
                                  proxy_.width() * 4);
            }
            pad_->setAspect(QSize(view_->document_width(), view_->document_height()));
        }
    }
    pad_->setCenter(QPointF(center.initial.value(0, 0.5), center.initial.value(1, 0.5)));
    pad_->moved = [this] { valuesChanged(true); };

    auto* ok = new QPushButton(QStringLiteral("OK"), this);
    ok->setDefault(true);
    auto* cancel = new QPushButton(QStringLiteral("Cancel"), this);
    connect(ok, &QPushButton::clicked, this, &QDialog::accept);
    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);
    preview_ = new QCheckBox(QStringLiteral("Preview"), this);
    preview_->setObjectName(QStringLiteral("filterPreview"));
    preview_->setChecked(true);
    connect(preview_, &QCheckBox::toggled, this, [this](bool on) {
        if (on) {
            valuesChanged(true);
        } else {
            discardPreview();
        }
    });
    auto* buttons = new QVBoxLayout;
    buttons->addWidget(ok);
    buttons->addWidget(cancel);
    buttons->addSpacing(6);
    buttons->addWidget(preview_);
    buttons->addStretch(1);

    auto* top = new QHBoxLayout;
    top->setSpacing(12);
    top->addWidget(pad_, 0, Qt::AlignTop);
    top->addLayout(buttons);

    brightness_ = new QSpinBox(this);
    brightness_->setObjectName(QStringLiteral("lensFlareBrightness"));
    brightness_->setRange(static_cast<int>(brightness.minimum),
                          static_cast<int>(brightness.maximum));
    brightness_->setSuffix(QStringLiteral("%"));
    brightness_->setValue(static_cast<int>(brightness.value));
    slider_ = new JumpSlider(Qt::Horizontal, this);
    slider_->setRange(brightness_->minimum(), brightness_->maximum());
    slider_->setValue(brightness_->value());
    auto* brightnessLabel = new QLabel(brightness.label, this);
    brightnessLabel->setBuddy(brightness_);
    auto* brightnessRow = new QHBoxLayout;
    brightnessRow->addWidget(brightnessLabel);
    brightnessRow->addWidget(brightness_, 1);
    // The pad follows the slider live; the canvas, a whole-layer render, waits
    // for the release.
    connect(slider_, &QSlider::valueChanged, this, [this](int value) {
        QSignalBlocker block(brightness_);
        brightness_->setValue(value);
        valuesChanged(!slider_->isSliderDown());
    });
    connect(slider_, &QSlider::sliderReleased, this, [this] { valuesChanged(true); });
    connect(brightness_, &QSpinBox::valueChanged, this, [this](int value) {
        QSignalBlocker block(slider_);
        slider_->setValue(value);
        valuesChanged(true);
    });

    QString lensTitle = lens.label;
    lensTitle.remove(QLatin1Char(':'));
    auto* lensBox = new QGroupBox(lensTitle, this);
    auto* lensLayout = new QVBoxLayout(lensBox);
    lens_ = new QButtonGroup(this);
    for (int i = 0; i < lens.choices.size(); ++i) {
        auto* radio = new QRadioButton(lens.choices.at(i), lensBox);
        lens_->addButton(radio, i);
        lensLayout->addWidget(radio);
    }
    if (QAbstractButton* initial = lens_->button(static_cast<int>(lens.value))) {
        initial->setChecked(true);
    }
    connect(lens_, &QButtonGroup::idToggled, this, [this](int, bool on) {
        if (on) {
            valuesChanged(true);
        }
    });

    auto* outer = new QVBoxLayout(this);
    outer->addLayout(top);
    outer->addLayout(brightnessRow);
    outer->addWidget(slider_);
    outer->addWidget(lensBox, 0, Qt::AlignLeft);
    outer->setSizeConstraint(QLayout::SetFixedSize);

    connect(this, &QDialog::rejected, this, &LensFlareDialog::discardPreview);
    valuesChanged(false);
}

QList<double> LensFlareDialog::values() const
{
    const QPointF at = pad_->center();
    return {static_cast<double>(brightness_->value()), at.x(), at.y(),
            static_cast<double>(qMax(0, lens_->checkedId()))};
}

void LensFlareDialog::setValues(const QList<double>& values)
{
    QSignalBlocker blockSpin(brightness_);
    QSignalBlocker blockSlider(slider_);
    QSignalBlocker blockLens(lens_);
    if (values.size() > 0) {
        brightness_->setValue(static_cast<int>(values.at(0)));
        slider_->setValue(brightness_->value());
    }
    if (values.size() > 2) {
        pad_->setCenter(QPointF(values.at(1), values.at(2)));
    }
    if (values.size() > 3) {
        if (QAbstractButton* button = lens_->button(static_cast<int>(values.at(3)))) {
            button->setChecked(true);
        }
    }
    valuesChanged(false);
}

QPointF LensFlareDialog::center() const
{
    return pad_->center();
}

void LensFlareDialog::setCenter(const QPointF& center)
{
    pad_->setCenter(center);
    valuesChanged(true);
}

QImage LensFlareDialog::padImage() const
{
    return pad_->image();
}

void LensFlareDialog::valuesChanged(bool canvas)
{
    const QList<double> current = values();
    // The flare is sized and placed as a fraction of the frame, so the proxy
    // shows the same picture the commit will.
    if (!proxy_.isNull()) {
        const QImage flared =
            filter_thumbnail(proxyRgba_, proxy_.width(), proxy_.height(), spec_.kind, current, 1.0);
        pad_->setImage(flared.isNull() ? proxy_ : flared);
    }
    if (!canvas || !shownOnce_ || !preview_->isChecked() || !view_ || !view_->has_document()) {
        return;
    }
    if (filter_preview(*view_, spec_.kind, current)) {
        previewShown_ = true;
    }
}

void LensFlareDialog::discardPreview()
{
    if (previewShown_ && view_) {
        filter_preview_cancel(*view_);
        previewShown_ = false;
    }
}

void LensFlareDialog::showEvent(QShowEvent* event)
{
    QDialog::showEvent(event);
    // Preview on open, deferred a turn so the dialog paints before the
    // whole-layer render runs.
    if (!shownOnce_) {
        shownOnce_ = true;
        QTimer::singleShot(0, this, [this] {
            if (isVisible()) {
                valuesChanged(true);
            }
        });
    }
}

bool LensFlareDialog::get(PictureView* view, const FilterCommandSpec& spec,
                          const QList<double>& initial, QList<double>* out, QWidget* parent)
{
    LensFlareDialog dialog(view, spec, parent);
    dialog.setValues(initial);
    if (runDialog(dialog, parent) != QDialog::Accepted) {
        return false;
    }
    *out = dialog.values();
    return true;
}

} // namespace pictura

#include "moc_lens_flare_dialog.cpp"
