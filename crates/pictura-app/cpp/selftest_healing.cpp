#include "selftest_healing.h"
#include "selftest_report.h"

#include "frame.h"
#include "icons.h"
#include "image_view.h"
#include "tools.h"

#include "color_picker_dialog.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/annotations.cxxqt.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtGui/QCursor>
#include <QtGui/QImage>
#include <QtGui/QPainter>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QToolButton>

#include <cmath>

int pictura::runHealingChecks(pictura::PicturaMainWindow& frame)
{
    // Seed: a white field with a black 6×6 blemish at (17, 17).
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(Qt::white);
    QPainter(&seed).fillRect(17, 17, 6, 6, Qt::black);
    const QString seedPath = QDir::temp().filePath(QStringLiteral("pictura_healing_seed.png"));

    const bool created = frame.newDocument(QStringLiteral("HealingCtl"), 40, 40,
                                           QStringLiteral("rgb"), 8, QStringLiteral("white"));
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    if (!created || !view || !canvas || !tools || !seed.save(seedPath, "PNG")
        || !view->open_image(seedPath)) {
        return pictura::selfTest().fail(536, "healing fixture");
    }
    const int doc = frame.activeDocumentIndex();
    const auto press = [canvas](QPointF at, Qt::KeyboardModifiers mods = Qt::NoModifier) {
        canvas->mousePressed(at, Qt::LeftButton, int(mods));
        canvas->mouseReleased(at);
    };

    // Spot Healing Brush: one click on the blemish rebuilds it from the white
    // surroundings and records exactly one "Spot Healing Brush" state.
    frame.setActiveTool(pictura::ToolId::SpotHealingBrush);
    const bool spotActive = tools->activeTool() == pictura::ToolId::SpotHealingBrush;
    const int base = view->history_index();
    press(QPointF(20, 20));
    const bool committed = view->history_index() == base + 1
        && view->history_label(base + 1) == QStringLiteral("Spot Healing Brush");
    const quint32 healedArgb = view->sample_argb(20, 20);
    const bool healed = qRed(healedArgb) > 200 && qGreen(healedArgb) > 200
        && qBlue(healedArgb) > 200;

    // Healing Brush: painting before an Alt sample is refused (no state).
    frame.setActiveTool(pictura::ToolId::HealingBrush);
    const int refusedBase = view->history_index();
    press(QPointF(30, 30));
    const bool refused = view->history_index() == refusedBase;
    // After an Alt sample, a stroke commits one "Healing Brush" state.
    press(QPointF(30, 30), Qt::AltModifier);
    press(QPointF(20, 20));
    const bool transplanted = view->history_index() == refusedBase + 1
        && view->history_label(refusedBase + 1) == QStringLiteral("Healing Brush");

    // Count (Extended): numbered marks in groups, the overlay, and the bar.
    frame.setActiveTool(pictura::ToolId::Count);
    const bool countActive = tools->activeTool() == pictura::ToolId::Count;
    auto* groupCombo = frame.findChild<QComboBox*>(QStringLiteral("optionsCountGroup"));
    auto* totalLabel = frame.findChild<QLabel*>(QStringLiteral("optionsCountTotal"));
    const bool bar = groupCombo && totalLabel && groupCombo->count() == 1
        && totalLabel->text().startsWith(QStringLiteral("Count:"));
    const int cbase = view->history_index();
    press(QPointF(5, 5));
    press(QPointF(35, 35));
    const bool counted = pictura::count_active_total(*view) == 2
        && canvas->countOverlayCountForTest() == 2 && view->history_index() == cbase + 2;
    // A new group is independent; its marks overlay alongside group 1's.
    const int g2 = pictura::count_add_group(*view, QStringLiteral("Count Group 2"));
    press(QPointF(20, 20));
    const bool grouped = g2 == 1 && pictura::count_group_total(*view, 0) == 2
        && pictura::count_group_total(*view, 1) == 1 && pictura::count_active_total(*view) == 1
        && canvas->countOverlayCountForTest() == 3;
    // Hiding group 1 leaves only group 2's mark on the overlay.
    pictura::count_set_visible(*view, 0, false);
    const bool hidden = canvas->countOverlayCountForTest() == 1;
    pictura::count_set_visible(*view, 0, true);
    // Colour and sizes are stored on the group.
    pictura::count_set_color(*view, 1, 0xff0000);
    pictura::count_set_marker_size(*view, 1, 6);
    pictura::count_set_label_size(*view, 1, 20);
    const bool styled = pictura::count_group_color(*view, 1) == 0xff0000
        && pictura::count_group_marker_size(*view, 1) == 6
        && pictura::count_group_label_size(*view, 1) == 20;
    // Delete the second group, then Clear the first.
    const bool deletedGroup =
        pictura::count_remove_group(*view, 1) && pictura::count_group_count(*view) == 1;
    const bool cleared = pictura::count_clear(*view) && pictura::count_active_total(*view) == 0
        && canvas->countOverlayCountForTest() == 0;
    frame.setActiveTool(pictura::ToolId::Move);

    // The full color picker (ported from photorust) carries its initial colour
    // and its web-safe / Lab helpers behave.
    const QColor initial(10, 20, 30);
    pictura::ColorPickerDialog picker(initial, nullptr, QStringLiteral("Count Group Color"));
    double l = 0.0;
    double a = 0.0;
    double b = 0.0;
    pictura::rgbToLab(QColor(0, 0, 0), &l, &a, &b);
    const bool pickerOk = picker.selectedColor() == initial
        && pictura::snapToWebColor(QColor(0x12, 0x12, 0x12)) == QColor(0, 0, 0)
        && pictura::isWebColor(QColor(0x33, 0x66, 0x99))
        && !pictura::isWebColor(QColor(0x12, 0x40, 0x80)) && std::abs(l) < 1e-6
        && std::abs(a) < 1e-6 && std::abs(b) < 1e-6;

    ST_BEGIN("healing_tools");
    ST_PASS("healing spot=%d/%d/%d heal=%d/%d count=%d/%d/%d/%d/%d/%d/%d picker=%d", spotActive ? 1 : 0,
            committed ? 1 : 0, healed ? 1 : 0, refused ? 1 : 0, transplanted ? 1 : 0,
            countActive ? 1 : 0, bar ? 1 : 0, counted ? 1 : 0, grouped ? 1 : 0, hidden ? 1 : 0,
            styled ? 1 : 0, deletedGroup ? 1 : 0, cleared ? 1 : 0, pickerOk ? 1 : 0);
    frame.closeDocument(doc, false);
    QFile::remove(seedPath);
    if (!spotActive || !committed || !healed || !refused || !transplanted || !countActive || !bar
        || !counted || !grouped || !hidden || !styled || !deletedGroup || !cleared || !pickerOk) {
        return pictura::selfTest().fail(536, "healing tools");
    }
    return 0;
}

