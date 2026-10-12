// The retouch and toning tools: Blur (#26), Sharpen (#27), Smudge (#28), Dodge
// (#29), Burn (#30), and Sponge (#31).

#include <QtTest/QtTest>

#include "panels/numeric_field.h"
#include "options_bar.h"
#include "selftest_paint_fixture.h"
#include "theme.h"

#include <QtGui/QPainter>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>

#include "qt_test_support.h"

namespace {
using paint_fixture::BrushState;
using paint_fixture::Fixture;
} // namespace

class RetouchToolsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void blurTool();
    void sharpenTool();
    void smudgeTool();
    void dodgeTool();
    void burnTool();
    void spongeTool();
    void brushTipBarIsCompact();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void RetouchToolsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void RetouchToolsTest::blurTool()
{
    pictura::PicturaMainWindow& frame = *window_;
    // White left of x = 20, black right of it.
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(Qt::white);
    QPainter(&seed).fillRect(20, 0, 20, 40, Qt::black);
    Fixture f(frame, seed, QStringLiteral("pictura_blur_seed"));
    if (!f.ok()) {
        QFAIL("blur fixture");
    }
    const BrushState brush(f.tools);
    f.tools->setBrushSize(12);
    const pictura::RetouchOptions options = f.tools->retouchOptions(pictura::ToolId::Blur);

    frame.setActiveTool(pictura::ToolId::Blur);
    auto* mode = frame.findChild<QComboBox*>(QStringLiteral("optionsBlurMode"));
    auto* strength = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsBlurStrength"));
    auto* sampleAll = frame.findChild<QCheckBox*>(QStringLiteral("optionsBlurSampleAll"));
    if (!mode || !strength || !sampleAll) {
        QFAIL("blur bar");
    }
    const bool bar = mode->count() == 7 && mode->currentIndex() == 0 && strength->value() == 50
        && !sampleAll->isChecked() && pictura::isBrushTool(pictura::ToolId::Blur);

    strength->setValue(100);
    const QRgb white = QColor(Qt::white).rgba();
    const QRgb black = QColor(Qt::black).rgba();
    const int base = f.view->history_index();
    f.drag({QPointF(20, 5), QPointF(20, 20), QPointF(20, 35), QPointF(20, 20)});
    const bool softened = f.committedOnce(base, "Blur") && f.view->sample_argb(19, 20) != white
        && f.view->sample_argb(20, 20) != black && f.view->sample_argb(2, 2) == white
        && f.view->sample_argb(37, 37) == black;

    // An empty layer has nothing of its own to blur; Sample All Layers reads the
    // composite and lands the result on it.
    const int layer = f.view->add_layer(0);
    f.view->set_active_layer(QString::number(layer));
    const int empty = f.view->history_index();
    f.drag({QPointF(20, 10), QPointF(20, 30)});
    const bool nothing = f.view->history_index() == empty;
    sampleAll->setChecked(true);
    f.drag({QPointF(20, 10), QPointF(20, 30)});
    const bool sampled = f.tools->retouchOptions(pictura::ToolId::Blur).sampleAllLayers && f.committedOnce(empty, "Blur");
    sampleAll->setChecked(false);
    strength->setValue(options.strength);

    QVERIFY2(bar, "bar");
    QVERIFY2(softened, "softened");
    QVERIFY2(nothing, "nothing");
    QVERIFY2(sampled, "sampled");
}

void RetouchToolsTest::sharpenTool()
{
    pictura::PicturaMainWindow& frame = *window_;
    // A grey step: 100 left of x = 20, 160 right of it.
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(QColor(100, 100, 100));
    QPainter(&seed).fillRect(20, 0, 20, 40, QColor(160, 160, 160));
    Fixture f(frame, seed, QStringLiteral("pictura_sharpen_seed"));
    if (!f.ok()) {
        QFAIL("sharpen fixture");
    }
    const BrushState brush(f.tools);
    f.tools->setBrushSize(12);
    const pictura::RetouchOptions options = f.tools->retouchOptions(pictura::ToolId::Sharpen);

    frame.setActiveTool(pictura::ToolId::Sharpen);
    auto* mode = frame.findChild<QComboBox*>(QStringLiteral("optionsSharpenMode"));
    auto* strength =
        frame.findChild<pictura::NumericField*>(QStringLiteral("optionsSharpenStrength"));
    auto* protect = frame.findChild<QCheckBox*>(QStringLiteral("optionsSharpenProtectDetail"));
    if (!mode || !strength || !protect) {
        QFAIL("sharpen bar");
    }
    const bool bar = mode->count() == 7 && strength->value() == 50 && protect->isChecked()
        && pictura::isBrushTool(pictura::ToolId::Sharpen);

    strength->setValue(100);
    protect->setChecked(false);
    const int base = f.view->history_index();
    f.drag({QPointF(20, 5), QPointF(20, 20), QPointF(20, 35)});
    const bool steeper = f.committedOnce(base, "Sharpen")
        && qRed(f.view->sample_argb(19, 20)) < 100 && qRed(f.view->sample_argb(20, 20)) > 160
        && qRed(f.view->sample_argb(2, 20)) == 100;
    protect->setChecked(options.protectDetail);
    strength->setValue(options.strength);

    QVERIFY2(bar, "bar");
    QVERIFY2(steeper, "steeper");
}

