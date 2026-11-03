// The Brush's Airbrush option (#1120): the options-bar toggle and Brush panel
// Build-up row are one flag; `Shift+Alt+P` flips the active tool's flag; a held
// button keeps painting on the exact, preview, GPU, and mask stroke backends.

#include <QtTest/QtTest>

#include "frame.h"
#include "options_bar.h"
#include "selftest_paint_fixture.h"
#include "tool_options_json.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_history/display.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools.cxxqt.h"

#include "qt_test_support.h"

#include <QtGui/QColor>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QToolButton>

#include <memory>

namespace {
using paint_fixture::BrushState;
using paint_fixture::Fixture;

// Long enough for several 50 ms held dabs.
constexpr int kHoldWaitMs = 250;

const QPointF kAt(16, 16);

QImage whiteSeed()
{
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(Qt::white);
    return seed;
}

// The canvas, one number per image state.
quint64 signature(const Fixture& f)
{
    quint64 sum = 0;
    for (int y = 0; y < 40; ++y) {
        for (int x = 0; x < 40; ++x) {
            sum = sum * 1099511628211ULL + f.view->sample_argb(x, y);
        }
    }
    return sum;
}

// A small, soft-ish tip at low Flow, so a repeated dab shows.
void armBrush(const Fixture& f, bool airbrush)
{
    auto* paint = f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush);
    paint->setSize(12);
    paint->setFlow(10);
    paint->setOpacity(100);
    paint->setAirbrush(airbrush);
    // Black over the white fixture, and black hides a reveal-all mask.
    f.tools->setForeground(Qt::black);
    f.frame.setActiveTool(pictura::ToolId::Brush);
}

void press(const Fixture& f)
{
    f.canvas->mousePressed(kAt, Qt::LeftButton, 0);
}

void release(const Fixture& f)
{
    f.canvas->mouseReleased(kAt);
}

void expectStill(const Fixture& f)
{
    press(f);
    QTest::qWait(0);
    const quint64 before = signature(f);
    QTest::qWait(kHoldWaitMs); // proves an absence, which QTRY_ cannot
    QCOMPARE(signature(f), before);
    release(f);
}

void expectBuilds(const Fixture& f, const char* label)
{
    const int base = f.view->history_index();
    press(f);
    QTest::qWait(0);
    const quint64 before = signature(f);
    QTRY_VERIFY(signature(f) != before);
    release(f);
    QVERIFY(f.committedOnce(base, label));
}

// A reveal-all mask on the fixture's Background, with the mask as the edit
// target. Black paint then hides the layer.
bool addMask(const Fixture& f)
{
    const QString path = f.view->add_solid_fill(0xff2244aau);
    if (path.isEmpty() || !f.view->rasterize_fill_content(path)) {
        return false;
    }
    pictura::layer_mask_delete_path(*f.view, path);
    f.frame.selectLayerPath(path);
    return pictura::layer_mask_add(*f.view, QStringLiteral("reveal-all"))
        && pictura::mask_edit_target_set(*f.view, path);
}

QToolButton* airbrushButton(pictura::PicturaMainWindow& frame, const char* name)
{
    auto* bar = frame.findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    return bar ? bar->findChild<QToolButton*>(QString::fromLatin1(name)) : nullptr;
}

QListWidgetItem* buildUpItem(pictura::PicturaMainWindow& frame)
{
    auto* sets = frame.findChild<QListWidget*>(QStringLiteral("brushOptionSets"));
    if (!sets) {
        return nullptr;
    }
    for (int i = 0; i < sets->count(); ++i) {
        if (sets->item(i)->text() == QStringLiteral("Build-up")) {
            return sets->item(i);
        }
    }
    return nullptr;
}

// A 600 px stroke on a 2048² document is over the 262 144 px raster budget and
// previews at level 3 (2048 -> 1024 -> 512 -> 256).
bool beginLargeStroke(pictura::PictureView& view)
{
    return view.begin_paint(0xff000000u, 0xffffffffu, 600, 100, 100, 0, 100, 10, 25,
                            QStringLiteral("normal"), false, false);
}

bool addLargeMask(pictura::PicturaMainWindow& frame, pictura::PictureView& view)
{
    const QString path = view.add_solid_fill(0xff2244aau);
    if (path.isEmpty() || !view.rasterize_fill_content(path)) {
        return false;
    }
    // A fill layer may carry the automatic default mask; replace it with a
    // reveal-all mask so the fill shows.
    pictura::layer_mask_delete_path(view, path);
    frame.selectLayerPath(path);
    return pictura::layer_mask_add(view, QStringLiteral("reveal-all"))
        && pictura::mask_edit_target_set(view, path);
}
} // namespace

class BrushAirbrushTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void airbrushOffStaysStill();
    void airbrushOnBuilds();
    void optionsBarAndPanelShareTheFlag();
    void pressureButtonsToggleAndPersist();
    void pressureSizeShrinksTheDab();
    void pressureOpacityLightensTheDab();
    void shiftAltPTogglesTheBrush();
    void shiftAltPTogglesAStamp();
    void shiftAltPIgnoresToolsWithoutTheOption();
    void holdStopsOnRelease();
    void holdStopsOnToolSwitch();
    void holdStopsOnSessionCancel();
    void toolSwitchCommitsStroke();
    void maskHoldBuilds();
    void previewHoldReachesTheCommit();
    void maskPreviewHoldReachesTheCommit();
    void gpuHoldBuilds();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void BrushAirbrushTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->show();
    window_->activateWindow();
}

void BrushAirbrushTest::cleanup()
{
    while (window_ && window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
}

void BrushAirbrushTest::airbrushOffStaysStill()
{
    Fixture f(*window_, whiteSeed(), QStringLiteral("pictura_brush_airbrush_off"));
    QVERIFY(f.ok());
    const BrushState brush(f.tools);
    armBrush(f, false);
    expectStill(f);
}

void BrushAirbrushTest::airbrushOnBuilds()
{
    Fixture f(*window_, whiteSeed(), QStringLiteral("pictura_brush_airbrush_on"));
    QVERIFY(f.ok());
    const BrushState brush(f.tools);
    armBrush(f, true);
    expectBuilds(f, "Brush");
}

void BrushAirbrushTest::optionsBarAndPanelShareTheFlag()
{
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(tools);
    const BrushState brush(tools);
    window_->setActiveTool(pictura::ToolId::Brush);

    auto* paint = tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush);
    auto* button = airbrushButton(*window_, "optionsBrushAirbrush");
    auto* item = buildUpItem(*window_);
    QVERIFY(paint && button && item);
    QVERIFY(item->flags().testFlag(Qt::ItemIsUserCheckable));
    QVERIFY(!paint->airbrush());
    QCOMPARE(item->checkState(), Qt::Unchecked);

    button->click();
    QVERIFY(paint->airbrush());
    QCOMPARE(item->checkState(), Qt::Checked);

    item->setCheckState(Qt::Unchecked);
    QVERIFY(!paint->airbrush());
    QVERIFY(!button->isChecked());

    // The Pencil has no Airbrush option even though it shares PaintOptions.
    auto* bar = window_->findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    QVERIFY(bar);
    QVERIFY(!bar->pageForTest(pictura::ToolId::Pencil)
                 ->findChild<QToolButton*>(QStringLiteral("optionsBrushAirbrush")));
}

void BrushAirbrushTest::pressureButtonsToggleAndPersist()
{
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(tools);
    const BrushState brush(tools);
    window_->setActiveTool(pictura::ToolId::Brush);
    auto* paint = tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush);
    auto* bar = window_->findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    QVERIFY(paint && bar);
    QWidget* page = bar->pageForTest(pictura::ToolId::Brush);
    auto* size = page->findChild<QToolButton*>(QStringLiteral("optionsBrushPressureSize"));
    auto* opacity = page->findChild<QToolButton*>(QStringLiteral("optionsBrushPressureOpacity"));
    QVERIFY(size && opacity);
    QVERIFY(!size->icon().isNull());
    QVERIFY(!opacity->icon().isNull());
    QVERIFY(!paint->pressureSize());
    QVERIFY(!paint->pressureOpacity());

    size->click();
    opacity->click();
    QVERIFY(paint->pressureSize());
    QVERIFY(paint->pressureOpacity());
    QVERIFY(size->isChecked());
    QVERIFY(opacity->isChecked());

    const QJsonObject saved = pictura::toolOptionsToJson(*tools);
    paint->setPressureSize(false);
    paint->setPressureOpacity(false);
    pictura::applyToolOptionsJson(*tools, saved);
    QVERIFY(paint->pressureSize());
    QVERIFY(paint->pressureOpacity());
}

