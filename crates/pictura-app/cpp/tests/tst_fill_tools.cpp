// The fill tools: Gradient (#24) and Paint Bucket (#25).

#include <QtTest/QtTest>

#include "panels/numeric_field.h"
#include "fill_dialog.h"
#include "stroke_dialog.h"
#include "selftest_paint_fixture.h"

#include <QtGui/QPainter>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtGui/QKeyEvent>
#include <QtWidgets/QApplication>
#include <QtWidgets/QMenu>
#include <QtWidgets/QToolButton>

#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"

#include "qt_test_support.h"

namespace {
using paint_fixture::BrushState;
using paint_fixture::Fixture;

// The foreground and background colours a check changes, restored on scope exit.
struct Colors {
    pictura::ToolController* tools;
    QColor foreground;
    QColor background;

    Colors(pictura::ToolController* t, const QColor& fg, const QColor& bg)
        : tools(t)
        , foreground(t->foreground())
        , background(t->background())
    {
        tools->setForeground(fg);
        tools->setBackground(bg);
    }
    ~Colors()
    {
        tools->setForeground(foreground);
        tools->setBackground(background);
    }
};
} // namespace

class FillToolsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void gradientTool();
    void paintBucketTool();
    void fillCommand();
    void strokeCommand();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void FillToolsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void FillToolsTest::gradientTool()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(QColor(0, 128, 0));
    Fixture f(frame, seed, QStringLiteral("pictura_gradient_seed"));
    if (!f.ok()) {
        QFAIL("gradient fixture");
    }
    const Colors colors(f.tools, Qt::black, Qt::white);
    const QRgb black = QColor(Qt::black).rgba();
    const QRgb white = QColor(Qt::white).rgba();

    // The G group's keys, through the window's shortcut path, which needs the
    // window shown.
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    frame.activateWindow();
    QCoreApplication::processEvents();
    const auto sendKey = [&frame](Qt::KeyboardModifiers mods, const QString& text) {
        QKeyEvent event(QEvent::KeyPress, Qt::Key_G, mods, text);
        QApplication::sendEvent(&frame, &event);
    };
    frame.setActiveTool(pictura::ToolId::Gradient);
    frame.setActiveTool(pictura::ToolId::Move);
    sendKey(Qt::NoModifier, QStringLiteral("g"));
    bool keys = frame.activeTool() == pictura::ToolId::Gradient;
    sendKey(Qt::ShiftModifier, QStringLiteral("G"));
    keys = keys && frame.activeTool() == pictura::ToolId::PaintBucket;
    sendKey(Qt::ShiftModifier, QStringLiteral("G"));
    keys = keys && frame.activeTool() == pictura::ToolId::Gradient;

    auto* sample = frame.findChild<QToolButton*>(QStringLiteral("optionsGradientSample"));
    QList<QToolButton*> styles;
    for (int i = 0; i < 5; ++i) {
        styles << frame.findChild<QToolButton*>(QStringLiteral("optionsGradientStyle%1").arg(i));
    }
    auto* mode = frame.findChild<QComboBox*>(QStringLiteral("optionsGradientMode"));
    auto* opacity =
        frame.findChild<pictura::NumericField*>(QStringLiteral("optionsGradientOpacity"));
    auto* reverse = frame.findChild<QCheckBox*>(QStringLiteral("optionsGradientReverse"));
    auto* dither = frame.findChild<QCheckBox*>(QStringLiteral("optionsGradientDither"));
    auto* transparency =
        frame.findChild<QCheckBox*>(QStringLiteral("optionsGradientTransparency"));
    if (!sample || !sample->menu() || styles.contains(nullptr) || !mode || !opacity || !reverse
        || !dither || !transparency) {
        QFAIL("gradient bar");
    }
    const QList<QAction*> presets = sample->menu()->actions();
    const bool bar = presets.size() == pictura::gradient_preset_count() && presets.size() > 1
        && presets.first()->isChecked() && !sample->icon().isNull() && styles.at(0)->isChecked()
        && mode->count() == 3 && mode->currentIndex() == 0 && opacity->value() == 100
        && !reverse->isChecked() && !dither->isChecked() && transparency->isChecked()
        && !pictura::isBrushTool(pictura::ToolId::Gradient);

    // The axis follows the drag; the gradient is drawn on release.
    const int base = f.view->history_index();
    f.canvas->mousePressed(QPointF(0, 20), Qt::LeftButton, 0);
    f.canvas->mouseMoved(QPointF(39, 20));
    const bool axisShown =
        f.canvas->hasRulerLineForTest() && f.view->history_index() == base;
    f.canvas->mouseReleased(QPointF(39, 20));
    const bool axis = axisShown && !f.canvas->hasRulerLineForTest();
    const int mid = qRed(f.view->sample_argb(20, 20));
    const bool linear = f.committedOnce(base, "Gradient") && f.view->sample_argb(0, 20) == black
        && f.view->sample_argb(39, 5) == white && mid > 100 && mid < 160;
    const bool undone = f.view->undo() && f.view->sample_argb(0, 20) == QColor(0, 128, 0).rgba()
        && f.view->redo() && f.view->sample_argb(0, 20) == black;

    f.drag({QPointF(10, 10)});
    const bool click = f.view->history_index() == base + 1;

    reverse->setChecked(true);
    f.drag({QPointF(0, 20), QPointF(39, 20)});
    const bool reversed = f.tools->optionsFor<pictura::GradientGroup>(pictura::ToolId::Gradient)->get().reverse
        && f.view->history_index() == base + 2 && f.view->sample_argb(0, 20) == white
        && f.view->sample_argb(39, 20) == black;
    reverse->setChecked(false);

    // Radial, inside a selection of the left half.
    styles.at(1)->click();
    f.view->select_rect(0, 0, 20, 40, QStringLiteral("new"), 0.0);
    const int inside = f.view->history_index();
    f.drag({QPointF(10, 20), QPointF(15, 20)});
    const bool selected = f.tools->optionsFor<pictura::GradientGroup>(pictura::ToolId::Gradient)->get().style == 1
        && f.committedOnce(inside, "Gradient") && f.view->sample_argb(10, 20) == black
        && f.view->sample_argb(10, 38) == white && f.view->sample_argb(39, 20) == black;
    f.view->deselect();
    styles.at(0)->click();

    presets.at(1)->trigger();
    const bool preset = f.tools->optionsFor<pictura::GradientGroup>(pictura::ToolId::Gradient)->get().preset == 1
        && sample->toolTip() == pictura::gradient_preset_name(1);
    presets.at(0)->trigger();

    QVERIFY2(keys, "keys");
    QVERIFY2(bar, "bar");
    QVERIFY2(axis, "axis");
    QVERIFY2(linear, "linear");
    QVERIFY2(undone, "undone");
    QVERIFY2(click, "click");
    QVERIFY2(reversed, "reversed");
    QVERIFY2(selected, "selected");
    QVERIFY2(preset, "preset");
}

