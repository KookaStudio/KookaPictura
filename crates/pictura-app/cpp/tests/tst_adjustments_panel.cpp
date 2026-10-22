#include <QtTest/QtTest>
#include "api/layers_api.h"

#include "frame.h"
#include "panels/adjustments_panel.h"
#include "panels/layers_panel.h"
#include "panels/properties_panel.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtWidgets/QScrollArea>
#include <QtWidgets/QScrollBar>
#include <QtWidgets/QToolButton>

#include "qt_test_support.h"

class AdjustmentsPanelTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void gridShowsAllNineteenInOrder();
    void gridEntriesCarryTooltips();
    void clickAdjustmentCreatesAndSelectsLayer();
    void repeatClickSelectsNewestLayer();
    void adjustmentCreateIsUndoable();
    void solidColorIconDispatchesFillCommand();
    void patternIconIsInert();
    void noDocumentDisablesGrid();
    void documentEnablesGrid();
    void narrowPanelWrapsGrid();

private:
    pictura::AdjustmentsPanel* panel() const;
    pictura::PictureView* newDocument();

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void AdjustmentsPanelTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void AdjustmentsPanelTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
}

pictura::AdjustmentsPanel* AdjustmentsPanelTest::panel() const
{
    return window_->findChild<pictura::AdjustmentsPanel*>(QStringLiteral("adjustmentsPanel"));
}

pictura::PictureView* AdjustmentsPanelTest::newDocument()
{
    if (!window_->newDocument(QStringLiteral("Adjust"), 16, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return nullptr;
    }
    QCoreApplication::processEvents();
    return window_->activeView();
}

// The CS6 grid: the three fill types first, then the sixteen adjustments in
// panel order. Buttons are icon-only; each carries a tooltip naming the type.
void AdjustmentsPanelTest::gridShowsAllNineteenInOrder()
{
    QVERIFY(newDocument());
    QVERIFY(panel());
    const QStringList expected = {
        QStringLiteral("solid-color"),       QStringLiteral("gradient"),
        QStringLiteral("pattern"),           QStringLiteral("brightness-contrast"),
        QStringLiteral("levels"),            QStringLiteral("curves"),
        QStringLiteral("exposure"),          QStringLiteral("vibrance"),
        QStringLiteral("hue-saturation"),    QStringLiteral("color-balance"),
        QStringLiteral("black-white"),       QStringLiteral("photo-filter"),
        QStringLiteral("channel-mixer"),     QStringLiteral("color-lookup"),
        QStringLiteral("invert"),            QStringLiteral("posterize"),
        QStringLiteral("threshold"),          QStringLiteral("gradient-map"),
        QStringLiteral("selective-color"),
    };
    QCOMPARE(panel()->buttonCountForTest(), 19);
    QCOMPARE(panel()->buttonIdsForTest(), expected);
    for (int i = 0; i < panel()->buttonCountForTest(); ++i) {
        QVERIFY2(panel()->buttonForTest(i)->text().isEmpty(),
                 qPrintable(QString::number(i)));
    }
}

void AdjustmentsPanelTest::gridEntriesCarryTooltips()
{
    QVERIFY(newDocument());
    QVERIFY(panel());
    const QStringList tips = panel()->buttonToolTipsForTest();
    QCOMPARE(tips.size(), 19);
    for (int i = 0; i < tips.size(); ++i) {
        QVERIFY2(!tips.at(i).isEmpty(), qPrintable(QString::number(i)));
    }
    QVERIFY(tips.at(2).contains(QStringLiteral("not implemented yet")));
    QCOMPARE(tips.at(4), QStringLiteral("Levels"));
    QCOMPARE(tips.at(8), QStringLiteral("Hue/Saturation"));
}

void AdjustmentsPanelTest::clickAdjustmentCreatesAndSelectsLayer()
{
    struct Kind {
        const char* id;
        const char* name;
    };
    const Kind kinds[] = {
        {"brightness-contrast", "Brightness/Contrast"},
        {"levels", "Levels"},
        {"curves", "Curves"},
        {"exposure", "Exposure"},
        {"vibrance", "Vibrance"},
        {"hue-saturation", "Hue/Saturation"},
        {"color-balance", "Color Balance"},
        {"black-white", "Black & White"},
        {"photo-filter", "Photo Filter"},
        {"channel-mixer", "Channel Mixer"},
        {"color-lookup", "Color Lookup"},
        {"invert", "Invert"},
        {"posterize", "Posterize"},
        {"threshold", "Threshold"},
        {"gradient-map", "Gradient Map"},
        {"selective-color", "Selective Color"},
    };
    for (const Kind& kind : kinds) {
        QVERIFY(newDocument());
        pictura::PictureView* view = window_->activeView();
        QVERIFY(view);
        auto* layers = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
        auto* props = window_->findChild<pictura::PropertiesPanel*>(QStringLiteral("propertiesPanel"));
        QVERIFY(layers && props);

        const int before = pictura::layers_api::rowCount(*view);
        QToolButton* button =
            panel()->buttonByIdForTest(QString::fromLatin1(kind.id));
        QVERIFY2(button, qPrintable(QString::fromLatin1(kind.id)));
        button->click();
        QCoreApplication::processEvents();

        QCOMPARE(pictura::layers_api::rowCount(*view), before + 1);
        QString path;
        for (int i = 0; i < pictura::layers_api::rowCount(*view); ++i) {
            const pictura::layers_api::LayerRow row = pictura::layers_api::rowAt(*view, i);
            if (row.hasAdjustment && row.name == QString::fromLatin1(kind.name)) {
                path = row.path;
                break;
            }
        }
        QVERIFY2(!path.isEmpty(), qPrintable(QString::fromLatin1(kind.name)));
        QCOMPARE(layers->currentPath(), path);
        QCOMPARE(props->pathForTest(), path);

        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
}

void AdjustmentsPanelTest::repeatClickSelectsNewestLayer()
{
    QVERIFY(newDocument());
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view);
    auto* layers = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(layers);

    panel()->buttonByIdForTest(QStringLiteral("levels"))->click();
    QCoreApplication::processEvents();
    const QString first = layers->currentPath();
    QVERIFY(!first.isEmpty());

    panel()->buttonByIdForTest(QStringLiteral("levels"))->click();
    QCoreApplication::processEvents();
    const QString second = layers->currentPath();
    QVERIFY(!second.isEmpty());
    QVERIFY(first != second);
    // Rows are topmost-first; the newest Level layer is the top row.
    QCOMPARE(pictura::layers_api::rowAt(*view, 0).name, QStringLiteral("Levels"));
    QCOMPARE(pictura::layers_api::rowAt(*view, 0).path, second);
}

void AdjustmentsPanelTest::adjustmentCreateIsUndoable()
{
    QVERIFY(newDocument());
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view);
    const int before = pictura::layers_api::rowCount(*view);
    panel()->buttonByIdForTest(QStringLiteral("levels"))->click();
    QCoreApplication::processEvents();
    QCOMPARE(pictura::layers_api::rowCount(*view), before + 1);
    QVERIFY(view->undo());
    QCOMPARE(pictura::layers_api::rowCount(*view), before);
}

