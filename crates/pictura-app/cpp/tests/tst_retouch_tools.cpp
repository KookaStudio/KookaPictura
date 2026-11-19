// The retouch and toning tools: Blur (#26), Sharpen (#27), Smudge (#28), Dodge
// (#29), Burn (#30), and Sponge (#31), plus the Spot Healing Brush click (#1023).

#include <QtTest/QtTest>

#include "panels/numeric_field.h"
#include "options_bar.h"
#include "selftest_paint_fixture.h"
#include "theme.h"
#include "toolbox.h"

#include <QtGui/QAction>
#include <QtGui/QPainter>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>

#include <algorithm>
#include <cmath>
#include <cstdlib>

#include "api/document_bridge.h"

#include "qt_test_support.h"

namespace {
using paint_fixture::BrushState;
using paint_fixture::Fixture;

// A sky: a blue gradient down the image, the same along every row.
QImage skySeed(int w, int h)
{
    QImage img(w, h, QImage::Format_RGB32);
    for (int y = 0; y < h; ++y) {
        const int v = 110 + y;
        for (int x = 0; x < w; ++x) {
            img.setPixel(x, y, qRgb(v / 2, v - 20, v + 40));
        }
    }
    return img;
}

// `img` with an anti-aliased black dot of `radius` centred on (cx, cy).
QImage withDot(QImage img, double cx, double cy, double radius)
{
    for (int y = 0; y < img.height(); ++y) {
        for (int x = 0; x < img.width(); ++x) {
            const double d = std::hypot(x + 0.5 - cx, y + 0.5 - cy);
            const double a = std::clamp(radius - d + 0.5, 0.0, 1.0);
            if (a > 0.0) {
                const QRgb p = img.pixel(x, y);
                const auto ink = [a](int c) { return int(std::lround(c * (1.0 - a))); };
                img.setPixel(x, y, qRgb(ink(qRed(p)), ink(qGreen(p)), ink(qBlue(p))));
            }
        }
    }
    return img;
}

// The tone tools' options, restored on scope exit.
struct ToneState {
    pictura::ToolController* tools;
    pictura::ToneOptions dodge;
    pictura::ToneOptions burn;
    pictura::ToneOptions sponge;

    explicit ToneState(pictura::ToolController* t)
        : tools(t)
        , dodge(t->optionsFor<pictura::ToneGroup>(pictura::ToolId::Dodge)->get())
        , burn(t->optionsFor<pictura::ToneGroup>(pictura::ToolId::Burn)->get())
        , sponge(t->optionsFor<pictura::ToneGroup>(pictura::ToolId::Sponge)->get())
    {
    }
    ~ToneState()
    {
        tools->optionsFor<pictura::ToneGroup>(pictura::ToolId::Dodge)->set(dodge);
        tools->optionsFor<pictura::ToneGroup>(pictura::ToolId::Burn)->set(burn);
        tools->optionsFor<pictura::ToneGroup>(pictura::ToolId::Sponge)->set(sponge);
    }
};

// The tone tool's flyout row in the toolbox.
QAction* toneFlyoutAction(pictura::Toolbox* box, pictura::ToolId id)
{
    if (!box) {
        return nullptr;
    }
    for (int group = 1; group <= 23; ++group) {
        for (QAction* action : box->slotMenuActionsForTest(group)) {
            if (action->data().toInt() == int(id)) {
                return action;
            }
        }
    }
    return nullptr;
}
} // namespace

class RetouchToolsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void blurTool();
    void sharpenTool();
    void smudgeTool();
    void smudgeSampleAllMatchesTheLiveLayer();
    void dodgeTool();
    void burnTool();
    void spongeTool();
    void airbrushOffStaysStill();
    void airbrushBuildsWhileHeld();
    void shiftAltPTogglesToneAirbrush();
    void rangeQuickKeys();
    void spongeQuickKeys();
    void quickKeyIgnoredForOtherTools();
    void grayscaleSpongeChangesContrast();
    void sixteenBitDodgeLightsTheCanvas();
    void thirtyTwoBitDisablesToneTools();
    void thirtyTwoBitFallsBackToMove();
    void strokeChangingNothingRebuildsNothing();
    void spotHealingClickRemovesADot();
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
    window_->show();
    window_->activateWindow();
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
    f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setSize(12);
    const pictura::RetouchOptions options = f.tools->optionsFor<pictura::RetouchGroup>(pictura::ToolId::Blur)->get();

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
    const bool sampled = f.tools->optionsFor<pictura::RetouchGroup>(pictura::ToolId::Blur)->get().sampleAllLayers && f.committedOnce(empty, "Blur");
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
    f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setSize(12);
    const pictura::RetouchOptions options = f.tools->optionsFor<pictura::RetouchGroup>(pictura::ToolId::Sharpen)->get();

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
    f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setSize(12);
    const pictura::RetouchOptions options = f.tools->optionsFor<pictura::RetouchGroup>(pictura::ToolId::Smudge)->get();
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

