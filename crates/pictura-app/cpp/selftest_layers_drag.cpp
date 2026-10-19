#include "selftest_layers_drag.h"
#include "selftest_report.h"
#include "api/layers_api.h"

#include "commands.h"
#include "frame.h"
#include "panels/layers_panel.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtGui/QAction>
#include <QtGui/QColor>

namespace {

int groupCount(pictura::PictureView* view)
{
    int groups = 0;
    for (int i = 0; i < pictura::layers_api::rowCount(*view); ++i) {
        if (pictura::layers_api::rowAt(*view, i).kind == QStringLiteral("group")) {
            ++groups;
        }
    }
    return groups;
}

int rowForPath(pictura::PictureView* view, const QString& path)
{
    for (int i = 0; i < pictura::layers_api::rowCount(*view); ++i) {
        if (pictura::layers_api::rowAt(*view, i).path == path) {
            return i;
        }
    }
    return -1;
}

} // namespace

int pictura::runLayersDragChecks(pictura::PicturaMainWindow& frame)
{
        auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
        const bool created = frame.newDocument(QStringLiteral("Batch1Interactions"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        pictura::PictureView* view = frame.activeView();
        if (!created || !view || !panel) {
            return pictura::selfTest().fail(299, "batch1 fixture");
        }
        const int doc = frame.activeDocumentIndex();
        panel->setView(view);
        const QString top = view->add_layer_in(QString());
        const QString bottom = view->add_layer_in(QString());
        view->set_layer_name_path(top, QStringLiteral("Top"));
        view->set_layer_name_path(bottom, QStringLiteral("Bottom"));
        panel->refresh();

        // lpr_drag_flags (299): the model advertises drag and drop for a row so
        // the view starts a drag instead of extending the selection. The
        // observable is the flags (the root cause); a real QDrag::exec is not
        // synthesized because it would block outside a live platform drag loop.
        const bool flagsTop = panel->layerDragFlagsForTest(top);
        const bool flagsBottom = panel->layerDragFlagsForTest(bottom);
        const bool viewDrag = panel->treeDragEnabledForTest();
        ST_BEGIN("lpr_drag_flags");
        ST_PASS("lpr_drag_flags top=%d bottom=%d view=%d", flagsTop ? 1 : 0,
                flagsBottom ? 1 : 0, viewDrag ? 1 : 0);
        if (!flagsTop || !flagsBottom || !viewDrag) {
            return pictura::selfTest().fail(299, "layer drag flags");
        }

        // lpr_label_tint (300): a non-None label fills the visibility column;
        // None leaves the normal row background. An unselected labeled row
        // sampled at the column's left edge must read redder than the
        // unlabeled row.
        const bool colored = view->set_layer_color(bottom.toInt(), 1);
        panel->selectPaths(QStringList{}, QString());
        panel->refresh();
        const QColor tinted = panel->rowGutterColorForTest(bottom);
        const QColor plain = panel->rowGutterColorForTest(top);
        const bool tintOk = colored && tinted.isValid() && plain.isValid()
            && tinted.red() > plain.red() && tinted.red() > tinted.green() + 20
            && tinted.red() > tinted.blue() + 20;
        ST_BEGIN("lpr_label_tint");
        ST_PASS("lpr_label_tint tinted=%02x%02x%02x plain=%02x%02x%02x", tinted.red(),
                tinted.green(), tinted.blue(), plain.red(), plain.green(), plain.blue());
        if (!tintOk) {
            return pictura::selfTest().fail(300, "color label gutter tint");
        }

        // lpr_rename_name_only (302): edit triggers are off; a double-click in
        // the name rect opens the editor, one in the eye gutter does not.
        const bool triggersOff = panel->editTriggersDisabledForTest();
        const bool nameOpens = panel->doubleClickAtForTest(top, true);
        const bool eyeOpens = panel->doubleClickAtForTest(top, false);
        ST_BEGIN("lpr_rename_name_only");
        ST_PASS("lpr_rename_name_only triggers=%d name=%d eye=%d", triggersOff ? 1 : 0,
                nameOpens ? 1 : 0, eyeOpens ? 1 : 0);
        if (!triggersOff || !nameOpens || eyeOpens) {
            return pictura::selfTest().fail(302, "rename name-only");
        }

        // lpr_ctrl_g (301): the accelerator's handler routes through the
        // selection-aware panel op, wrapping the whole selection in one group in
        // one undo state, exactly as the menu command does.
        frame.registry()->refresh();
        QAction* groupAction = frame.registry()->action(command_ids::LayerGroupLayers);
        panel->selectPaths(QStringList{top, bottom}, top);
        const int groupBase = view->history_count();
        if (groupAction) {
            groupAction->trigger();
        }
        const bool groupOk = groupAction && groupCount(view) == 1
            && view->history_count() == groupBase + 1;
        ST_BEGIN("lpr_ctrl_g");
        ST_PASS("lpr_ctrl_g action=%d groups=%d history=%d", groupAction ? 1 : 0,
                groupCount(view), view->history_count() - groupBase);
        if (!groupOk) {
            return pictura::selfTest().fail(301, "ctrl+g groups selection");
        }

        frame.closeDocument(doc, false);

        // lpr_drag_into_above (392): a layer above a group drops into it through
        // the panel tree pipeline. It lands as the group's child in one undo
        // step -- not refused, not mis-nested into a sibling.
        {
            const bool created = frame.newDocument(QStringLiteral("DragInto"), 8, 8,
                                                   QStringLiteral("rgb"), 8,
                                                   QStringLiteral("white"));
            PictureView* view = frame.activeView();
            auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
            if (!created || !view || !panel) {
                return pictura::selfTest().fail(392, "drag into group fixture");
            }
            const int intoDoc = frame.activeDocumentIndex();
            panel->setView(view);
            const QString a = view->add_layer_in(QString());
            const QString g = view->add_group_in(QString());
            view->set_layer_name_path(a, QStringLiteral("A"));
            view->set_layer_name_path(g, QStringLiteral("G"));
            panel->refresh();
            const int base = view->history_count();
            const bool valid = panel->canMoveForTest(a, g, 2);
            const bool dropped = panel->dropIntoForTest(a, g);
            const int childRow = rowForPath(view, QStringLiteral("1/0"));
            const int groupRow = rowForPath(view, QStringLiteral("1"));
            const bool nested = childRow >= 0
                && pictura::layers_api::rowAt(*view, childRow).name == QStringLiteral("A")
                && groupRow >= 0 && pictura::layers_api::rowAt(*view, groupRow).kind == QStringLiteral("group")
                && pictura::layers_api::rowAt(*view, groupRow).name == QStringLiteral("G")
                && pictura::layers_api::rowCount(*view) == 3;
            const bool oneStep = view->history_count() == base + 1;
            ST_BEGIN("lpr_drag_into_above");
            ST_PASS("lpr_drag_into_above valid=%d drop=%d nested=%d history=%d", valid ? 1 : 0,
                    dropped ? 1 : 0, nested ? 1 : 0, view->history_count() - base);
            frame.closeDocument(intoDoc, false);
            if (!valid || !dropped || !nested || !oneStep) {
                return pictura::selfTest().fail(392, "drag layer into group");
            }
        }

        // lpr_drag_out (393): a group child drops back out to the root above a
        // root sibling through the panel tree pipeline, in one undo step.
        {
            const bool created = frame.newDocument(QStringLiteral("DragOut"), 8, 8,
                                                   QStringLiteral("rgb"), 8,
                                                   QStringLiteral("white"));
            PictureView* view = frame.activeView();
            auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
            if (!created || !view || !panel) {
                return pictura::selfTest().fail(393, "drag out of group fixture");
            }
            const int outDoc = frame.activeDocumentIndex();
            panel->setView(view);
            const QString g = view->add_group_in(QString());
            const QString child = view->add_layer_in(g);
            const QString sibling = view->add_layer_in(QString());
            view->set_layer_name_path(child, QStringLiteral("C"));
            view->set_layer_name_path(sibling, QStringLiteral("S"));
            panel->refresh();
            const int base = view->history_count();
            const bool dropped = panel->dropAtForTest(child, sibling, true);
            const bool leftGroup = rowForPath(view, QStringLiteral("1/0")) < 0;
            const int movedRow = rowForPath(view, QStringLiteral("3"));
            const bool atRoot = movedRow >= 0
                && pictura::layers_api::rowAt(*view, movedRow).name == QStringLiteral("C")
                && pictura::layers_api::rowCount(*view) == 4;
            const bool oneStep = view->history_count() == base + 1;
            ST_BEGIN("lpr_drag_out");
            ST_PASS("lpr_drag_out drop=%d left=%d root=%d history=%d", dropped ? 1 : 0,
                    leftGroup ? 1 : 0, atRoot ? 1 : 0, view->history_count() - base);
            frame.closeDocument(outDoc, false);
            if (!dropped || !leftGroup || !atRoot || !oneStep) {
                return pictura::selfTest().fail(393, "drag layer out of group");
            }
        }

        return 0;
}