void FillToolsTest::paintBucketTool()
{
    pictura::PicturaMainWindow& frame = *window_;
    // Two separate red squares on white.
    const QColor red(220, 20, 20);
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(Qt::white);
    {
        QPainter painter(&seed);
        painter.fillRect(5, 5, 10, 10, red);
        painter.fillRect(25, 25, 10, 10, red);
    }
    Fixture f(frame, seed, QStringLiteral("pictura_paint_bucket_seed"));
    if (!f.ok()) {
        QFAIL("paint bucket fixture");
    }
    const QColor blue(0, 0, 255);
    const QColor green(0, 200, 0);
    const Colors colors(f.tools, blue, Qt::white);

    frame.setActiveTool(pictura::ToolId::PaintBucket);
    auto* fill = frame.findChild<QComboBox*>(QStringLiteral("optionsBucketFill"));
    auto* pattern = frame.findChild<QComboBox*>(QStringLiteral("optionsBucketPattern"));
    auto* mode = frame.findChild<QComboBox*>(QStringLiteral("optionsBucketMode"));
    auto* opacity = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsBucketOpacity"));
    auto* tolerance =
        frame.findChild<pictura::NumericField*>(QStringLiteral("optionsBucketTolerance"));
    auto* antialias = frame.findChild<QCheckBox*>(QStringLiteral("optionsBucketAntialias"));
    auto* contiguous = frame.findChild<QCheckBox*>(QStringLiteral("optionsBucketContiguous"));
    auto* allLayers = frame.findChild<QCheckBox*>(QStringLiteral("optionsBucketAllLayers"));
    if (!fill || !pattern || !mode || !opacity || !tolerance || !antialias || !contiguous
        || !allLayers) {
        QFAIL("paint bucket bar");
    }
    const bool bar = fill->currentIndex() == 0 && !pattern->isEnabled() && pattern->count() > 0
        && mode->count() == 4 && opacity->value() == 100 && tolerance->value() == 32
        && antialias->isChecked() && contiguous->isChecked() && !allLayers->isChecked()
        && !pictura::isBrushTool(pictura::ToolId::PaintBucket);

    const int base = f.view->history_index();
    f.drag({QPointF(1, 1)});
    const bool filled = f.committedOnce(base, "Paint Bucket")
        && f.view->sample_argb(1, 1) == blue.rgba() && f.view->sample_argb(20, 20) == blue.rgba()
        && f.view->sample_argb(9, 9) == red.rgba() && f.view->sample_argb(29, 29) == red.rgba();

    // All Layers: the composite's red square, filled on an empty layer above.
    const int layer = f.view->add_layer(0);
    f.view->set_active_layer(QString::number(layer));
    f.tools->setForeground(green);
    allLayers->setChecked(true);
    const int above = f.view->history_index();
    f.drag({QPointF(9, 9)});
    const bool composite = f.committedOnce(above, "Paint Bucket")
        && f.view->sample_argb(9, 9) == green.rgba() && f.view->sample_argb(29, 29) == red.rgba()
        && f.view->sample_argb(1, 1) == blue.rgba();
    allLayers->setChecked(false);

    f.view->set_layer_lock(layer, QStringLiteral("pixels"), true);
    const int locked = f.view->history_index();
    f.drag({QPointF(1, 1)});
    const bool refused = f.view->history_index() == locked;

    // Contiguous off: every red pixel of the Background, under the green one.
    f.view->set_active_layer(QStringLiteral("0"));
    f.tools->setForeground(blue);
    contiguous->setChecked(false);
    f.drag({QPointF(29, 29)});
    const bool global = !f.tools->optionsFor<pictura::BucketGroup>(pictura::ToolId::PaintBucket)->get().contiguous
        && f.view->sample_argb(29, 29) == blue.rgba() && f.view->sample_argb(9, 9) == green.rgba();

    // Pattern: the checkerboard tile from the document origin, over all the blue.
    fill->setCurrentIndex(1);
    const int plain = f.view->history_index();
    f.drag({QPointF(1, 1)});
    const bool patterned = pattern->isEnabled() && f.tools->optionsFor<pictura::BucketGroup>(pictura::ToolId::PaintBucket)->get().fill == 1
        && f.committedOnce(plain, "Paint Bucket")
        && f.view->sample_argb(1, 1) == QColor(215, 215, 215).rgba()
        && f.view->sample_argb(33, 1) == QColor(90, 90, 90).rgba();
    fill->setCurrentIndex(0);
    contiguous->setChecked(true);

    QVERIFY2(bar, "bar");
    QVERIFY2(filled, "filled");
    QVERIFY2(composite, "composite");
    QVERIFY2(refused, "refused");
    QVERIFY2(global, "global");
    QVERIFY2(patterned, "patterned");
}

