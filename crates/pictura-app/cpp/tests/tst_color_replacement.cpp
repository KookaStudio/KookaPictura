// The Color Replacement tool's full CS6 paint-mode list and its document-mode
// gate (#547): the Mode menu, the 32-bit subset, and the Bitmap/Indexed/
// Multichannel unavailability.

#include <QtTest/QtTest>

#include "api/document_bridge.h"
#include "api/paint_bridge.h"
#include "image_view.h"
#include "options_bar.h"
#include "selftest_paint_fixture.h"
#include "toolbox.h"
#include "tools.h"

#include <QtGui/QAction>
#include <QtGui/QPainter>
#include <QtWidgets/QComboBox>

#include "qt_test_support.h"

namespace {

QComboBox* modeCombo(pictura::PicturaMainWindow& frame)
{
    return frame.findChild<QComboBox*>(QStringLiteral("optionsColorReplaceMode"));
}

QAction* flyoutAction(pictura::Toolbox* toolbox, pictura::ToolId id)
{
    const int group = pictura::toolInfo(id).group;
    for (QAction* action : toolbox->slotMenuActionsForTest(group)) {
        if (action->data().toInt() == int(id)) {
            return action;
        }
    }
    return nullptr;
}

} // namespace

class ColorReplacementTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void modeMenuListsTheCs6PaintModes();
    void thirtyTwoBitDocumentFiltersTheModeList();
    void bitmapDocumentDisablesTheTool();
    void indexedDocumentDisablesTheTool();
    void multichannelModeIsUnavailable();
    void rgbDocumentKeepsTheToolAvailable();
    void repaintAField();

private:
    pictura::PictureView* newDoc(const QString& name, const QString& mode, int bits)
    {
        if (!window_->newDocument(name, 20, 16, mode, bits, QStringLiteral("white"))) {
            return nullptr;
        }
        window_->refresh();
        return window_->activeView();
    }

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void ColorReplacementTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void ColorReplacementTest::cleanup()
{
    window_->setActiveTool(pictura::ToolId::Move);
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
}

void ColorReplacementTest::modeMenuListsTheCs6PaintModes()
{
    QVERIFY(newDoc(QStringLiteral("Modes"), QStringLiteral("rgb"), 8));
    window_->setActiveTool(pictura::ToolId::ColorReplacement);
    QCOMPARE(window_->activeTool(), pictura::ToolId::ColorReplacement);

    QComboBox* mode = modeCombo(*window_);
    QVERIFY(mode);
    QCOMPARE(mode->count(), 25);
    QCOMPARE(mode->itemText(0), QStringLiteral("Normal"));
    QCOMPARE(mode->itemText(2), QStringLiteral("Behind"));
    QCOMPARE(mode->itemText(24), QStringLiteral("Luminosity"));
    QCOMPARE(mode->currentText(), QStringLiteral("Color"));
    QCOMPARE(mode->currentData().toInt(), 23);
}

void ColorReplacementTest::thirtyTwoBitDocumentFiltersTheModeList()
{
    pictura::PictureView* view = newDoc(QStringLiteral("HDR"), QStringLiteral("rgb"), 8);
    QVERIFY(view);
    QVERIFY(pictura::convert_image_depth(*view, 32));
    QCOMPARE(pictura::image_depth_bits(*view), 32);
    window_->setActiveTool(pictura::ToolId::ColorReplacement);

    QComboBox* mode = modeCombo(*window_);
    QVERIFY(mode);
    QCOMPARE(mode->count(), 11);
    QCOMPARE(mode->findText(QStringLiteral("Color Burn")), -1);
    QVERIFY(mode->findText(QStringLiteral("Color")) >= 0);
    QCOMPARE(mode->currentData().toInt(), 23);

    QVERIFY(view->convert_depth(8, 0.0, 1.0));
    window_->refresh();
    QCOMPARE(pictura::image_depth_bits(*view), 8);
    QCOMPARE(mode->count(), 25);
}

