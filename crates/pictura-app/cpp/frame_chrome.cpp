#include "frame.h"

#include "icons.h"
#include "theme.h"

#include <QtCore/QSize>
#include <QtCore/QUrl>
#include <QtGui/QDesktopServices>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QMenuBar>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QWidget>

namespace pictura {

void PicturaMainWindow::setThemeFamily(int family)
{
    const int clamped = family == 1 ? 1 : 0;
    if (clamped == themeFamily_) {
        return;
    }
    themeFamily_ = clamped;
    Theme::apply(themeFamily_ == 1 ? Theme::ThemeFamily::Light : Theme::ThemeFamily::Dark,
                 brightnessLevel_);
    applyWorkspaceCanvasColor();
    updateThemeCornerIcon();
    refresh();
    saveSession();
}

void PicturaMainWindow::installMenuCornerButtons()
{
    auto* corner = new QWidget(menuBar());
    corner->setObjectName(QStringLiteral("menuCorner"));
    auto* layout = new QHBoxLayout(corner);
    layout->setContentsMargins(0, 0, 6, 0);
    layout->setSpacing(2);

    themeCornerButton_ = new QToolButton(corner);
    themeCornerButton_->setObjectName(QStringLiteral("menuThemeButton"));
    themeCornerButton_->setAutoRaise(true);
    themeCornerButton_->setIconSize(QSize(16, 16));
    themeCornerButton_->setToolTip(tr("Toggle the light / dark theme"));
    connect(themeCornerButton_, &QToolButton::clicked, this,
            [this]() { setThemeFamily(themeFamily_ == 1 ? 0 : 1); });
    layout->addWidget(themeCornerButton_);

    const auto addLink = [&](const QString& iconId, const QString& tip, const QString& url) {
        auto* button = new QToolButton(corner);
        button->setAutoRaise(true);
        button->setIcon(icon(iconId));
        button->setIconSize(QSize(16, 16));
        button->setToolTip(tip);
        connect(button, &QToolButton::clicked, this,
                [url]() { QDesktopServices::openUrl(QUrl(url)); });
        layout->addWidget(button);
    };
    addLink(QStringLiteral("link.discord"), tr("Join the Kooka Pictura Discord"),
            QStringLiteral("https://discord.gg/VFvFgJ3MF5"));
    addLink(QStringLiteral("link.github"), tr("Kooka Pictura on GitHub"),
            QStringLiteral("https://github.com/KookaStudio/KookaPictura"));

    menuBar()->setCornerWidget(corner, Qt::TopRightCorner);
    updateThemeCornerIcon();
}

void PicturaMainWindow::updateThemeCornerIcon()
{
    if (themeCornerButton_) {
        themeCornerButton_->setIcon(icon(themeFamily_ == 1 ? QStringLiteral("theme.moon")
                                                           : QStringLiteral("theme.sun")));
    }
}

} // namespace pictura
