#include "acr_panels.h"
#include "acr_settings.h"

#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QWidget>

namespace pictura {

AcrHslPanel::AcrHslPanel(AcrSettings* settings, QWidget* parent)
    : AcrPanel(settings, parent)
{
    auto* column = new QVBoxLayout(this);
    column->setContentsMargins(0, 0, 0, 0);

    addCheck(column, acr::GrayScale, tr("Convert to Grayscale"), false);

    auto rangeKey = [](const char* prefix, char code) {
        QByteArray k(prefix);
        k.append(code);
        return k;
    };

    const char* headings[3] = {"Hue", "Saturation", "Luminance"};
    const char* prefixes[3] = {"HA_", "SA_", "LA_"};
    for (int h = 0; h < 3; ++h) {
        addSection(column, tr(headings[h]));
        for (int i = 0; i < 8; ++i) {
            addSlider(column, rangeKey(prefixes[h], acr::kRangeCodes[i]),
                      tr(acr::kRangeNames[i]), -100, 100, 0, 0);
        }
    }

    auto* gray = new QWidget(this);
    auto* grayCol = new QVBoxLayout(gray);
    grayCol->setContentsMargins(0, 0, 0, 0);
    addSection(grayCol, tr("Grayscale Mix"));
    for (int i = 0; i < 8; ++i) {
        addSlider(grayCol, rangeKey("GM_", acr::kRangeCodes[i]),
                  tr(acr::kRangeNames[i]), -100, 100, 0, 0);
    }
    column->addWidget(gray);
    gray->setVisible(settings_->boolValue(acr::GrayScale, false));

    auto* grayCheck = findChild<AcrCheck*>(QString::fromLatin1(acr::GrayScale));
    if (grayCheck) {
        connect(grayCheck, &AcrCheck::toggled, gray, &QWidget::setVisible);
    }

    auto* defaultButton = new QPushButton(tr("Default"), this);
    connect(defaultButton, &QPushButton::clicked, this, [this, rangeKey]() {
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
