#include "selftest_paint_tools.h"
#include "selftest_report.h"

#include "selftest_paint_fixture.h"

#include "frame.h"
#include "image_view.h"
#include "options_bar.h"
#include "panels/brush_panel.h"
#include "panels/brush_preset_picker.h"
#include "panels/numeric_field.h"
#include "panels/panel_column.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools.cxxqt.h"

#include <QtCore/QDir>
#include <QtCore/QCoreApplication>
#include <QtCore/QFile>
#include <QtGui/QImage>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QSlider>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>

#include <functional>

namespace {
using paint_fixture::BrushState;
using paint_fixture::Fixture;
} // namespace

int pictura::runRedEyeChecks(pictura::PicturaMainWindow& frame)
{
    // Seed: skin tone with a red 5×5 pupil at (18, 18).
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(QColor(215, 175, 150));
    QPainter(&seed).fillRect(18, 18, 5, 5, QColor(220, 30, 30));
    Fixture f(frame, seed, QStringLiteral("pictura_redeye_seed"));
    if (!f.ok()) {
        return pictura::selfTest().fail(539, "red eye fixture");
    }

    frame.setActiveTool(pictura::ToolId::RedEye);
    const bool active = f.tools->activeTool() == pictura::ToolId::RedEye;
    auto* pupil = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsRedEyePupil"));
    auto* darken =
        frame.findChild<pictura::NumericField*>(QStringLiteral("optionsRedEyeDarken"));
    const bool bar = pupil && darken && pupil->value() == 50 && darken->value() == 50
        && f.tools->optionsFor<RedEyeGroup>(ToolId::RedEye)->get().pupil == 50 && f.tools->optionsFor<RedEyeGroup>(ToolId::RedEye)->get().darken == 50;

    const int base = f.view->history_index();
    f.drag({QPointF(14, 14), QPointF(20, 20), QPointF(26, 26)});
    const bool committed = f.committedOnce(base, "Red Eye Tool");
    const quint32 pupilArgb = f.view->sample_argb(20, 20);
    const bool fixed = qRed(pupilArgb) < 80;
    const bool skinKept = f.view->sample_argb(15, 15) == QColor(215, 175, 150).rgba();
    // A click on skin alone (no red in its box) records nothing.
    f.drag({QPointF(5, 5)});
    const bool clickNoop = f.view->history_index() == base + 1;

    ST_BEGIN("red_eye_tool");
    ST_PASS("red eye active=%d bar=%d commit=%d fixed=%d skin=%d click=%d", active ? 1 : 0,
            bar ? 1 : 0, committed ? 1 : 0, fixed ? 1 : 0, skinKept ? 1 : 0, clickNoop ? 1 : 0);
    if (!active || !bar || !committed || !fixed || !skinKept || !clickNoop) {
        return pictura::selfTest().fail(539, "red eye tool");
    }
    return 0;
}