void RetouchToolsTest::smudgeSampleAllMatchesTheLiveLayer()
{
    pictura::PicturaMainWindow& frame = *window_;
    // Black left of x = 20, white right of it, as in `smudgeTool`.
    QImage seed(80, 40, QImage::Format_RGB32);
    seed.fill(Qt::white);
    QPainter(&seed).fillRect(0, 0, 20, 40, Qt::black);
    Fixture f(frame, seed, QStringLiteral("pictura_smudge_live"));
    if (!f.ok()) {
        QFAIL("smudge live fixture");
    }
    const BrushState brush(f.tools);
    f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setSize(12);
    const pictura::RetouchOptions options = f.tools->optionsFor<pictura::RetouchGroup>(pictura::ToolId::Smudge)->get();

    frame.setActiveTool(pictura::ToolId::Smudge);
    auto* strength = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsSmudgeStrength"));
    auto* sampleAll = frame.findChild<QCheckBox*>(QStringLiteral("optionsSmudgeSampleAll"));
    if (!strength || !sampleAll) {
        QFAIL("smudge live bar");
    }
    strength->setValue(90);

    QList<QPointF> path;
    for (int x = 14; x <= 50; ++x) {
        path << QPointF(x, 20);
    }
    const auto reach = [&]() {
        int last = -1;
        for (int x = 0; x < seed.width(); ++x) {
            if (qRed(f.view->sample_argb(x, 20)) < 240) {
                last = x;
            }
        }
        return last;
    };

    // On a single opaque layer the composite is the layer, so Sample All Layers
    // must smear exactly as far as the live layer does. A press-time snapshot
    // stops the stroke short.
    sampleAll->setChecked(false);
    f.drag(path);
    const int off = reach();
    QVERIFY(f.view->undo());
    sampleAll->setChecked(true);
    f.drag(path);
    const int on = reach();
    sampleAll->setChecked(options.sampleAllLayers);
    strength->setValue(options.strength);

    QVERIFY2(off > 20, "the live stroke did not smear");
    QVERIFY2(std::abs(on - off) <= 1,
             qPrintable(QStringLiteral("sample all stopped short: off %1, on %2").arg(off).arg(on)));
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
    f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setSize(12);

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
    f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setSize(12);

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
    f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setSize(12);
    const pictura::ToneOptions options = f.tools->optionsFor<pictura::ToneGroup>(pictura::ToolId::Sponge)->get();

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
    QVERIFY2(f.tools->optionsFor<pictura::ToneGroup>(pictura::ToolId::Sponge)->get().spongeMode == 1, "Saturate not set");
    QVERIFY2(spread(20, 32) > 120, "Saturate lifted nothing");
    mode->setCurrentIndex(options.spongeMode);
}

// [tsr_no_change]
void RetouchToolsTest::strokeChangingNothingRebuildsNothing()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(Qt::black);
    Fixture f(frame, seed, QStringLiteral("pictura_sponge_black_seed"));
    QVERIFY2(f.ok(), "sponge fixture");
    const BrushState brush(f.tools);
    f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setSize(12);
    frame.setActiveTool(pictura::ToolId::Sponge);

    const int base = f.view->history_index();
    QSignalSpy rebuilt(f.view, &pictura::PictureView::changed);
    f.drag({QPointF(5, 20), QPointF(20, 20), QPointF(35, 20)});
    QCOMPARE(rebuilt.count(), 0);
    QCOMPARE(f.view->history_index(), base);
    QCOMPARE(qRed(f.view->sample_argb(20, 20)), 0);
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