int pictura::runPatchChecks(pictura::PicturaMainWindow& frame)
{
    // Seed: a white field with a black 8×8 blemish at (6, 6).
    QImage seed(60, 20, QImage::Format_RGB32);
    seed.fill(Qt::white);
    QPainter(&seed).fillRect(6, 6, 8, 8, Qt::black);
    const QString seedPath = QDir::temp().filePath(QStringLiteral("pictura_patch_seed.png"));

    const bool created = frame.newDocument(QStringLiteral("PatchCtl"), 60, 20,
                                           QStringLiteral("rgb"), 8, QStringLiteral("white"));
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    if (!created || !view || !canvas || !tools || !seed.save(seedPath, "PNG")
        || !view->open_image(seedPath)) {
        return pictura::selfTest().fail(537, "patch fixture");
    }
    const int doc = frame.activeDocumentIndex();
    const auto drag = [canvas](const QList<QPointF>& path) {
        canvas->mousePressed(path.first(), Qt::LeftButton, 0);
        for (const QPointF& p : path.mid(1)) {
            canvas->mouseMoved(p);
        }
        canvas->mouseReleased(path.last());
    };

    frame.setActiveTool(pictura::ToolId::Patch);
    const bool active = tools->activeTool() == pictura::ToolId::Patch;
    // The cursor is an arrow whose tip (2, 2) is the point, like the Lasso's.
    const pictura::ToolInfo& info = pictura::toolInfo(pictura::ToolId::Patch);
    const QCursor patchCursor = pictura::cursor(
        pictura::toolCursorId(pictura::ToolId::Patch, Qt::NoModifier), info.hotspotX,
        info.hotspotY);
    const bool arrow = info.hotspotX == 2 && info.hotspotY == 2
        && !patchCursor.pixmap().isNull();
    // Content-Aware disables the sampling controls; Normal re-enables them.
    auto* mode = frame.findChild<QComboBox*>(QStringLiteral("optionsPatchMode"));
    auto* source = frame.findChild<QToolButton*>(QStringLiteral("optionsPatchSource"));
    bool bar = mode && source && source->isChecked() && source->isEnabled();
    if (bar) {
        mode->setCurrentIndex(1);
        bar = !source->isEnabled() && tools->optionsFor<PatchGroup>(ToolId::Patch)->get().contentAware;
        mode->setCurrentIndex(0);
        bar = bar && source->isEnabled() && !tools->optionsFor<PatchGroup>(ToolId::Patch)->get().contentAware;
    }

    // Step one: a drag outside any selection outlines the blemish.
    drag({QPointF(4, 4), QPointF(16, 4), QPointF(16, 16), QPointF(4, 16), QPointF(4, 5)});
    const bool outlined = view->has_selection() && view->selection_coverage(10, 10) > 0
        && view->selection_coverage(30, 10) == 0;
    // Step two: dragging the outline onto clean pixels repairs the selection
    // from there, in one "Patch Tool" state, and leaves the source alone.
    const int base = view->history_index();
    drag({QPointF(10, 10), QPointF(25, 10), QPointF(40, 10)});
    const bool committed = view->history_index() == base + 1
        && view->history_label(base + 1) == QStringLiteral("Patch Tool");
    const quint32 patchedArgb = view->sample_argb(10, 10);
    const bool patched = qRed(patchedArgb) > 200 && qGreen(patchedArgb) > 200
        && qBlue(patchedArgb) > 200 && view->sample_argb(40, 10) == 0xffffffffu;
    // A click inside the selection (no drag) records nothing.
    drag({QPointF(10, 10)});
    const bool clickNoop = view->history_index() == base + 1;
    frame.setActiveTool(pictura::ToolId::Move);

    ST_BEGIN("patch_tool");
    ST_PASS("patch active=%d arrow=%d bar=%d outline=%d commit=%d patched=%d click=%d",
            active ? 1 : 0, arrow ? 1 : 0, bar ? 1 : 0, outlined ? 1 : 0, committed ? 1 : 0, patched ? 1 : 0,
            clickNoop ? 1 : 0);
    frame.closeDocument(doc, false);
    QFile::remove(seedPath);
    if (!active || !arrow || !bar || !outlined || !committed || !patched || !clickNoop) {
        return pictura::selfTest().fail(537, "patch tool");
    }
    return 0;
}