int pictura::runColorReplacementChecks(pictura::PicturaMainWindow& frame)
{
    // Seed: a blue field (x < 30) beside a yellow one.
    QImage seed(60, 20, QImage::Format_RGB32);
    seed.fill(QColor(220, 220, 40));
    QPainter(&seed).fillRect(0, 0, 30, 20, QColor(40, 40, 200));
    Fixture f(frame, seed, QStringLiteral("pictura_replace_seed"));
    if (!f.ok()) {
        return pictura::selfTest().fail(540, "color replacement fixture");
    }

    frame.setActiveTool(pictura::ToolId::ColorReplacement);
    const bool active = f.tools->activeTool() == pictura::ToolId::ColorReplacement;
    auto combo = [&frame](const char* name) {
        return frame.findChild<QComboBox*>(QString::fromLatin1(name));
    };
    QComboBox* mode = combo("optionsColorReplaceMode");
    QComboBox* sampling = combo("optionsColorReplaceSampling");
    QComboBox* limits = combo("optionsColorReplaceLimits");
    bool bar = mode && sampling && limits && mode->currentIndex() == 2
        && sampling->currentIndex() == 0 && limits->currentIndex() == 1;
    if (bar) {
        mode->setCurrentIndex(3);
        bar = f.tools->optionsFor<ColorReplaceGroup>(ToolId::ColorReplacement)->get().mode == 3;
        mode->setCurrentIndex(2);
        bar = bar && f.tools->optionsFor<ColorReplaceGroup>(ToolId::ColorReplacement)->get().mode == 2;
    }

    const QColor foreground = f.tools->foreground();
    const int size = f.tools->optionsFor<PaintOptions>(ToolId::Brush)->size();
    f.tools->setForeground(QColor(40, 200, 40));
    f.tools->optionsFor<PaintOptions>(ToolId::Brush)->setSize(10);
    const int base = f.view->history_index();
    f.drag({QPointF(8, 10), QPointF(14, 10), QPointF(20, 10)});
    const bool committed = f.committedOnce(base, "Color Replacement Tool");
    const quint32 argb = f.view->sample_argb(14, 10);
    const bool recoloured = qGreen(argb) > qRed(argb) + 20 && qGreen(argb) > qBlue(argb);
    const bool farKept = f.view->sample_argb(45, 10) == QColor(220, 220, 40).rgba();
    f.tools->setForeground(foreground);
    f.tools->optionsFor<PaintOptions>(ToolId::Brush)->setSize(size);

    ST_BEGIN("color_replacement_tool");
    ST_PASS("color replacement active=%d bar=%d commit=%d recoloured=%d far=%d",
            active ? 1 : 0, bar ? 1 : 0, committed ? 1 : 0, recoloured ? 1 : 0,
            farKept ? 1 : 0);
    if (!active || !bar || !committed || !recoloured || !farKept) {
        return pictura::selfTest().fail(540, "color replacement tool");
    }
    return 0;
}

int pictura::runMixerBrushChecks(pictura::PicturaMainWindow& frame)
{
    // Seed: a black band (x < 20) on white.
    QImage seed(64, 40, QImage::Format_RGB32);
    seed.fill(Qt::white);
    QPainter(&seed).fillRect(0, 0, 20, 40, Qt::black);
    Fixture f(frame, seed, QStringLiteral("pictura_mixer_seed"));
    if (!f.ok()) {
        return pictura::selfTest().fail(541, "mixer brush fixture");
    }

    frame.setActiveTool(pictura::ToolId::MixerBrush);
    const bool active = f.tools->activeTool() == pictura::ToolId::MixerBrush;
    const pictura::MixerOptions saved = f.tools->optionsFor<MixerGroup>(ToolId::MixerBrush)->get();
    const QColor savedPaint = f.tools->optionsFor<MixerGroup>(ToolId::MixerBrush)->get().reservoir;
    const int savedSize = f.tools->optionsFor<PaintOptions>(ToolId::Brush)->size();

    // The preset menu sets Wet/Load/Mix; Load and Mix grey out while dry.
    auto* preset = frame.findChild<QComboBox*>(QStringLiteral("optionsMixerPreset"));
    auto* mix = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsMixerMix"));
    bool bar = preset && mix && preset->currentText() == QStringLiteral("Dry")
        && !mix->isEnabled();
    if (bar) {
        const int wetHeavy = preset->findText(QStringLiteral("Wet, Heavy Mix"));
        preset->setCurrentIndex(wetHeavy);
        emit preset->activated(wetHeavy);
        const pictura::MixerOptions o = f.tools->optionsFor<MixerGroup>(ToolId::MixerBrush)->get();
        bar = o.wet == 50 && o.load == 50 && o.mix == 60 && mix->isEnabled()
            && mix->value() == 60;
    }

    // A fully wet smear from the black band into the white drags black along
    // and leaves the pickup on the brush.
    pictura::MixerOptions wet = saved;
    wet.wet = 100;
    wet.load = 100;
    wet.mix = 100;
    f.tools->optionsFor<MixerGroup>(ToolId::MixerBrush)->set(wet);
    {
        auto* mixer = f.tools->optionsFor<MixerGroup>(ToolId::MixerBrush);
        MixerOptions o = mixer->get();
        o.reservoir = Qt::white;
        mixer->set(o);
    }
    f.tools->optionsFor<PaintOptions>(ToolId::Brush)->setSize(10);
    const int base = f.view->history_index();
    QList<QPointF> path;
    for (int x = 10; x <= 40; x += 2) {
        path << QPointF(x, 20);
    }
    f.drag(path);
    const bool committed = f.committedOnce(base, "Mixer Brush Tool");
    const bool smeared = qRed(f.view->sample_argb(26, 20)) < 250;
    const bool carried = f.tools->optionsFor<MixerGroup>(ToolId::MixerBrush)->get().reservoir.red() < 255;
    // Clean After Stroke empties the brush.
    wet.cleanAfterStroke = true;
    f.tools->optionsFor<MixerGroup>(ToolId::MixerBrush)->set(wet);
    f.drag({QPointF(50, 30), QPointF(56, 30)});
    const bool cleaned = f.tools->optionsFor<MixerGroup>(ToolId::MixerBrush)->get().reservoir.alpha() == 0;
    // Alt-click loads the brush from the image.
    f.drag({QPointF(5, 5)}, Qt::AltModifier);
    const bool loaded = f.tools->optionsFor<MixerGroup>(ToolId::MixerBrush)->get().reservoir == QColor(Qt::black);

    f.tools->optionsFor<MixerGroup>(ToolId::MixerBrush)->set(saved);
    {
        auto* mixer = f.tools->optionsFor<MixerGroup>(ToolId::MixerBrush);
        MixerOptions o = mixer->get();
        o.reservoir = savedPaint;
        mixer->set(o);
    }
    f.tools->optionsFor<PaintOptions>(ToolId::Brush)->setSize(savedSize);

    ST_BEGIN("mixer_brush_tool");
    ST_PASS("mixer active=%d bar=%d commit=%d smeared=%d carried=%d cleaned=%d loaded=%d",
            active ? 1 : 0, bar ? 1 : 0, committed ? 1 : 0, smeared ? 1 : 0, carried ? 1 : 0,
            cleaned ? 1 : 0, loaded ? 1 : 0);
    if (!active || !bar || !committed || !smeared || !carried || !cleaned || !loaded) {
        return pictura::selfTest().fail(541, "mixer brush tool");
    }
    return 0;
}


