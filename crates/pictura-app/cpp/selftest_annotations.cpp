#include "selftest_annotations.h"
#include "selftest_report.h"

#include "frame.h"
#include "image_view.h"
#include "panels/info_panel.h"
#include "panels/notes_panel.h"
#include "panels/panel_column.h"
#include "tools.h"

#include "api/paint_bridge.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtGui/QImage>
#include <QtGui/QPainter>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPlainTextEdit>

#include <cmath>

int pictura::runAnnotationChecks(pictura::PicturaMainWindow& frame)
{
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(Qt::white);
    QPainter(&seed).fillRect(10, 10, 20, 20, Qt::black);
    const QString seedPath = QDir::temp().filePath(QStringLiteral("pictura_annotation_seed.png"));
    const bool created = frame.newDocument(QStringLiteral("AnnotationCtl"), 40, 40,
                                           QStringLiteral("rgb"), 8, QStringLiteral("white"));
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    auto* info = frame.findChild<pictura::InfoPanel*>();
    auto* notes = frame.findChild<pictura::NotesPanel*>();
    if (!created || !view || !canvas || !tools || !info || !notes || !seed.save(seedPath, "PNG")
        || !view->open_image(seedPath)) {
        return pictura::selfTest().fail(535, "annotation tools fixture");
    }
    const int doc = frame.activeDocumentIndex();
    const auto press = [canvas](QPointF at, Qt::KeyboardModifiers mods = Qt::NoModifier) {
        canvas->mousePressed(at, Qt::LeftButton, int(mods));
        canvas->mouseReleased(at);
    };
    const auto drag = [canvas](QPointF from, QPointF to) {
        canvas->mousePressed(from, Qt::LeftButton, int(Qt::NoModifier));
        canvas->mouseMoved(to);
        canvas->mouseReleased(to);
    };
    const auto added = [view](int base, const char* label) {
        return view->history_index() == base + 1
            && view->history_label(base + 1) == QString::fromLatin1(label);
    };
    const auto at = [view](int kind, int index) {
        const ::rust::Vec<std::int32_t> p = pictura::marker_at(*view, kind, index);
        return p.size() == 2 ? QPoint(p[0], p[1]) : QPoint(-1, -1);
    };

    // Color Sampler.
    frame.setActiveTool(pictura::ToolId::ColorSampler);
    const bool active = tools->activeTool() == pictura::ToolId::ColorSampler;
    int base = view->history_index();
    press(QPointF(15, 15));
    info->refresh();
    const bool placed = added(base, "Color Sampler") && pictura::marker_count(*view, 0) == 1
        && canvas->samplerOverlayCountForTest() == 1
        && info->samplerTextForTest() == QStringLiteral("#1  15, 15  R 0  G 0  B 0");
    press(QPointF(2, 2));
    press(QPointF(38, 2));
    press(QPointF(2, 38));
    base = view->history_index();
    press(QPointF(38, 38));
    const bool capped = pictura::marker_count(*view, 0) == 4 && view->history_index() == base;
    drag(QPointF(15, 15), QPointF(5, 6));
    const bool moved = added(base, "Move Color Sampler") && at(0, 0) == QPoint(5, 6);
    base = view->history_index();
    press(QPointF(2, 2), Qt::AltModifier);
    const bool altDeleted = added(base, "Delete Color Sampler") && pictura::marker_count(*view, 0) == 3;
    base = view->history_index();
    drag(QPointF(38, 2), QPointF(45, 2));
    const bool draggedOff = added(base, "Delete Color Sampler") && pictura::marker_count(*view, 0) == 2;
    base = view->history_index();
    tools->clearAnnotations();
    const bool cleared = added(base, "Clear Color Samplers") && pictura::marker_count(*view, 0) == 0
        && canvas->samplerOverlayCountForTest() == 0;
    view->undo();
    frame.setActiveTool(pictura::ToolId::Move);
    const bool restored = pictura::marker_count(*view, 0) == 2
        && canvas->samplerOverlayCountForTest() == 2;
    const bool sampler = active && placed && capped && moved && altDeleted && draggedOff && cleared
        && restored;

    // Ruler.
    frame.setActiveTool(pictura::ToolId::Ruler);
    base = view->history_index();
    drag(QPointF(0, 0), QPointF(3, 4));
    const ::rust::Vec<double> m = pictura::ruler_measurement(*view);
    QLabel* readout = nullptr;
    for (QLabel* label : frame.findChildren<QLabel*>(QStringLiteral("optionsRulerReadout"))) {
        readout = label;
    }
    const bool measured = m.size() == 6 && m[2] == 3.0 && m[3] == 4.0
        && std::abs(m[4] + 53.1301) < 1e-3 && m[5] == 5.0 && canvas->hasRulerLineForTest()
        && view->history_index() == base && readout
        && readout->text().contains(QStringLiteral("A: -53.1°"))
        && readout->text().contains(QStringLiteral("D1: 5.0"));
    drag(QPointF(3, 4), QPointF(6, 8));
    const ::rust::Vec<double> e = pictura::ruler_measurement(*view);
    const bool edited = e.size() == 6 && e[0] == 0.0 && e[5] == 10.0;
    frame.setActiveTool(pictura::ToolId::Move);
    const bool hidden = !canvas->hasRulerLineForTest();
    frame.setActiveTool(pictura::ToolId::Ruler);
    const bool shown = canvas->hasRulerLineForTest();
    press(QPointF(30, 20));
    const bool clickDropped = pictura::ruler_line(*view).empty() && !canvas->hasRulerLineForTest()
        && view->history_index() == base;
    const bool ruler = measured && edited && hidden && shown && clickDropped;

    // Note.
    frame.setActiveTool(pictura::ToolId::Note);
    base = view->history_index();
    press(QPointF(20, 20));
    const bool noteAdded = added(base, "New Note") && pictura::marker_count(*view, 1) == 1
        && canvas->noteOverlayCountForTest() == 1 && notes->currentNoteForTest() == 0
        && frame.panelColumn()->isPanelVisible(QStringLiteral("notesPanel"));
    base = view->history_index();
    notes->editorForTest()->setPlainText(QStringLiteral("check this edge"));
    notes->commitText();
    const bool noteEdited = added(base, "Edit Note")
        && QString(pictura::note_text(*view, 0)) == QStringLiteral("check this edge");
    press(QPointF(5, 5));
    const bool second = pictura::marker_count(*view, 1) == 2 && notes->currentNoteForTest() == 1;
    press(QPointF(21, 19));
    const bool reopened = notes->currentNoteForTest() == 0
        && notes->editorForTest()->toPlainText() == QStringLiteral("check this edge");
    base = view->history_index();
    press(QPointF(5, 5), Qt::AltModifier);
    const bool noteDeleted = added(base, "Delete Note") && pictura::marker_count(*view, 1) == 1;
    tools->clearAnnotations();
    const bool notesCleared = pictura::marker_count(*view, 1) == 0 && notes->currentNoteForTest() == -1;
    const bool note = noteAdded && noteEdited && second && reopened && noteDeleted && notesCleared;
    frame.setActiveTool(pictura::ToolId::Move);

    ST_BEGIN("annotation_tools");
    ST_PASS("annotation_tools sampler=%d/%d/%d/%d/%d/%d/%d/%d ruler=%d/%d/%d/%d/%d "
            "note=%d/%d/%d/%d/%d/%d",
            active ? 1 : 0, placed ? 1 : 0, capped ? 1 : 0, moved ? 1 : 0, altDeleted ? 1 : 0,
            draggedOff ? 1 : 0, cleared ? 1 : 0, restored ? 1 : 0, measured ? 1 : 0,
            edited ? 1 : 0, hidden ? 1 : 0, shown ? 1 : 0, clickDropped ? 1 : 0,
            noteAdded ? 1 : 0, noteEdited ? 1 : 0, second ? 1 : 0, reopened ? 1 : 0,
            noteDeleted ? 1 : 0, notesCleared ? 1 : 0);
    frame.closeDocument(doc, false);
    QFile::remove(seedPath);
    if (!sampler || !ruler || !note) {
        return pictura::selfTest().fail(535, "annotation tools");
    }
    return 0;
}
