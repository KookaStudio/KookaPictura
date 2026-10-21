#include "photo_filter_dialog.h"

#include "color_picker_dialog.h"
#include "panels/color_swatch_widget.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

struct Filter {
    const char* name;
    QRgb rgb;
};

// photorust's filter table.
constexpr Filter kFilters[] = {
    {"Warming Filter (85)", 0xec8a00},  {"Warming Filter (LBA)", 0xfa962d},
    {"Warming Filter (81)", 0xebb113},  {"Cooling Filter (80)", 0x006dff},
    {"Cooling Filter (LBB)", 0x005dba}, {"Cooling Filter (82)", 0x0088ea},
    {"Red", 0xea1a00},                  {"Orange", 0xeb7500},
    {"Yellow", 0xffe600},               {"Green", 0x009400},
    {"Cyan", 0x00b7ef},                 {"Blue", 0x0033d1},
    {"Violet", 0x4b0082},               {"Magenta", 0xff0090},
    {"Sepia", 0xac7a33},                {"Deep Red", 0x820500},
    {"Deep Blue", 0x000082},            {"Deep Emerald", 0x006412},
    {"Deep Yellow", 0xffcc00},          {"Underwater", 0x00c2be},
};

} // namespace

PhotoFilterDialog::PhotoFilterDialog(PictureView* view, const QByteArray& block,
                                     const QRect& visible, QWidget* parent)
    : AdjustmentDialog(view, block, visible, QStringLiteral("Photo Filter"), parent)
{
    auto* outer = new QHBoxLayout(this);
    outer->setContentsMargins(12, 12, 12, 12);
    outer->setSpacing(16);
    auto* left = new QVBoxLayout;
    left->setSpacing(8);

    auto* use = new QGroupBox(QStringLiteral("Use"), this);
    use->setObjectName(QStringLiteral("photoFilterUse"));
    auto* grid = new QGridLayout(use);
    useFilter_ = new QRadioButton(QStringLiteral("Filter:"), use);
    useFilter_->setObjectName(QStringLiteral("useFilter"));
    useColor_ = new QRadioButton(QStringLiteral("Color:"), use);
    useColor_->setObjectName(QStringLiteral("useColor"));
    filter_ = new QComboBox(use);
    filter_->setObjectName(QStringLiteral("filter"));
    for (const Filter& filter : kFilters) {
        filter_->addItem(QLatin1String(filter.name), uint(filter.rgb));
    }
    color_ = swatch(QStringLiteral("color"));
    grid->addWidget(useFilter_, 0, 0);
    grid->addWidget(filter_, 0, 1);
    grid->addWidget(useColor_, 1, 0);
    grid->addWidget(color_, 1, 1, Qt::AlignLeft);
    grid->setColumnStretch(1, 1);
    left->addWidget(use);

    // A version-3 block has no editable colour: param() falls back to -1.
    const double rgb = param(QStringLiteral("color"), -1.0);
    use->setEnabled(rgb >= 0.0);
    const QColor current = QColor::fromRgb(QRgb(uint(std::max(rgb, 0.0))));
    paintSwatch(color_, current);
    const int named = filter_->findData(uint(current.rgb() & 0xffffffu));
    filter_->setCurrentIndex(std::max(named, 0));
    (named >= 0 || rgb < 0.0 ? useFilter_ : useColor_)->setChecked(true);

    auto* densityRow = new QHBoxLayout;
    densityRow->addWidget(new QLabel(QStringLiteral("Density:"), this));
    auto* density = new QSpinBox(this);
    density->setObjectName(QStringLiteral("density"));
    density->setRange(1, 100);
    density->setSuffix(QStringLiteral(" %"));
    density->setFixedWidth(kFieldWidth);
    density->setValue(qRound(param(QStringLiteral("density"), 25.0)));
    densityRow->addWidget(density);
    densityRow->addStretch(1);
    left->addLayout(densityRow);
    auto* slider = new QSlider(Qt::Horizontal, this);
    slider->setObjectName(QStringLiteral("densitySlider"));
    slider->setRange(1, 100);
    slider->setValue(density->value());
    left->addWidget(slider);

    auto* luminosity = new QCheckBox(QStringLiteral("Preserve Luminosity"), this);
    luminosity->setObjectName(QStringLiteral("preserveLuminosity"));
    luminosity->setChecked(param(QStringLiteral("preserveLuminosity"), 1.0) != 0.0);
    left->addWidget(luminosity);
    left->addStretch(1);
    outer->addLayout(left, 1);

    QVBoxLayout* buttons = buttonColumn();
    buttons->addSpacing(10);
    buttons->addWidget(previewCheck());
    buttons->addStretch(1);
    outer->addLayout(buttons);

    connect(slider, &QSlider::valueChanged, density, &QSpinBox::setValue);
    connect(density, &QSpinBox::valueChanged, this, [this, slider](int v) {
        slider->setValue(v);
        setParam(QStringLiteral("density"), v);
    });
    connect(luminosity, &QCheckBox::toggled, this,
            [this](bool on) { setParam(QStringLiteral("preserveLuminosity"), on ? 1.0 : 0.0); });
    connect(useFilter_, &QRadioButton::toggled, this, &PhotoFilterDialog::applySource);
    connect(filter_, &QComboBox::currentIndexChanged, this, [this]() {
        useFilter_->setChecked(true);
        applySource();
    });
    connect(color_, &QPushButton::clicked, this, [this]() {
        const QColor picked = ColorPickerDialog::getColor(
            color_->property("color").value<QColor>(), this, QStringLiteral("Photo Filter Color"),
            tools(), [this](const QPoint& pos) { return originalColorAt(pos); });
        if (picked.isValid()) {
            paintSwatch(color_, picked);
            useColor_->setChecked(true);
            applySource();
        }
    });
    setMinimumWidth(440);
}

void PhotoFilterDialog::applySource()
{
    const QRgb rgb = useFilter_->isChecked()
                         ? QRgb(filter_->currentData().toUInt())
                         : color_->property("color").value<QColor>().rgb() & 0xffffffu;
    setParam(QStringLiteral("color"), double(rgb));
}

QWidget* PhotoFilterDialog::controlForTest(const QString& key) const
{
    return findChild<QWidget*>(key);
}

} // namespace pictura

#include "moc_photo_filter_dialog.cpp"