void AdjustmentsPanelTest::solidColorIconDispatchesFillCommand()
{
    QVERIFY(newDocument());
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view);
    const int before = pictura::layers_api::rowCount(*view);
    panel()->buttonByIdForTest(QStringLiteral("solid-color"))->click();
    QCoreApplication::processEvents();
    QCOMPARE(pictura::layers_api::rowCount(*view), before + 1);
}

void AdjustmentsPanelTest::patternIconIsInert()
{
    QVERIFY(newDocument());
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view);
    const int before = pictura::layers_api::rowCount(*view);
    panel()->buttonByIdForTest(QStringLiteral("pattern"))->click();
    QCoreApplication::processEvents();
    QCOMPARE(pictura::layers_api::rowCount(*view), before);
    QVERIFY(!panel()->buttonByIdForTest(QStringLiteral("pattern"))->isEnabled());
}

void AdjustmentsPanelTest::noDocumentDisablesGrid()
{
    QVERIFY(panel());
    QVERIFY(window_->documentCount() == 0);
    for (int i = 0; i < panel()->buttonCountForTest(); ++i) {
        QVERIFY2(!panel()->buttonForTest(i)->isEnabled(), qPrintable(QString::number(i)));
    }
}

void AdjustmentsPanelTest::documentEnablesGrid()
{
    QVERIFY(newDocument());
    for (int i = 0; i < panel()->buttonCountForTest(); ++i) {
        if (i == 2) {
            QVERIFY2(!panel()->buttonForTest(i)->isEnabled(),
                     qPrintable(QString::number(i)));
        } else {
            QVERIFY2(panel()->buttonForTest(i)->isEnabled(),
                     qPrintable(QString::number(i)));
        }
    }
}

void AdjustmentsPanelTest::narrowPanelWrapsGrid()
{
    QVERIFY(newDocument());
    QVERIFY(panel());
    auto* scroll = panel()->findChild<QScrollArea*>();
    QVERIFY(scroll);
    // Icon-only buttons keep a 3-column grid narrower than any dock can grow,
    // so force a width where the columns cannot fit and assert scrolling keeps
    // every entry reachable.
    panel()->resize(60, 400);
    QCoreApplication::processEvents();
    QVERIFY(scroll->horizontalScrollBar()->maximum() > 0);
}

QTEST_MAIN(AdjustmentsPanelTest)
#include "tst_adjustments_panel.moc"
