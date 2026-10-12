#include "selftest_click_deselect.h"
#include "selftest_report.h"

#include "frame.h"
#include "image_view.h"
#include "tools.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

int pictura::runClickDeselectChecks(pictura::PicturaMainWindow& frame)
{
    const bool created = frame.newDocument(QStringLiteral("ClickDeselectCtl"), 20, 20,
                                           QStringLiteral("rgb"), 8, QStringLiteral("white"));
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    auto* tools = frame.findChild<pictura::ToolController*>();
    if (!created || !view || !canvas || !tools) {
        return pictura::selfTest().fail(531, "click deselect fixture");
    }
    const int doc = frame.activeDocumentIndex();
    tools->optionsFor<SelectionOptions>(ToolId::Marquee)->setCombineMode(pictura::SelectionMode::New);
    tools->optionsFor<SelectionOptions>(ToolId::Marquee)->setFeather(0.0);
    const auto click = [canvas](qreal x, qreal y, Qt::KeyboardModifiers mods) {
        canvas->mousePressed(QPointF(x, y), Qt::LeftButton, int(mods));
        canvas->mouseReleased(QPointF(x, y));
    };
    // Select a square, click outside it with `tool`, and report whether exactly
    // one "Deselect" state cleared it and Reselect brings it back.
    const auto deselects = [&](pictura::ToolId tool, int clicks) {
        frame.setActiveTool(tool);
        view->select_rect(2, 2, 5, 5, QStringLiteral("new"), 0.0);
        const int base = view->history_count();
        for (int i = 0; i < clicks; ++i) {
            click(15, 15, Qt::NoModifier);
        }
        const bool cleared = !view->has_selection() && view->history_count() == base + 1
            && view->history_label(base) == QStringLiteral("Deselect");
        return cleared && view->reselect() && view->selection_coverage(3, 3) > 0;
    };
    const bool marquee = deselects(pictura::ToolId::Marquee, 1);
    const bool lasso = deselects(pictura::ToolId::Lasso, 1);
    const bool polygon = deselects(pictura::ToolId::PolygonalLasso, 2);

    frame.setActiveTool(pictura::ToolId::Marquee);
    view->select_rect(2, 2, 5, 5, QStringLiteral("new"), 0.0);
    const int addBase = view->history_count();
    click(15, 15, Qt::ShiftModifier);
    const bool addKeeps = view->has_selection() && view->selection_coverage(3, 3) > 0
        && view->history_count() == addBase;

    ST_BEGIN("click_deselect");
    ST_PASS("click_deselect marquee=%d lasso=%d polygon=%d addKeeps=%d", marquee ? 1 : 0,
            lasso ? 1 : 0, polygon ? 1 : 0, addKeeps ? 1 : 0);
    frame.setActiveTool(pictura::ToolId::Move);
    frame.closeDocument(doc, false);
    if (!marquee || !lasso || !polygon || !addKeeps) {
        return pictura::selfTest().fail(531, "click deselect");
    }
    return 0;
}