void BrushAirbrushTest::pressureSizeShrinksTheDab()
{
    quint32 full = 0;
    {
        Fixture f(*window_, whiteSeed(), QStringLiteral("pictura_brush_pressure_size_full"));
        QVERIFY(f.ok());
        const BrushState brush(f.tools);
        armBrush(f, false);
        auto* paint = f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush);
        paint->setPressureSize(true);
        paint->setFlow(100);
        f.tools->setPressure(1.0);
        press(f);
        release(f);
        full = f.view->sample_argb(20, 16);
    }
    quint32 half = 0;
    {
        Fixture f(*window_, whiteSeed(), QStringLiteral("pictura_brush_pressure_size_half"));
        QVERIFY(f.ok());
        const BrushState brush(f.tools);
        armBrush(f, false);
        auto* paint = f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush);
        paint->setPressureSize(true);
        paint->setFlow(100);
        f.tools->setPressure(0.5);
        press(f);
        release(f);
        half = f.view->sample_argb(20, 16);
    }
    QCOMPARE(full, 0xff000000u);
    QCOMPARE(half, 0xffffffffu);
}

void BrushAirbrushTest::pressureOpacityLightensTheDab()
{
    quint32 full = 0;
    {
        Fixture f(*window_, whiteSeed(), QStringLiteral("pictura_brush_pressure_opacity_full"));
        QVERIFY(f.ok());
        const BrushState brush(f.tools);
        armBrush(f, false);
        auto* paint = f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush);
        paint->setPressureOpacity(true);
        paint->setFlow(100);
        f.tools->setPressure(1.0);
        press(f);
        release(f);
        full = f.view->sample_argb(16, 16);
    }
    quint32 half = 0;
    {
        Fixture f(*window_, whiteSeed(), QStringLiteral("pictura_brush_pressure_opacity_half"));
        QVERIFY(f.ok());
        const BrushState brush(f.tools);
        armBrush(f, false);
        auto* paint = f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush);
        paint->setPressureOpacity(true);
        paint->setFlow(100);
        f.tools->setPressure(0.5);
        press(f);
        release(f);
        half = f.view->sample_argb(16, 16);
    }
    QVERIFY2(qRed(full) < qRed(half), "lower pressure paints lighter");
}

void BrushAirbrushTest::shiftAltPTogglesTheBrush()
{
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(tools);
    const BrushState brush(tools);
    auto* paint = tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush);
    QVERIFY(paint);
    paint->setAirbrush(false);
    window_->setActiveTool(pictura::ToolId::Brush);

    QTest::keyClick(window_.get(), Qt::Key_P, Qt::ShiftModifier | Qt::AltModifier);
    QVERIFY(paint->airbrush());
    auto* button = airbrushButton(*window_, "optionsBrushAirbrush");
    auto* item = buildUpItem(*window_);
    QVERIFY(button && item);
    QVERIFY(button->isChecked());
    QCOMPARE(item->checkState(), Qt::Checked);

    QTest::keyClick(window_.get(), Qt::Key_P, Qt::ShiftModifier | Qt::AltModifier);
    QVERIFY(!paint->airbrush());
    QVERIFY(!button->isChecked());
    QCOMPARE(item->checkState(), Qt::Unchecked);
}

void BrushAirbrushTest::shiftAltPTogglesAStamp()
{
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(tools);
    const BrushState brush(tools);
    auto* paint = tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush);
    auto* group = tools->optionsFor<pictura::StampGroup>(pictura::ToolId::CloneStamp);
    QVERIFY(paint && group);
    paint->setAirbrush(false);

    const auto toggle = [this, tools](pictura::ToolId tool, bool pictura::StampOptions::*flag,
                                      const char* buttonName) -> bool {
        window_->setActiveTool(tool);
        auto* group = tools->optionsFor<pictura::StampGroup>(tool);
        if (!group) {
            return false;
        }
        pictura::StampOptions o = group->get();
        o.*flag = false;
        group->set(o);

        QTest::keyClick(window_.get(), Qt::Key_P, Qt::ShiftModifier | Qt::AltModifier);
        if (!(group->get().*flag)) {
            return false;
        }
        auto* button = airbrushButton(*window_, buttonName);
        if (!button || !button->isChecked()) {
            return false;
        }

        QTest::keyClick(window_.get(), Qt::Key_P, Qt::ShiftModifier | Qt::AltModifier);
        if (group->get().*flag) {
            return false;
        }
        return !button->isChecked();
    };

    QVERIFY(toggle(pictura::ToolId::CloneStamp, &pictura::StampOptions::cloneAirbrush,
                   "optionsCloneAirbrush"));
    QVERIFY(!group->get().patternAirbrush);
    QVERIFY(!group->get().historyAirbrush);
    QVERIFY(toggle(pictura::ToolId::PatternStamp, &pictura::StampOptions::patternAirbrush,
                   "optionsPatternAirbrush"));
    QVERIFY(!group->get().cloneAirbrush);
    QVERIFY(!group->get().historyAirbrush);
    QVERIFY(toggle(pictura::ToolId::HistoryBrush, &pictura::StampOptions::historyAirbrush,
                   "optionsHistoryAirbrush"));
    QVERIFY(!group->get().cloneAirbrush);
    QVERIFY(!group->get().patternAirbrush);
    QVERIFY(!paint->airbrush());
}