int pictura::runContentAwareMoveChecks(pictura::PicturaMainWindow& frame)
{
    // Seed: a white field with a black 8×8 subject at (6, 6).
    QImage seed(60, 20, QImage::Format_RGB32);
    seed.fill(Qt::white);
    QPainter(&seed).fillRect(6, 6, 8, 8, Qt::black);
    const QString seedPath = QDir::temp().filePath(QStringLiteral("pictura_cam_seed.png"));

    const bool created = frame.newDocument(QStringLiteral("MoveCtl"), 60, 20,
                                           QStringLiteral("rgb"), 8, QStringLiteral("white"));
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    if (!created || !view || !canvas || !tools || !seed.save(seedPath, "PNG")
        || !view->open_image(seedPath)) {
        return pictura::selfTest().fail(538, "content-aware move fixture");
    }
    const int doc = frame.activeDocumentIndex();
    const auto drag = [canvas](const QList<QPointF>& path) {
        canvas->mousePressed(path.first(), Qt::LeftButton, 0);
        for (const QPointF& p : path.mid(1)) {
            canvas->mouseMoved(p);
        }
        canvas->mouseReleased(path.last());
    };
    const auto white = [view](int x, int y) {
        const quint32 argb = view->sample_argb(x, y);
        return qRed(argb) > 200 && qGreen(argb) > 200 && qBlue(argb) > 200;
    };

    frame.setActiveTool(pictura::ToolId::ContentAwareMove);
    const bool active = tools->activeTool() == pictura::ToolId::ContentAwareMove;
    const pictura::ToolInfo& info = pictura::toolInfo(pictura::ToolId::ContentAwareMove);
    const bool arrow = info.hotspotX == 2 && info.hotspotY == 2;
    // The bar's Mode and Adaptation drive the controller; Medium is the default.
    auto* mode = frame.findChild<QComboBox*>(QStringLiteral("optionsContentAwareMoveMode"));
    auto* adaptation =
        frame.findChild<QComboBox*>(QStringLiteral("optionsContentAwareAdaptation"));
    bool bar = mode && adaptation && adaptation->count() == 5
        && adaptation->currentIndex() == 2 && tools->optionsFor<ContentAwareMoveGroup>(ToolId::ContentAwareMove)->get().adaptation == 2;
    if (bar) {
        mode->setCurrentIndex(1);
        bar = tools->optionsFor<ContentAwareMoveGroup>(ToolId::ContentAwareMove)->get().extend;
        mode->setCurrentIndex(0);
        bar = bar && !tools->optionsFor<ContentAwareMoveGroup>(ToolId::ContentAwareMove)->get().extend;
    }

    // Outline the subject, then drag it 30 px right: the subject lands there,
    // its old place is rebuilt from the white field, the selection follows, and
    // one "Content-Aware Move" state is recorded.
    drag({QPointF(4, 4), QPointF(16, 4), QPointF(16, 16), QPointF(4, 16), QPointF(4, 5)});
    const bool outlined = view->has_selection() && view->selection_coverage(10, 10) > 0;
    const int base = view->history_index();
    drag({QPointF(10, 10), QPointF(25, 10), QPointF(40, 10)});
    const bool committed = view->history_index() == base + 1
        && view->history_label(base + 1) == QStringLiteral("Content-Aware Move");
    const bool moved = white(10, 10) && qRed(view->sample_argb(40, 10)) < 60;
    const bool followed =
        view->selection_coverage(40, 10) > 0 && view->selection_coverage(10, 10) == 0;
    frame.setActiveTool(pictura::ToolId::Move);

    ST_BEGIN("content_aware_move");
    ST_PASS("cam active=%d arrow=%d bar=%d outline=%d commit=%d moved=%d follow=%d",
            active ? 1 : 0, arrow ? 1 : 0, bar ? 1 : 0, outlined ? 1 : 0, committed ? 1 : 0,
            moved ? 1 : 0, followed ? 1 : 0);
    frame.closeDocument(doc, false);
    QFile::remove(seedPath);
    if (!active || !arrow || !bar || !outlined || !committed || !moved || !followed) {
        return pictura::selfTest().fail(538, "content-aware move");
    }
    return 0;
}
