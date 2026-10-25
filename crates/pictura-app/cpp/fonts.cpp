#include "fonts.h"

#include "theme.h"

#include <QtCore/QString>
#include <QtGui/QFontDatabase>
#include <QtWidgets/QApplication>
#include <QtWidgets/QTabBar>

namespace pictura {

const char* const kBundledUiFamily = "Noto Sans";

namespace {

// The bundled faces, resource paths. Weight/style pairing is carried by each
// file's own name/style tables; Qt sorts them into the family on registration.
const char* const kFaces[] = {
    ":/fonts/NotoSans-Regular.ttf",  ":/fonts/NotoSans-Medium.ttf",
    ":/fonts/NotoSans-SemiBold.ttf", ":/fonts/NotoSans-Bold.ttf",
    ":/fonts/NotoSans-Italic.ttf",   ":/fonts/NotoSans-MediumItalic.ttf",
    ":/fonts/NotoSans-SemiBoldItalic.ttf", ":/fonts/NotoSans-BoldItalic.ttf",
};

bool g_registered = false;

} // namespace

void registerBundledFonts()
{
    if (g_registered) {
        return;
    }
    for (const char* path : kFaces) {
        QFontDatabase::addApplicationFont(QString::fromLatin1(path));
    }
    g_registered = true;
}

QFont bundledUiFont()
{
    QFont font(QString::fromLatin1(kBundledUiFamily));
    font.setStyleHint(QFont::SansSerif);
    font.setPixelSize(Theme::scaled(kBundledUiFontPx));
    return font;
}

void applyBundledUiFont()
{
    registerBundledFonts();
    QApplication::setFont(bundledUiFont());
}

void applyTabBarFont(QTabBar* bar)
{
    if (!bar) {
        return;
    }
    registerBundledFonts();
    int baseFontPx = QApplication::font().pixelSize();
    if (baseFontPx <= 0) {
        baseFontPx = Theme::scaled(kBundledUiFontPx);
    }
    QFont tabFont = bar->font();
    tabFont.setPixelSize(qMax(1, baseFontPx - 2));
    tabFont.setWeight(QFont::Bold);
    bar->setFont(tabFont);
}

} // namespace pictura