void BrushAirbrushTest::shiftAltPIgnoresToolsWithoutTheOption()
{
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(tools);
    const BrushState brush(tools);
    auto* paint = tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush);
    QVERIFY(paint);
    paint->setAirbrush(false);

    for (pictura::ToolId tool : {pictura::ToolId::Pencil, pictura::ToolId::Move}) {
        window_->setActiveTool(tool);
        QTest::keyClick(window_.get(), Qt::Key_P, Qt::ShiftModifier | Qt::AltModifier);
        QVERIFY(!paint->airbrush());
    }
}

void BrushAirbrushTest::holdStopsOnRelease()
{
    Fixture f(*window_, whiteSeed(), QStringLiteral("pictura_brush_hold_release"));
    QVERIFY(f.ok());
    const BrushState brush(f.tools);
    armBrush(f, true);
    press(f);
    release(f);
    QTest::qWait(0);
    const quint64 after = signature(f);
    QTest::qWait(kHoldWaitMs); // proves an absence, which QTRY_ cannot
    QCOMPARE(signature(f), after);
}

void BrushAirbrushTest::holdStopsOnToolSwitch()
{
    Fixture f(*window_, whiteSeed(), QStringLiteral("pictura_brush_hold_switch"));
    QVERIFY(f.ok());
    const BrushState brush(f.tools);
    armBrush(f, true);
    press(f);
    window_->setActiveTool(pictura::ToolId::Move);
    QTest::qWait(0);
    const quint64 after = signature(f);
    QTest::qWait(kHoldWaitMs); // proves an absence, which QTRY_ cannot
    QCOMPARE(signature(f), after);
}

void BrushAirbrushTest::holdStopsOnSessionCancel()
{
    Fixture f(*window_, whiteSeed(), QStringLiteral("pictura_brush_hold_cancel"));
    QVERIFY(f.ok());
    const BrushState brush(f.tools);
    armBrush(f, true);
    const int base = f.view->history_index();
    const quint64 before = signature(f);
    press(f);
    f.view->cancel_paint();
    f.tools->onSessionCancelled();
    QTest::qWait(kHoldWaitMs); // proves an absence, which QTRY_ cannot
    QCOMPARE(signature(f), before);
    QCOMPARE(f.view->history_index(), base);
}

void BrushAirbrushTest::toolSwitchCommitsStroke()
{
    Fixture f(*window_, whiteSeed(), QStringLiteral("pictura_brush_hold_commit"));
    QVERIFY(f.ok());
    const BrushState brush(f.tools);
    armBrush(f, true);
    const int base = f.view->history_index();
    press(f);
    window_->setActiveTool(pictura::ToolId::Move);
    QVERIFY(f.committedOnce(base, "Brush"));
    QVERIFY(!f.view->is_painting());
}

void BrushAirbrushTest::maskHoldBuilds()
{
    // A mask stroke does not patch the document composite mid-stroke, so
    // compare two commits: a held stroke and the same stroke released at once.
    quint64 held = 0;
    {
        Fixture f(*window_, whiteSeed(), QStringLiteral("pictura_brush_mask_hold"));
        QVERIFY(f.ok());
        const BrushState brush(f.tools);
        QVERIFY(addMask(f));
        armBrush(f, true);
        const int base = f.view->history_index();
        press(f);
        QTest::qWait(kHoldWaitMs); // the held build-up, not an absence
        release(f);
        QVERIFY(f.committedOnce(base, "Brush"));
        held = signature(f);
    }

    quint64 plain = 0;
    {
        Fixture f(*window_, whiteSeed(), QStringLiteral("pictura_brush_mask_plain"));
        QVERIFY(f.ok());
        const BrushState brush(f.tools);
        QVERIFY(addMask(f));
        armBrush(f, true);
        const int base = f.view->history_index();
        press(f);
        release(f);
        QVERIFY(f.committedOnce(base, "Brush"));
        plain = signature(f);
    }
    QVERIFY2(held != plain, "held dabs did not reach the mask commit");
}

