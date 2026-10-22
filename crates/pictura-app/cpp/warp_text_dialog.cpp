// The Warp Text dialog's widgets. Previewing, committing and cancelling are
// the frame's (frame_menus_type.cpp).

#include "warp_text_dialog.h"

#include "panels/jump_slider.h"
#include "warp_styles.h"

#include <QtCore/QSignalBlocker>
#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

const QString kNone = QStringLiteral("none");

} // namespace

WarpTextDialog::WarpTextDialog(const type_api::WarpSetting& initial, QWidget* parent)
    : QDialog(parent)
{
    setObjectName(QStringLiteral("warpTextDialog"));
    setWindowTitle(tr("Warp Text"));
    auto* outer = new QHBoxLayout(this);
    outer->setContentsMargins(12, 12, 12, 12);
    outer->setSpacing(16);
    auto* left = new QVBoxLayout;
    outer->addLayout(left, 1);

    auto* styleRow = new QHBoxLayout;
    styleRow->addWidget(new QLabel(tr("Style:"), this));
    style_ = new QComboBox(this);
    style_->setObjectName(QStringLiteral("warpTextStyle"));
    for (const WarpStyleEntry& entry : kWarpStyles) {
        if (QLatin1String(entry.id) != QLatin1String("custom")) {
            style_->addItem(tr(entry.name), QString::fromLatin1(entry.id));
        }
    }
    styleRow->addWidget(style_, 1);
    left->addLayout(styleRow);

    auto* orientationRow = new QHBoxLayout;
    orientation_ = new QButtonGroup(this);
    const char* const orientations[2][2] = {{"warpTextHorizontal", "Horizontal"},
                                           {"warpTextVertical", "Vertical"}};
    for (int i = 0; i < 2; ++i) {
        auto* radio = new QRadioButton(tr(orientations[i][1]), this);
        radio->setObjectName(QString::fromLatin1(orientations[i][0]));
        orientation_->addButton(radio, i);
        orientationRow->addWidget(radio);
    }
    orientationRow->addStretch(1);
    left->addLayout(orientationRow);

    auto* grid = new QGridLayout;
    grid->setColumnStretch(0, 1);
    grid->setVerticalSpacing(4);
    const char* const rows[3][2] = {{"warpTextBend", "Bend:"},
                                   {"warpTextHorizontalDistortion", "Horizontal Distortion:"},
                                   {"warpTextVerticalDistortion", "Vertical Distortion:"}};
    for (int i = 0; i < 3; ++i) {
        const QString name = QString::fromLatin1(rows[i][0]);
        grid->addWidget(new QLabel(tr(rows[i][1]), this), i * 2, 0);
        values_[i] = new QSpinBox(this);
        values_[i]->setObjectName(name);
        values_[i]->setRange(-100, 100);
        values_[i]->setSuffix(QStringLiteral(" %"));
        grid->addWidget(values_[i], i * 2, 1);
        sliders_[i] = new JumpSlider(Qt::Horizontal, this);
        sliders_[i]->setObjectName(name + QStringLiteral("Slider"));
        sliders_[i]->setRange(-100, 100);
        grid->addWidget(sliders_[i], i * 2 + 1, 0, 1, 2);
        connect(values_[i], &QSpinBox::valueChanged, this, [this, i](int value) {
            const QSignalBlocker block(sliders_[i]);
            sliders_[i]->setValue(value);
            emit changed();
        });
        connect(sliders_[i], &QSlider::valueChanged, values_[i], &QSpinBox::setValue);
    }
    left->addLayout(grid);
    left->addStretch(1);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel,
                                         Qt::Vertical, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    outer->addWidget(buttons, 0, Qt::AlignTop);

    const int index = style_->findData(initial.style.isEmpty() ? kNone : initial.style);
    style_->setCurrentIndex(index < 0 ? 0 : index);
    lastStyle_ = style_->currentData().toString();
    orientation_->button(initial.verticalOrientation ? 1 : 0)->setChecked(true);
    const double initialValues[3] = {initial.bend, initial.horizontal, initial.vertical};
    for (int i = 0; i < 3; ++i) {
        const QSignalBlocker blockValue(values_[i]);
        const QSignalBlocker blockSlider(sliders_[i]);
        values_[i]->setValue(qRound(initialValues[i]));
        sliders_[i]->setValue(values_[i]->value());
    }
    syncEnabled();
    connect(style_, &QComboBox::currentIndexChanged, this, &WarpTextDialog::styleChosen);
    connect(orientation_, &QButtonGroup::idClicked, this, &WarpTextDialog::changed);
}

type_api::WarpSetting WarpTextDialog::setting() const
{
    const QString id = style_->currentData().toString();
    type_api::WarpSetting out;
    out.style = id == kNone ? QString() : id;
    out.bend = values_[0]->value();
    out.horizontal = values_[1]->value();
    out.vertical = values_[2]->value();
    out.verticalOrientation = orientation_->checkedId() == 1;
    return out;
}

void WarpTextDialog::styleChosen()
{
    const QString id = style_->currentData().toString();
    if (lastStyle_ == kNone && id != kNone && values_[0]->value() == 0) {
        const QSignalBlocker block(values_[0]);
        values_[0]->setValue(50);
        sliders_[0]->setValue(50);
    }
    lastStyle_ = id;
    syncEnabled();
    emit changed();
}

void WarpTextDialog::syncEnabled()
{
    const bool on = style_->currentData().toString() != kNone;
    for (QAbstractButton* button : orientation_->buttons()) {
        button->setEnabled(on);
    }
    for (int i = 0; i < 3; ++i) {
        values_[i]->setEnabled(on);
        sliders_[i]->setEnabled(on);
    }
}

} // namespace pictura

#include "moc_warp_text_dialog.cpp"
