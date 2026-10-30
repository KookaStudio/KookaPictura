#include <QtTest/QtTest>

#include "frame.h"
#include "options_bar.h"
#include "panels/layers_panel.h"
#include "session.h"
#include "theme.h"

#include <QtCore/QFile>
#include <QtCore/QPointer>
#include <QtGui/QPalette>
#include <QtWidgets/QStatusBar>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QTabWidget>

#include "qt_test_support.h"

namespace {

// Records the application style sheet the first time a layers panel and the
// options bar take a child, which is while they build their contents.
class BuildSnapshot : public QObject {
public:
    QString layersPanelSheet;
    QString optionsBarSheet;
    bool sawLayersPanel = false;
    bool sawOptionsBar = false;

protected:
    bool eventFilter(QObject* watched, QEvent* event) override
    {
        if (event->type() != QEvent::ChildAdded) {
            return false;
        }
        if (!sawLayersPanel && qobject_cast<pictura::LayersPanel*>(watched)) {
            sawLayersPanel = true;
            layersPanelSheet = qApp->styleSheet();
        } else if (!sawOptionsBar && qobject_cast<pictura::OptionsBar*>(watched)) {
            sawOptionsBar = true;
            optionsBarSheet = qApp->styleSheet();
        }
        return false;
    }
};

void writeSession(int family, int level)
{
    pictura::SessionState state;
    state.themeFamily = family;
    state.brightnessLevel = level;
    QVERIFY(pictura::saveSession(state));
}

} // namespace

class ThemeApplyTest : public QObject {
    Q_OBJECT

private slots:
    void init();
    void cleanup();
    void secondWindowKeepsTheme();
    void brightnessStepApplies();
    void chromeBuiltUnderRestoredTheme();
    void reapplyAfterExternalStyleSheet();
    void chromeFontsSurvive();

private:
    pictura::test::ScopedStateHome stateHome_;
};

void ThemeApplyTest::init()
{
    QVERIFY(stateHome_.isValid());
    QFile::remove(pictura::sessionFilePath());
    pictura::Theme::setUiScalePercent(pictura::Theme::kDefaultUiScalePercent);
}

void ThemeApplyTest::cleanup()
{
    QFile::remove(pictura::sessionFilePath());
}

void ThemeApplyTest::secondWindowKeepsTheme()
{
    auto first = pictura::test::makeMainWindow();
    const quint64 generation = pictura::Theme::paletteGeneration();
    QStyle* const style = QApplication::style();
    first.reset();

    auto second = pictura::test::makeMainWindow();
    QCOMPARE(pictura::Theme::paletteGeneration(), generation);
    QCOMPARE(QApplication::style(), style);
}

void ThemeApplyTest::brightnessStepApplies()
{
    writeSession(0, 1);
    auto window = pictura::test::makeMainWindow();
    window->show();
    QVERIFY(QTest::qWaitForWindowExposed(window.get()));
    QCOMPARE(window->brightnessLevel(), 1);
    const quint64 generation = pictura::Theme::paletteGeneration();
    const QPalette before = qApp->palette();

    window->setBrightnessLevel(2);

    QCOMPARE(window->brightnessLevel(), 2);
    QVERIFY(pictura::Theme::paletteGeneration() > generation);
    QVERIFY(qApp->palette() != before);
    QCOMPARE(qApp->styleSheet(), pictura::Theme::styleSheet(pictura::Theme::ThemeFamily::Dark, 2));
}

void ThemeApplyTest::chromeBuiltUnderRestoredTheme()
{
    using pictura::Theme;
    // Start from the other family, so the window has to change the theme.
    Theme::apply(Theme::ThemeFamily::Dark, Theme::kDefaultLevel);
    writeSession(1, 2);
    const QString lightSheet = Theme::styleSheet(Theme::ThemeFamily::Light, 2);

    BuildSnapshot snapshot;
    qApp->installEventFilter(&snapshot);
    auto window = pictura::test::makeMainWindow();
    qApp->removeEventFilter(&snapshot);

    QVERIFY(snapshot.sawLayersPanel);
    QVERIFY(snapshot.sawOptionsBar);
    QVERIFY2(snapshot.layersPanelSheet == lightSheet,
             "the layers panel was built before the restored theme was applied");
    QVERIFY2(snapshot.optionsBarSheet == lightSheet,
             "the options bar was built before the restored theme was applied");

    QCOMPARE(qApp->styleSheet(), lightSheet);
    const QPalette built = qApp->palette();
    // Force a full application with the same settings and compare.
    qApp->setStyleSheet(QString());
    Theme::apply(Theme::ThemeFamily::Light, 2);
    QCOMPARE(qApp->styleSheet(), lightSheet);
    QVERIFY(qApp->palette() == built);
}

void ThemeApplyTest::reapplyAfterExternalStyleSheet()
{
    using pictura::Theme;
    Theme::apply(Theme::ThemeFamily::Dark, 1);
    const QString sheet = Theme::styleSheet(Theme::ThemeFamily::Dark, 1);
    QCOMPARE(qApp->styleSheet(), sheet);

    qApp->setStyleSheet(QString());
    Theme::apply(Theme::ThemeFamily::Dark, 1);

    QCOMPARE(qApp->styleSheet(), sheet);
}

void ThemeApplyTest::chromeFontsSurvive()
{
    auto window = pictura::test::makeMainWindow();
    const int basePx = QApplication::font().pixelSize();
    QVERIFY(basePx > 0);

    auto* tabs = window->findChild<QTabWidget*>(QStringLiteral("documentTabs"));
    QVERIFY(tabs != nullptr);
    const QFont tabFont = tabs->tabBar()->font();
    QCOMPARE(tabFont.pixelSize(), qMax(1, basePx - 2));
    QCOMPARE(tabFont.weight(), QFont::Bold);

    QCOMPARE(window->statusBar()->font().pixelSize(), basePx - 2);
}

int main(int argc, char* argv[])
{
    return pictura::test::shardedMain<ThemeApplyTest>(argc, argv);
}

#include "tst_theme_apply.moc"
