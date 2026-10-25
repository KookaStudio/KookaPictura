#include <memory>

#include <QtTest/QtTest>

#include <QtCore/QFile>
#include <QtGui/QAction>
#include <QtGui/QFont>
#include <QtWidgets/QApplication>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMenuBar>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QStatusBar>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QWidgetAction>
#include <QtGui/QScreen>
#include <QtWidgets/QSplitter>

#include "commands.h"
#include "fonts.h"
#include "frame.h"
#include "image_view.h"
#include "preferences_dialog.h"
#include "session.h"
#include "theme.h"

#include "qt_test_support.h"

class UiScaleTest : public QObject {
    Q_OBJECT

private slots:
    void init();
    void cleanup();
    void themeScaleContract();
    void fontAndStyleSheetFollowScale();
    void frameChromeFollowsScale();
    void canvasUnaffectedByScale();
    void sessionPersistsScale();
    void windowMenuSingleControl();
    void windowMenuControlSitsBelowOptions();
    void menuBarUsesBundledFont();
    void windowMenuResizesWithScale();
    void windowMenuStepsAndBounds();
    void windowMenuReadout();
    void uiScaleShortcuts();
    void windowStaysResizableAtHighScale();
    void preferencesMirrorScale();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void UiScaleTest::init()
{
    QVERIFY(stateHome_.isValid());
    QFile::remove(pictura::sessionFilePath());
    pictura::Theme::setUiScalePercent(pictura::Theme::kDefaultUiScalePercent);
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void UiScaleTest::cleanup()
{
    window_.reset();
    pictura::Theme::setUiScalePercent(pictura::Theme::kDefaultUiScalePercent);
}

void UiScaleTest::themeScaleContract()
{
    using pictura::Theme;
    QCOMPARE(Theme::uiScalePercent(), 100);
    QCOMPARE(Theme::scaled(12), 12);
    QCOMPARE(Theme::normalizedUiScalePercent(99), 100);
    QCOMPARE(Theme::normalizedUiScalePercent(84), 80);
    QCOMPARE(Theme::normalizedUiScalePercent(250), 200);

    Theme::setUiScalePercent(75);
    QCOMPARE(Theme::uiScalePercent(), 75);
    QCOMPARE(Theme::scaled(1), 1);
    QCOMPARE(Theme::scaled(10), 8);
    QCOMPARE(Theme::scaled(12), 9);
}

void UiScaleTest::fontAndStyleSheetFollowScale()
{
    using pictura::Theme;
    QCOMPARE(QApplication::font().pixelSize(), 12);
    QCOMPARE(QApplication::font().family(), QStringLiteral("Noto Sans"));
    QVERIFY(Theme::styleSheet(Theme::ThemeFamily::Dark, 1)
                .contains(QStringLiteral("padding: 4px 5px;")));

    window_->setUiScalePercent(150);
    QCOMPARE(QApplication::font().pixelSize(), 18);
    auto* tabBar = window_->findChild<QTabBar*>(QStringLiteral("documentTabBar"));
    QVERIFY(tabBar != nullptr);
    QCOMPARE(tabBar->font().pixelSize(), 16);

    Theme::setUiScalePercent(200);
    const QString sheet = Theme::styleSheet(Theme::ThemeFamily::Dark, 1);
    QVERIFY(sheet.contains(QStringLiteral("padding: 8px 10px;")));
}

void UiScaleTest::frameChromeFollowsScale()
{
    using pictura::Theme;
    window_->show();
    QCoreApplication::processEvents();

    QStatusBar* bar = window_->statusBar();
    QVERIFY(bar != nullptr);
    QCOMPARE(bar->minimumHeight(), 28);
    auto* slot = window_->findChild<QToolButton*>(QStringLiteral("toolSlotButton"));
    QVERIFY(slot != nullptr);
    QCOMPARE(slot->width(), 36);
    auto* corner = window_->findChild<QToolButton*>(QStringLiteral("menuThemeButton"));
    QVERIFY(corner != nullptr);
    QCOMPARE(corner->iconSize().width(), 16);
    const int baseFloor = window_->panelColumn()->minimumWidth();

    window_->setUiScalePercent(150);
    QCoreApplication::processEvents();
    QCOMPARE(bar->minimumHeight(), Theme::scaled(28));
    QCOMPARE(slot->width(), Theme::scaled(36));
    QCOMPARE(corner->iconSize().width(), Theme::scaled(16));
    QCOMPARE(pictura::PanelColumn::minNormalWidth(), Theme::scaled(300));
    QCOMPARE(pictura::PanelColumn::maxNormalWidth(), Theme::scaled(400));
    QVERIFY(window_->panelColumn()->minimumWidth() >= baseFloor);
}

void UiScaleTest::canvasUnaffectedByScale()
{
    QVERIFY(window_->newDocument(QStringLiteral("Untitled"), 256, 256,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    pictura::ImageView* canvas = window_->imageView();
    QVERIFY(canvas != nullptr);
    const double zoom = canvas->zoom();
    const QSize imageSize = canvas->size();

    window_->setUiScalePercent(200);
    QCOMPARE(canvas->zoom(), zoom);
    QCOMPARE(canvas->size(), imageSize);
}

void UiScaleTest::sessionPersistsScale()
{
    window_->setUiScalePercent(130);
    QCOMPARE(pictura::loadSession().uiScalePercent, 130);
    QVERIFY(window_->uiScalePercent() == 130);

    window_.reset();
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    QCOMPARE(window_->uiScalePercent(), 130);
}

void UiScaleTest::windowMenuSingleControl()
{
    using namespace pictura::command_ids;
    auto* control = window_->findChild<QWidget*>(QStringLiteral("uiSizeControl"));
    QVERIFY(control != nullptr);
    auto* menu = qobject_cast<QMenu*>(control->parentWidget());
    QVERIFY(menu != nullptr);
    auto* decrease = control->findChild<QToolButton*>(QStringLiteral("uiSizeDecrease"));
    auto* increase = control->findChild<QToolButton*>(QStringLiteral("uiSizeIncrease"));
    auto* reset = control->findChild<QToolButton*>(QStringLiteral("uiSizeReset"));
    auto* readout = control->findChild<QLabel*>(QStringLiteral("uiSizeReadout"));
    auto* caption = control->findChild<QLabel*>(QStringLiteral("uiSizeCaption"));
    QVERIFY(decrease && increase && reset && readout && caption);
    QCOMPARE(caption->text(), QStringLiteral("UI size"));
    QVERIFY(!window_->registry()->action(QString::fromLatin1(WindowUiSize))->isVisible());
    for (const char* id : {WindowUiEnlarge, WindowUiDecrease, WindowUiReset}) {
        QVERIFY(!menu->actions().contains(window_->registry()->action(QString::fromLatin1(id))));
    }

    menu->popup(QPoint(20, 20));
    QVERIFY(menu->isVisible());
    QCoreApplication::processEvents();
    const auto topInControl = [control](QWidget* widget) {
        return widget->mapTo(control, widget->rect().topLeft()).y();
    };
    const int stepY = topInControl(decrease);
    QCOMPARE(topInControl(increase), stepY);
    QCOMPARE(topInControl(readout), stepY);
    QVERIFY(topInControl(caption) < stepY);
    QVERIFY(topInControl(reset) > stepY);

    QVERIFY(increase->isEnabled());
    QVERIFY(window_->registry()->action(QString::fromLatin1(WindowUiEnlarge))->isEnabled());
    increase->click();
    QCOMPARE(window_->uiScalePercent(), 110);
    increase->click();
    QCOMPARE(window_->uiScalePercent(), 120);
    QVERIFY(menu->isVisible());
    QCOMPARE(readout->text(), QStringLiteral("120%"));
    decrease->click();
    QCOMPARE(window_->uiScalePercent(), 110);
    reset->click();
    QCOMPARE(window_->uiScalePercent(), 100);
    QVERIFY(menu->isVisible());
    menu->close();
}

void UiScaleTest::windowMenuControlSitsBelowOptions()
{
    using namespace pictura::command_ids;
    auto* control = window_->findChild<QWidget*>(QStringLiteral("uiSizeControl"));
    QVERIFY(control != nullptr);
    auto* menu = qobject_cast<QMenu*>(control->parentWidget());
    QVERIFY(menu != nullptr);
    QWidgetAction* rowAction = nullptr;
    for (QAction* action : menu->actions()) {
        auto* widgetAction = qobject_cast<QWidgetAction*>(action);
        if (widgetAction != nullptr && widgetAction->defaultWidget() == control) {
            rowAction = widgetAction;
            break;
        }
    }
    QVERIFY(rowAction != nullptr);
    QAction* options = window_->registry()->action(QString::fromLatin1(ViewOptions));
    QVERIFY(options != nullptr);
    const QList<QAction*> actions = menu->actions();
    const int rowIndex = actions.indexOf(rowAction);
    const int optionsIndex = actions.indexOf(options);
    QVERIFY2(rowIndex >= 0 && optionsIndex >= 0, "row and Options are in the Window menu");
    QVERIFY2(rowIndex > optionsIndex, "the UI size row follows Options");
    for (int i = rowIndex + 1; i < actions.size(); ++i) {
        QVERIFY2(!actions.at(i)->isVisible(), "the UI size row is the last visible row");
    }
}

void UiScaleTest::menuBarUsesBundledFont()
{
    auto* control = window_->findChild<QWidget*>(QStringLiteral("uiSizeControl"));
    QVERIFY(control != nullptr);
    auto* menu = qobject_cast<QMenu*>(control->parentWidget());
    QVERIFY(menu != nullptr);
    QCOMPARE(window_->menuBar()->font().family(), QStringLiteral("Noto Sans"));
    QCOMPARE(window_->menuBar()->font().pixelSize(), QApplication::font().pixelSize());
    QCOMPARE(menu->font().family(), window_->menuBar()->font().family());
    QCOMPARE(menu->font().pixelSize(), window_->menuBar()->font().pixelSize());

    window_->setUiScalePercent(150);
    QCOMPARE(window_->menuBar()->font().family(), QStringLiteral("Noto Sans"));
    QCOMPARE(window_->menuBar()->font().pixelSize(), 18);
    QCOMPARE(menu->font().pixelSize(), 18);
}

void UiScaleTest::windowMenuResizesWithScale()
{
    auto* control = window_->findChild<QWidget*>(QStringLiteral("uiSizeControl"));
    QVERIFY(control != nullptr);
    auto* menu = qobject_cast<QMenu*>(control->parentWidget());
    QVERIFY(menu != nullptr);
    auto* increase = control->findChild<QToolButton*>(QStringLiteral("uiSizeIncrease"));
    QVERIFY(increase != nullptr);

    menu->popup(QPoint(20, 20));
    QVERIFY(menu->isVisible());
    const QSize before = menu->size();

    for (int step = 0; step < 5; ++step) {
        increase->click();
    }
    QCOMPARE(window_->uiScalePercent(), 150);

    // No event-loop turn: the refit must have happened by the time the click
    // returns, so the open popup is never left at its show-time size.
    const QSize hint = menu->sizeHint();
    QVERIFY2(hint.height() > before.height(), "the grown row needs a taller menu");
    QVERIFY2(menu->width() >= hint.width() && menu->height() >= hint.height(),
             qPrintable(QStringLiteral("menu=%1x%2 hint=%3x%4")
                            .arg(menu->width())
                            .arg(menu->height())
                            .arg(hint.width())
                            .arg(hint.height())));
    QVERIFY(menu->rect().contains(control->geometry()));
    menu->close();
}

void UiScaleTest::uiScaleShortcuts()
{
    using namespace pictura::command_ids;
    const auto sequence = [](const char* text) { return QKeySequence(QString::fromLatin1(text)); };
    QCOMPARE(window_->registry()->shortcuts(QString::fromLatin1(WindowUiEnlarge)),
             QList<QKeySequence>{sequence("Ctrl+Alt++")});
    QCOMPARE(window_->registry()->shortcuts(QString::fromLatin1(WindowUiDecrease)),
             QList<QKeySequence>{sequence("Ctrl+Alt+-")});
    QCOMPARE(window_->registry()->shortcuts(QString::fromLatin1(WindowUiReset)),
             QList<QKeySequence>{sequence("Ctrl+Alt+0")});
    QCOMPARE(window_->registry()->shortcuts(QString::fromLatin1(ViewZoomIn)),
             QList<QKeySequence>{sequence("Ctrl++")});
    QCOMPARE(window_->registry()->shortcuts(QString::fromLatin1(ViewZoomOut)),
             QList<QKeySequence>{sequence("Ctrl+-")});
    QCOMPARE(window_->registry()->shortcuts(QString::fromLatin1(ViewFitOnScreen)),
             QList<QKeySequence>{sequence("Ctrl+0")});

    window_->show();
    window_->activateWindow();
    QVERIFY(QTest::qWaitForWindowActive(window_.get()));
    QTest::keyClick(window_.get(), Qt::Key_Plus,
                    Qt::ControlModifier | Qt::AltModifier);
    QCOMPARE(window_->uiScalePercent(), 110);
    QTest::keyClick(window_.get(), Qt::Key_0,
                    Qt::ControlModifier | Qt::AltModifier);
    QCOMPARE(window_->uiScalePercent(), 100);
}

void UiScaleTest::windowStaysResizableAtHighScale()
{
    window_->show();
    QCoreApplication::processEvents();
    window_->setUiScalePercent(200);
    QCoreApplication::processEvents();

    auto* scroll = window_->findChild<QScrollArea*>(QStringLiteral("toolsScroll"));
    QVERIFY(scroll != nullptr);
    QVERIFY(scroll->isVisible());
    const int screenHeight = QGuiApplication::primaryScreen()->availableGeometry().height();
    QVERIFY2(window_->minimumSizeHint().height() < screenHeight,
             qPrintable(QStringLiteral("minHeight=%1 screen=%2")
                            .arg(window_->minimumSizeHint().height())
                            .arg(screenHeight)));

    QCOMPARE(window_->maximumSize(), QSize(QWIDGETSIZE_MAX, QWIDGETSIZE_MAX));
    window_->resize(800, 500);
    QCoreApplication::processEvents();
    QCOMPARE(window_->height(), 500);
    window_->resize(700, 450);
    QCoreApplication::processEvents();
    QCOMPARE(window_->height(), 450);
}

void UiScaleTest::windowMenuStepsAndBounds()
{
    using namespace pictura::command_ids;
    window_->setUiScalePercent(100);
    QVERIFY(window_->registry()->dispatch(QString::fromLatin1(WindowUiEnlarge)));
    QCOMPARE(window_->uiScalePercent(), 110);

    window_->setUiScalePercent(90);
    QVERIFY(window_->registry()->dispatch(QString::fromLatin1(WindowUiDecrease)));
    QCOMPARE(window_->uiScalePercent(), 80);

    auto* decrease = window_->findChild<QToolButton*>(QStringLiteral("uiSizeDecrease"));
    auto* increase = window_->findChild<QToolButton*>(QStringLiteral("uiSizeIncrease"));
    auto* reset = window_->findChild<QToolButton*>(QStringLiteral("uiSizeReset"));
    QVERIFY(decrease && increase && reset);

    window_->setUiScalePercent(100);
    QVERIFY(increase->isEnabled() && decrease->isEnabled() && !reset->isEnabled());
    QVERIFY(window_->registry()->action(QString::fromLatin1(WindowUiEnlarge))->isEnabled());
    QVERIFY(window_->registry()->action(QString::fromLatin1(WindowUiDecrease))->isEnabled());
    QVERIFY(!window_->registry()->action(QString::fromLatin1(WindowUiReset))->isEnabled());

    window_->setUiScalePercent(75);
    QVERIFY(!decrease->isEnabled());
    QVERIFY(!window_->registry()->action(QString::fromLatin1(WindowUiDecrease))->isEnabled());
    window_->setUiScalePercent(200);
    QVERIFY(!increase->isEnabled());
    QVERIFY(!window_->registry()->action(QString::fromLatin1(WindowUiEnlarge))->isEnabled());
    QCOMPARE(reset->text(), QStringLiteral("Reset"));
    QVERIFY(reset->isEnabled());
    window_->setUiScalePercent(100);
    QVERIFY(!reset->isEnabled());
}

void UiScaleTest::windowMenuReadout()
{
    using namespace pictura::command_ids;
    window_->setUiScalePercent(130);
    QAction* readout =
        window_->registry()->action(QString::fromLatin1(WindowUiSize));
    QVERIFY(readout != nullptr);
    QCOMPARE(readout->text(), QStringLiteral("UI Size: 130%"));
    auto* label = window_->findChild<QLabel*>(QStringLiteral("uiSizeReadout"));
    QVERIFY(label != nullptr);
    QCOMPARE(label->text(), QStringLiteral("130%"));
}

void UiScaleTest::preferencesMirrorScale()
{
    using namespace pictura::command_ids;
    QVERIFY(window_->registry()->dispatch(QString::fromLatin1(EditPreferencesInterface)));
    pictura::PreferencesDialog* dialog = window_->preferencesDialog();
    QVERIFY(dialog != nullptr);
    auto* combo = dialog->findChild<QComboBox*>(QStringLiteral("preferencesUiScale"));
    QVERIFY(combo != nullptr);

    QCOMPARE(combo->count(), pictura::Theme::uiScaleChoices().size());
    QCOMPARE(combo->itemData(0).toInt(), 75);
    QCOMPARE(combo->itemData(1).toInt(), 80);
    QCOMPARE(combo->itemData(combo->count() - 1).toInt(), 200);

    combo->setCurrentIndex(combo->findData(120));
    QCOMPARE(window_->uiScalePercent(), 120);

    QVERIFY(window_->registry()->dispatch(QString::fromLatin1(WindowUiEnlarge)));
    QCOMPARE(window_->uiScalePercent(), 130);
    QCOMPARE(combo->currentData().toInt(), 130);
}

QTEST_MAIN(UiScaleTest)
#include "tst_ui_scale.moc"
