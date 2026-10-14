#include <QtTest/QtTest>

#include <QtCore/QSet>
#include <QtGui/QImage>
#include <QtGui/QPalette>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFrame>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHeaderView>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMenu>
#include <QtWidgets/QPlainTextEdit>
#include <QtWidgets/QProgressBar>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QScrollBar>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QSplitter>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QWidget>

#include "design_system_dialog.h"
#include "theme.h"

using pictura::DesignSystemDialog;
using pictura::Theme;

// The dialog is the design system by construction: its pages are the real
// widgets, so these checks assert structure and the live brightness contract,
// not pixels.
class DesignSystemDialogTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();

    void storiesAreListed();
    void everyPageBuilds();
    void curatedPrimitivesArePresent();
    void filterNarrowsStories();
    void brightnessPreviewsAndRestores();
    void reopenRefreshesRestoreSnapshot();
    void foundationsFollowTheFamily();
    void pagesRender();
};

void DesignSystemDialogTest::initTestCase()
{
    Theme::apply(Theme::kDefaultLevel);
}

void DesignSystemDialogTest::storiesAreListed()
{
    DesignSystemDialog dialog;
    QVERIFY2(dialog.storyCountForTest() >= 30, "the registry covers the design system");
    const QStringList groups = dialog.groupsForTest();
    for (const char* group : {"Foundations", "Shell", "Controls", "Patterns"}) {
        QVERIFY2(groups.contains(QLatin1String(group)), group);
    }
    QCOMPARE(dialog.visibleStoryCountForTest(), dialog.storyCountForTest());
}

void DesignSystemDialogTest::everyPageBuilds()
{
    DesignSystemDialog dialog;
    dialog.buildAllPagesForTest();
    QVERIFY2(dialog.storyCountForTest() >= 30, "the registry covers the design system");
    QCOMPARE(dialog.builtPageCountForTest(), dialog.storyCountForTest());
}

void DesignSystemDialogTest::curatedPrimitivesArePresent()
{
    DesignSystemDialog dialog;
    dialog.buildAllPagesForTest();

    QVERIFY(dialog.findChild<QPushButton*>() != nullptr);
    QVERIFY(dialog.findChild<QToolButton*>() != nullptr);
    QVERIFY(dialog.findChild<QLineEdit*>() != nullptr);
    QVERIFY(dialog.findChild<QPlainTextEdit*>() != nullptr);
    QVERIFY(dialog.findChild<QSpinBox*>() != nullptr);
    QVERIFY(dialog.findChild<QComboBox*>() != nullptr);
    QVERIFY(dialog.findChild<QCheckBox*>() != nullptr);
    QVERIFY(dialog.findChild<QRadioButton*>() != nullptr);
    QVERIFY(dialog.findChild<QSlider*>() != nullptr);
    QVERIFY(dialog.findChild<QProgressBar*>() != nullptr);
    QVERIFY(dialog.findChild<QGroupBox*>() != nullptr);
    QVERIFY(dialog.findChild<QHeaderView*>() != nullptr);
    QVERIFY(dialog.findChild<QScrollBar*>() != nullptr);
    QVERIFY(dialog.findChild<QTabBar*>() != nullptr);
    QVERIFY(dialog.findChild<QMenu*>() != nullptr);
    QVERIFY(dialog.findChild<QSplitter*>() != nullptr);

    // Object-scoped primitives keep their identity for the global stylesheet.
    QVERIFY(dialog.findChild<QWidget*>(QStringLiteral("colorSwatch")) != nullptr);
    QVERIFY(dialog.findChild<QWidget*>(QStringLiteral("numericField")) != nullptr);
    QVERIFY(dialog.findChild<QWidget*>(QStringLiteral("panelHeaderBand")) != nullptr);

    // QToolTip is a top-level popup; assert something carries a tooltip instead.
    bool hasTooltip = false;
    for (QWidget* widget : dialog.findChildren<QWidget*>()) {
        if (!widget->toolTip().isEmpty()) {
            hasTooltip = true;
            break;
        }
    }
    QVERIFY2(hasTooltip, "a control demonstrates QToolTip");
}