int pictura::runCloneStampChecks(pictura::PicturaMainWindow& frame)
{
    // Seed: red (x < 10), green (10..30), white beyond.
    QImage seed(60, 20, QImage::Format_RGB32);
    seed.fill(Qt::white);
    QPainter painter(&seed);
    painter.fillRect(0, 0, 10, 20, QColor(220, 30, 30));
    painter.fillRect(10, 0, 20, 20, QColor(30, 200, 30));
    painter.end();
    Fixture f(frame, seed, QStringLiteral("pictura_clone_seed"));
    if (!f.ok()) {
        return pictura::selfTest().fail(542, "clone stamp fixture");
    }
    frame.setActiveTool(pictura::ToolId::CloneStamp);
    const bool active = f.tools->activeTool() == pictura::ToolId::CloneStamp;
    const BrushState brush(f.tools);

    auto* sample = frame.findChild<QComboBox*>(QStringLiteral("optionsCloneSample"));
    auto* ignore = frame.findChild<QCheckBox*>(QStringLiteral("optionsCloneIgnoreAdjustments"));
    bool bar = sample && ignore && sample->currentIndex() == 0 && ignore->isHidden();
    if (bar) {
        sample->setCurrentIndex(2);
        bar = !ignore->isHidden() && f.tools->optionsFor<StampGroup>(ToolId::CloneStamp)->get().cloneSample == 2;
        sample->setCurrentIndex(0);
        bar = bar && ignore->isHidden() && f.tools->optionsFor<StampGroup>(ToolId::CloneStamp)->get().cloneSample == 0;
    }

    const int base = f.view->history_index();
    f.drag({QPointF(35, 10), QPointF(38, 10)});
    const bool refused = f.view->history_index() == base;
    f.drag({QPointF(5, 10)}, Qt::AltModifier);
    f.drag({QPointF(35, 10), QPointF(38, 10)});
    const bool committed = f.committedOnce(base, "Clone Stamp");
    const bool copied = f.view->sample_argb(36, 10) == QColor(220, 30, 30).rgba();
    const bool farKept = f.view->sample_argb(55, 10) == QColor(Qt::white).rgba();
    // Aligned keeps the -30 offset, so a stroke at x=45 copies the green.
    f.drag({QPointF(45, 10)});
    const bool aligned = f.view->sample_argb(45, 10) == QColor(30, 200, 30).rgba();

    ST_BEGIN("clone_stamp_tool");
    ST_PASS("clone stamp active=%d bar=%d refused=%d commit=%d copied=%d far=%d aligned=%d",
            active ? 1 : 0, bar ? 1 : 0, refused ? 1 : 0, committed ? 1 : 0, copied ? 1 : 0,
            farKept ? 1 : 0, aligned ? 1 : 0);
    if (!active || !bar || !refused || !committed || !copied || !farKept || !aligned) {
        return pictura::selfTest().fail(542, "clone stamp tool");
    }
    return 0;
}

