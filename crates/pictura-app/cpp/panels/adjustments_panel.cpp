#include "adjustments_panel.h"

#include "adjustment_catalogue.h"
#include "icons.h"

#include "pictura_app/src/cxxqt_object/impl_layers/fill_content.cxxqt.h"

#include <QtCore/QSize>
#include <QtCore/QStringList>
#include <QtWidgets/QFrame>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QWidget>

namespace pictura {

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

    // The engine catalogue's tiles: the fill types first, then the sixteen
    // adjustments in CS6 panel order.
    constexpr int kColumns = 5;
    for (const AdjustmentKind& entry : adjustmentCatalogue()) {
        if (entry.panelIcon.isEmpty()) {
            continue;
        }
        const int i = buttons_.size();
        QToolButton* button = makeButton(entry);
        buttons_.append(button);
        grid->addWidget(button, i / kColumns, i % kColumns);
    }

    scroll->setWidget(gridHost);
    auto* outer = new QVBoxLayout(this);
    outer->setContentsMargins(0, 0, 0, 0);
    outer->addWidget(scroll);
}

QToolButton* AdjustmentsPanel::makeButton(const AdjustmentKind& entry)
{
    auto* button = new QToolButton(this);
    button->setObjectName(QStringLiteral("adjustmentsButton_") + entry.kind);
    button->setIcon(pictura::icon(entry.panelIcon));
    button->setProperty("iconId", entry.panelIcon);
    button->setIconSize(QSize(24, 24));
    button->setToolButtonStyle(Qt::ToolButtonIconOnly);
    button->setToolTip(entry.title);
    button->setAutoRaise(true);
    button->setSizePolicy(QSizePolicy::Preferred, QSizePolicy::Preferred);
    if (entry.fill) {
        connect(button, &QToolButton::clicked, this,
                [this, id = entry.kind] { emit fillRequested(id); });
    } else {
        connect(button, &QToolButton::clicked, this,
                [this, id = entry.kind] { emit adjustmentRequested(id); });
    }
    return button;
}

void AdjustmentsPanel::setView(PictureView* view)
{
    view_ = view;
    const bool enabled = view != nullptr;
    for (QToolButton* button : buttons_) {
        button->setEnabled(enabled);
    }
    refresh();
}

void AdjustmentsPanel::refresh()
{
    const quint64 generation = view_ ? view_->document_generation() : 0;
    if (view_ == patternView_ && generation == patternGeneration_) {
        return;
    }
    patternView_ = view_;
    patternGeneration_ = generation;
    if (QToolButton* pattern = buttonByIdForTest(QStringLiteral("pattern"))) {
        const bool hasPatterns = view_ && layer_has_patterns(*view_);
        pattern->setEnabled(hasPatterns);
        pattern->setToolTip(hasPatterns ? QStringLiteral("Pattern")
                                        : QStringLiteral("No patterns are loaded"));
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