void ColorReplacementTest::bitmapDocumentDisablesTheTool()
{
    pictura::PictureView* view = newDoc(QStringLiteral("Bitmap"), QStringLiteral("rgb"), 8);
    QVERIFY(view);
    QVERIFY(pictura::convert_image_mode(*view, QStringLiteral("grayscale")));
    QVERIFY(pictura::convert_to_bitmap(*view, 0));
    QCOMPARE(pictura::image_mode(*view), QStringLiteral("bitmap"));
    QVERIFY(!pictura::toolAllowedForDocument(pictura::ToolId::ColorReplacement, view));

    auto* toolbox = window_->findChild<pictura::Toolbox*>();
    QVERIFY(toolbox);
    QAction* row = flyoutAction(toolbox, pictura::ToolId::ColorReplacement);
    QVERIFY(row);
    QVERIFY(!row->isEnabled());

    window_->setActiveTool(pictura::ToolId::ColorReplacement);
    QCOMPARE(window_->activeTool(), pictura::ToolId::Move);
}

void ColorReplacementTest::indexedDocumentDisablesTheTool()
{
    pictura::PictureView* view = newDoc(QStringLiteral("Indexed"), QStringLiteral("rgb"), 8);
    QVERIFY(view);
    window_->setActiveTool(pictura::ToolId::ColorReplacement);
    QCOMPARE(window_->activeTool(), pictura::ToolId::ColorReplacement);

    QVERIFY(pictura::convert_to_indexed(*view, 0, 256, 0, 0));
    QCOMPARE(pictura::image_mode(*view), QStringLiteral("indexed"));
    window_->refresh();
    QCOMPARE(window_->activeTool(), pictura::ToolId::Move);

    auto* toolbox = window_->findChild<pictura::Toolbox*>();
    QVERIFY(toolbox);
    QAction* row = flyoutAction(toolbox, pictura::ToolId::ColorReplacement);
    QVERIFY(row);
    QVERIFY(!row->isEnabled());
}

void ColorReplacementTest::multichannelModeIsUnavailable()
{
    QVERIFY(!pictura::color_replacement_available(QStringLiteral("multichannel")));
    QVERIFY(pictura::color_replacement_available(QStringLiteral("rgb")));
}

void ColorReplacementTest::rgbDocumentKeepsTheToolAvailable()
{
    pictura::PictureView* view = newDoc(QStringLiteral("RGB"), QStringLiteral("rgb"), 8);
    QVERIFY(view);
    QVERIFY(pictura::toolAllowedForDocument(pictura::ToolId::ColorReplacement, view));

    auto* toolbox = window_->findChild<pictura::Toolbox*>();
    QVERIFY(toolbox);
    QAction* row = flyoutAction(toolbox, pictura::ToolId::ColorReplacement);
    QVERIFY(row);
    QVERIFY(row->isEnabled());

    window_->setActiveTool(pictura::ToolId::ColorReplacement);
    QCOMPARE(window_->activeTool(), pictura::ToolId::ColorReplacement);
}

void ColorReplacementTest::repaintAField()
{
    QImage seed(60, 20, QImage::Format_RGB32);
    seed.fill(QColor(220, 220, 40));
    QPainter(&seed).fillRect(0, 0, 30, 20, QColor(40, 40, 200));
    paint_fixture::Fixture fixture(*window_, seed, QStringLiteral("pictura_color_replacement_modes"));
    QVERIFY2(fixture.ok(), "color replacement fixture");

    window_->setActiveTool(pictura::ToolId::ColorReplacement);
    const QColor foreground = fixture.tools->foreground();
    fixture.tools->setForeground(QColor(40, 200, 40));
    fixture.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setSize(10);
    const int base = fixture.view->history_index();
    fixture.drag({QPointF(8, 10), QPointF(14, 10), QPointF(20, 10)});
    QVERIFY(fixture.committedOnce(base, "Color Replacement Tool"));

    const quint32 argb = fixture.view->sample_argb(14, 10);
    QVERIFY(qGreen(argb) > qRed(argb) + 20);
    QVERIFY(qGreen(argb) > qBlue(argb));
    QCOMPARE(fixture.view->sample_argb(45, 10), QColor(220, 220, 40).rgba());
    fixture.tools->setForeground(foreground);
}

PICTURA_QT_TEST_MAIN(ColorReplacementTest)

#include "tst_color_replacement.moc"
