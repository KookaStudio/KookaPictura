#include "acr_panels.h"
#include "acr_settings.h"

#include <QtGui/QColor>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QTabWidget>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QWidget>

namespace pictura {

namespace {
QByteArray rangeKey(const char* prefix, char code)
{
    QByteArray key(prefix);
    key.append(code);
    return key;
}

QList<QColor> spectrumRamp()
{
    return {QColor(0xff, 0x00, 0x00), QColor(0xff, 0xff, 0x00), QColor(0x00, 0xff, 0x00),
            QColor(0x00, 0xff, 0xff), QColor(0x00, 0x00, 0xff), QColor(0xff, 0x00, 0xff),
            QColor(0xff, 0x00, 0x00)};
}

const QList<QColor> kGrayRamp = {QColor(0x33, 0x33, 0x33), QColor(0xdd, 0xdd, 0xdd)};
} // namespace

AcrHslPanel::AcrHslPanel(AcrSettings* settings, QWidget* parent)
    : AcrPanel(settings, parent)
{
    auto* column = new QVBoxLayout(this);
    column->setContentsMargins(0, 0, 0, 0);

    addCheck(column, acr::GrayScale, tr("Convert to Grayscale"), false);

    auto* subtabs = new QTabWidget(this);
    subtabs->setObjectName(QStringLiteral("acrHslTabs"));
    const char* headings[3] = {"Hue", "Saturation", "Luminance"};
    const char* prefixes[3] = {"HA_", "SA_", "LA_"};
    for (int h = 0; h < 3; ++h) {
        auto* page = new QWidget(this);
        auto* pageColumn = new QVBoxLayout(page);
        pageColumn->setContentsMargins(0, 0, 0, 0);
        const QList<QColor> ramp = h == 0 ? spectrumRamp() : kGrayRamp;
        for (int i = 0; i < 8; ++i) {
            addSlider(pageColumn, rangeKey(prefixes[h], acr::kRangeCodes[i]),
                      tr(acr::kRangeNames[i]), -100, 100, 0, 0, ramp);
        }
        pageColumn->addStretch(1);
        subtabs->addTab(page, tr(headings[h]));
    }
    column->addWidget(subtabs);

    auto* gray = new QWidget(this);
    auto* grayCol = new QVBoxLayout(gray);
    grayCol->setContentsMargins(0, 0, 0, 0);
    addSection(grayCol, tr("Grayscale Mix"));
    for (int i = 0; i < 8; ++i) {
        addSlider(grayCol, rangeKey("GM_", acr::kRangeCodes[i]), tr(acr::kRangeNames[i]), -100, 100,
                  0, 0, kGrayRamp);
    }
    column->addWidget(gray);
    gray->setVisible(settings_->boolValue(acr::GrayScale, false));

    auto* grayCheck = findChild<AcrCheck*>(QString::fromLatin1(acr::GrayScale));
    if (grayCheck) {
        connect(grayCheck, &AcrCheck::toggled, gray, &QWidget::setVisible);
    }

    auto* defaultButton = new QPushButton(tr("Default"), this);
    connect(defaultButton, &QPushButton::clicked, this, [this]() {
        const char* prefixes[4] = {"HA_", "SA_", "LA_", "GM_"};
        for (const char* prefix : prefixes) {
            for (int i = 0; i < 8; ++i) {
                QByteArray key = rangeKey(prefix, acr::kRangeCodes[i]);
                settings_->setInt(key, 0);
                if (auto* s = findChild<AcrSlider*>(QString::fromLatin1(key))) {
                    s->setValue(0.0);
                }
            }
        }
        emit changed();
    });
    column->addWidget(defaultButton);

    column->addStretch(1);
}

} // namespace pictura