void BrushAirbrushTest::previewHoldReachesTheCommit()
{
    QVERIFY(window_->newDocument(QStringLiteral("BrushPreviewHold"), 2048, 2048,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view);
    view->set_gpu_compute(false);
    QCoreApplication::processEvents();
    QVERIFY(!view->gpu_compute());

    QVERIFY(beginLargeStroke(*view));
    QCOMPARE(pictura::preview_present_level(*view), 3);
    QVERIFY(view->paint_dab(1024.0, 1024.0, 1.0));
    for (int i = 0; i < 6; ++i) {
        QVERIFY(pictura::paint_hold(*view));
    }
    QVERIFY(view->end_paint());
    const quint32 held = view->sample_argb(1024, 1024);
    QVERIFY(view->undo());

    // The same stroke without the held dabs: the commit must differ, or the
    // preview's build-up was dropped by the replay.
    QVERIFY(beginLargeStroke(*view));
    QCOMPARE(pictura::preview_present_level(*view), 3);
    QVERIFY(view->paint_dab(1024.0, 1024.0, 1.0));
    QVERIFY(view->end_paint());
    const quint32 plain = view->sample_argb(1024, 1024);
    QVERIFY2(held != plain, "held dabs did not reach the preview commit");
    QVERIFY(qRed(held) < qRed(plain));
}

void BrushAirbrushTest::maskPreviewHoldReachesTheCommit()
{
    QVERIFY(window_->newDocument(QStringLiteral("BrushMaskPreviewHold"), 2048, 2048,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view);
    QVERIFY(addLargeMask(*window_, *view));
    view->set_gpu_compute(false);
    QCoreApplication::processEvents();
    QVERIFY(!view->gpu_compute());
    const quint32 fill = view->sample_argb(1024, 1024);

    QVERIFY(beginLargeStroke(*view));
    QCOMPARE(pictura::preview_present_level(*view), 3);
    QVERIFY(view->paint_dab(1024.0, 1024.0, 1.0));
    for (int i = 0; i < 6; ++i) {
        QVERIFY(pictura::paint_hold(*view));
    }
    QVERIFY(view->end_paint());
    const quint32 held = view->sample_argb(1024, 1024);
    QVERIFY(held != fill);
    QVERIFY(view->undo());

    QVERIFY(beginLargeStroke(*view));
    QCOMPARE(pictura::preview_present_level(*view), 3);
    QVERIFY(view->paint_dab(1024.0, 1024.0, 1.0));
    QVERIFY(view->end_paint());
    const quint32 plain = view->sample_argb(1024, 1024);
    QVERIFY2(held != plain, "held dabs did not reach the mask preview commit");
    // Black mask coverage hides the blue fill over the white background.
    QVERIFY(qRed(held) > qRed(plain));
    QVERIFY(qGreen(held) > qGreen(plain));
    QVERIFY(qBlue(held) > qBlue(plain));
}

void BrushAirbrushTest::gpuHoldBuilds()
{
    QVERIFY(window_->newDocument(QStringLiteral("BrushGpuHold"), 2048, 2048,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view);
    view->set_gpu_compute(true);
    QCoreApplication::processEvents();
    if (!view->gpu_compute()) {
        QSKIP("no GPU compute device");
    }

    QVERIFY(beginLargeStroke(*view));
    if (pictura::preview_present_level(*view) != 0) {
        view->cancel_paint();
        QSKIP("the GPU stroke was not taken");
    }
    QVERIFY(view->paint_dab(1024.0, 1024.0, 1.0));
    for (int i = 0; i < 6; ++i) {
        QVERIFY(pictura::paint_hold(*view));
    }
    QVERIFY(view->end_paint());
    const quint32 held = view->sample_argb(1024, 1024);
    QVERIFY(view->undo());

    QVERIFY(beginLargeStroke(*view));
    if (pictura::preview_present_level(*view) != 0) {
        view->cancel_paint();
        QSKIP("the GPU control stroke was not taken");
    }
    QVERIFY(view->paint_dab(1024.0, 1024.0, 1.0));
    QVERIFY(view->end_paint());
    const quint32 plain = view->sample_argb(1024, 1024);
    QVERIFY2(held != plain, "held dabs did not reach the GPU commit");

    // The same held path on a mask GPU stroke.
    QVERIFY(pictura::layer_mask_add(*view, QStringLiteral("reveal-all")));
    QVERIFY(pictura::mask_edit_target_set(*view, QStringLiteral("0")));
    QVERIFY(beginLargeStroke(*view));
    if (pictura::preview_present_level(*view) != 0) {
        view->cancel_paint();
        QSKIP("the GPU mask stroke was not taken");
    }
    QVERIFY(view->paint_dab(1024.0, 1024.0, 1.0));
    for (int i = 0; i < 6; ++i) {
        QVERIFY(pictura::paint_hold(*view));
    }
    QVERIFY(view->end_paint());
    QVERIFY(view->undo());
}

QTEST_MAIN(BrushAirbrushTest)
#include "tst_brush_airbrush.moc"
