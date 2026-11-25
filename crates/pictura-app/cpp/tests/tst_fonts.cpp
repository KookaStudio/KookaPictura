#include <QtTest/QtTest>

#include <QtCore/QFile>
#include <QtCore/QStringList>
#include <QtGui/QFont>
#include <QtGui/QFontDatabase>
#include <QtGui/QFontInfo>
#include <QtWidgets/QApplication>
#include <QtWidgets/QTabBar>

#include "fonts.h"
#include "frame.h"
#include "qt_test_support.h"

namespace {
QString bundledFamily()
{
    return QString::fromLatin1(pictura::kBundledUiFamily);
}
} // namespace

class FontsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void bundledFacesExist();
    void familyHasWeightedStyles();
    void resourceFaceLoads();
    void mediumResolvesToMedium();
    void semiboldResolves();
    void boldResolves();
    void italicsResolve();
    void applicationFontIsBundled();
    void tabBarsAreBold();
};

void FontsTest::initTestCase()
{
    pictura::registerBundledFonts();
    pictura::applyBundledUiFont();
}

void FontsTest::bundledFacesExist()
{
    const QStringList faces = {
        QStringLiteral(":/fonts/NotoSans-Regular.ttf"),
        QStringLiteral(":/fonts/NotoSans-Medium.ttf"),
        QStringLiteral(":/fonts/NotoSans-SemiBold.ttf"),
        QStringLiteral(":/fonts/NotoSans-Bold.ttf"),
        QStringLiteral(":/fonts/NotoSans-Italic.ttf"),
        QStringLiteral(":/fonts/NotoSans-MediumItalic.ttf"),
        QStringLiteral(":/fonts/NotoSans-SemiBoldItalic.ttf"),
        QStringLiteral(":/fonts/NotoSans-BoldItalic.ttf"),
    };
    for (const QString& face : faces) {
        QVERIFY2(QFile(face).exists(), qPrintable(face + QStringLiteral(" missing")));
    }
}

void FontsTest::familyHasWeightedStyles()
{
    const QString family = QString::fromLatin1(pictura::kBundledUiFamily);
    QVERIFY2(QFontDatabase::hasFamily(family), "bundled family not registered");
    const QStringList styles = QFontDatabase::styles(family);
    for (const QString& style :
         {QStringLiteral("Regular"), QStringLiteral("Medium"), QStringLiteral("SemiBold"),
          QStringLiteral("Bold")}) {
        QVERIFY2(styles.contains(style), qPrintable(style + QStringLiteral(" face missing")));
    }
}

void FontsTest::resourceFaceLoads()
{
    // Prove the qrc resource actually loads (not merely that some host Noto
    // Sans exists): the returned application-font id must map to the family.
    const int id = QFontDatabase::addApplicationFont(QStringLiteral(":/fonts/NotoSans-Medium.ttf"));
    QVERIFY2(id >= 0, "addApplicationFont failed for the bundled Medium face");
    QVERIFY2(QFontDatabase::applicationFontFamilies(id).contains(bundledFamily()),
             "bundled Medium face did not register as Noto Sans");
}

void FontsTest::mediumResolvesToMedium()
{
    // The bug this guards: with a family that has no Medium face (e.g. DejaVu
    // Sans) the request snaps to Regular. The bundled face must answer 500.
    QFont font(bundledFamily());
    font.setWeight(QFont::Medium);
    const QFontInfo info(font);
    QCOMPARE(info.family(), bundledFamily());
    QCOMPARE(info.weight(), QFont::Medium);
}

void FontsTest::semiboldResolves()
{
    // The tab chrome asks for 600; the bundled family must answer SemiBold.
    QFont font(bundledFamily());
    font.setWeight(QFont::DemiBold);
    const QFontInfo info(font);
    QCOMPARE(info.family(), bundledFamily());
    QCOMPARE(info.weight(), QFont::DemiBold);
}

void FontsTest::boldResolves()
{
    // The tab chrome asks for 700; the bundled family must answer Bold.
    QFont font(bundledFamily());
    font.setWeight(QFont::Bold);
    const QFontInfo info(font);
    QCOMPARE(info.family(), bundledFamily());
    QCOMPARE(info.weight(), QFont::Bold);
}

void FontsTest::italicsResolve()
{
    QFont font(bundledFamily());
    font.setWeight(QFont::DemiBold);
    font.setItalic(true);
    const QFontInfo info(font);
    QCOMPARE(info.family(), bundledFamily());
    QVERIFY2(info.italic(), "bundled italic did not resolve");
    QCOMPARE(info.weight(), QFont::DemiBold);
}

void FontsTest::applicationFontIsBundled()
{
    const QFont app = QApplication::font();
    QCOMPARE(app.family(), bundledFamily());
    QCOMPARE(app.pixelSize(), pictura::kBundledUiFontPx);
}

void FontsTest::tabBarsAreBold()
{
    // Both chrome tab bars must carry the bold chrome font on the bar itself —
    // the panel bar used to be the only one set programmatically, so the file
    // bar stayed regular. Guard that both are the bundled family at app px − 2.
    pictura::test::ScopedStateHome stateHome;
    QVERIFY(stateHome.isValid());
    const auto window = pictura::test::makeMainWindow();
    QVERIFY(window != nullptr);

    QTabBar* document = window->findChild<QTabBar*>(QStringLiteral("documentTabBar"));
    QTabBar* panel = window->findChild<QTabBar*>(QStringLiteral("panelTabBar"));
    QVERIFY2(document != nullptr, "no document tab bar");
    QVERIFY2(panel != nullptr, "no panel tab bar");

    const int expectedPx = qMax(1, QApplication::font().pixelSize() - 2);
    for (QTabBar* bar : {document, panel}) {
        const QFontInfo info(bar->font());
        QCOMPARE(info.family(), bundledFamily());
        QCOMPARE(info.weight(), QFont::Bold);
        QCOMPARE(bar->font().pixelSize(), expectedPx);
    }
}

PICTURA_QT_TEST_MAIN(FontsTest)
#include "tst_fonts.moc"
