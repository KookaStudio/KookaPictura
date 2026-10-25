#include "frame_includes.h"

#include "fonts.h"
#include "panels/numeric_field.h"
#include "search_bar.h"

#include <QtWidgets/QApplication>
#include <QtWidgets/QWidgetAction>

namespace pictura {

void PicturaMainWindow::buildUiSizeMenu()
{
    QAction* readoutAction = registry_->action(command_ids::WindowUiSize);
    if (!readoutAction) {
        return;
    }
    QMenu* windowMenu = nullptr;
    for (QObject* object : readoutAction->associatedObjects()) {
        if ((windowMenu = qobject_cast<QMenu*>(object))) {
            break;
        }
    }
    if (!windowMenu) {
        return;
    }

    auto* row = new QWidget(windowMenu);
    row->setObjectName(QStringLiteral("uiSizeControl"));
    auto* layout = new QVBoxLayout(row);
    layout->setContentsMargins(8, 2, 8, 2);
    layout->setSpacing(4);
    auto* caption = new QLabel(tr("UI size"), row);
    caption->setObjectName(QStringLiteral("uiSizeCaption"));
    layout->addWidget(caption);

    auto* stepRow = new QHBoxLayout();
    stepRow->setContentsMargins(0, 0, 0, 0);
    stepRow->setSpacing(4);
    stepRow->addStretch(1);
    layout->addLayout(stepRow);

    const auto addButton = [row](QBoxLayout* target, const QString& name,
                                 const QString& text, const QString& tip) {
        auto* button = new QToolButton(row);
        button->setObjectName(name);
        button->setText(text);
        button->setAutoRaise(true);
        button->setToolTip(tip);
        target->addWidget(button);
        return button;
    };
    auto* decrease = addButton(stepRow, QStringLiteral("uiSizeDecrease"), QStringLiteral("-"),
                               tr("Decrease the UI size by one step"));
    auto* readout = new QLabel(row);
    readout->setObjectName(QStringLiteral("uiSizeReadout"));
    readout->setAlignment(Qt::AlignCenter);
    readout->setMinimumWidth(Theme::scaled(44));
    stepRow->addWidget(readout);
    auto* increase = addButton(stepRow, QStringLiteral("uiSizeIncrease"), QStringLiteral("+"),
                               tr("Increase the UI size by one step"));
    stepRow->addStretch(1);
    auto* reset = addButton(layout, QStringLiteral("uiSizeReset"), tr("Reset"),
                            tr("Reset the UI size to 100%"));
    layout->setAlignment(reset, Qt::AlignHCenter);

    connect(decrease, &QToolButton::clicked, this,
            [this]() { registry_->dispatch(command_ids::WindowUiDecrease); });
    connect(increase, &QToolButton::clicked, this,
            [this]() { registry_->dispatch(command_ids::WindowUiEnlarge); });
    connect(reset, &QToolButton::clicked, this,
            [this]() { registry_->dispatch(command_ids::WindowUiReset); });

    auto* widgetAction = new QWidgetAction(windowMenu);
    widgetAction->setDefaultWidget(row);
    windowMenu->insertAction(readoutAction, widgetAction);
    readoutAction->setVisible(false);
    for (const char* id : {command_ids::WindowUiEnlarge, command_ids::WindowUiDecrease,
                           command_ids::WindowUiReset}) {
        if (QAction* action = registry_->action(QString::fromLatin1(id))) {
            // Keep the command on the window rather than in the menu: this
            // hides its menu row and still installs its shortcut.
            windowMenu->removeAction(action);
            addAction(action);
        }
    }
    connect(windowMenu, &QMenu::aboutToShow, this, [this]() {
        registry_->refresh();
        updateUiSizeMenu();
    });
    updateUiSizeMenu();
}

void PicturaMainWindow::updateUiSizeMenu()
{
    if (auto* readout = findChild<QLabel*>(QStringLiteral("uiSizeReadout"))) {
        readout->setText(tr("%1%").arg(uiScalePercent_));
    }
    const auto sync = [this](const char* name, const char* command) {
        if (auto* button = findChild<QToolButton*>(QString::fromLatin1(name))) {
            if (QAction* action = registry_->action(QString::fromLatin1(command))) {
                button->setEnabled(action->isEnabled());
            }
        }
    };
    sync("uiSizeDecrease", command_ids::WindowUiDecrease);
    sync("uiSizeIncrease", command_ids::WindowUiEnlarge);
    sync("uiSizeReset", command_ids::WindowUiReset);
    if (auto* row = findChild<QWidget*>(QStringLiteral("uiSizeControl"))) {
        if (QLayout* layout = row->layout()) {
            // The scale change above resizes the row's fonts, but the layout
            // has not run yet; activate it so the menu reads a current hint.
            layout->activate();
        }
        if (auto* menu = qobject_cast<QMenu*>(row->parentWidget())) {
            if (menu->isVisible()) {
                // The popup keeps its show-time geometry; growing the row would
                // clip it, so refit the open menu to its rows.
                menu->resize(menu->sizeHint());
            }
        }
    }
}

void PicturaMainWindow::setUiScalePercent(int percent)
{
    const int next = Theme::normalizedUiScalePercent(percent);
    if (next == uiScalePercent_) {
        return;
    }
    const int old = uiScalePercent_;
    uiScalePercent_ = next;
    Theme::setUiScalePercent(next);
    applyBundledUiFont();
    setFont(QApplication::font());
    applyBrightness(brightnessLevel_);
    // Applying the theme recreates the style, which resets widget fonts; the
    // tab chrome font is therefore set after it.
    applyTabBarFont(tabs_->tabBar());
    applyUiScaleToChrome(old, next);
    if (preferencesDialog_) {
        preferencesDialog_->setUiScalePercent(next);
    }
    registry_->refresh();
    updateUiSizeMenu();
    saveSession();
}

void PicturaMainWindow::applyStatusBarScale()
{
    QStatusBar* bar = statusBar();
    if (!bar) {
        return;
    }
    bar->setFixedHeight(Theme::scaled(28));
    const int itemHeight = Theme::scaled(22);
    for (QWidget* item : {static_cast<QWidget*>(zoomField_), static_cast<QWidget*>(sizeLabel_),
                          static_cast<QWidget*>(statusOptionsButton_),
                          static_cast<QWidget*>(backendLabel_), static_cast<QWidget*>(actionLabel_),
                          static_cast<QWidget*>(hintBar_), static_cast<QWidget*>(searchHint_)}) {
        if (item) {
            item->setFixedHeight(itemHeight);
        }
    }
    for (QFrame* separator : bar->findChildren<QFrame*>(QStringLiteral("statusSeparator"))) {
        separator->setFixedHeight(Theme::scaled(16));
    }
}

void PicturaMainWindow::applyUiScaleToChrome(int oldPercent, int newPercent)
{
    const double ratio = double(newPercent) / double(oldPercent);
    if (toolbox_) {
        toolbox_->applyUiScale();
    }
    for (PanelColumn* column : allPanelColumns()) {
        if (!column) {
            continue;
        }
        if (column == toolsColumn_) {
            column->applyUiScale();
            continue;
        }
        const int width = column->persistedWidth();
        column->applyUiScale();
        if (width > 0) {
            column->setRestoredWidth(qRound(width * ratio));
        }
    }
    PanelColumn::refreshSharedFloor(this);
    updateColumnFrameEdges();
    applyStatusBarScale();
    if (QWidget* corner = menuBar()->findChild<QWidget*>(QStringLiteral("menuCorner"))) {
        const int side = Theme::scaled(16);
        for (QToolButton* button : corner->findChildren<QToolButton*>()) {
            button->setIconSize(QSize(side, side));
        }
    }
}

} // namespace pictura
