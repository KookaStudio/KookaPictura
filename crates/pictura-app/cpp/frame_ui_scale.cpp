#include "frame_includes.h"
#include "panels/panel_column.h"
#include "menu_registrar.h"

#include "fonts.h"

#include <QtWidgets/QApplication>

namespace pictura {

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
    applyTabBarFont(documents_->tabs()->tabBar());
    applyUiScaleToChrome(old, next);
    if (preferencesDialog_) {
        preferencesDialog_->setUiScalePercent(next);
    }
    commandRegistry()->refresh();
    if (menu_) {
        menu_->updateUiSizeMenu();
    }
    saveSession();
}

void PicturaMainWindow::applyStatusBarScale()
{
    QStatusBar* bar = statusBar();
    if (!bar) {
        return;
    }
    // Footer text runs 2px under the app default, matching the tab chrome. A
    // point-sized default font (pixelSize() < 0) falls back to 12px at 96 DPI.
    int baseFontPx = QApplication::font().pixelSize();
    if (baseFontPx <= 0) {
        baseFontPx = 12;
    }
    QFont footerFont = QApplication::font();
    footerFont.setPixelSize(qMax(1, baseFontPx - 2));
    // An application stylesheet resolves a child's font against the
    // application rather than the bar, so the readouts need it set directly.
    bar->setFont(footerFont);
    bar->setFixedHeight(Theme::scaled(28));
    const int itemHeight = Theme::scaled(22);
    for (QWidget* item : {static_cast<QWidget*>(zoomField_), static_cast<QWidget*>(sizeLabel_),
                          static_cast<QWidget*>(statusOptionsButton_),
                          static_cast<QWidget*>(backendLabel_), static_cast<QWidget*>(actionLabel_),
                          static_cast<QWidget*>(hintBar_), static_cast<QWidget*>(searchHint_)}) {
        if (item) {
            item->setFont(footerFont);
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
        if (column == panels_->toolsColumn()) {
            column->applyUiScale();
            continue;
        }
        const int width = column->persistedWidth();
        column->applyUiScale();
        if (width > 0) {
            column->setRestoredWidth(qRound(width * ratio));
        }
    }
    panels_->refreshSharedFloor();
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