void RetouchToolsTest::spotHealingClickRemovesADot()
{
    // #1023: a click with a brush the dot's size used to leave a grey smudge,
    // because the dot's soft rim lay just outside the brush.
    pictura::PicturaMainWindow& frame = *window_;
    const QImage clean = skySeed(64, 64);
    Fixture f(frame, withDot(clean, 32.0, 32.0, 4.0), QStringLiteral("pictura_spot_heal_dot"));
    QVERIFY2(f.ok(), "spot healing fixture");
    auto* paint = f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::SpotHealingBrush);
    auto* healing = f.tools->optionsFor<pictura::HealingGroup>(pictura::ToolId::SpotHealingBrush);
    QVERIFY(paint && healing);
    const int size = paint->size();
    const int hardness = paint->hardness();
    const pictura::HealingOptions options = healing->get();
    paint->setSize(8);
    paint->setHardness(100);
    pictura::HealingOptions proximity = options;
    proximity.spotType = 0;
    healing->set(proximity);

    frame.setActiveTool(pictura::ToolId::SpotHealingBrush);
    const int base = f.view->history_index();
    f.drag({QPointF(32, 32)});
    const bool committed = f.committedOnce(base, "Spot Healing Brush");
    int worst = 0;
    for (int y = 25; y < 40; ++y) {
        for (int x = 25; x < 40; ++x) {
            const QRgb got = f.view->sample_argb(x, y);
            const QRgb want = clean.pixel(x, y);
            worst = std::max({worst, std::abs(qRed(got) - qRed(want)),
                              std::abs(qGreen(got) - qGreen(want)),
                              std::abs(qBlue(got) - qBlue(want))});
        }
    }

    paint->setSize(size);
    paint->setHardness(hardness);
    healing->set(options);
    QVERIFY2(committed, "one Spot Healing Brush state");
    QVERIFY2(worst <= 6, qPrintable(QStringLiteral("dot left behind: off by %1").arg(worst)));
}

void RetouchToolsTest::airbrushOffStaysStill()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(QColor(100, 100, 100));
    Fixture f(frame, seed, QStringLiteral("pictura_tone_airbrush_off"));
    QVERIFY2(f.ok(), "airbrush fixture");
    const BrushState brush(f.tools);
    const ToneState toneState(f.tools);
    f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setSize(12);
    frame.setActiveTool(pictura::ToolId::Dodge);
    auto* tone = f.tools->optionsFor<pictura::ToneGroup>(pictura::ToolId::Dodge);
    pictura::ToneOptions options = tone->get();
    options.airbrush = false;
    tone->set(options);

    const int base = f.view->history_index();
    f.canvas->mousePressed(QPointF(20, 20), Qt::LeftButton, 0);
    QTest::qWait(0);
    const int before = qRed(f.view->sample_argb(20, 20));
    QTest::qWait(250); // held dabs are time-driven; no signal marks one
    QCOMPARE(qRed(f.view->sample_argb(20, 20)), before);
    f.canvas->mouseReleased(QPointF(20, 20));
    QVERIFY2(f.committedOnce(base, "Dodge"), "one Dodge state");
}

void RetouchToolsTest::airbrushBuildsWhileHeld()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(QColor(100, 100, 100));
    Fixture f(frame, seed, QStringLiteral("pictura_tone_airbrush_on"));
    QVERIFY2(f.ok(), "airbrush fixture");
    const BrushState brush(f.tools);
    const ToneState toneState(f.tools);
    f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setSize(12);
    frame.setActiveTool(pictura::ToolId::Dodge);
    auto* tone = f.tools->optionsFor<pictura::ToneGroup>(pictura::ToolId::Dodge);
    pictura::ToneOptions options = tone->get();
    options.airbrush = true;
    tone->set(options);

    const int base = f.view->history_index();
    f.canvas->mousePressed(QPointF(20, 20), Qt::LeftButton, 0);
    QTest::qWait(0);
    const int before = qRed(f.view->sample_argb(20, 20));
    QTest::qWait(250); // held dabs are time-driven; no signal marks one
    const int after = qRed(f.view->sample_argb(20, 20));
    f.canvas->mouseReleased(QPointF(20, 20));
    QVERIFY2(after > before, "Airbrush did not build while held");
    QVERIFY2(f.committedOnce(base, "Dodge"), "one Dodge state");
}

void RetouchToolsTest::shiftAltPTogglesToneAirbrush()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(QColor(100, 100, 100));
    Fixture f(frame, seed, QStringLiteral("pictura_tone_airbrush_key"));
    QVERIFY2(f.ok(), "airbrush fixture");
    const ToneState toneState(f.tools);
    frame.setActiveTool(pictura::ToolId::Dodge);
    auto* tone = f.tools->optionsFor<pictura::ToneGroup>(pictura::ToolId::Dodge);
    pictura::ToneOptions options = tone->get();
    options.airbrush = false;
    tone->set(options);
    auto* button = frame.findChild<QToolButton*>(QStringLiteral("optionsDodgeAirbrush"));
    QVERIFY(button);
    QVERIFY(!button->isChecked());

    QTest::keyClick(window_.get(), Qt::Key_P, Qt::ShiftModifier | Qt::AltModifier);
    QVERIFY(tone->get().airbrush);
    QVERIFY(button->isChecked());
    QTest::keyClick(window_.get(), Qt::Key_P, Qt::ShiftModifier | Qt::AltModifier);
    QVERIFY(!tone->get().airbrush);
    QVERIFY(!button->isChecked());
}