void FillToolsTest::fillCommand()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(16, 16, QImage::Format_RGB32);
    seed.fill(QColor(0, 128, 0));
    Fixture f(frame, seed, QStringLiteral("pictura_fill_command"));
    QVERIFY2(f.ok(), "fill fixture");

    // The dialog reports its widgets' values and defaults to Foreground 100 %.
    pictura::FillDialog dialog(QColor(10, 20, 30), QColor(200, 210, 220));
    QVERIFY(!dialog.isPatternFill());
    QCOMPARE(dialog.fillColor(), QColor(10, 20, 30));
    QCOMPARE(dialog.blendMode(), QStringLiteral("normal"));
    QCOMPARE(dialog.opacity(), 100);
    QVERIFY(!dialog.preserveTransparency());

    // The Edit > Fill command fills the active layer through the engine and
    // records one "Fill" state.
    const QColor foreground(10, 20, 30);
    const int base = f.view->history_index();
    QVERIFY(pictura::edit_fill(*f.view, foreground.rgba(), -1, QStringLiteral("normal"), 100,
                               false));
    QVERIFY2(f.committedOnce(base, "Fill"), "one fill state");
    QCOMPARE(f.view->sample_argb(8, 8), foreground.rgba());

    // A pattern fill uses the tile from the document origin.
    const int patterned = f.view->history_index();
    QVERIFY(pictura::edit_fill(*f.view, 0, 0, QStringLiteral("normal"), 100, false));
    QVERIFY2(f.committedOnce(patterned, "Fill"), "one pattern state");
    QCOMPARE(f.view->sample_argb(0, 0), QColor(215, 215, 215).rgba());
}

void FillToolsTest::strokeCommand()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(24, 24, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_stroke_command"));
    QVERIFY2(f.ok(), "stroke fixture");

    // The dialog reports its widgets' values and defaults to Center 1 px black.
    pictura::StrokeDialog dialog(QColor(Qt::black));
    QCOMPARE(dialog.strokeWidth(), 1);
    QCOMPARE(dialog.strokeColor(), QColor(Qt::black));
    QCOMPARE(dialog.location(), 1);
    QCOMPARE(dialog.blendMode(), QStringLiteral("normal"));
    QCOMPARE(dialog.opacity(), 100);

    // Select a 10×10 square in the middle, then stroke it 2 px Outside in red.
    QVERIFY(f.view->select_rect(7, 7, 10, 10, QStringLiteral("new"), 0.0));
    QVERIFY(f.view->has_selection());
    const QColor red(220, 0, 0);
    const int base = f.view->history_index();
    QVERIFY(pictura::edit_stroke(*f.view, red.rgba(), 2, 2, QStringLiteral("normal"), 100));
    QVERIFY2(f.committedOnce(base, "Stroke"), "one stroke state");
    // The band rings the selection just outside it; the selection and the far
    // corner stay white.
    QCOMPARE(f.view->sample_argb(6, 12), red.rgba());
    QCOMPARE(f.view->sample_argb(12, 12), QColor(Qt::white).rgba());
    QCOMPARE(f.view->sample_argb(0, 0), QColor(Qt::white).rgba());
}

QTEST_MAIN(FillToolsTest)
#include "tst_fill_tools.moc"
