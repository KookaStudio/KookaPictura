#include "selftest_crop_tool.h"
#include "selftest_report.h"

#include "frame.h"
#include "image_view.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtGui/QImage>
#include <QtGui/QKeyEvent>
#include <QtGui/QPainter>
#include <QtWidgets/QApplication>
#include <QtWidgets/QToolButton>

int pictura::runCropToolChecks(pictura::PicturaMainWindow& frame)
{
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(Qt::white);
    QPainter(&seed).fillRect(10, 10, 20, 20, Qt::black);
    const QString seedPath = QDir::temp().filePath(QStringLiteral("pictura_crop_tool_seed.png"));
    const bool created = frame.newDocument(QStringLiteral("CropToolCtl"), 40, 40,
                                           QStringLiteral("rgb"), 8, QStringLiteral("white"));
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    if (!created || !view || !canvas || !tools || !seed.save(seedPath, "PNG")
        || !view->open_image(seedPath)) {
        return pictura::selfTest().fail(534, "crop tool fixture");
    }
    const int doc = frame.activeDocumentIndex();
    const auto drag = [canvas](QPointF from, QPointF to) {
        canvas->mousePressed(from, Qt::LeftButton, int(Qt::NoModifier));
        canvas->mouseMoved(to);
        canvas->mouseReleased(to);
    };
    const auto key = [&frame](Qt::Key k) {
        QKeyEvent press(QEvent::KeyPress, k, Qt::NoModifier);
        QApplication::sendEvent(&frame, &press);
    };
    // The layer's rect as "left top right bottom".
    const QString layer = view->layer_row_path(0);

    tools->setCropRatio(0.0);
    tools->setCropDeletePixels(true);
    tools->setCropClassicMode(false);  // start with the full-canvas preview
    frame.setActiveTool(pictura::ToolId::Crop);
    const bool defaultBox = canvas->hasCropBoxForTest() && !frame.hasPendingCrop();

    // Adopt the Modern preview, then resize its bottom-right handle.
    drag(QPointF(20, 20), QPointF(20, 20));
    drag(QPointF(40, 40), QPointF(30, 30));
    const bool resized = tools->pendingCropRect() == QRect(0, 0, 30, 30);
    tools->setCropClassicMode(true);  // a classic drag moves the box
    drag(QPointF(15, 15), QPointF(25, 25));
    const bool moved = tools->pendingCropRect() == QRect(10, 10, 30, 30);
    tools->setCropClassicMode(false);  // full box on every reset again
    key(Qt::Key_Escape);
    const bool escReset = !frame.hasPendingCrop() && !canvas->hasCropBoxForTest();
    // Escape leaves the init mode with no box; restore the full-canvas box for
    // the remaining checks.
    drag(QPointF(0, 0), QPointF(40, 40));

    drag(QPointF(5, 5), QPointF(20, 20));
    auto* cancel = frame.findChild<QToolButton*>(QStringLiteral("optionsCropCancel"));
    if (cancel) {
        cancel->click();
    }
    const bool cancelReset = cancel && !frame.hasPendingCrop();
    drag(QPointF(0, 0), QPointF(40, 40));

    // Inside the default box a press moves it, so shape boxes by the handles.
    tools->setCropRatio(1.0);
    drag(QPointF(40, 40), QPointF(30, 20));
    const bool ratioLocked = tools->pendingCropRect() == QRect(0, 0, 20, 20);
    tools->setCropRatio(0.0);
    key(Qt::Key_Escape);
    drag(QPointF(0, 0), QPointF(40, 40));

    // Delete Cropped Pixels on: the layer is trimmed to the new canvas.
    const auto box10 = [&drag]() {
        // From the full box, pull the bottom-right then the top-left handle to
        // land a 20x20 box at (10, 10).
        drag(QPointF(40, 40), QPointF(30, 30));
        drag(QPointF(0, 0), QPointF(10, 10));
    };
    box10();
    const int base = view->history_count();
    key(Qt::Key_Return);
    const bool deleted = view->history_count() == base + 1
        && view->history_label(base) == QStringLiteral("Crop") && view->document_width() == 20
        && view->sample_argb(10, 10) == 0xff000000u
        && view->layer_rect(layer) == QStringLiteral("0 0 20 20") && !frame.hasPendingCrop();

    // Off: the pixels outside the canvas stay with the layer.
    view->undo();
    tools->setCropDeletePixels(false);
    // Undo re-fits the tool to the restored canvas (Modern preview), so
    // re-establish the full-canvas box the handle drags start from.
    drag(QPointF(0, 0), QPointF(40, 40));
    box10();
    key(Qt::Key_Return);
    const bool kept = view->document_width() == 20
        && view->layer_rect(layer) == QStringLiteral("-10 -10 30 30");
    tools->setCropDeletePixels(true);

    // A double-click inside the box commits.
    view->undo();
    drag(QPointF(0, 0), QPointF(40, 40));
    box10();
    // After Undo the new state replaces the redo one, so compare positions.
    const int dblBase = view->history_index();
    drag(QPointF(20, 20), QPointF(20, 20));
    drag(QPointF(20, 20), QPointF(20, 20));
    const bool doubleClick = view->history_index() == dblBase + 1
        && view->history_label(view->history_index()) == QStringLiteral("Crop")
        && view->document_width() == 20;

    ST_BEGIN("crop_tool");
    ST_PASS("crop_tool default=%d resized=%d moved=%d esc=%d cancel=%d ratio=%d deleted=%d "
            "kept=%d dblclick=%d",
            defaultBox ? 1 : 0, resized ? 1 : 0, moved ? 1 : 0, escReset ? 1 : 0,
            cancelReset ? 1 : 0, ratioLocked ? 1 : 0, deleted ? 1 : 0, kept ? 1 : 0,
            doubleClick ? 1 : 0);
    tools->setCropClassicMode(true);
    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
    QFile::remove(seedPath);
    if (!defaultBox || !resized || !moved || !escReset || !cancelReset || !ratioLocked
        || !deleted || !kept || !doubleClick) {
        return pictura::selfTest().fail(534, "crop tool");
    }
    return 0;
}