void RetouchToolsTest::smudgeTool()
{
    pictura::PicturaMainWindow& frame = *window_;
    // Black left of x = 20, white right of it.
    QImage seed(60, 40, QImage::Format_RGB32);
    seed.fill(Qt::white);
    QPainter(&seed).fillRect(0, 0, 20, 40, Qt::black);
    Fixture f(frame, seed, QStringLiteral("pictura_smudge_seed"));
    if (!f.ok()) {
        QFAIL("smudge fixture");
    }
    const BrushState brush(f.tools);
    f.tools->setBrushSize(12);
    const pictura::RetouchOptions options = f.tools->retouchOptions(pictura::ToolId::Smudge);
    const QColor foreground = f.tools->foreground();

    frame.setActiveTool(pictura::ToolId::Smudge);
    auto* strength =
        frame.findChild<pictura::NumericField*>(QStringLiteral("optionsSmudgeStrength"));
    auto* finger = frame.findChild<QCheckBox*>(QStringLiteral("optionsSmudgeFingerPainting"));
    if (!strength || !finger) {
        QFAIL("smudge bar");
    }
    const bool bar = strength->value() == 50 && !finger->isChecked()
        && frame.findChild<QCheckBox*>(QStringLiteral("optionsSmudgeSampleAll"))
        && pictura::isBrushTool(pictura::ToolId::Smudge);

    strength->setValue(90);
    QList<QPointF> path;
    for (int x = 14; x <= 40; ++x) {
        path << QPointF(x, 12);
    }
    const int base = f.view->history_index();
    f.drag(path);
    const QRgb white = QColor(Qt::white).rgba();
    const bool dragged = f.committedOnce(base, "Smudge")
        && qRed(f.view->sample_argb(30, 12)) < 240 && f.view->sample_argb(50, 35) == white;

    // Finger Painting drags the foreground into the white.
    finger->setChecked(true);
    f.tools->setForeground(QColor(220, 20, 20));
    path.clear();
    for (int x = 30; x <= 50; ++x) {
        path << QPointF(x, 30);
    }
    f.drag(path);
    const QRgb painted = f.view->sample_argb(34, 30);
    const bool fingerOk = f.view->history_index() == base + 2
        && qRed(painted) > qGreen(painted) + 40;
    finger->setChecked(options.fingerPainting);
    strength->setValue(options.strength);
    f.tools->setForeground(foreground);

    QVERIFY2(bar, "bar");
    QVERIFY2(dragged, "dragged");
    QVERIFY2(fingerOk, "fingerOk");
}

void RetouchToolsTest::dodgeTool()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(QColor(90, 90, 90));
    Fixture f(frame, seed, QStringLiteral("pictura_dodge_seed"));
    if (!f.ok()) {
        QFAIL("dodge fixture");
    }
    const BrushState brush(f.tools);
    f.tools->setBrushSize(12);

    frame.setActiveTool(pictura::ToolId::Dodge);
    auto* range = frame.findChild<QComboBox*>(QStringLiteral("optionsDodgeRange"));
    auto* exposure =
        frame.findChild<pictura::NumericField*>(QStringLiteral("optionsDodgeExposure"));
    auto* protect = frame.findChild<QCheckBox*>(QStringLiteral("optionsDodgeProtectTones"));
    if (!range || !exposure || !protect) {
        QFAIL("dodge bar");
    }
    const bool bar = range->count() == 3 && range->currentIndex() == 1 && exposure->value() == 50
        && protect->isChecked() && pictura::isBrushTool(pictura::ToolId::Dodge);

    const int base = f.view->history_index();
    f.drag({QPointF(5, 20), QPointF(15, 20), QPointF(25, 20), QPointF(35, 20)});
    const int once = qRed(f.view->sample_argb(20, 20));
    const bool lifted = f.committedOnce(base, "Dodge") && once > 95 && once < 150
        && qRed(f.view->sample_argb(20, 2)) == 90;
    f.drag({QPointF(5, 20), QPointF(15, 20), QPointF(25, 20), QPointF(35, 20)});
    const bool deeper = qRed(f.view->sample_argb(20, 20)) > once;

    QVERIFY2(bar, "bar");
    QVERIFY2(lifted, "lifted");
    QVERIFY2(deeper, "deeper");
}

