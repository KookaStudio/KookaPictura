#include "selftest_crop_group.h"
#include "selftest_report.h"

#include "frame.h"
#include "icons.h"
#include "image_view.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/crop_group.cxxqt.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QRect>
#include <QtGui/QImage>
#include <QtGui/QKeyEvent>
#include <QtGui/QPainter>
#include <QtWidgets/QApplication>

int pictura::runCropGroupChecks(pictura::PicturaMainWindow& frame)
{
    QImage seed(40, 40, QImage::Format_RGB32);
    seed.fill(Qt::white);
    QPainter(&seed).fillRect(10, 10, 20, 20, Qt::black);
    const QString seedPath = QDir::temp().filePath(QStringLiteral("pictura_crop_group_seed.png"));
    const bool created = frame.newDocument(QStringLiteral("CropGroupCtl"), 40, 40,
                                           QStringLiteral("rgb"), 8, QStringLiteral("white"));
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    if (!created || !view || !canvas || !tools || !seed.save(seedPath, "PNG")
        || !view->open_image(seedPath)) {
        return pictura::selfTest().fail(532, "crop group fixture");
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
    const auto click = [canvas](QPointF p) {
        canvas->mousePressed(p, Qt::LeftButton, int(Qt::NoModifier));
        canvas->mouseReleased(p);
    };

    // Perspective Crop: four corner clicks stage the quad.
    frame.setActiveTool(pictura::ToolId::PerspectiveCrop);
    click(QPointF(8, 8));
    click(QPointF(32, 8));
    click(QPointF(32, 32));
    click(QPointF(8, 32));
    const bool staged = canvas->hasPerspectiveCropQuadForTest();
    // A crosshair inside the quad, the move cursor over a corner handle.
    canvas->mouseMoved(QPointF(20, 20));
    const bool crosshair = canvas->cursor().shape() == Qt::BitmapCursor
        && canvas->cursor().pixmap().toImage()
            == pictura::cursor(QStringLiteral("tool.marquee"), 12, 12).pixmap().toImage();
    canvas->mouseMoved(QPointF(8, 8));
    const bool cornerCursor = canvas->cursor().shape() == Qt::SizeAllCursor;
    key(Qt::Key_Escape);
    const bool discarded = !canvas->hasPerspectiveCropQuadForTest();

    // Stage a quad, then drag the top-left corner onto the top-right one: a
    // degenerate quad is refused and stays.
    click(QPointF(10, 10));
    click(QPointF(30, 10));
    click(QPointF(30, 30));
    click(QPointF(10, 30));
    drag(QPointF(10, 10), QPointF(30, 10));
    const int badBase = view->history_count();
    key(Qt::Key_Return);
    const bool degenerateKept = view->history_count() == badBase
        && view->document_width() == 40 && canvas->hasPerspectiveCropQuadForTest();
    key(Qt::Key_Escape);

    click(QPointF(10, 10));
    click(QPointF(30, 10));
    click(QPointF(30, 30));
    click(QPointF(10, 30));
    const int cropBase = view->history_count();
    key(Qt::Key_Return);
    const bool cropped = view->history_count() == cropBase + 1
        && view->history_label(cropBase) == QStringLiteral("Perspective Crop")
        && view->document_width() == 20 && view->document_height() == 20
        && view->sample_argb(10, 10) == 0xff000000u && !canvas->hasPerspectiveCropQuadForTest();

    // Slice.
    frame.setActiveTool(pictura::ToolId::Slice);
    const bool unsliced = pictura::slice_count(*view) == 1 && canvas->sliceOverlayCountForTest() == 1;
    const int sliceBase = view->history_count();
    drag(QPointF(2, 2), QPointF(10, 8));
    bool userSlice = false;
    for (int i = 0; i < pictura::slice_count(*view); ++i) {
        const ::rust::Vec<std::int32_t> f = pictura::slice_at(*view, i);
        userSlice |= f.size() == 6 && f[0] == 2 && f[1] == 2 && f[2] == 8 && f[3] == 6 && f[5] == 0;
    }
    const int sliced = pictura::slice_count(*view);
    const bool added = userSlice && sliced > 1 && canvas->sliceOverlayCountForTest() == sliced
        && view->history_count() == sliceBase + 1
        && view->history_label(sliceBase) == QStringLiteral("Slice");
    drag(QPointF(15, 15), QPointF(15, 15));
    const bool clickNoSlice = pictura::slice_count(*view) == sliced
        && view->history_count() == sliceBase + 1;
    view->undo();
    const bool undone = pictura::slice_count(*view) == 1 && canvas->sliceOverlayCountForTest() == 1;
    frame.setActiveTool(pictura::ToolId::Move);
    const bool hidden = canvas->sliceOverlayCountForTest() == 0;

    // Slice Select: select, move, resize from an edge, deselect, delete.
    const auto userSlice0 = [view]() {
        for (int i = 0; i < pictura::slice_count(*view); ++i) {
            const ::rust::Vec<std::int32_t> f = pictura::slice_at(*view, i);
            if (f.size() == 6 && f[5] == 0) {
                return QRect(f[0], f[1], f[2], f[3]);
            }
        }
        return QRect();
    };
    const auto lastLabel = [view](int base, const char* label) {
        return view->history_count() == base + 1
            && view->history_label(base) == QString::fromLatin1(label);
    };
    frame.setActiveTool(pictura::ToolId::Slice);
    drag(QPointF(2, 2), QPointF(18, 18));
    frame.setActiveTool(pictura::ToolId::SliceSelect);
    // Away from any handle the pointer is the precise crosshair.
    canvas->mouseMoved(QPointF(1, 1));
    const bool selectCrosshair = canvas->cursor().shape() == Qt::BitmapCursor
        && canvas->cursor().pixmap().toImage()
            == pictura::cursor(QStringLiteral("tool.marquee"), 12, 12).pixmap().toImage();
    drag(QPointF(10, 10), QPointF(10, 10));
    const bool selected = selectCrosshair && canvas->sliceOverlayHasSelectionForTest();
    int base = view->history_count();
    drag(QPointF(10, 10), QPointF(11, 11));
    const bool moved = lastLabel(base, "Edit Slice") && userSlice0() == QRect(3, 3, 16, 16);
    base = view->history_count();
    drag(QPointF(19, 10), QPointF(15, 10));
    const bool resized = lastLabel(base, "Edit Slice") && userSlice0() == QRect(3, 3, 12, 16);
    key(Qt::Key_Escape);
    const bool deselected = !canvas->sliceOverlayHasSelectionForTest();
    drag(QPointF(8, 8), QPointF(8, 8));
    base = view->history_count();
    key(Qt::Key_Delete);
    const bool deleted = lastLabel(base, "Delete Slice") && pictura::slice_count(*view) == 1
        && !canvas->sliceOverlayHasSelectionForTest();
    const bool sliceSelect = selected && moved && resized && deselected && deleted;
    frame.setActiveTool(pictura::ToolId::Move);

    ST_BEGIN("crop_group");
    ST_PASS("crop_group staged=%d cursor=%d discarded=%d degenerate=%d cropped=%d unsliced=%d added=%d "
            "click=%d undone=%d hidden=%d select=%d/%d/%d/%d/%d",
            staged ? 1 : 0, crosshair && cornerCursor ? 1 : 0, discarded ? 1 : 0, degenerateKept ? 1 : 0, cropped ? 1 : 0,
            unsliced ? 1 : 0, added ? 1 : 0, clickNoSlice ? 1 : 0, undone ? 1 : 0,
            hidden ? 1 : 0, selected ? 1 : 0, moved ? 1 : 0, resized ? 1 : 0,
            deselected ? 1 : 0, deleted ? 1 : 0);
    frame.closeDocument(doc, false);
    QFile::remove(seedPath);
    if (!staged || !crosshair || !cornerCursor || !discarded || !degenerateKept || !cropped || !unsliced || !added
        || !clickNoSlice || !undone || !hidden || !sliceSelect) {
        return pictura::selfTest().fail(532, "crop group");
    }
    return 0;
}