void DesignSystemDialogTest::filterNarrowsStories()
{
    DesignSystemDialog dialog;
    const int all = dialog.visibleStoryCountForTest();
    dialog.setFilterForTest(QStringLiteral("button"));
    const int filtered = dialog.visibleStoryCountForTest();
    QVERIFY2(filtered > 0 && filtered < all, "the filter hides non-matching stories");
    dialog.setFilterForTest(QString());
    QCOMPARE(dialog.visibleStoryCountForTest(), all);
}

void DesignSystemDialogTest::brightnessPreviewsAndRestores()
{
    const int original = Theme::currentLevel();
    DesignSystemDialog dialog;
    QCOMPARE(dialog.currentLevelForTest(), original);

    dialog.setLevelForTest(3);
    QCOMPARE(Theme::currentLevel(), 3);

    dialog.setLevelForTest(0);
    QCOMPARE(Theme::currentLevel(), 0);

    // Closing restores the level that was in effect when the dialog opened, so a
    // preview never changes the user's brightness preference.
    dialog.close();
    QCOMPARE(Theme::currentLevel(), original);
}

void DesignSystemDialogTest::reopenRefreshesRestoreSnapshot()
{
    // Guard: the dialog is reused, so reopening must re-anchor the restore
    // snapshot to the theme in effect now, not the one captured at first open.
    const int original = Theme::currentLevel();
    Theme::apply(original);

    DesignSystemDialog dialog;
    dialog.show();
    QCoreApplication::processEvents();
    QCOMPARE(dialog.currentLevelForTest(), original);

    // Preview a different level, then close: the user's level is restored.
    dialog.setLevelForTest(original == 0 ? 1 : 0);
    dialog.close();
    QCOMPARE(Theme::currentLevel(), original);

    // The user changes brightness while the dialog is hidden.
    const int changed = original == 0 ? 2 : 0;
    Theme::apply(changed);

    // Reopening re-anchors to the new level; closing keeps it.
    dialog.show();
    QCoreApplication::processEvents();
    QCOMPARE(dialog.currentLevelForTest(), changed);
    dialog.close();
    QCOMPARE(Theme::currentLevel(), changed);

    Theme::apply(original);
}

void DesignSystemDialogTest::foundationsFollowTheFamily()
{
    const int original = Theme::currentLevel();
    Theme::apply(Theme::ThemeFamily::Light, Theme::kDefaultLevel);
    const QColor lightWorkspace = Theme::tokenColor(QStringLiteral("workspace"));
    {
        DesignSystemDialog dialog;
        QVERIFY(dialog.selectStoryForTest(QStringLiteral("Palette")));
        const auto chips = dialog.findChildren<QFrame*>(QStringLiteral("dsChip"));
        QVERIFY(!chips.isEmpty());
        bool found = false;
        for (QFrame* chip : chips) {
            if (chip->palette().color(QPalette::Window) == lightWorkspace) {
                found = true;
                break;
            }
        }
        QVERIFY2(found, "the palette chips paint the light family's tokens");
    }
    Theme::apply(original);
}

void DesignSystemDialogTest::pagesRender()
{
    DesignSystemDialog dialog;
    dialog.resize(1000, 700);

    // The palette page paints several distinct token colours; a non-blank grab
    // proves the chips render through QPalette rather than a stylesheet.
    QVERIFY(dialog.selectStoryForTest(QStringLiteral("Palette")));
    const QImage palette = dialog.grab().toImage();
    QVERIFY(!palette.isNull());
    QSet<QRgb> colors;
    for (int y = 0; y < palette.height(); y += 4) {
        for (int x = 0; x < palette.width(); x += 4) {
            colors.insert(palette.pixel(x, y) & 0x00ffffffu);
        }
    }
    QVERIFY2(colors.size() > 5, "the palette renders multiple token colours");

    // The Menu page embeds a real QMenu as a child widget; it must render too.
    QVERIFY(dialog.selectStoryForTest(QStringLiteral("Menu")));
    QVERIFY(!dialog.grab().toImage().isNull());
}

QTEST_MAIN(DesignSystemDialogTest)
#include "tst_design_system_dialog.moc"