void RetouchToolsTest::burnTool()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(QColor(170, 170, 170));
    Fixture f(frame, seed, QStringLiteral("pictura_burn_seed"));
    QVERIFY2(f.ok(), "burn fixture");
    const BrushState brush(f.tools);
    f.tools->setBrushSize(12);

    frame.setActiveTool(pictura::ToolId::Burn);
    auto* range = frame.findChild<QComboBox*>(QStringLiteral("optionsBurnRange"));
    auto* exposure =
        frame.findChild<pictura::NumericField*>(QStringLiteral("optionsBurnExposure"));
    auto* protect = frame.findChild<QCheckBox*>(QStringLiteral("optionsBurnProtectTones"));
    QVERIFY2(range && exposure && protect, "burn bar");
    QVERIFY2(range->currentIndex() == 1 && exposure->value() == 50 && protect->isChecked()
                 && pictura::isBrushTool(pictura::ToolId::Burn),
             "burn bar defaults");

    const int base = f.view->history_index();
    f.drag({QPointF(5, 20), QPointF(15, 20), QPointF(25, 20), QPointF(35, 20)});
    const int once = qRed(f.view->sample_argb(20, 20));
    QVERIFY2(f.committedOnce(base, "Burn"), "one Burn state");
    QVERIFY2(once < 170 && once > 100, qPrintable(QStringLiteral("one pass gave %1").arg(once)));
    QVERIFY2(qRed(f.view->sample_argb(20, 2)) == 170, "burned outside the tip");
    f.drag({QPointF(5, 20), QPointF(15, 20), QPointF(25, 20), QPointF(35, 20)});
    QVERIFY2(qRed(f.view->sample_argb(20, 20)) < once, "a second stroke did not deepen");
}

void RetouchToolsTest::spongeTool()
{
    pictura::PicturaMainWindow& frame = *window_;
    const QColor colour(200, 80, 80);
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(colour);
    Fixture f(frame, seed, QStringLiteral("pictura_sponge_seed"));
    QVERIFY2(f.ok(), "sponge fixture");
    const BrushState brush(f.tools);
    f.tools->setBrushSize(12);
    const pictura::ToneOptions options = f.tools->toneOptions(pictura::ToolId::Sponge);

    frame.setActiveTool(pictura::ToolId::Sponge);
    auto* mode = frame.findChild<QComboBox*>(QStringLiteral("optionsSpongeMode"));
    auto* flow = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsSpongeFlow"));
    auto* vibrance = frame.findChild<QCheckBox*>(QStringLiteral("optionsSpongeVibrance"));
    QVERIFY2(mode && flow && vibrance, "sponge bar");
    QVERIFY2(mode->count() == 2 && mode->currentIndex() == 0 && flow->value() == 50
                 && vibrance->isChecked() && pictura::isBrushTool(pictura::ToolId::Sponge),
             "sponge bar defaults");

    const auto spread = [&f](int x, int y) {
        const QRgb p = f.view->sample_argb(x, y);
        return qRed(p) - qBlue(p);
    };
    const QList<QPointF> path{QPointF(5, 12), QPointF(15, 12), QPointF(25, 12), QPointF(35, 12)};
    const int base = f.view->history_index();
    f.drag(path);
    QVERIFY2(f.committedOnce(base, "Sponge"), "one Sponge state");
    QVERIFY2(spread(20, 12) < 120, "Desaturate drained nothing");
    QVERIFY2(spread(20, 32) == 120, "sponged outside the tip");

    mode->setCurrentIndex(1);
    f.drag({QPointF(5, 32), QPointF(15, 32), QPointF(25, 32), QPointF(35, 32)});
    QVERIFY2(f.tools->toneOptions(pictura::ToolId::Sponge).spongeMode == 1, "Saturate not set");
    QVERIFY2(spread(20, 32) > 120, "Saturate lifted nothing");
    mode->setCurrentIndex(options.spongeMode);
}

void RetouchToolsTest::brushTipBarIsCompact()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_bar_compact_seed"));
    QVERIFY2(f.ok(), "compact bar fixture");
    frame.setActiveTool(pictura::ToolId::Blur);

    auto* bar = frame.findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    QVERIFY(bar);
    auto* stack = bar->findChild<QStackedWidget*>();
    QWidget* page = stack ? stack->currentWidget() : nullptr;
    QVERIFY(page);
    auto* tip = page->findChild<QToolButton*>(QStringLiteral("optionsBrushTip"));
    auto* peer = page->findChild<QComboBox*>(QStringLiteral("optionsBlurMode"));
    QVERIFY(tip && peer);

    QCOMPARE(tip->objectName(), QStringLiteral("optionsBrushTip"));
    const QString tipQss = pictura::Theme::styleSheet(pictura::Theme::kDefaultLevel);
    QVERIFY2(tipQss.contains(QStringLiteral("QToolButton#optionsBrushTip:!hover:!pressed"))
                 && tipQss.contains(QStringLiteral("padding: 0")),
             "brush tip is idle-less in the global stylesheet");
    QCOMPARE(tip->toolButtonStyle(), Qt::ToolButtonTextBesideIcon);
    QVERIFY2(tip->sizeHint().height() <= tip->iconSize().height() + 8,
             "tip is one row, not icon over text");

    QVERIFY2(tip->sizeHint().height() <= peer->sizeHint().height(),
             "brush tip does not set the bar height");
    QVERIFY2(page->sizeHint().height() <= peer->sizeHint().height() + 12,
             "options bar hugs its controls");
}

QTEST_MAIN(RetouchToolsTest)
#include "tst_retouch_tools.moc"
