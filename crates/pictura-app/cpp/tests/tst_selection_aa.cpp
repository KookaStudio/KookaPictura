// Anti-aliased selection tools: the options bar, the ellipse and lasso commits,
// the Magic Wand's inferred fringe, and the 56-channel cap refusal.

#include <QtTest/QtTest>

#include "api/layers_bridge.h"
#include "api/selection_bridge.h"
#include "commands.h"
#include "frame.h"
#include "options_bar.h"
#include "panels/channels_panel.h"
#include "selftest_paint_fixture.h"
#include "tools.h"

#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QStatusBar>
#include <QtWidgets/QToolButton>

#include <memory>

#include "qt_test_support.h"

namespace {

QImage white(int width, int height)
{
    QImage image(width, height, QImage::Format_RGB32);
    image.fill(Qt::white);
    return image;
}

QImage gradient(int width, int height)
{
    QImage image(width, height, QImage::Format_RGB32);
    for (int y = 0; y < height; ++y) {
        for (int x = 0; x < width; ++x) {
            image.setPixelColor(x, y, QColor(qMin(255, x * 4), 0, 0));
        }
    }
    return image;
}

QImage twoRamps(int width, int height)
{
    QImage image(width, height, QImage::Format_RGB32);
    for (int y = 0; y < height; ++y) {
        for (int x = 0; x < width; ++x) {
            const int v = x < 8 ? x * 4 : ((x >= 16 && x < 24) ? (x - 16) * 4 : 255);
            image.setPixelColor(x, y, QColor(v, v == 255 ? 255 : 0, v == 255 ? 255 : 0));
        }
    }
    return image;
}

bool hasPartial(pictura::PictureView* view, int width, int height)
{
    for (int y = 0; y < height; ++y) {
        for (int x = 0; x < width; ++x) {
            const int coverage = view->selection_coverage(x, y);
            if (coverage > 0 && coverage < 255) {
                return true;
            }
        }
    }
    return false;
}

bool allBinary(pictura::PictureView* view, int width, int height)
{
    for (int y = 0; y < height; ++y) {
        for (int x = 0; x < width; ++x) {
            const int coverage = view->selection_coverage(x, y);
            if (coverage != 0 && coverage != 255) {
                return false;
            }
        }
    }
    return true;
}

QImage blackSquare(int width, int height)
{
    QImage image = white(width, height);
    for (int y = 10; y < 30; ++y) {
        for (int x = 10; x < 30; ++x) {
            image.setPixelColor(x, y, QColor(Qt::black));
        }
    }
    return image;
}

int softCount(pictura::PictureView* view, int width, int height)
{
    int count = 0;
    for (int y = 0; y < height; ++y) {
        for (int x = 0; x < width; ++x) {
            const int coverage = view->selection_coverage(x, y);
            if (coverage > 0 && coverage < 255) {
                ++count;
            }
        }
    }
    return count;
}

pictura::SelectionOptions* selectionOptions(paint_fixture::Fixture& f, pictura::ToolId id)
{
    f.frame.setActiveTool(id);
    return f.tools->optionsFor<pictura::SelectionOptions>(id);
}

void dragEllipse(paint_fixture::Fixture& f, const QPointF& from, const QPointF& to)
{
    f.canvas->mousePressed(from, Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseMoved(to);
    f.canvas->mouseReleased(to);
}

void dragLasso(paint_fixture::Fixture& f)
{
    const QPoint points[] = {QPoint(3, 3), QPoint(27, 5), QPoint(23, 27), QPoint(5, 21)};
    f.canvas->mousePressed(QPointF(points[0]), Qt::LeftButton, int(Qt::NoModifier));
    for (int i = 1; i < 4; ++i) {
        f.canvas->mouseMoved(QPointF(points[i]));
    }
    f.canvas->mouseReleased(QPointF(points[3]));
}

void clickCanvas(paint_fixture::Fixture& f, qreal x, qreal y)
{
    f.canvas->mousePressed(QPointF(x, y), Qt::LeftButton, int(Qt::NoModifier));
    f.canvas->mouseReleased(QPointF(x, y));
}

} // namespace

class SelectionAaTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void optionsEnableAntiAlias();
    void antiAliasedEllipseCommitsPartialCoverage();
    void lassoAntiAliasCommitsPartialCoverage();
    void polygonalAntiAliasFollowsThePolygon();
    void magneticAntiAliasCommitsPartialCoverage();
    void wandAntiAliasFringe();
    void wandAntiAliasGlobalFringe();
    void antiAliasCombineAndFeather();
    void saveSelectionRefusesAtCap();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void SelectionAaTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void SelectionAaTest::optionsEnableAntiAlias()
{
    auto* bar = window_->findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(bar && tools);
    const auto checkbox = [bar](pictura::ToolId id, const char* name) -> QCheckBox* {
        QWidget* page = bar->pageForTest(id);
        return page ? page->findChild<QCheckBox*>(QString::fromLatin1(name)) : nullptr;
    };

    for (pictura::ToolId id : {pictura::ToolId::EllipticalMarquee, pictura::ToolId::Lasso,
                               pictura::ToolId::PolygonalLasso, pictura::ToolId::MagneticLasso}) {
        QCheckBox* aa = checkbox(id, "optionsSelectionAntiAlias");
        QVERIFY2(aa && aa->isEnabled() && aa->isChecked(), "selection Anti-alias is enabled");
    }
    QCheckBox* wand = checkbox(pictura::ToolId::MagicWand, "optionsWandAntiAlias");
    QVERIFY(wand && wand->isEnabled() && wand->isChecked());
    QCheckBox* sample = checkbox(pictura::ToolId::MagicWand, "optionsWandSampleAllLayers");
    QVERIFY(sample && !sample->isEnabled());
    QVERIFY(!checkbox(pictura::ToolId::Marquee, "optionsSelectionAntiAlias"));

    pictura::SelectionOptions* ellipse =
        tools->optionsFor<pictura::SelectionOptions>(pictura::ToolId::EllipticalMarquee);
    pictura::SelectionOptions* lasso =
        tools->optionsFor<pictura::SelectionOptions>(pictura::ToolId::Marquee);
    pictura::SelectionOptions* wandOptions =
        tools->optionsFor<pictura::SelectionOptions>(pictura::ToolId::MagicWand);
    QVERIFY(ellipse && lasso && wandOptions);

    checkbox(pictura::ToolId::EllipticalMarquee, "optionsSelectionAntiAlias")->setChecked(false);
    QVERIFY(!ellipse->antiAlias());
    checkbox(pictura::ToolId::EllipticalMarquee, "optionsSelectionAntiAlias")->setChecked(true);
    QVERIFY(ellipse->antiAlias());
    checkbox(pictura::ToolId::Lasso, "optionsSelectionAntiAlias")->setChecked(false);
    QVERIFY(!lasso->antiAlias());
    wand->setChecked(false);
    QVERIFY(!wandOptions->antiAlias());
}

void SelectionAaTest::antiAliasedEllipseCommitsPartialCoverage()
{
    paint_fixture::Fixture f(*window_, white(32, 24), QStringLiteral("pictura_selection_aa_ellipse"));
    QVERIFY2(f.ok(), "selection fixture");
    auto* options = selectionOptions(f, pictura::ToolId::EllipticalMarquee);
    QVERIFY(options);
    options->setCombineMode(pictura::SelectionMode::New);
    options->setMarqueeStyle(pictura::MarqueeStyle::Normal);
    options->setFeather(0.0);
    options->setAntiAlias(true);
    dragEllipse(f, QPointF(3, 3), QPointF(25, 19));
    QVERIFY(f.view->has_selection());
    QVERIFY(hasPartial(f.view, 32, 24));

    options->setAntiAlias(false);
    f.view->deselect();
    dragEllipse(f, QPointF(3, 3), QPointF(25, 19));
    QVERIFY(f.view->has_selection());
    QVERIFY(allBinary(f.view, 32, 24));
}

void SelectionAaTest::lassoAntiAliasCommitsPartialCoverage()
{
    paint_fixture::Fixture f(*window_, white(32, 32), QStringLiteral("pictura_selection_aa_lasso"));
    QVERIFY2(f.ok(), "selection fixture");
    auto* options = selectionOptions(f, pictura::ToolId::Lasso);
    QVERIFY(options);
    options->setCombineMode(pictura::SelectionMode::New);
    options->setFeather(0.0);
    options->setAntiAlias(true);
    dragLasso(f);
    QVERIFY(f.view->has_selection());
    QVERIFY(hasPartial(f.view, 32, 32));

    options->setAntiAlias(false);
    f.view->deselect();
    dragLasso(f);
    QVERIFY(f.view->has_selection());
    QVERIFY(allBinary(f.view, 32, 32));
}

void SelectionAaTest::polygonalAntiAliasFollowsThePolygon()
{
    paint_fixture::Fixture f(*window_, white(32, 32), QStringLiteral("pictura_selection_aa_polygon"));
    QVERIFY2(f.ok(), "selection fixture");
    auto* options = selectionOptions(f, pictura::ToolId::PolygonalLasso);
    QVERIFY(options);
    options->setCombineMode(pictura::SelectionMode::New);
    options->setFeather(0.0);
    options->setAntiAlias(true);
    clickCanvas(f, 3, 3);
    clickCanvas(f, 27, 5);
    clickCanvas(f, 23, 27);
    clickCanvas(f, 5, 21);
    clickCanvas(f, 3, 3);
    QVERIFY(f.view->has_selection());
    QVERIFY(hasPartial(f.view, 32, 32));
    QCOMPARE(f.view->selection_coverage(12, 12), 255);

    options->setAntiAlias(false);
    f.view->deselect();
    clickCanvas(f, 3, 3);
    clickCanvas(f, 27, 5);
    clickCanvas(f, 23, 27);
    clickCanvas(f, 5, 21);
    clickCanvas(f, 3, 3);
    QVERIFY(f.view->has_selection());
    QVERIFY(allBinary(f.view, 32, 32));
}

void SelectionAaTest::magneticAntiAliasCommitsPartialCoverage()
{
    paint_fixture::Fixture f(*window_, blackSquare(40, 40),
                             QStringLiteral("pictura_selection_aa_magnetic"));
    QVERIFY2(f.ok(), "selection fixture");
    auto* options = selectionOptions(f, pictura::ToolId::MagneticLasso);
    QVERIFY(options);
    options->setCombineMode(pictura::SelectionMode::New);
    options->setFeather(0.0);
    options->setAntiAlias(true);
    options->setMagneticWidth(6);
    options->setMagneticContrast(10);
    options->setMagneticFrequency(0);
    f.view->deselect();
    const int base = f.view->history_count();
    clickCanvas(f, 12, 12);
    clickCanvas(f, 27, 15);
    clickCanvas(f, 23, 27);
    clickCanvas(f, 9, 22);
    clickCanvas(f, 12, 12);
    QVERIFY(f.view->has_selection());
    QCOMPARE(f.view->history_count(), base + 1);
    QVERIFY(hasPartial(f.view, 40, 40));

    options->setAntiAlias(false);
    f.view->deselect();
    const int binaryBase = f.view->history_count();
    clickCanvas(f, 12, 12);
    clickCanvas(f, 27, 15);
    clickCanvas(f, 23, 27);
    clickCanvas(f, 9, 22);
    clickCanvas(f, 12, 12);
    QVERIFY(f.view->has_selection());
    QCOMPARE(f.view->history_count(), binaryBase + 1);
    QVERIFY(allBinary(f.view, 40, 40));
}

void SelectionAaTest::wandAntiAliasFringe()
{
    paint_fixture::Fixture f(*window_, gradient(32, 8), QStringLiteral("pictura_selection_aa_wand"));
    QVERIFY2(f.ok(), "selection fixture");
    auto* options = selectionOptions(f, pictura::ToolId::MagicWand);
    QVERIFY(options);
    options->setCombineMode(pictura::SelectionMode::New);
    options->setTolerance(10);
    options->setContiguous(true);
    options->setAntiAlias(true);
    clickCanvas(f, 0, 4);
    QVERIFY(f.view->has_selection());
    QVERIFY(hasPartial(f.view, 32, 8));

    f.view->deselect();
    QVERIFY(f.view->select_rect(24, 0, 4, 4, QStringLiteral("new"), 0.0));
    options->setCombineMode(pictura::SelectionMode::Add);
    const int base = f.view->history_count();
    clickCanvas(f, 0, 4);
    QCOMPARE(f.view->history_count(), base + 1);
    QCOMPARE(f.view->selection_coverage(24, 0), 255);
    QCOMPARE(f.view->selection_coverage(0, 4), 255);

    f.view->deselect();
    options->setCombineMode(pictura::SelectionMode::New);
    options->setAntiAlias(false);
    clickCanvas(f, 0, 4);
    QVERIFY(f.view->has_selection());
    QVERIFY(allBinary(f.view, 32, 8));
}

void SelectionAaTest::wandAntiAliasGlobalFringe()
{
    paint_fixture::Fixture f(*window_, twoRamps(32, 8),
                             QStringLiteral("pictura_selection_aa_wand_global"));
    QVERIFY2(f.ok(), "selection fixture");
    auto* options = selectionOptions(f, pictura::ToolId::MagicWand);
    QVERIFY(options);
    options->setCombineMode(pictura::SelectionMode::New);
    options->setTolerance(10);
    options->setContiguous(false);
    options->setAntiAlias(true);
    clickCanvas(f, 0, 4);
    QVERIFY(f.view->has_selection());
    const int first = f.view->selection_coverage(3, 4);
    const int second = f.view->selection_coverage(19, 4);
    QVERIFY(first > 0 && first < 255);
    QVERIFY(second > 0 && second < 255);
    QCOMPARE(f.view->selection_coverage(12, 4), 0);
}

void SelectionAaTest::antiAliasCombineAndFeather()
{
    paint_fixture::Fixture f(*window_, white(40, 40),
                             QStringLiteral("pictura_selection_aa_combine"));
    QVERIFY2(f.ok(), "selection fixture");
    auto* options = selectionOptions(f, pictura::ToolId::EllipticalMarquee);
    QVERIFY(options);
    options->setMarqueeStyle(pictura::MarqueeStyle::Normal);
    options->setAntiAlias(true);
    options->setFeather(0.0);

    QVERIFY(f.view->select_rect(5, 5, 12, 12, QStringLiteral("new"), 0.0));
    options->setCombineMode(pictura::SelectionMode::Add);
    const int addBase = f.view->history_count();
    dragEllipse(f, QPointF(18, 18), QPointF(34, 34));
    QCOMPARE(f.view->history_count(), addBase + 1);
    QCOMPARE(f.view->selection_coverage(6, 6), 255);
    QCOMPARE(f.view->selection_coverage(28, 28), 255);
    QVERIFY(hasPartial(f.view, 40, 40));

    f.view->deselect();
    QVERIFY(f.view->select_rect(5, 5, 20, 20, QStringLiteral("new"), 0.0));
    options->setCombineMode(pictura::SelectionMode::Subtract);
    const int subtractBase = f.view->history_count();
    dragEllipse(f, QPointF(26, 26), QPointF(10, 10));
    QCOMPARE(f.view->history_count(), subtractBase + 1);
    QCOMPARE(f.view->selection_coverage(20, 20), 0);
    QCOMPARE(f.view->selection_coverage(6, 6), 255);

    f.view->deselect();
    QVERIFY(f.view->select_rect(10, 10, 12, 12, QStringLiteral("new"), 0.0));
    options->setCombineMode(pictura::SelectionMode::Intersect);
    dragEllipse(f, QPointF(4, 4), QPointF(24, 24));
    QCOMPARE(f.view->selection_coverage(16, 16), 255);
    QCOMPARE(f.view->selection_coverage(8, 8), 0);

    f.view->deselect();
    options->setCombineMode(pictura::SelectionMode::New);
    options->setFeather(0.0);
    dragEllipse(f, QPointF(14, 14), QPointF(26, 24));
    const int sharpSoft = softCount(f.view, 40, 40);
    QCOMPARE(f.view->selection_coverage(12, 19), 0);
    f.view->deselect();
    options->setFeather(4.0);
    dragEllipse(f, QPointF(14, 14), QPointF(26, 24));
    QVERIFY(softCount(f.view, 40, 40) > sharpSoft);
    QVERIFY(f.view->selection_coverage(12, 19) > 0);
}

void SelectionAaTest::saveSelectionRefusesAtCap()
{
    QVERIFY(window_->newDocument(QStringLiteral("Cap"), 8, 8, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    auto* panel = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view && panel);
    panel->setView(view);
    for (int i = 0; i < 53; ++i) {
        QCOMPARE(pictura::channels_new_alpha(*view), i);
    }
    QVERIFY(view->select_rect(2, 2, 4, 4, QStringLiteral("new"), 0.0));
    panel->refresh();
    auto* save = panel->findChild<QToolButton*>(QStringLiteral("channelsSaveSelection"));
    QVERIFY(save && save->isEnabled());
    const int history = view->history_count();
    save->click();
    QVERIFY(window_->statusBar()->currentMessage().contains(QStringLiteral("56 channels")));
    QCOMPARE(view->selection_channel_count(), 53);
    QCOMPARE(view->history_count(), history);

    const int menuChannels = view->selection_channel_count();
    const int menuHistory = view->history_count();
    window_->statusBar()->clearMessage();
    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::SelectSave)));
    QVERIFY(window_->statusBar()->currentMessage().contains(QStringLiteral("56 channels")));
    QCOMPARE(view->selection_channel_count(), menuChannels);
    QCOMPARE(view->history_count(), menuHistory);
    window_->closeDocument(window_->activeDocumentIndex(), false);
}

QTEST_MAIN(SelectionAaTest)
#include "tst_selection_aa.moc"
