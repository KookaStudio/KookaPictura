#include "adjustments_panel.h"

#include "icons.h"

#include <QtCore/QSize>
#include <QtCore/QStringList>
#include <QtWidgets/QFrame>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QWidget>

namespace pictura {

const AdjustmentsPanel::Entry AdjustmentsPanel::kEntries[19] = {
    // Fill-layer types first, mirroring Layer > New Fill Layer and the Layers
    // panel's New Fill/Adjustment menu.
    {"solid-color", "Solid Color", "adjustments.solid-color", true},
    {"gradient", "Gradient", "adjustments.gradient", true},
    {"pattern", "Pattern", "adjustments.pattern", true},
    // The sixteen adjustments in CS6 panel order, matching the panel menu rows.
    {"brightness-contrast", "Brightness/Contrast", "adjustments.brightness-contrast", false},
    {"levels", "Levels", "adjustments.levels", false},
    {"curves", "Curves", "adjustments.curves", false},
    {"exposure", "Exposure", "adjustments.exposure", false},
    {"vibrance", "Vibrance", "adjustments.vibrance", false},
    {"hue-saturation", "Hue/Saturation", "adjustments.hue-saturation", false},
    {"color-balance", "Color Balance", "adjustments.color-balance", false},
    {"black-white", "Black & White", "adjustments.black-white", false},
    {"photo-filter", "Photo Filter", "adjustments.photo-filter", false},
    {"channel-mixer", "Channel Mixer", "adjustments.channel-mixer", false},
    {"color-lookup", "Color Lookup", "adjustments.color-lookup", false},
    {"invert", "Invert", "adjustments.invert", false},
    {"posterize", "Posterize", "adjustments.posterize", false},
    {"threshold", "Threshold", "adjustments.threshold", false},
    {"gradient-map", "Gradient Map", "adjustments.gradient-map", false},
    {"selective-color", "Selective Color", "adjustments.selective-color", false},
};

AdjustmentsPanel::AdjustmentsPanel(QWidget* parent)
    : QWidget(parent)
{
    auto* scroll = new QScrollArea(this);
    scroll->setWidgetResizable(true);
    scroll->setFrameShape(QFrame::NoFrame);

    auto* gridHost = new QWidget;
    auto* grid = new QGridLayout(gridHost);
    grid->setContentsMargins(6, 6, 6, 6);
    grid->setSpacing(1);
    grid->setAlignment(Qt::AlignHCenter | Qt::AlignTop);

    constexpr int kColumns = 5;
    for (int i = 0; i < 19; ++i) {
        QToolButton* button = makeButton(kEntries[i]);
        buttons_.append(button);
        grid->addWidget(button, i / kColumns, i % kColumns);
    }

    scroll->setWidget(gridHost);
    auto* outer = new QVBoxLayout(this);
    outer->setContentsMargins(0, 0, 0, 0);
    outer->addWidget(scroll);
}

QToolButton* AdjustmentsPanel::makeButton(const Entry& entry)
{
    auto* button = new QToolButton(this);
    button->setObjectName(QStringLiteral("adjustmentsButton_") + QString::fromLatin1(entry.id));
    button->setIcon(pictura::icon(QString::fromLatin1(entry.iconId)));
    button->setIconSize(QSize(24, 24));
    button->setToolButtonStyle(Qt::ToolButtonIconOnly);
    button->setToolTip(QString::fromUtf8(entry.label));
    button->setAutoRaise(true);
    button->setSizePolicy(QSizePolicy::Preferred, QSizePolicy::Preferred);
    if (entry.fill) {
        connect(button, &QToolButton::clicked, this, [this, id = QString::fromLatin1(entry.id)] {
            if (id == QLatin1String("pattern")) {
                return;
            }
            emit fillRequested(id);
        });
    } else {
        connect(button, &QToolButton::clicked, this,
                [this, id = QString::fromLatin1(entry.id)] { emit adjustmentRequested(id); });
    }
    return button;
}

void AdjustmentsPanel::setView(PictureView* view)
{
    const bool enabled = view != nullptr;
    for (QToolButton* button : buttons_) {
        button->setEnabled(enabled);
    }
    if (QToolButton* pattern = buttonByIdForTest(QStringLiteral("pattern"))) {
        pattern->setEnabled(false);
        pattern->setToolTip(QStringLiteral("Pattern — not implemented yet"));
    }
}

int AdjustmentsPanel::buttonCountForTest() const
{
    return buttons_.size();
}

QToolButton* AdjustmentsPanel::buttonForTest(int index) const
{
    return index >= 0 && index < buttons_.size() ? buttons_.at(index) : nullptr;
}

QStringList AdjustmentsPanel::buttonIdsForTest() const
{
    QStringList ids;
    for (QToolButton* button : buttons_) {
        ids << button->objectName().remove(0, QStringLiteral("adjustmentsButton_").size());
    }
    return ids;
}

QStringList AdjustmentsPanel::buttonToolTipsForTest() const
{
    QStringList tips;
    for (QToolButton* button : buttons_) {
        tips << button->toolTip();
    }
    return tips;
}

QToolButton* AdjustmentsPanel::buttonByIdForTest(const QString& id) const
{
    return findChild<QToolButton*>(QStringLiteral("adjustmentsButton_") + id);
}

} // namespace pictura

#include "moc_adjustments_panel.cpp"
