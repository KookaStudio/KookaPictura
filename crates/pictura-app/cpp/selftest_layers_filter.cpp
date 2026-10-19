#include "selftest_layers_filter.h"
#include "selftest_report.h"
#include "api/layers_api.h"

#include "frame.h"
#include "panels/layers_panel.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QSet>
#include <QtCore/QString>
#include <QtCore/QStringList>

namespace {

QSet<QString> pathSet(const QStringList& paths)
{
    return QSet<QString>(paths.begin(), paths.end());
}

QString pathForKind(pictura::PictureView* view, const QString& kind)
{
    for (int i = 0; i < pictura::layers_api::rowCount(*view); ++i) {
        if (pictura::layers_api::rowAt(*view, i).kind == kind) {
            return pictura::layers_api::rowAt(*view, i).path;
        }
    }
    return QString();
}

QStringList allPaths(pictura::PictureView* view)
{
    QStringList paths;
    for (int i = 0; i < pictura::layers_api::rowCount(*view); ++i) {
        paths.push_back(pictura::layers_api::rowAt(*view, i).path);
    }
    return paths;
}

} // namespace

int pictura::runLayersFilterChecks(pictura::PicturaMainWindow& frame)
{
    // lfs_name (201): Name is a case-insensitive substring; a match is shown and
    // every non-matching row (including the background) is hidden.
    {
        const bool created = frame.newDocument(QStringLiteral("FilterName"), 16, 16,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        pictura::PictureView* view = frame.activeView();
        auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
        if (!created || !view || !panel) {
            return pictura::selfTest().fail(201, "name fixture");
        }
        const int doc = frame.activeDocumentIndex();
        panel->setView(view);
        const QString sky = view->add_layer_in(QString());
        const QString ground = view->add_layer_in(QString());
        view->set_layer_name_path(sky, QStringLiteral("sky"));
        view->set_layer_name_path(ground, QStringLiteral("ground"));
        panel->refresh();
        const int full = panel->visiblePathsForTest().size();
        panel->setFilterNameForTest(QStringLiteral("sky"), true);
        const bool named = pathSet(panel->visiblePathsForTest()) == QSet<QString>{sky};
        panel->setFilterNameForTest(QStringLiteral("SKY"), true);
        const bool cased = pathSet(panel->visiblePathsForTest()) == QSet<QString>{sky};
        ST_BEGIN("lfs_name");
        ST_PASS("lfs_name full=%d named=%d cased=%d", full, named ? 1 : 0, cased ? 1 : 0);
        if (full < 3 || !named || !cased) {
            return pictura::selfTest().fail(201, "name filter");
        }
        frame.closeDocument(doc, false);
    }

    // lfs_kind (202): pixel and adjustment selected shows exactly those two;
    // group and background are hidden.
    {
        const bool created = frame.newDocument(QStringLiteral("FilterKind"), 16, 16,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        pictura::PictureView* view = frame.activeView();
        auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
        if (!created || !view || !panel) {
            return pictura::selfTest().fail(202, "kind fixture");
        }
        const int doc = frame.activeDocumentIndex();
        panel->setView(view);
        const QString pixel = view->add_layer_in(QString());
        view->set_layer_name_path(pixel, QStringLiteral("raster"));
        view->add_adjustment(QStringLiteral("invert"));
        view->add_group_in(QString());
        view->background_from_layer(QStringLiteral("0"));
        panel->refresh();
        const QString adjustment = pathForKind(view, QStringLiteral("adjustment"));
        panel->setFilterKindForTest({QStringLiteral("pixel"), QStringLiteral("adjustment")}, true);
        const QSet<QString> visible = pathSet(panel->visiblePathsForTest());
        const bool kindOk =
            visible == QSet<QString>{pixel, adjustment} && !adjustment.isEmpty();
        ST_BEGIN("lfs_kind");
        ST_PASS("lfs_kind visible=%d", visible.size());
        if (!kindOk) {
            return pictura::selfTest().fail(202, "kind filter");
        }
        frame.closeDocument(doc, false);
    }

    // lfs_mode (203): Multiply narrows to Multiply layers, and a second Name
    // criterion ANDs with it.
    {
        const bool created = frame.newDocument(QStringLiteral("FilterMode"), 16, 16,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        pictura::PictureView* view = frame.activeView();
        auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
        if (!created || !view || !panel) {
            return pictura::selfTest().fail(203, "mode fixture");
        }
        const int doc = frame.activeDocumentIndex();
        panel->setView(view);
        const QString shadowMul = view->add_layer_in(QString());
        const QString shadowNorm = view->add_layer_in(QString());
        const QString otherMul = view->add_layer_in(QString());
        view->set_layer_name_path(shadowMul, QStringLiteral("shadow"));
        view->set_layer_name_path(shadowNorm, QStringLiteral("shadow"));
        view->set_layer_name_path(otherMul, QStringLiteral("other"));
        view->set_layers_blend({shadowMul}, QStringLiteral("mul "));
        view->set_layers_blend({otherMul}, QStringLiteral("mul "));
        panel->refresh();
        panel->setFilterModeForTest(QStringLiteral("mul "), true);
        const QSet<QString> modeOnly = pathSet(panel->visiblePathsForTest());
        panel->setFilterNameForTest(QStringLiteral("shadow"), true);
        const QSet<QString> modeAndName = pathSet(panel->visiblePathsForTest());
        const bool modeOk = modeOnly == QSet<QString>{shadowMul, otherMul};
        const bool andOk = modeAndName == QSet<QString>{shadowMul};
        ST_BEGIN("lfs_mode");
        ST_PASS("lfs_mode mode=%d and=%d", modeOk ? 1 : 0, andOk ? 1 : 0);
        if (!modeOk || !andOk) {
            return pictura::selfTest().fail(203, "mode filter");
        }
        frame.closeDocument(doc, false);
    }

    // lfs_color (204): the Red label matches exactly the red-labelled row.
    {
        const bool created = frame.newDocument(QStringLiteral("FilterColor"), 16, 16,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        pictura::PictureView* view = frame.activeView();
        auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
        if (!created || !view || !panel) {
            return pictura::selfTest().fail(204, "color fixture");
        }
        const int doc = frame.activeDocumentIndex();
        panel->setView(view);
        const QString red = view->add_layer_in(QString());
        const QString plain = view->add_layer_in(QString());
        view->set_layer_name_path(red, QStringLiteral("red-one"));
        view->set_layer_name_path(plain, QStringLiteral("plain-one"));
        view->set_layers_color({red}, 1);
        panel->refresh();
        panel->setFilterColorForTest(1, true);
        const bool colorOk = pathSet(panel->visiblePathsForTest()) == QSet<QString>{red};
        ST_BEGIN("lfs_color");
        ST_PASS("lfs_color ok=%d", colorOk ? 1 : 0);
        if (!colorOk) {
            return pictura::selfTest().fail(204, "color filter");
        }
        frame.closeDocument(doc, false);
    }

    // lfs_none (205): a no-match filter empties the view without touching the
    // document; toggling it off restores every row.
    {
        const bool created = frame.newDocument(QStringLiteral("FilterNone"), 16, 16,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        pictura::PictureView* view = frame.activeView();
        auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
        if (!created || !view || !panel) {
            return pictura::selfTest().fail(205, "none fixture");
        }
        const int doc = frame.activeDocumentIndex();
        panel->setView(view);
        view->add_layer_in(QString());
        view->add_layer_in(QString());
        panel->refresh();
        const int rows = pictura::layers_api::rowCount(*view);
        panel->setFilterNameForTest(QStringLiteral("no-such-layer-xyz"), true);
        const bool empty = panel->visiblePathsForTest().isEmpty();
        const bool unchanged = pictura::layers_api::rowCount(*view) == rows;
        panel->setFilterNameForTest(QString(), false);
        const bool restored = panel->visiblePathsForTest().size() == rows;
        ST_BEGIN("lfs_none");
        ST_PASS("lfs_none empty=%d unchanged=%d restored=%d", empty ? 1 : 0,
                unchanged ? 1 : 0, restored ? 1 : 0);
        if (!empty || !unchanged || !restored) {
            return pictura::selfTest().fail(205, "none filter");
        }
        frame.closeDocument(doc, false);
    }

    // lfs_ancestor (206): a group with one matching child is promoted and
    // expanded; its non-matching sibling stays hidden.
    {
        const bool created = frame.newDocument(QStringLiteral("FilterAncestor"), 16, 16,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        pictura::PictureView* view = frame.activeView();
        auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
        if (!created || !view || !panel) {
            return pictura::selfTest().fail(206, "ancestor fixture");
        }
        const int doc = frame.activeDocumentIndex();
        panel->setView(view);
        const QString group = view->add_group_in(QString());
        const QString match = view->add_layer_in(group);
        const QString sibling = view->add_layer_in(group);
        view->set_layer_name_path(match, QStringLiteral("match"));
        view->set_layer_name_path(sibling, QStringLiteral("sibling"));
        panel->refresh();
        panel->setFilterNameForTest(QStringLiteral("match"), true);
        const QSet<QString> visible = pathSet(panel->visiblePathsForTest());
        const bool ancestorOk = visible == QSet<QString>{group, match};
        ST_BEGIN("lfs_ancestor");
        ST_PASS("lfs_ancestor visible=%d ok=%d", visible.size(), ancestorOk ? 1 : 0);
        if (!ancestorOk) {
            return pictura::selfTest().fail(206, "ancestor promotion");
        }
        frame.closeDocument(doc, false);
    }

    // lfs_toggle (207): enabling, changing, and disabling the filter adds no
    // history and does not dirty the document.
    {
        const bool created = frame.newDocument(QStringLiteral("FilterToggle"), 16, 16,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        pictura::PictureView* view = frame.activeView();
        auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
        if (!created || !view || !panel) {
            return pictura::selfTest().fail(207, "toggle fixture");
        }
        const int doc = frame.activeDocumentIndex();
        panel->setView(view);
        view->add_layer_in(QString());
        panel->refresh();
        const int history = view->history_count();
        const bool dirty = view->is_dirty();
        panel->setFilterNameForTest(QStringLiteral("layer"), true);
        panel->setFilterKindForTest({QStringLiteral("pixel")}, true);
        panel->setFilterAttributeForTest(QStringLiteral("visible"), true);
        panel->setFilterNameForTest(QString(), false);
        const bool historyOk = view->history_count() == history;
        const bool dirtyOk = view->is_dirty() == dirty;
        ST_BEGIN("lfs_toggle");
        ST_PASS("lfs_toggle history=%d dirty=%d", historyOk ? 1 : 0, dirtyOk ? 1 : 0);
        if (!historyOk || !dirtyOk) {
            return pictura::selfTest().fail(207, "filter history");
        }
        frame.closeDocument(doc, false);
    }

    // lfs_live (208): renaming a layer to match the active Name filter makes it
    // appear on the next refresh without re-entering the filter.
    {
        const bool created = frame.newDocument(QStringLiteral("FilterLive"), 16, 16,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        pictura::PictureView* view = frame.activeView();
        auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
        if (!created || !view || !panel) {
            return pictura::selfTest().fail(208, "live fixture");
        }
        const int doc = frame.activeDocumentIndex();
        panel->setView(view);
        const QString layer = view->add_layer_in(QString());
        view->set_layer_name_path(layer, QStringLiteral("ground"));
        panel->refresh();
        panel->setFilterNameForTest(QStringLiteral("sky"), true);
        const bool hidden = panel->visiblePathsForTest().isEmpty();
        view->set_layer_name_path(layer, QStringLiteral("sky"));
        panel->refresh();
        const bool shown = pathSet(panel->visiblePathsForTest()) == QSet<QString>{layer};
        ST_BEGIN("lfs_live");
        ST_PASS("lfs_live hidden=%d shown=%d", hidden ? 1 : 0, shown ? 1 : 0);
        if (!hidden || !shown) {
            return pictura::selfTest().fail(208, "live filter");
        }
        frame.closeDocument(doc, false);
    }

    // lfs_reset (209): switching documents returns the filter to Kind/off and
    // shows the new document's full tree.
    {
        const bool createdA = frame.newDocument(QStringLiteral("FilterResetA"), 16, 16,
                                                QStringLiteral("rgb"), 8,
                                                QStringLiteral("white"));
        pictura::PictureView* viewA = frame.activeView();
        auto* panel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
        if (!createdA || !viewA || !panel) {
            return pictura::selfTest().fail(209, "reset fixture A");
        }
        const int docA = frame.activeDocumentIndex();
        panel->setView(viewA);
        const QString target = viewA->add_layer_in(QString());
        viewA->set_layer_name_path(target, QStringLiteral("target"));
        panel->refresh();
        panel->setFilterNameForTest(QStringLiteral("target"), true);
        const bool filtered = pathSet(panel->visiblePathsForTest()) == QSet<QString>{target};

        const bool createdB = frame.newDocument(QStringLiteral("FilterResetB"), 16, 16,
                                                QStringLiteral("rgb"), 8,
                                                QStringLiteral("white"));
        pictura::PictureView* viewB = frame.activeView();
        if (!createdB || !viewB) {
            return pictura::selfTest().fail(209, "reset fixture B");
        }
        const int docB = frame.activeDocumentIndex();
        panel->setView(viewB);
        panel->refresh();
        const bool minimal = pathSet(panel->visiblePathsForTest()) == pathSet(allPaths(viewB));
        const bool dimensionOk = panel->filterDimensionForTest() == 1;
        ST_BEGIN("lfs_reset");
        ST_PASS("lfs_reset filtered=%d dimension=%d minimal=%d", filtered ? 1 : 0,
                dimensionOk ? 1 : 0, minimal ? 1 : 0);
        if (!filtered || !dimensionOk || !minimal) {
            return pictura::selfTest().fail(209, "filter reset");
        }
        frame.closeDocument(docB, false);
        frame.closeDocument(docA, false);
    }

    return 0;
}