int pictura::runPatternStampChecks(pictura::PicturaMainWindow& frame)
{
    QImage seed(64, 64, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_pattern_seed"));
    if (!f.ok()) {
        return pictura::selfTest().fail(543, "pattern stamp fixture");
    }
    frame.setActiveTool(pictura::ToolId::PatternStamp);
    const bool active = f.tools->activeTool() == pictura::ToolId::PatternStamp;
    const BrushState brush(f.tools);

    auto* pattern = frame.findChild<QComboBox*>(QStringLiteral("optionsPatternStampPattern"));
    auto* impressionist =
        frame.findChild<QCheckBox*>(QStringLiteral("optionsPatternImpressionist"));
    const bool bar = pattern && impressionist && pattern->count() == 8
        && pattern->currentText() == QStringLiteral("Checkerboard")
        && !pattern->itemIcon(0).isNull() && !impressionist->isEnabled();

    // The checkerboard's 32 px quadrants: light (215) where x < 32 matches
    // y < 32, dark (90) elsewhere.
    const int base = f.view->history_index();
    f.drag({QPointF(18, 16), QPointF(28, 16), QPointF(40, 16)});
    const bool committed = f.committedOnce(base, "Pattern Stamp");
    const bool tiledOk =
        qRed(f.view->sample_argb(20, 16)) == 215 && qRed(f.view->sample_argb(40, 16)) == 90;
    // Unaligned pins the tile's corner to the stroke start: light there, where
    // the document-aligned tile is dark.
    pictura::StampOptions o = f.tools->optionsFor<StampGroup>(ToolId::CloneStamp)->get();
    o.patternAligned = false;
    f.tools->optionsFor<StampGroup>(ToolId::CloneStamp)->set(o);
    f.drag({QPointF(20, 50)});
    const bool unaligned = qRed(f.view->sample_argb(20, 50)) == 215;

    ST_BEGIN("pattern_stamp_tool");
    ST_PASS("pattern stamp active=%d bar=%d commit=%d tiled=%d unaligned=%d", active ? 1 : 0,
            bar ? 1 : 0, committed ? 1 : 0, tiledOk ? 1 : 0, unaligned ? 1 : 0);
    if (!active || !bar || !committed || !tiledOk || !unaligned) {
        return pictura::selfTest().fail(543, "pattern stamp tool");
    }
    return 0;
}

int pictura::runHistoryBrushChecks(pictura::PicturaMainWindow& frame)
{
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_history_brush_seed"));
    if (!f.ok()) {
        return pictura::selfTest().fail(544, "history brush fixture");
    }
    const BrushState brush(f.tools);
    const QColor foreground = f.tools->foreground();
    f.tools->setForeground(Qt::black);
    frame.setActiveTool(pictura::ToolId::Brush);
    const int base = f.view->history_index();
    f.drag({QPointF(5, 20), QPointF(20, 20), QPointF(35, 20)});
    const bool painted = f.committedOnce(base, "Brush")
        && f.view->sample_argb(10, 20) == QColor(Qt::black).rgba();
    f.tools->setForeground(foreground);

    frame.setActiveTool(pictura::ToolId::HistoryBrush);
    const bool active = f.tools->activeTool() == pictura::ToolId::HistoryBrush;
    f.drag({QPointF(5, 20), QPointF(10, 20), QPointF(15, 20)});
    const bool committed = f.committedOnce(base + 1, "History Brush");
    const bool restored = f.view->sample_argb(10, 20) == QColor(Qt::white).rgba()
        && f.view->sample_argb(30, 20) == QColor(Qt::black).rgba();

    // A press in the History panel's left column makes the Brush state the
    // source; painting from it brings the stroke back.
    bool chosen = false;
    auto* panel = frame.findChild<QWidget*>(QStringLiteral("historyPanel"));
    auto* list = panel ? panel->findChild<QListWidget*>() : nullptr;
    pictura::PanelColumn* column = frame.panelColumn();
    if (list && column) {
        column->showPanel(QStringLiteral("historyPanel"), true);
        QCoreApplication::processEvents();
        const int rowIndex = f.view->history_snapshot_count() + base + 1;
        if (QListWidgetItem* row = list->item(rowIndex)) {
            const QRect rect = list->visualItemRect(row);
            const QPointF at(rect.left() + 4, rect.center().y());
            QMouseEvent press(QEvent::MouseButtonPress, at, list->viewport()->mapToGlobal(at),
                              Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
            // The press rebuilds the rows, so `row` is gone afterwards.
            QApplication::sendEvent(list->viewport(), &press);
            chosen = f.view->history_index() == base + 2
                && pictura::history_brush_source_state(*f.view) == base + 1;
        }
        column->showPanel(QStringLiteral("historyPanel"), false);
    }
    f.drag({QPointF(5, 20), QPointF(10, 20), QPointF(15, 20)});
    const bool repainted = chosen && f.view->sample_argb(10, 20) == QColor(Qt::black).rgba();

    ST_BEGIN("history_brush_tool");
    ST_PASS("history brush painted=%d active=%d commit=%d restored=%d chosen=%d repainted=%d",
            painted ? 1 : 0, active ? 1 : 0, committed ? 1 : 0, restored ? 1 : 0,
            chosen ? 1 : 0, repainted ? 1 : 0);
    if (!painted || !active || !committed || !restored || !chosen || !repainted) {
        return pictura::selfTest().fail(544, "history brush tool");
    }
    return 0;
}

namespace {

bool panelVisible(pictura::PicturaMainWindow& frame, const QString& panel)
{
    pictura::PanelColumn* column = frame.columnForPanel(panel);
    return column && column->isPanelVisible(panel);
}

// Click an options-bar button by object name; false when it is missing.
bool clickBar(pictura::PicturaMainWindow& frame, const char* name)
{
    auto* button = frame.findChild<QToolButton*>(QString::fromLatin1(name));
    if (!button) {
        return false;
    }
    button->click();
    QCoreApplication::processEvents();
    return true;
}

} // namespace

int pictura::runBrushPanelChecks(pictura::PicturaMainWindow& frame)
{
    QImage seed(60, 40, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_brush_panel_seed"));
    if (!f.ok()) {
        return pictura::selfTest().fail(545, "brush panel fixture");
    }
    const BrushState brush(f.tools);
    frame.setActiveTool(pictura::ToolId::CloneStamp);
    // Earlier layout checks may leave the panel either way; each click flips it.
    const QString name = QStringLiteral("brushPanel");
    const bool before = panelVisible(frame, name);
    const bool shown = clickBar(frame, "optionsToggleBrushPanel") && panelVisible(frame, name) != before;
    const bool hidden = clickBar(frame, "optionsToggleBrushPanel") && panelVisible(frame, name) == before;

    auto* panel = frame.findChild<pictura::BrushPanel*>(name);
    auto* roundness =
        frame.findChild<pictura::NumericField*>(QStringLiteral("brushPanelRoundness"));
    bool wired = panel && roundness;
    bool previewed = false;
    if (wired) {
        emit roundness->valueChanged(30);
        wired = f.tools->optionsFor<PaintOptions>(ToolId::Brush)->roundness() == 30 && roundness->value() == 30;
        const QImage preview = panel->previewForTest();
        for (int x = 0; x < preview.width() && !previewed; ++x) {
            for (int y = 0; y < preview.height(); ++y) {
                if (qAlpha(preview.pixel(x, y)) > 0) {
                    previewed = true;
                    break;
                }
            }
        }
    }
    // A 30 % round, 0° tip is flat: one dab covers more across than down.
    const QColor foreground = f.tools->foreground();
    f.tools->setForeground(Qt::black);
    f.tools->optionsFor<PaintOptions>(ToolId::Brush)->setSize(20);
    frame.setActiveTool(pictura::ToolId::Brush);
    f.drag({QPointF(30, 20)});
    auto painted = [&f](int x, int y) { return f.view->sample_argb(x, y) != QColor(Qt::white).rgba(); };
    const bool flat = painted(37, 20) && !painted(30, 27);
    f.tools->setForeground(foreground);
    f.tools->optionsFor<PaintOptions>(ToolId::Brush)->setRoundness(100);

    ST_BEGIN("brush_panel");
    ST_PASS("brush panel shown=%d hidden=%d wired=%d preview=%d flat=%d", shown ? 1 : 0,
            hidden ? 1 : 0, wired ? 1 : 0, previewed ? 1 : 0, flat ? 1 : 0);
    if (!shown || !hidden || !wired || !previewed || !flat) {
        return pictura::selfTest().fail(545, "brush panel");
    }
    return 0;
}

int pictura::runCloneSourcePanelChecks(pictura::PicturaMainWindow& frame)
{
    // Seed: red (x < 5), green (5..10), white beyond.
    QImage seed(60, 20, QImage::Format_RGB32);
    seed.fill(Qt::white);
    QPainter painter(&seed);
    painter.fillRect(0, 0, 5, 20, QColor(220, 30, 30));
    painter.fillRect(5, 0, 5, 20, QColor(30, 200, 30));
    painter.end();
    Fixture f(frame, seed, QStringLiteral("pictura_clone_source_seed"));
    if (!f.ok()) {
        return pictura::selfTest().fail(546, "clone source panel fixture");
    }
    const BrushState brush(f.tools);
    frame.setActiveTool(pictura::ToolId::CloneStamp);
    const QString name = QStringLiteral("cloneSourcePanel");
    const bool before = panelVisible(frame, name);
    const bool shown =
        clickBar(frame, "optionsToggleCloneSourcePanel") && panelVisible(frame, name) != before;

    // Earlier checks may have left sources in the controller's slots.
    for (int slot : {1, 0}) {
        f.tools->optionsFor<CloneSourceOptions>(ToolId::CloneStamp)->setSlot(slot);
        f.tools->optionsFor<CloneSourceOptions>(ToolId::CloneStamp)->setActive(pictura::CloneSource{});
    }
    // Slot 2 gets a source; slot 1 stays empty, so a stroke there is refused.
    const bool slotButtons = clickBar(frame, "cloneSourceSlot2") && f.tools->optionsFor<CloneSourceOptions>(ToolId::CloneStamp)->slot() == 1;
    f.drag({QPointF(5, 10)}, Qt::AltModifier);
    clickBar(frame, "cloneSourceSlot1");
    const int base = f.view->history_index();
    f.drag({QPointF(40, 10)});
    const bool perSlot = slotButtons && f.view->history_index() == base
        && !f.tools->optionsFor<CloneSourceOptions>(ToolId::CloneStamp)->active().hasSource;
    clickBar(frame, "cloneSourceSlot2");

    // Flip Horizontal mirrors about the stroke start at x = 40.
    const bool flipped = clickBar(frame, "cloneSourceFlipH") && f.tools->optionsFor<CloneSourceOptions>(ToolId::CloneStamp)->active().flipH;
    f.drag({QPointF(40, 10), QPointF(42, 10), QPointF(44, 10)});
    const bool mirrored = f.committedOnce(base, "Clone Stamp")
        && f.view->sample_argb(43, 10) == QColor(220, 30, 30).rgba();
    auto* offsetX = frame.findChild<pictura::NumericField*>(QStringLiteral("cloneSourceOffsetX"));
    const bool offset = offsetX && offsetX->value() == 35;
    clickBar(frame, "cloneSourceReset");
    const bool reset = !f.tools->optionsFor<CloneSourceOptions>(ToolId::CloneStamp)->active().flipH;
    f.tools->optionsFor<CloneSourceOptions>(ToolId::CloneStamp)->setActive(pictura::CloneSource{});
    clickBar(frame, "cloneSourceSlot1");
    const bool hidden =
        clickBar(frame, "optionsToggleCloneSourcePanel") && panelVisible(frame, name) == before;

    ST_BEGIN("clone_source_panel");
    ST_PASS("clone source shown=%d slots=%d flip=%d mirrored=%d offset=%d reset=%d hidden=%d",
            shown ? 1 : 0, perSlot ? 1 : 0, flipped ? 1 : 0, mirrored ? 1 : 0, offset ? 1 : 0,
            reset ? 1 : 0, hidden ? 1 : 0);
    if (!shown || !perSlot || !flipped || !mirrored || !offset || !reset || !hidden) {
        return pictura::selfTest().fail(546, "clone source panel");
    }
    return 0;
}

int pictura::runArtHistoryBrushChecks(pictura::PicturaMainWindow& frame)
{
    QImage seed(60, 40, QImage::Format_RGB32);
    seed.fill(Qt::red);
    Fixture f(frame, seed, QStringLiteral("pictura_art_history_seed"));
    if (!f.ok()) {
        return pictura::selfTest().fail(548, "art history brush fixture");
    }
    const BrushState brush(f.tools);
    const QColor foreground = f.tools->foreground();
    const pictura::ArtHistoryOptions options = f.tools->optionsFor<ArtHistoryGroup>(ToolId::ArtHistoryBrush)->get();

    // Paint the left edge (x < 25) white so it no longer matches the red source.
    f.tools->setForeground(Qt::white);
    f.tools->optionsFor<PaintOptions>(ToolId::Brush)->setSize(20);
    frame.setActiveTool(pictura::ToolId::Brush);
    f.drag({QPointF(10, 20), QPointF(14, 20)});
    f.tools->optionsFor<PaintOptions>(ToolId::Brush)->setSize(4);

    frame.setActiveTool(pictura::ToolId::ArtHistoryBrush);
    auto* style = frame.findChild<QComboBox*>(QStringLiteral("optionsArtHistoryStyle"));
    auto* area = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsArtHistoryArea"));
    auto* tolerance =
        frame.findChild<pictura::NumericField*>(QStringLiteral("optionsArtHistoryTolerance"));
    const bool bar = style && area && tolerance && style->count() == 10
        && style->currentText() == QStringLiteral("Tight Short") && area->value() == 50
        && tolerance->value() == 0;

    const auto redCount = [&f] {
        int n = 0;
        for (int y = 0; y < 40; ++y) {
            for (int x = 0; x < 25; ++x) {
                const QRgb px = f.view->sample_argb(x, y);
                n += qRed(px) > 200 && qGreen(px) < 100 ? 1 : 0;
            }
        }
        return n;
    };
    const int redBefore = redCount();
    const int base = f.view->history_index();
    f.drag({QPointF(8, 20), QPointF(14, 20), QPointF(20, 20)});
    const bool committed = f.committedOnce(base, "Art History Brush");
    // The stroke over white laid source red back, well beyond the 4 px tip.
    const int reddened = redCount() - redBefore;
    const bool painted = committed && reddened > 100;

    // At full Tolerance, red that already matches the source is left alone
    // (a 10 px Area keeps the scatter clear of the white).
    pictura::ArtHistoryOptions strict = options;
    strict.tolerance = 100;
    strict.area = 10;
    f.tools->optionsFor<ArtHistoryGroup>(ToolId::ArtHistoryBrush)->set(strict);
    f.drag({QPointF(45, 20), QPointF(50, 20), QPointF(55, 20)});
    const bool gated = f.view->history_index() == base + 1;

    f.tools->optionsFor<ArtHistoryGroup>(ToolId::ArtHistoryBrush)->set(options);
    f.tools->setForeground(foreground);

    ST_BEGIN("art_history_brush_tool");
    ST_PASS("art history bar=%d commit=%d reddened=%d gated=%d", bar ? 1 : 0, committed ? 1 : 0,
            reddened, gated ? 1 : 0);
    if (!bar || !painted || !gated) {
        return pictura::selfTest().fail(548, "art history brush tool");
    }
    return 0;
}

int pictura::runBrushPresetPickerChecks(pictura::PicturaMainWindow& frame)
{
    namespace scale = pictura::brush_size_scale;
    // The first half of the Size slider runs 1-100 px; the second climbs to 5000.
    bool mapping = scale::sizeAt(0) == 1 && scale::sizeAt(scale::kSteps / 2) == 100
        && scale::sizeAt(scale::kSteps) == 5000 && scale::sizeAt(scale::kSteps / 4) == 51
        && scale::sizeAt(scale::kSteps * 3 / 4) == 707;
    // Every size up to 100 has a position; above it the travel only climbs.
    for (int size = 1; size <= 100; ++size) {
        mapping = mapping && scale::sizeAt(scale::positionOf(size)) == size;
    }
    for (int p = 1; p <= scale::kSteps; ++p) {
        mapping = mapping && scale::sizeAt(p) >= scale::sizeAt(p - 1);
    }

    QImage seed(60, 60, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_brush_picker_seed"));
    if (!f.ok()) {
        return pictura::selfTest().fail(549, "brush preset picker fixture");
    }
    const BrushState brush(f.tools);
    const int hardness = f.tools->optionsFor<PaintOptions>(ToolId::Brush)->hardness();
    const int roundness = f.tools->optionsFor<PaintOptions>(ToolId::Brush)->roundness();
    const int angle = f.tools->optionsFor<PaintOptions>(ToolId::Brush)->tipAngle();
    const int spacing = f.tools->optionsFor<PaintOptions>(ToolId::Brush)->spacing();
    const QColor foreground = f.tools->foreground();

    // Clicking the Eraser's tip button opens the picker with the default set.
    frame.setActiveTool(pictura::ToolId::Eraser);
    auto* bar = frame.findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    auto* stack = bar ? bar->findChild<QStackedWidget*>() : nullptr;
    QWidget* page = stack ? stack->currentWidget() : nullptr;
    auto* tip = page ? page->findChild<QToolButton*>(QStringLiteral("optionsBrushTip")) : nullptr;
    bool opened = false;
    bool slider = false;
    bool preset = false;
    if (tip) {
        tip->click();
        pictura::BrushPresetPicker* picker = bar->brushPicker();
        auto* grid = picker->findChild<QListWidget*>(QStringLiteral("brushPickerGrid"));
        opened = picker->isVisible() && grid && grid->count() == picker->presetCount()
            && picker->presetCount() >= 40;
        auto* sizeSlider = picker->findChild<QSlider*>(QStringLiteral("brushPickerSizeSlider"));
        if (sizeSlider) {
            sizeSlider->setValue(scale::kSteps / 2);
            slider = f.tools->optionsFor<PaintOptions>(ToolId::Brush)->size() == 100 && tip->text() == QStringLiteral("100");
        }
        // "Spatter 24": a cluster of seven dabs scattered 150 % of the size.
        picker->choosePresetForTest(25);
        const pictura::BrushDynamics d = f.tools->optionsFor<PaintOptions>(ToolId::Brush)->dynamics();
        preset = f.tools->optionsFor<PaintOptions>(ToolId::Brush)->size() == 24 && d.count == 7 && d.scatter == 150;
        picker->hide();
    }

    // One Brush click with the spatter tip lands paint well off the 24 px tip.
    f.tools->setForeground(Qt::black);
    frame.setActiveTool(pictura::ToolId::Brush);
    f.drag({QPointF(30, 30)});
    bool scattered = false;
    for (int y = 0; y < 60 && !scattered; ++y) {
        for (int x = 0; x < 60; ++x) {
            const bool far = (x - 30) * (x - 30) + (y - 30) * (y - 30) > 14 * 14;
            if (far && f.view->sample_argb(x, y) != QColor(Qt::white).rgba()) {
                scattered = true;
                break;
            }
        }
    }

    f.tools->setForeground(foreground);
    f.tools->optionsFor<PaintOptions>(ToolId::Brush)->setHardness(hardness);
    f.tools->optionsFor<PaintOptions>(ToolId::Brush)->setRoundness(roundness);
    f.tools->optionsFor<PaintOptions>(ToolId::Brush)->setTipAngle(angle);
    f.tools->optionsFor<PaintOptions>(ToolId::Brush)->setSpacing(spacing);
    f.tools->optionsFor<PaintOptions>(ToolId::Brush)->setDynamics(pictura::BrushDynamics{});

    ST_BEGIN("brush_preset_picker");
    ST_PASS("brush preset picker mapping=%d opened=%d slider=%d preset=%d scattered=%d",
            mapping ? 1 : 0, opened ? 1 : 0, slider ? 1 : 0, preset ? 1 : 0, scattered ? 1 : 0);
    if (!mapping || !opened || !slider || !preset || !scattered) {
        return pictura::selfTest().fail(549, "brush preset picker");
    }
    return 0;
}
