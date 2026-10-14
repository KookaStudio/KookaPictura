#include <QtTest/QtTest>

#include <QtCore/QCoreApplication>
#include <QtGui/QGuiApplication>
#include <QtWidgets/QApplication>
#include <QtWidgets/QMenuBar>

#include <memory>

#include "frame.h"

#include "qt_test_support.h"

namespace {

// Qt reports the menu bar as native only when the platform bar is actually in
// use: the macOS system-wide bar under the cocoa plugin. CI runs the suites
// under the offscreen plugin, where the bar is drawn in the window on every
// platform, so the expected value tracks the platform plugin, not the OS.
bool nativeBarExpected()
{
    return QGuiApplication::platformName().contains(QStringLiteral("cocoa"));
}

} // namespace

class NativeMenuBarTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void nativeBarMatchesPlatform();
    void nativeBarUnchangedAcrossScreenModes();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void NativeMenuBarTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->resize(1100, 700);
    window_->show();
    QCoreApplication::processEvents();
}

void NativeMenuBarTest::nativeBarMatchesPlatform()
{
    // The shell SHALL NOT opt out of the native menu bar. On macOS that is the
    // system-wide bar; without one Qt draws the bar inside the window.
    QVERIFY(!QApplication::instance()->testAttribute(Qt::AA_DontUseNativeMenuBar));
    QMenuBar* bar = window_->menuBar();
    QVERIFY(bar != nullptr);
    QCOMPARE(bar->isNativeMenuBar(), nativeBarExpected());
}

void NativeMenuBarTest::nativeBarUnchangedAcrossScreenModes()
{
    using ScreenMode = pictura::PicturaMainWindow::ScreenMode;
    QMenuBar* bar = window_->menuBar();
    QVERIFY(bar != nullptr);
    const bool initial = bar->isNativeMenuBar();

    // Entering or leaving a full-screen mode changes chrome visibility but SHALL
    // NOT switch the platform's menu-bar presentation.
    window_->setScreenMode(ScreenMode::FullWithMenuBar);
    QCOMPARE(bar->isNativeMenuBar(), initial);
    window_->setScreenMode(ScreenMode::Full);
    QCOMPARE(bar->isNativeMenuBar(), initial);
    window_->setScreenMode(ScreenMode::Standard);
    QCOMPARE(bar->isNativeMenuBar(), initial);
}

QTEST_MAIN(NativeMenuBarTest)
#include "tst_native_menu_bar.moc"
