#include "color_balance_dialog.h"

#include "api/layers_bridge.h"

#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

const QStringList kTones = {QStringLiteral("shadows"), QStringLiteral("midtones"),
                            QStringLiteral("highlights")};
const QStringList kKeys = {QStringLiteral("cyanRed"), QStringLiteral("magentaGreen"),
                           QStringLiteral("yellowBlue")};

} // namespace

ColorBalanceDialog::ColorBalanceDialog(PictureView* view, const QByteArray& block,
                                       const QRect& visible, QWidget* parent)
    : AdjustmentDialog(view, block, visible, QStringLiteral("Color Balance"), parent)
{
    tone_ = new QButtonGroup(this);
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(14, 12, 14, 12);
    layout->setSpacing(6);

    auto* toneRow = new QHBoxLayout;
    toneRow->addWidget(new QLabel(QStringLiteral("Tone:"), this));
    for (int i = 0; i < 3; ++i) {
        auto* radio = new QRadioButton(
            QStringList{QStringLiteral("Shadows"), QStringLiteral("Midtones"),
                        QStringLiteral("Highlights")}[i],
            this);
        radio->setObjectName(kTones[i]);
        tone_->addButton(radio, i);
        toneRow->addWidget(radio);
    }
    toneRow->addStretch(1);
    tone_->button(1)->setChecked(true);
    layout->addLayout(toneRow);
    layout->addSpacing(4);

    // The Help's "values above the color bars": the active tone's channel
    // changes, named for the document's mode (R/G/B, A/B, or C/M/Y).
    mode_ = view ? QString(channels_mode(*view)) : QStringLiteral("rgb");
    readout_ = new QLabel(this);
    readout_->setObjectName(QStringLiteral("colorBalanceReadout"));
    readout_->setTextInteractionFlags(Qt::TextSelectableByMouse);
    layout->addWidget(readout_);
    layout->addSpacing(2);

    const auto addRow = [this, layout](const QString& label, const QString& key,
                                       const QList<QColor>& ramp) {
        QSpinBox* spin = addRampRow(layout, label, key, ramp);
        connect(spin, &QSpinBox::valueChanged, this, &ColorBalanceDialog::updateReadout);
    };
    addRow(QStringLiteral("Cyan · Red"), kKeys[0],
           {QColor(0, 200, 220), QColor(130, 130, 130), QColor(230, 40, 40)});
    layout->addSpacing(6);
    addRow(QStringLiteral("Magenta · Green"), kKeys[1],
           {QColor(220, 40, 200), QColor(130, 130, 130), QColor(40, 190, 60)});
    layout->addSpacing(6);
    addRow(QStringLiteral("Yellow · Blue"), kKeys[2],
           {QColor(225, 210, 30), QColor(130, 130, 130), QColor(50, 70, 220)});
    layout->addSpacing(8);

    auto* luminosity = new QCheckBox(QStringLiteral("Preserve Luminosity"), this);
    luminosity->setObjectName(QStringLiteral("preserveLuminosity"));
    luminosity->setChecked(param(QStringLiteral("preserveLuminosity"), 1.0) != 0.0);
    layout->addWidget(luminosity);
    layout->addWidget(previewCheck());
    layout->addStretch(1);
    layout->addLayout(bottomButtons());

    connect(luminosity, &QCheckBox::toggled, this,
            [this](bool on) { setParam(QStringLiteral("preserveLuminosity"), on ? 1.0 : 0.0); });
    connect(tone_, &QButtonGroup::idClicked, this, &ColorBalanceDialog::loadTone);
    setMinimumWidth(460);
    loadTone();
}

QString ColorBalanceDialog::paramKey(const QString& key) const
{
    const int tone = tone_ ? tone_->checkedId() : 1;
    return kTones.value(tone < 0 ? 1 : tone) + QLatin1Char('.') + key;
}

void ColorBalanceDialog::loadTone()
{
    // Each field writes back the value just read: a no-op edit.
    for (const QString& key : kKeys) {
        findChild<QSpinBox*>(key)->setValue(qRound(param(paramKey(key))));
    }
    updateReadout();
}

void ColorBalanceDialog::updateReadout()
{
    if (!readout_) {
        return;
    }
    const int cyanRed = findChild<QSpinBox*>(kKeys[0])->value();
    const int magentaGreen = findChild<QSpinBox*>(kKeys[1])->value();
    const int yellowBlue = findChild<QSpinBox*>(kKeys[2])->value();
    const auto signedValue = [](int value) {
        return value > 0 ? QStringLiteral("+%1").arg(value) : QString::number(value);
    };
    if (mode_ == QLatin1String("lab")) {
        readout_->setText(QStringLiteral("A: %1    B: %2")
                              .arg(signedValue(cyanRed - magentaGreen),
                                   signedValue(-yellowBlue)));
    } else if (mode_ == QLatin1String("cmyk")) {
        readout_->setText(QStringLiteral("C: %1    M: %2    Y: %3")
                              .arg(signedValue(-cyanRed), signedValue(-magentaGreen),
                                   signedValue(-yellowBlue)));
    } else {
        readout_->setText(QStringLiteral("R: %1    G: %2    B: %3")
                              .arg(signedValue(cyanRed), signedValue(magentaGreen),
                                   signedValue(yellowBlue)));
    }
}

QWidget* ColorBalanceDialog::controlForTest(const QString& key) const
{
    return findChild<QWidget*>(key);
}

} // namespace pictura

#include "moc_color_balance_dialog.cpp"