void RetouchToolsTest::rangeQuickKeys()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(QColor(100, 100, 100));
    Fixture f(frame, seed, QStringLiteral("pictura_tone_range_keys"));
    QVERIFY2(f.ok(), "tone fixture");
    const ToneState toneState(f.tools);
    frame.setActiveTool(pictura::ToolId::Dodge);
    auto* tone = f.tools->optionsFor<pictura::ToneGroup>(pictura::ToolId::Dodge);
    auto* range = frame.findChild<QComboBox*>(QStringLiteral("optionsDodgeRange"));
    QVERIFY(tone && range);

    QTest::keyClick(window_.get(), Qt::Key_S, Qt::ShiftModifier | Qt::AltModifier);
    QCOMPARE(tone->get().range, 0);
    QCOMPARE(range->currentIndex(), 0);
    QTest::keyClick(window_.get(), Qt::Key_M, Qt::ShiftModifier | Qt::AltModifier);
    QCOMPARE(tone->get().range, 1);
    QCOMPARE(range->currentIndex(), 1);
    QTest::keyClick(window_.get(), Qt::Key_H, Qt::ShiftModifier | Qt::AltModifier);
    QCOMPARE(tone->get().range, 2);
    QCOMPARE(range->currentIndex(), 2);
}

void RetouchToolsTest::spongeQuickKeys()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(QColor(100, 100, 100));
    Fixture f(frame, seed, QStringLiteral("pictura_tone_sponge_keys"));
    QVERIFY2(f.ok(), "tone fixture");
    const ToneState toneState(f.tools);
    frame.setActiveTool(pictura::ToolId::Sponge);
    auto* tone = f.tools->optionsFor<pictura::ToneGroup>(pictura::ToolId::Sponge);
    auto* mode = frame.findChild<QComboBox*>(QStringLiteral("optionsSpongeMode"));
    QVERIFY(tone && mode);

    QTest::keyClick(window_.get(), Qt::Key_S, Qt::ShiftModifier | Qt::AltModifier);
    QCOMPARE(tone->get().spongeMode, 1);
    QCOMPARE(mode->currentIndex(), 1);
    QTest::keyClick(window_.get(), Qt::Key_D, Qt::ShiftModifier | Qt::AltModifier);
    QCOMPARE(tone->get().spongeMode, 0);
    QCOMPARE(mode->currentIndex(), 0);
}

void RetouchToolsTest::quickKeyIgnoredForOtherTools()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(QColor(100, 100, 100));
    Fixture f(frame, seed, QStringLiteral("pictura_tone_key_ignored"));
    QVERIFY2(f.ok(), "tone fixture");
    const ToneState toneState(f.tools);
    frame.setActiveTool(pictura::ToolId::Brush);
    auto* dodge = f.tools->optionsFor<pictura::ToneGroup>(pictura::ToolId::Dodge);
    auto* sponge = f.tools->optionsFor<pictura::ToneGroup>(pictura::ToolId::Sponge);
    const pictura::ToneOptions d = dodge->get();
    const pictura::ToneOptions s = sponge->get();

    for (const Qt::Key key : {Qt::Key_S, Qt::Key_M, Qt::Key_H, Qt::Key_D}) {
        QTest::keyClick(window_.get(), key, Qt::ShiftModifier | Qt::AltModifier);
    }
    QCOMPARE(dodge->get().range, d.range);
    QCOMPARE(dodge->get().spongeMode, d.spongeMode);
    QCOMPARE(dodge->get().airbrush, d.airbrush);
    QCOMPARE(sponge->get().range, s.range);
    QCOMPARE(sponge->get().spongeMode, s.spongeMode);
    QCOMPARE(sponge->get().airbrush, s.airbrush);
}

