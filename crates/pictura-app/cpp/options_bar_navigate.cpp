// The options bars of the Hand and Zoom tools: only their multi-window
// checkboxes, Scroll All Windows, and Resize Windows To Fit and Zoom All
// Windows. Part of OptionsBar.

#include "options_bar.h"

#include <QtWidgets/QCheckBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>

namespace pictura {

QWidget* OptionsBar::buildHandPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));

    auto* scrollAll = new QCheckBox(QStringLiteral("Scroll All Windows"), page);
    scrollAll->setObjectName(QStringLiteral("optionsHandScrollAll"));
    scrollAll->setChecked(hand_ && hand_->get().scrollAll);
    layout->addWidget(scrollAll);
    if (hand_) {
        connect(scrollAll, &QCheckBox::toggled, this, [this](bool on) {
            HandOptions options = hand_->get();
            options.scrollAll = on;
            hand_->set(options);
        });
    }

    layout->addStretch(1);
    return page;
}

QWidget* OptionsBar::buildZoomPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));

    auto* resize = new QCheckBox(QStringLiteral("Resize Windows To Fit"), page);
    resize->setObjectName(QStringLiteral("optionsZoomResizeToFit"));
    resize->setChecked(zoom_ && zoom_->get().resizeToFit);
    layout->addWidget(resize);
    auto* zoomAll = new QCheckBox(QStringLiteral("Zoom All Windows"), page);
    zoomAll->setObjectName(QStringLiteral("optionsZoomAllWindows"));
    zoomAll->setChecked(zoom_ && zoom_->get().zoomAll);
    layout->addWidget(zoomAll);
    if (zoom_) {
        connect(resize, &QCheckBox::toggled, this, [this](bool on) {
            ZoomOptions options = zoom_->get();
            options.resizeToFit = on;
            zoom_->set(options);
        });
        connect(zoomAll, &QCheckBox::toggled, this, [this](bool on) {
            ZoomOptions options = zoom_->get();
            options.zoomAll = on;
            zoom_->set(options);
        });
    }

    layout->addStretch(1);
    return page;
}

} // namespace pictura
