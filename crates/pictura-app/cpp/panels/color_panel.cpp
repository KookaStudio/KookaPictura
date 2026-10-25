#include "color_panel.h"

#include "color_picker_dialog.h"
#include "panels/color_swatch_widget.h"

#include <QtCore/QSignalBlocker>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

ColorState::ColorState(QObject* parent)
    : QObject(parent)
{
}

void ColorState::setForeground(const QColor& color)
{
    if (!color.isValid() || color == foreground_) {
        return;
    }
    foreground_ = color;
    emit foregroundChanged(foreground_);
}

void ColorState::setBackground(const QColor& color)
{
    if (!color.isValid() || color == background_) {
        return;
    }
    background_ = color;
    emit backgroundChanged(background_);
}

void ColorState::setForegroundActive(bool foreground)
{
    if (foregroundActive_ == foreground) {
        return;
    }
    foregroundActive_ = foreground;
    emit activeChanged(foregroundActive_);
}

void ColorState::setMaskActive(bool mask)
{
    maskActive_ = mask;
}

ColorPanel::ColorPanel(ColorState* state, QWidget* parent)
    : QWidget(parent)
    , state_(state)
{
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(4, 4, 4, 4);
    layout->setSpacing(6);

    // Foreground/background swatches: clicking one makes it the active colour
    // and opens the picker for that swatch.
    auto* swatchRow = new QHBoxLayout();
    fgSwatch_ = new ColorSwatchWidget(this);
    bgSwatch_ = new ColorSwatchWidget(this);
    fgSwatch_->setFixedSize(44, 26);
    bgSwatch_->setFixedSize(44, 26);
    fgSwatch_->setFlat(true);
    bgSwatch_->setFlat(true);
    fgSwatch_->setObjectName(QStringLiteral("colorPanelForeground"));
    bgSwatch_->setObjectName(QStringLiteral("colorPanelBackground"));
    fgSwatch_->setToolTip(tr("Foreground color"));
    bgSwatch_->setToolTip(tr("Background color"));
    swatchRow->addWidget(fgSwatch_);
    swatchRow->addWidget(bgSwatch_);
    swatchRow->addStretch(1);
    layout->addLayout(swatchRow);

    // Photoshop's Color panel: the colour field with the hue ramp beside it.
    auto* pickerRow = new QHBoxLayout();
    pickerRow->setSpacing(6);
    plane_ = new ColorPlane(this);
    ramp_ = new ColorRamp(this);
    pickerRow->addWidget(plane_, 1);
    pickerRow->addWidget(ramp_);
    layout->addLayout(pickerRow, 1);

    const auto pick = [this](int h, int s, int v) {
        selectColor(QColor::fromHsv(h, s, v));
    };
    connect(plane_, &ColorPlane::picked, this, pick);
    connect(ramp_, &ColorRamp::picked, this, pick);

    connect(fgSwatch_, &QPushButton::clicked, this, [this] {
        if (state_) {
            state_->setForegroundActive(true);
        }
        const QColor start = state_ ? state_->foreground() : QColor(Qt::black);
        const QColor picked = ColorPickerDialog::getColor(start, this, tr("Foreground Color"));
        if (state_ && picked.isValid()) {
            state_->setForeground(picked);
        }
        syncControls();
    });
    connect(bgSwatch_, &QPushButton::clicked, this, [this] {
        if (state_) {
            state_->setForegroundActive(false);
        }
        const QColor start = state_ ? state_->background() : QColor(Qt::white);
        const QColor picked = ColorPickerDialog::getColor(start, this, tr("Background Color"));
        if (state_ && picked.isValid()) {
            state_->setBackground(picked);
        }
        syncControls();
    });
    if (state_) {
        connect(state_, &ColorState::foregroundChanged, this,
                [this](const QColor&) { syncControls(); });
        connect(state_, &ColorState::backgroundChanged, this,
                [this](const QColor&) { syncControls(); });
        connect(state_, &ColorState::activeChanged, this, [this](bool) { syncControls(); });
    }

    syncControls();
}

void ColorPanel::selectColor(const QColor& color)
{
    if (!state_ || !color.isValid()) {
        return;
    }
    if (state_->foregroundActive()) {
        state_->setForeground(color);
    } else {
        state_->setBackground(color);
    }
}

QColor ColorPanel::activeColor() const
{
    if (!state_) {
        return QColor();
    }
    return state_->foregroundActive() ? state_->foreground() : state_->background();
}

void ColorPanel::syncControls()
{
    if (!state_) {
        return;
    }
    const QColor color = activeColor();
    int h = 0;
    int s = 0;
    int v = 0;
    color.getHsv(&h, &s, &v);
    plane_->setHsv(h, s, v);
    ramp_->setHsv(h, s, v);
    paintSwatch(fgSwatch_, state_->foreground(), state_->foregroundActive());
    paintSwatch(bgSwatch_, state_->background(), !state_->foregroundActive());
}

void ColorPanel::paintSwatch(ColorSwatchWidget* button, const QColor& color, bool active)
{
    button->setColor(color);
    button->setActive(active);
}

} // namespace pictura

#include "moc_color_panel.cpp"
