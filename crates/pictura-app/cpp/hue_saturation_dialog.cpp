#include "hue_saturation_dialog.h"

#include "panels/spectrum_bar.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

const QStringList kRangePrefixes = {
    QString(),           QStringLiteral("reds."),  QStringLiteral("yellows."),
    QStringLiteral("greens."), QStringLiteral("cyans."), QStringLiteral("blues."),
    QStringLiteral("magentas."),
};
const QStringList kKeys = {QStringLiteral("hue"), QStringLiteral("saturation"),
                           QStringLiteral("lightness")};

struct Preset {
    const char* name;
    int hue;
    int saturation;
    int lightness;
};

// photorust's Master-only presets.
constexpr Preset kPresets[] = {
    {"Default", 0, 0, 0},
    {"Increase Saturation More", 0, 60, 0},
    {"Increase Saturation", 0, 30, 0},
    {"Old Style", 0, -40, 5},
    {"Red Boost", -5, 20, 0},
    {"Strong Saturation", 0, 50, 0},
    {"Yellow Boost", 5, 20, 0},
};
const QString kCustom = QStringLiteral("Custom");

} // namespace

HueSaturationDialog::HueSaturationDialog(PictureView* view, const QByteArray& block,
                                         const QRect& visible, QWidget* parent)
    : AdjustmentDialog(view, block, visible, QStringLiteral("Hue/Saturation"), parent)
{
    auto* outer = new QHBoxLayout(this);
    outer->setContentsMargins(12, 12, 12, 12);
    outer->setSpacing(16);
    auto* left = new QVBoxLayout;
    left->setSpacing(6);

    auto* presetRow = new QHBoxLayout;
    presetRow->addWidget(new QLabel(QStringLiteral("Preset:"), this));
    preset_ = new QComboBox(this);
    preset_->setObjectName(QStringLiteral("hueSaturationPreset"));
    for (const Preset& preset : kPresets) {
        preset_->addItem(QLatin1String(preset.name));
    }
    preset_->insertSeparator(1);
    preset_->insertSeparator(preset_->count());
    preset_->addItem(kCustom);
    presetRow->addWidget(preset_, 1);
    left->addLayout(presetRow);
    left->addSpacing(6);

    range_ = new QComboBox(this);
    range_->setObjectName(QStringLiteral("hueSaturationRange"));
    range_->addItems({QStringLiteral("Master"), QStringLiteral("Reds"), QStringLiteral("Yellows"),
                      QStringLiteral("Greens"), QStringLiteral("Cyans"), QStringLiteral("Blues"),
                      QStringLiteral("Magentas")});
    range_->setMinimumWidth(120);
    left->addWidget(range_, 0, Qt::AlignHCenter);
    left->addSpacing(6);

    auto* grid = new QGridLayout;
    grid->setColumnStretch(1, 1);
    grid->setVerticalSpacing(4);
    addStackedRow(grid, 0, QStringLiteral("Hue:"), kKeys[0], -180, 180, 14);
    grid->setRowMinimumHeight(2, 6);
    addStackedRow(grid, 3, QStringLiteral("Saturation:"), kKeys[1], -100, 100, 14);
    grid->setRowMinimumHeight(5, 6);
    addStackedRow(grid, 6, QStringLiteral("Lightness:"), kKeys[2], -100, 100, 14);
    left->addLayout(grid);
    left->addSpacing(8);

    auto* checks = new QHBoxLayout;
    checks->addStretch(1);
    auto* colorize = new QCheckBox(QStringLiteral("Colorize"), this);
    colorize->setObjectName(QStringLiteral("hueSaturationColorize"));
    colorize->setEnabled(false);
    checks->addWidget(colorize);
    checks->addWidget(previewCheck());
    left->addLayout(checks);

    left->addWidget(new SpectrumBar(this));
    shifted_ = new SpectrumBar(this);
    left->addWidget(shifted_);
    left->addStretch(1);
    outer->addLayout(left, 1);

    QVBoxLayout* buttons = buttonColumn();
    buttons->addStretch(1);
    outer->addLayout(buttons);

    for (const QString& key : kKeys) {
        connect(findChild<QSpinBox*>(key), &QSpinBox::valueChanged, this, [this, key](int v) {
            if (key == kKeys[0]) {
                shifted_->setHueShift(v);
            }
            if (!loading_) {
                markCustom();
            }
        });
    }
    connect(range_, &QComboBox::currentIndexChanged, this, &HueSaturationDialog::loadRange);
    connect(preset_, &QComboBox::activated, this, &HueSaturationDialog::applyPreset);
    setMinimumWidth(500);
    loadRange();
}

QString HueSaturationDialog::paramKey(const QString& key) const
{
    return kRangePrefixes.value(range_ ? range_->currentIndex() : 0) + key;
}

void HueSaturationDialog::loadRange()
{
    // The fields' own edits are no-ops here: each writes back the value just read.
    loading_ = true;
    for (const QString& key : kKeys) {
        findChild<QSpinBox*>(key)->setValue(qRound(param(paramKey(key))));
    }
    loading_ = false;
}

void HueSaturationDialog::applyPreset(int index)
{
    const QString name = preset_->itemText(index);
    for (const Preset& preset : kPresets) {
        if (name != QLatin1String(preset.name)) {
            continue;
        }
        QList<QPair<QString, double>> edits;
        for (const QString& prefix : kRangePrefixes) {
            for (const QString& key : kKeys) {
                edits << qMakePair(prefix + key, 0.0);
            }
        }
        edits << qMakePair(kKeys[0], double(preset.hue))
              << qMakePair(kKeys[1], double(preset.saturation))
              << qMakePair(kKeys[2], double(preset.lightness));
        setParams(edits);
        const QSignalBlocker block(range_);
        range_->setCurrentIndex(0);
        loadRange();
        return;
    }
}

void HueSaturationDialog::markCustom()
{
    const QSignalBlocker block(preset_);
    preset_->setCurrentIndex(preset_->findText(kCustom));
}

QWidget* HueSaturationDialog::controlForTest(const QString& key) const
{
    if (key == QLatin1String("preset")) {
        return preset_;
    }
    if (key == QLatin1String("range")) {
        return range_;
    }
    return findChild<QWidget*>(key);
}

} // namespace pictura

#include "moc_hue_saturation_dialog.cpp"