void RetouchToolsTest::grayscaleSpongeChangesContrast()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(40, 40, QImage::Format_RGB32);
    for (int y = 0; y < seed.height(); ++y) {
        const int v = y < 20 ? 80 : 180;
        for (int x = 0; x < seed.width(); ++x) {
            seed.setPixel(x, y, qRgb(v, v, v));
        }
    }
    Fixture f(frame, seed, QStringLiteral("pictura_sponge_grayscale"));
    QVERIFY2(f.ok(), "grayscale fixture");
    QVERIFY(pictura::convert_image_mode(*f.view, QStringLiteral("grayscale")));
    QCOMPARE(pictura::image_mode(*f.view), QStringLiteral("grayscale"));
    const BrushState brush(f.tools);
    const ToneState toneState(f.tools);
    f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setSize(12);

    frame.setActiveTool(pictura::ToolId::Sponge);
    auto* tone = f.tools->optionsFor<pictura::ToneGroup>(pictura::ToolId::Sponge);
    const pictura::ToneOptions saved = tone->get();
    pictura::ToneOptions options = saved;
    options.amount = 100;
    options.vibrance = false;
    options.spongeMode = 1;
    tone->set(options);

    const int base = f.view->history_index();
    f.drag({QPointF(5, 10), QPointF(15, 10), QPointF(25, 10), QPointF(35, 10)});
    QVERIFY2(f.committedOnce(base, "Sponge"), "one Sponge state");
    QVERIFY2(qRed(f.view->sample_argb(20, 10)) < 80, "Saturate did not darken a dark grey");
    f.drag({QPointF(5, 30), QPointF(15, 30), QPointF(25, 30), QPointF(35, 30)});
    QVERIFY2(qRed(f.view->sample_argb(20, 30)) > 180, "Saturate did not lighten a light grey");
}

void RetouchToolsTest::sixteenBitDodgeLightsTheCanvas()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(QColor(90, 90, 90));
    Fixture f(frame, seed, QStringLiteral("pictura_tone_16bpc"));
    QVERIFY2(f.ok(), "16-bpc fixture");
    QVERIFY(pictura::create_document(*f.view, 40, 40, QStringLiteral("rgb"), 16, 0x5a5a5a,
                                     false, 72.0, false));
    QCOMPARE(f.view->document_depth_bits(), 16);
    const BrushState brush(f.tools);
    f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setSize(12);
    frame.setActiveTool(pictura::ToolId::Dodge);
    const int base = f.view->history_index();
    f.drag({QPointF(5, 20), QPointF(15, 20), QPointF(25, 20), QPointF(35, 20)});
    const int once = qRed(f.view->sample_argb(20, 20));
    QVERIFY2(f.committedOnce(base, "Dodge"), "one Dodge state");
    QVERIFY2(once > 90,
             qPrintable(QStringLiteral("16-bpc Dodge left the canvas at %1").arg(once)));
    QVERIFY2(qRed(f.view->sample_argb(20, 2)) == 90, "dodged outside the tip");
}

void RetouchToolsTest::thirtyTwoBitDisablesToneTools()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(QColor(100, 100, 100));
    Fixture f(frame, seed, QStringLiteral("pictura_tone_32bpc_off"));
    QVERIFY2(f.ok(), "32-bpc fixture");
    QVERIFY(pictura::create_document(*f.view, 40, 40, QStringLiteral("rgb"), 32, 0xffffffu, false,
                                     72.0, false));
    QCOMPARE(f.view->document_depth_bits(), 32);
    window_->refresh();
    auto* box = window_->findChild<pictura::Toolbox*>();
    QVERIFY(box);
    for (const pictura::ToolId id :
         {pictura::ToolId::Dodge, pictura::ToolId::Burn, pictura::ToolId::Sponge}) {
        QAction* row = toneFlyoutAction(box, id);
        QVERIFY2(row, "tone flyout row");
        QVERIFY2(!row->isEnabled(), "tone flyout row stayed enabled at 32 bpc");
    }
    window_->setActiveTool(pictura::ToolId::Dodge);
    QCOMPARE(window_->activeTool(), pictura::ToolId::Move);
}

void RetouchToolsTest::thirtyTwoBitFallsBackToMove()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(QColor(100, 100, 100));
    Fixture f(frame, seed, QStringLiteral("pictura_tone_32bpc_fallback"));
    QVERIFY2(f.ok(), "32-bpc fixture");
    frame.setActiveTool(pictura::ToolId::Dodge);
    QCOMPARE(window_->activeTool(), pictura::ToolId::Dodge);
    QVERIFY(pictura::create_document(*f.view, 40, 40, QStringLiteral("rgb"), 32, 0xffffffu, false,
                                     72.0, false));
    window_->refresh();
    QCOMPARE(window_->activeTool(), pictura::ToolId::Move);
}

QTEST_MAIN(RetouchToolsTest)
#include "tst_retouch_tools.moc"
