#include "slider_dialog.h"

#include "adjustment_presets.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <cmath>

namespace pictura {

SliderDialog::SliderDialog(PictureView* view, const QByteArray& block, const QRect& visible,
                           const QString& title, const QVector<Row>& rows,
                           const QStringList& extraButtons, QWidget* parent)
    : AdjustmentDialog(view, block, visible, title, parent)
{
    auto* outer = new QHBoxLayout(this);
    outer->setContentsMargins(12, 12, 12, 12);
    outer->setSpacing(16);

    auto* left = new QVBoxLayout;
    addPresetRow(left);
    rows_ = rows;
    auto* grid = new QGridLayout;
    grid->setColumnStretch(1, 1);
    grid->setVerticalSpacing(6);
    // Later rows indent their label a little, as CS6 does.
    for (int i = 0; i < rows.size(); ++i) {
        addStackedRow(grid, i * 3, rows[i].label, rows[i].key, i == 0 ? 0 : 14);
        grid->setRowMinimumHeight(i * 3 + 2, 8);
    }
    left->addLayout(grid);
    left->addStretch(1);
    left->addWidget(previewCheck(), 0, Qt::AlignRight);
    outer->addLayout(left, 1);

    QVBoxLayout* buttons = buttonColumn(extraButtons);
    buttons->addStretch(1);
    outer->addLayout(buttons);

    setMinimumWidth(420);
}

void SliderDialog::applyAuto()
{
    bool brightnessRow = false;
    bool contrastRow = false;
    for (const Row& row : rows_) {
        brightnessRow = brightnessRow || row.key == QLatin1String("brightness");
        contrastRow = contrastRow || row.key == QLatin1String("contrast");
    }
    int lo = 0;
    int hi = 0;
    if (!brightnessRow || !contrastRow || !autoEndpoints(0, lo, hi)) {
        return;
    }
    // Behavioral approximation of CS6's closed fit: the modern gamma curve
    // centres the occupied range, then the S-curve's central slope expands it.
    const double mid = (lo + hi) / 510.0;
    const double gamma = std::log(mid) / std::log(0.5);
    const double brightness = std::clamp(150.0 * std::log2(gamma), -150.0, 150.0);
    const double effective = std::pow(2.0, brightness / 150.0);
    const double low = std::pow(lo / 255.0, 1.0 / effective);
    const double high = std::pow(hi / 255.0, 1.0 / effective);
    const double half = std::max(0.5 - low, high - 0.5);
    const double slope = half > 0.0 ? 0.5 / half : 1.0;
    const double contrast = std::clamp(100.0 * (slope - 1.0), -50.0, 100.0);
    if (!setParams({{QStringLiteral("brightness"), std::round(brightness)},
                    {QStringLiteral("contrast"), std::round(contrast)}})) {
        return;
    }
    reloadFromBlock();
}

void SliderDialog::reloadFromBlock()
{
    for (const Row& row : rows_) {
        auto* spin = findChild<QSpinBox*>(row.key);
        auto* slider = findChild<QSlider*>(row.key + QStringLiteral("Slider"));
        if (!spin || !slider) {
            continue;
        }
        const QSignalBlocker blockSpin(spin);
        const QSignalBlocker blockSlider(slider);
        spin->setValue(qRound(param(row.key)));
        slider->setValue(spin->value());
    }
}

QWidget* SliderDialog::controlForTest(const QString& key) const
{
    return findChild<QSpinBox*>(key);
}

} // namespace pictura

#include "moc_slider_dialog.cpp"
