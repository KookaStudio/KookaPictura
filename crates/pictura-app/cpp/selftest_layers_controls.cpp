#include "selftest_layers_controls.h"
#include "selftest_active_layer.h"
#include "selftest_canvas_view.h"
#include "selftest_layer_locks.h"
#include "selftest_layers_adjustments.h"
#include "selftest_layers_drag.h"
#include "selftest_layers_interactions.h"
#include "selftest_layers_round3.h"
#include "selftest_layers_smart_object.h"
#include "selftest_warp_preset.h"
#include "selftest_ports.h"
#include "selftest_numeric.h"
#include "selftest_paint_perf.h"
#include "selftest_paint_live.h"
#include "selftest_large_doc.h"
#include "selftest_shell_round3.h"
#include "selftest_shell_round4.h"
#include "selftest_report.h"
#include "api/layers_api.h"
#include "api/type_api.h"
#include "selftest_session.h"
#include "selftest_tools_selection.h"
#include "selftest_tool_canvas.h"
#include "selftest_ui_persistence.h"
#include "selftest_visibility.h"
#include "selftest_workspace_input.h"

#include "commands.h"
#include "frame.h"
#include "layer_new_dialog.h"
#include "panels/layers_panel.h"
#include "panels/panel_column.h"

#include "render_text.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QRect>
#include <QtGui/QAction>
#include <QtGui/QImage>
#include <QtWidgets/QMenu>
#include <QtWidgets/QToolButton>

int pictura::runLayersControlsChecks(pictura::PicturaMainWindow& frame)
{
        // lpc_percent (199): the Opacity control carries 0..100 % and converts
        // to the stored 0..255 byte through the panel, a stored byte displays as
        // its percentage, a programmatic sync adds no history, and the lock
        // strip is five toggles.
        const bool lpcCreated = frame.newDocument(QStringLiteral("PercentCtl"), 16, 16,
                                                  QStringLiteral("rgb"), 8,
                                                  QStringLiteral("white"));
        pictura::PictureView* lpcView = frame.activeView();
        auto* lpcPanel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
        if (!lpcCreated || !lpcView || !lpcPanel) {
            return pictura::selfTest().fail(199, "percent fixture");
        }
        const int lpcDoc = frame.activeDocumentIndex();
        lpcPanel->setView(lpcView);
        lpcView->select_all();
        lpcPanel->refresh();
        const int lpcBase = lpcView->history_count();
        lpcPanel->setOpacityPercentForTest(50);
        const bool lpcByteOk =
            lpcView->layer_opacity(0) == 128 && lpcView->history_count() == lpcBase + 1;
        lpcView->set_layers_opacity(QStringList{QStringLiteral("0")}, 128);
        lpcPanel->refresh();
        const bool lpcDisplayOk = lpcPanel->opacityPercentForTest() == 50
            && lpcPanel->lockButtonCountForTest() == 5;
        const int lpcSyncBase = lpcView->history_count();
        lpcPanel->refresh();
        const bool lpcSyncOk = lpcView->history_count() == lpcSyncBase
            && lpcPanel->opacityPercentForTest() == 50;
        ST_BEGIN("lpc_percent");
        ST_PASS("lpc_percent byte=%d display=%d sync=%d", lpcByteOk ? 1 : 0,
                     lpcDisplayOk ? 1 : 0, lpcSyncOk ? 1 : 0);
        if (!lpcByteOk || !lpcDisplayOk || !lpcSyncOk) {
            return pictura::selfTest().fail(199, "percent controls");
        }
        frame.closeDocument(lpcDoc, false);

        // wpx_blend_row (452): the blend combo and the opacity field share the
        // control row's slack — the opacity field is stretched past its own size
        // hint (a no-stretch field would sit exactly at the hint) while the blend
        // combo still takes the larger share. The slack depends on the column
        // width, which the earlier layout checks leave wherever their group
        // shuffles put the floor, so measure the Layers tab at the widest normal
        // width.
        if (pictura::PanelColumn* lpcColumn = frame.columnForPanel(QStringLiteral("layersPanel"))) {
            lpcColumn->setPreferredWidth(pictura::PanelColumn::kMaxNormalWidth);
            // A background tab keeps stale geometry; bring Layers forward.
            lpcColumn->showPanel(QStringLiteral("layersPanel"), true);
            QCoreApplication::processEvents();
        }
        const QRect blendRow = lpcPanel->blendControlRectForTest();
        const QRect opacityCol = lpcPanel->opacityControlRectForTest();
        const int opacityHint = lpcPanel->opacitySizeHintWidthForTest();
        const bool rowBoth = blendRow.width() > 0 && opacityCol.width() > 0;
        const bool rowSharesSlack = opacityHint > 0 && opacityCol.width() > opacityHint;
        const bool rowBlendLarger = blendRow.width() > opacityCol.width();
        ST_BEGIN("wpx_blend_row");
        ST_PASS("wpx_blend_row both=%d shares_slack=%d blend_bigger=%d blend=%d opacity=%d "
                "hint=%d",
                rowBoth ? 1 : 0, rowSharesSlack ? 1 : 0, rowBlendLarger ? 1 : 0,
                blendRow.width(), opacityCol.width(), opacityHint);
        if (!(rowBoth && rowSharesSlack && rowBlendLarger)) {
            return pictura::selfTest().fail(452, "layers blend row");
        }

        // lpc_preview (214): a preview stream adds no history and one commit
        // adds exactly one state for the finished edit.
        const bool pvCreated = frame.newDocument(QStringLiteral("PreviewCtl"), 16, 16,
                                                 QStringLiteral("rgb"), 8,
                                                 QStringLiteral("white"));
        pictura::PictureView* pvView = frame.activeView();
        auto* pvPanel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
        if (!pvCreated || !pvView || !pvPanel) {
            return pictura::selfTest().fail(214, "preview fixture");
        }
        const int pvDoc = frame.activeDocumentIndex();
        pvPanel->setView(pvView);
        pvView->select_all();
        pvPanel->refresh();
        const int pvBase = pvView->history_count();
        pvView->preview_layers_opacity(QStringList{QStringLiteral("0")}, 100);
        pvView->preview_layers_opacity(QStringList{QStringLiteral("0")}, 200);
        pvView->preview_layers_opacity(QStringList{QStringLiteral("0")}, 50);
        const bool pvNoHistory = pvView->history_count() == pvBase;
        const bool pvValueOk = pvView->layer_opacity(0) == 50;
        pvView->commit_layers_opacity(QStringList{QStringLiteral("0")}, 50);
        const bool pvCommitOk = pvView->history_count() == pvBase + 1;
        const int pvFillBase = pvView->history_count();
        pvView->preview_layers_fill(QStringList{QStringLiteral("0")}, 77);
        pvView->commit_layers_fill(QStringList{QStringLiteral("0")}, 77);
        const bool pvFillOk = pvView->history_count() == pvFillBase + 1;
        const int pvSetBase = pvView->history_count();
        pvView->set_layers_opacity(QStringList{QStringLiteral("0")}, 200);
        const bool pvSetOk = pvView->history_count() == pvSetBase + 1;
        ST_BEGIN("lpc_preview");
        ST_PASS("lpc_preview nohist=%d value=%d commit=%d fill=%d set=%d",
                pvNoHistory ? 1 : 0, pvValueOk ? 1 : 0, pvCommitOk ? 1 : 0,
                pvFillOk ? 1 : 0, pvSetOk ? 1 : 0);
        if (!pvNoHistory || !pvValueOk || !pvCommitOk || !pvFillOk || !pvSetOk) {
            return pictura::selfTest().fail(214, "preview commit");
        }

        // lpr_percent (215): the `%` renders inside the value box, which is
        // wide enough to show the largest value "100 %".
        const bool pvSuffix = pvPanel->opacitySuffixInsideEditForTest();
        const bool pvFits = pvPanel->opacityValueFitsForTest();
        ST_BEGIN("lpr_percent");
        ST_PASS("lpr_percent inside=%d fits=%d", pvSuffix ? 1 : 0, pvFits ? 1 : 0);
        if (!pvSuffix || !pvFits) {
            return pictura::selfTest().fail(215, "percent suffix");
        }

        // lpc_lockbadge (216): an unlocked row has no badge; locking draws one
        // at the right edge, right of the eye.
        const bool pvUnlocked = pvPanel->lockBadgeLeftForTest(QStringLiteral("0")) == -1;
        pvView->set_layers_lock(QStringList{QStringLiteral("0")},
                                QStringLiteral("transparency"), true);
        pvPanel->refresh();
        const int pvBadge = pvPanel->lockBadgeLeftForTest(QStringLiteral("0"));
        const int pvEye = pvPanel->eyeLeftForTest(QStringLiteral("0"));
        const bool pvBadgeOk = pvUnlocked && pvBadge >= 0 && pvBadge > pvEye;
        ST_BEGIN("lpc_lockbadge");
        ST_PASS("lpc_lockbadge unlocked=%d badge=%d eye=%d", pvUnlocked ? 1 : 0, pvBadge, pvEye);
        if (!pvBadgeOk) {
            return pictura::selfTest().fail(216, "lock badge");
        }

        // lpr_eye (217): the model exposes no check state; the eye is the only
        // visibility control.
        const bool pvNoCheck = !pvPanel->rowCheckStateForTest(QStringLiteral("0"));
        ST_BEGIN("lpr_eye");
        ST_PASS("lpr_eye nocheck=%d", pvNoCheck ? 1 : 0);
        if (!pvNoCheck) {
            return pictura::selfTest().fail(217, "row eye");
        }

        // lpr_slider (218): clicking the groove jumps the handle to that point
        // (not a page step) and holding the button tracks the cursor.
        const int sliderJump = pvPanel->dragOpacitySliderForTest(750, 750);
        const int sliderDrag = pvPanel->dragOpacitySliderForTest(100, 800);
        const bool sliderJumpOk = sliderJump >= 55 && sliderJump <= 95;
        const bool sliderDragOk = sliderDrag >= 60 && sliderDrag <= 95;
        ST_BEGIN("lpr_slider");
        ST_PASS("lpr_slider jump=%d drag=%d", sliderJump, sliderDrag);
        if (!sliderJumpOk || !sliderDragOk) {
            return pictura::selfTest().fail(218, "slider jump");
        }
        frame.closeDocument(pvDoc, false);

        // lpr_merge_down (219): two pixel layers merge to one; the result
        // inherits the lower layer's name/blend/opacity in one undo state.
        const bool mdCreated = frame.newDocument(QStringLiteral("MergeDownCtl"), 16, 16,
                                                 QStringLiteral("rgb"), 8,
                                                 QStringLiteral("white"));
        pictura::PictureView* mdView = frame.activeView();
        if (!mdCreated || !mdView) {
            return pictura::selfTest().fail(219, "merge down fixture");
        }
        const int mdDoc = frame.activeDocumentIndex();
        mdView->add_layer(0);
        mdView->add_layer(1);
        mdView->set_layer_name(1, QStringLiteral("Lower"));
        mdView->set_layer_blend(1, QStringLiteral("mul "));
        mdView->set_layer_opacity(1, 128);
        mdView->set_layer_name(2, QStringLiteral("Upper"));
        const int mdBase = mdView->history_count();
        const int mdReplaced = mdView->merge_layers(QStringList{QStringLiteral("2")});
        const bool mdOk = mdReplaced == 2 && mdView->layer_count() == 2
            && mdView->layer_name(1) == QStringLiteral("Lower")
            && mdView->layer_blend(1) == QStringLiteral("mul ")
            && mdView->layer_opacity(1) == 128 && mdView->history_count() == mdBase + 1;
        ST_BEGIN("lpr_merge_down");
        ST_PASS("lpr_merge_down replaced=%d count=%d", mdReplaced, mdView->layer_count());
        if (!mdOk) {
            return pictura::selfTest().fail(219, "merge down");
        }
        frame.closeDocument(mdDoc, false);

        // lpr_merge_visible (220): visible layers collapse while the hidden
        // layer survives untouched, in one undo state.
        const bool mvCreated = frame.newDocument(QStringLiteral("MergeVisibleCtl"), 16, 16,
                                                 QStringLiteral("rgb"), 8,
                                                 QStringLiteral("white"));
        pictura::PictureView* mvView = frame.activeView();
        if (!mvCreated || !mvView) {
            return pictura::selfTest().fail(220, "merge visible fixture");
        }
        const int mvDoc = frame.activeDocumentIndex();
        mvView->add_layer(0);
        mvView->add_layer(1);
        mvView->set_layer_name(1, QStringLiteral("Visible1"));
        mvView->set_layer_name(2, QStringLiteral("Hidden"));
        mvView->set_layer_visible(2, false);
        const int mvBase = mvView->history_count();
        const int mvReplaced = mvView->merge_visible(QStringLiteral("1"));
        bool mvHiddenSurvives = false;
        for (int i = 0; i < pictura::layers_api::rowCount(*mvView); ++i) {
            if (pictura::layers_api::rowAt(*mvView, i).name == QStringLiteral("Hidden")) {
                mvHiddenSurvives = !pictura::layers_api::rowAt(*mvView, i).visible;
            }
        }
        const bool mvOk = mvReplaced == 2 && pictura::layers_api::rowCount(*mvView) == 2 && mvHiddenSurvives
            && mvView->history_count() == mvBase + 1;
        ST_BEGIN("lpr_merge_visible");
        ST_PASS("lpr_merge_visible replaced=%d rows=%d hidden=%d", mvReplaced,
                pictura::layers_api::rowCount(*mvView), mvHiddenSurvives ? 1 : 0);
        if (!mvOk) {
            return pictura::selfTest().fail(220, "merge visible");
        }
        frame.closeDocument(mvDoc, false);

        // lpr_flatten (221): one opaque Background, transparency filled white,
        // a hidden layer discarded, in one undo state.
        const bool flCreated = frame.newDocument(QStringLiteral("FlattenCtl"), 4, 4,
                                                 QStringLiteral("rgb"), 8,
                                                 QStringLiteral("transparent"));
        pictura::PictureView* flView = frame.activeView();
        if (!flCreated || !flView) {
            return pictura::selfTest().fail(221, "flatten fixture");
        }
        const int flDoc = frame.activeDocumentIndex();
        flView->add_layer(0);
        flView->set_layer_visible(1, false);
        const bool flTransparentBefore = flView->sample_argb(0, 0) == 0u;
        const int flBase = flView->history_count();
        const int flReplaced = flView->flatten_image();
        const bool flOk = flTransparentBefore && flReplaced == 2 && flView->layer_count() == 1
            && flView->layer_name(0) == QStringLiteral("Background") && flView->layer_visible(0)
            && flView->sample_argb(0, 0) == 0xFFFFFFFFu && flView->history_count() == flBase + 1;
        ST_BEGIN("lpr_flatten");
        ST_PASS("lpr_flatten replaced=%d count=%d white=%08x", flReplaced,
                flView->layer_count(), flView->sample_argb(0, 0));
        if (!flOk) {
            return pictura::selfTest().fail(221, "flatten image");
        }
        frame.closeDocument(flDoc, false);

        // lpr_merge_clip (222): a plain layer is not a clipping base, so the
        // command refuses without recording history.
        const bool mcCreated =
            frame.newDocument(QStringLiteral("MergeClipCtl"), 8, 8, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"));
        pictura::PictureView* mcView = frame.activeView();
        if (!mcCreated || !mcView) {
            return pictura::selfTest().fail(222, "merge clip fixture");
        }
        const int mcDoc = frame.activeDocumentIndex();
        const int mcBase = mcView->history_count();
        const int mcReplaced = mcView->merge_clipping_mask(QStringLiteral("0"));
        const bool mcOk = mcReplaced == 0 && mcView->history_count() == mcBase
            && mcView->layer_count() == 1;
        ST_BEGIN("lpr_merge_clip");
        ST_PASS("lpr_merge_clip replaced=%d history=%d", mcReplaced,
                mcView->history_count() - mcBase);
        if (!mcOk) {
            return pictura::selfTest().fail(222, "merge clipping mask");
        }
        frame.closeDocument(mcDoc, false);

        // lpr_new_layer_dialog (223): the bridge creates a configured layer with
        // the chosen name/color/blend/opacity, and a Multiply-neutral fill is
        // opaque white, in one undo state.
        const bool nldCreated =
            frame.newDocument(QStringLiteral("NewLayerCtl"), 4, 4, QStringLiteral("rgb"), 8,
                              QStringLiteral("transparent"));
        pictura::PictureView* nldView = frame.activeView();
        if (!nldCreated || !nldView) {
            return pictura::selfTest().fail(223, "new layer dialog fixture");
        }
        const int nldDoc = frame.activeDocumentIndex();
        const int nldBase = nldView->history_count();
        const QString nldPath = nldView->new_layer_dialog(
            QString(), QStringLiteral("Neutral"), 1, QStringLiteral("mul "), 128, 255, true, false);
        const bool nldOk = nldPath == QStringLiteral("1") && nldView->layer_count() == 2
            && nldView->layer_name(1) == QStringLiteral("Neutral")
            && nldView->layer_blend(1) == QStringLiteral("mul ")
            && nldView->layer_opacity(1) == 128 && nldView->layer_color(1) == 1
            && nldView->sample_argb(0, 0) == 0x80FFFFFFu
            && nldView->history_count() == nldBase + 1;
        ST_BEGIN("lpr_new_layer_dialog");
        ST_PASS("lpr_new_layer_dialog path=%s pixel=%08x", qPrintable(nldPath),
                nldView->sample_argb(0, 0));
        if (!nldOk) {
            return pictura::selfTest().fail(223, "new layer dialog");
        }
        frame.closeDocument(nldDoc, false);

        // lpr_new_group_dialog (224): the group carries name/color/blend/opacity
        // and never a clipping flag, in one undo state.
        const bool ngdCreated =
            frame.newDocument(QStringLiteral("NewGroupCtl"), 4, 4, QStringLiteral("rgb"), 8,
                              QStringLiteral("transparent"));
        pictura::PictureView* ngdView = frame.activeView();
        if (!ngdCreated || !ngdView) {
            return pictura::selfTest().fail(224, "new group dialog fixture");
        }
        const int ngdDoc = frame.activeDocumentIndex();
        const int ngdBase = ngdView->history_count();
        const QString ngdPath = ngdView->new_group_dialog(QString(), QStringLiteral("Grp"), 3,
                                                          QStringLiteral("scrn"), 200);
        int ngdRow = -1;
        for (int i = 0; i < pictura::layers_api::rowCount(*ngdView); ++i) {
            if (pictura::layers_api::rowAt(*ngdView, i).path == ngdPath) {
                ngdRow = i;
            }
        }
        const bool ngdOk = ngdPath == QStringLiteral("1")
            && pictura::layers_api::rowAt(*ngdView, ngdRow).kind == QStringLiteral("group")
            && pictura::layers_api::rowAt(*ngdView, ngdRow).name == QStringLiteral("Grp")
            && pictura::layers_api::rowAt(*ngdView, ngdRow).blend == QStringLiteral("scrn")
            && pictura::layers_api::rowAt(*ngdView, ngdRow).opacity == 200
            && pictura::layers_api::rowAt(*ngdView, ngdRow).color == 3
            && !pictura::layers_api::rowAt(*ngdView, ngdRow).clipping
            && ngdView->history_count() == ngdBase + 1;
        ST_BEGIN("lpr_new_group_dialog");
        ST_PASS("lpr_new_group_dialog path=%s row=%d clip=%d", qPrintable(ngdPath), ngdRow,
                pictura::layers_api::rowAt(*ngdView, ngdRow).clipping ? 1 : 0);
        if (!ngdOk) {
            return pictura::selfTest().fail(224, "new group dialog");
        }
        frame.closeDocument(ngdDoc, false);

        // lpr_new_neutral_option (225): the dialog disables the neutral fill for
        // Normal and hides clipping for a group; the engine's missing neutral
        // color creates a fully transparent layer.
        pictura::LayerNewDialog nnoLayer(false);
        nnoLayer.setModeForTest(QStringLiteral("norm"));
        const bool nnoNormalOff = !nnoLayer.neutralEnabledForTest();
        nnoLayer.setModeForTest(QStringLiteral("mul "));
        const bool nnoMultiplyOn = nnoLayer.neutralEnabledForTest();
        pictura::LayerNewDialog nnoGroup(true);
        const bool nnoClippingHidden = !nnoGroup.clippingVisibleForTest();
        const bool nnoCreated =
            frame.newDocument(QStringLiteral("NeutralNoneCtl"), 4, 4, QStringLiteral("rgb"), 8,
                              QStringLiteral("transparent"));
        pictura::PictureView* nnoView = frame.activeView();
        if (!nnoCreated || !nnoView) {
            return pictura::selfTest().fail(225, "neutral option fixture");
        }
        const int nnoDoc = frame.activeDocumentIndex();
        const int nnoBase = nnoView->history_count();
        const QString nnoPath = nnoView->new_layer_dialog(
            QString(), QStringLiteral("Normal"), 0, QStringLiteral("norm"), 255, 255, true, false);
        const bool nnoOk = nnoNormalOff && nnoMultiplyOn && nnoClippingHidden
            && !nnoPath.isEmpty() && nnoView->sample_argb(0, 0) == 0u
            && nnoView->history_count() == nnoBase + 1;
        ST_BEGIN("lpr_new_neutral_option");
        ST_PASS("lpr_new_neutral_option normal=%d multiply=%d clip=%d pixel=%08x",
                nnoNormalOff ? 1 : 0, nnoMultiplyOn ? 1 : 0, nnoClippingHidden ? 1 : 0,
                nnoView->sample_argb(0, 0));
        if (!nnoOk) {
            return pictura::selfTest().fail(225, "neutral option");
        }
        frame.closeDocument(nnoDoc, false);

        // lpr_background_roundtrip (226): a flagged layer is written under the
        // "Background" name and re-read from disk as the Background.
        const bool bgRtCreated = frame.newDocument(QStringLiteral("BackgroundRoundtrip"), 4, 4,
                                                   QStringLiteral("rgb"), 8,
                                                   QStringLiteral("transparent"));
        pictura::PictureView* bgRtView = frame.activeView();
        if (!bgRtCreated || !bgRtView) {
            return pictura::selfTest().fail(226, "background roundtrip fixture");
        }
        const int bgRtDoc = frame.activeDocumentIndex();
        const int bgRtBase = bgRtView->history_count();
        const bool bgRtTo = bgRtView->background_from_layer(QStringLiteral("0"));
        const bool bgRtState = bgRtTo && bgRtView->history_count() == bgRtBase + 1
            && pictura::layers_api::rowAt(*bgRtView, 0).kind == QStringLiteral("background");
        const QString bgRtPath =
            QDir::tempPath() + QStringLiteral("/kooka-pictura-background.psd");
        const bool bgRtSaved = frame.saveActiveAs(bgRtPath);
        const bool bgRtReopened = frame.openPath(bgRtPath);
        pictura::PictureView* bgRtReload = frame.activeView();
        const int bgRtReloadDoc = frame.activeDocumentIndex();
        const bool bgRtOk = bgRtState && bgRtSaved && bgRtReopened && bgRtReload
            && bgRtReload->layer_name(0) == QStringLiteral("Background")
            && pictura::layers_api::rowAt(*bgRtReload, 0).kind == QStringLiteral("background");
        ST_BEGIN("lpr_background_roundtrip");
        ST_PASS("lpr_background_roundtrip state=%d saved=%d reopened=%d kind=%s",
                bgRtState ? 1 : 0, bgRtSaved ? 1 : 0, bgRtReopened ? 1 : 0,
                qPrintable(bgRtReload ? pictura::layers_api::rowAt(*bgRtReload, 0).kind : QString()));
        if (!bgRtOk) {
            return pictura::selfTest().fail(226, "background roundtrip");
        }
        frame.closeDocument(bgRtReloadDoc, false);
        frame.closeDocument(bgRtDoc, false);

        // lpr_background_independence (227): the flag is independent of name and
        // position — a non-bottom layer named "Background" is not the Background,
        // and a flagged bottom layer under another name is.
        const bool bgIndCreated =
            frame.newDocument(QStringLiteral("BackgroundIndependence"), 4, 4,
                              QStringLiteral("rgb"), 8, QStringLiteral("transparent"));
        pictura::PictureView* bgIndView = frame.activeView();
        if (!bgIndCreated || !bgIndView) {
            return pictura::selfTest().fail(227, "background independence fixture");
        }
        const int bgIndDoc = frame.activeDocumentIndex();
        const QString bgIndTop = bgIndView->add_layer_in(QString());
        const bool bgIndNamed = bgIndView->set_layer_name_path(bgIndTop,
                                                               QStringLiteral("Background"));
        const bool bgIndFlagged = bgIndView->background_from_layer(QStringLiteral("0"));
        int bgIndBottomRow = -1;
        int bgIndTopRow = -1;
        for (int i = 0; i < pictura::layers_api::rowCount(*bgIndView); ++i) {
            if (pictura::layers_api::rowAt(*bgIndView, i).path == QStringLiteral("0")) {
                bgIndBottomRow = i;
            }
            if (pictura::layers_api::rowAt(*bgIndView, i).path == bgIndTop) {
                bgIndTopRow = i;
            }
        }
        const bool bgIndOk = bgIndNamed && bgIndFlagged && bgIndBottomRow >= 0 && bgIndTopRow >= 0
            && pictura::layers_api::rowAt(*bgIndView, bgIndBottomRow).kind == QStringLiteral("background")
            && pictura::layers_api::rowAt(*bgIndView, bgIndTopRow).kind == QStringLiteral("pixel")
            && pictura::layers_api::rowAt(*bgIndView, bgIndTopRow).name == QStringLiteral("Background");
        ST_BEGIN("lpr_background_independence");
        ST_PASS("lpr_background_independence named=%d bottom=%d top=%d",
                bgIndNamed ? 1 : 0,
                bgIndBottomRow >= 0 ? pictura::layers_api::rowAt(*bgIndView, bgIndBottomRow).kind ==
                                          QStringLiteral("background")
                                    : 0,
                bgIndTopRow >= 0 ? pictura::layers_api::rowAt(*bgIndView, bgIndTopRow).kind == QStringLiteral("pixel")
                                 : 0);
        if (!bgIndOk) {
            return pictura::selfTest().fail(227, "background independence");
        }
        frame.closeDocument(bgIndDoc, false);

        // lpr_background_convert (228): Background From Layer makes transparency
        // opaque and moves the node to the bottom; Layer from Background clears
        // the flag and unlocks.
        const bool bgConvCreated =
            frame.newDocument(QStringLiteral("BackgroundConvert"), 4, 4, QStringLiteral("rgb"), 8,
                              QStringLiteral("transparent"));
        pictura::PictureView* bgConvView = frame.activeView();
        if (!bgConvCreated || !bgConvView) {
            return pictura::selfTest().fail(228, "background convert fixture");
        }
        const int bgConvDoc = frame.activeDocumentIndex();
        const QString bgConvTop = bgConvView->add_layer_in(QString());
        bgConvView->set_layer_name_path(bgConvTop, QStringLiteral("Convertible"));
        bgConvView->set_layers_lock(QStringList{bgConvTop}, QStringLiteral("all"), true);
        const int bgConvBase = bgConvView->history_count();
        const bool bgConvTo = bgConvView->background_from_layer(bgConvTop);
        const bool bgConvToOk = bgConvTo && bgConvView->history_count() == bgConvBase + 1
            && bgConvView->sample_argb(0, 0) == 0xFFFFFFFFu
            && bgConvView->layer_kind(0) == QStringLiteral("background")
            && bgConvView->layer_name(0) == QStringLiteral("Convertible");
        const int bgConvFromBase = bgConvView->history_count();
        const bool bgConvFrom = bgConvView->layer_from_background(QStringLiteral("0"));
        const bool bgConvFromOk = bgConvFrom
            && bgConvView->history_count() == bgConvFromBase + 1
            && bgConvView->layer_kind(0) == QStringLiteral("pixel")
            && bgConvView->layer_lock(0) == 0;
        ST_BEGIN("lpr_background_convert");
        ST_PASS("lpr_background_convert to=%d pixel=%08x from=%d lock=%d",
                bgConvTo ? 1 : 0, bgConvView->sample_argb(0, 0), bgConvFrom ? 1 : 0,
                bgConvView->layer_lock(0));
        if (!bgConvToOk || !bgConvFromOk) {
            return pictura::selfTest().fail(228, "background convert");
        }
        frame.closeDocument(bgConvDoc, false);

        // lpr_via_copy (229): Layer via Copy adds a "Layer 0 copy" above the
        // source holding only the selected pixels; the source is untouched, in
        // one undo state.
        const bool vcCreated = frame.newDocument(QStringLiteral("ViaCopyCtl"), 4, 4,
                                                 QStringLiteral("rgb"), 8,
                                                 QStringLiteral("white"));
        pictura::PictureView* vcView = frame.activeView();
        if (!vcCreated || !vcView) {
            return pictura::selfTest().fail(229, "via copy fixture");
        }
        const int vcDoc = frame.activeDocumentIndex();
        vcView->select_rect(0, 0, 2, 4, QStringLiteral("new"), 0.0);
        const int vcBase = vcView->history_count();
        const QString vcPath = vcView->layer_via_copy(QStringLiteral("0"));
        const QImage vcCopy = vcView->layer_thumbnail(1, 4);
        const QImage vcSrc = vcView->layer_thumbnail(0, 4);
        const bool vcOk = vcPath == QStringLiteral("1") && vcView->layer_count() == 2
            && vcView->layer_name(1) == QStringLiteral("Layer 0 copy")
            && vcView->history_count() == vcBase + 1 && !vcCopy.isNull() && !vcSrc.isNull()
            && qAlpha(vcCopy.pixel(0, 0)) == 255 && qAlpha(vcCopy.pixel(3, 0)) == 0
            && qAlpha(vcSrc.pixel(0, 0)) == 255 && qAlpha(vcSrc.pixel(3, 0)) == 255;
        ST_BEGIN("lpr_via_copy");
        ST_PASS("lpr_via_copy path=%s copy=%d/%d source=%d/%d", qPrintable(vcPath),
                qAlpha(vcCopy.pixel(0, 0)), qAlpha(vcCopy.pixel(3, 0)),
                qAlpha(vcSrc.pixel(0, 0)), qAlpha(vcSrc.pixel(3, 0)));
        if (!vcOk) {
            return pictura::selfTest().fail(229, "layer via copy");
        }
        frame.closeDocument(vcDoc, false);

        // lpr_via_cut (230): Layer via Cut fills the new layer with the selected
        // pixels and clears them from the source, in one undo state.
        const bool vcutCreated = frame.newDocument(QStringLiteral("ViaCutCtl"), 4, 4,
                                                   QStringLiteral("rgb"), 8,
                                                   QStringLiteral("white"));
        pictura::PictureView* vcutView = frame.activeView();
        if (!vcutCreated || !vcutView) {
            return pictura::selfTest().fail(230, "via cut fixture");
        }
        const int vcutDoc = frame.activeDocumentIndex();
        vcutView->select_rect(0, 0, 2, 4, QStringLiteral("new"), 0.0);
        const int vcutBase = vcutView->history_count();
        const QString vcutPath = vcutView->layer_via_cut(QStringLiteral("0"));
        const QImage vcutCopy = vcutView->layer_thumbnail(1, 4);
        const QImage vcutSrc = vcutView->layer_thumbnail(0, 4);
        const bool vcutOk = vcutPath == QStringLiteral("1") && vcutView->layer_count() == 2
            && vcutView->history_count() == vcutBase + 1 && !vcutCopy.isNull() && !vcutSrc.isNull()
            && qAlpha(vcutCopy.pixel(0, 0)) == 255 && qAlpha(vcutCopy.pixel(3, 0)) == 0
            && qAlpha(vcutSrc.pixel(0, 0)) == 0 && qAlpha(vcutSrc.pixel(3, 0)) == 255;
        ST_BEGIN("lpr_via_cut");
        ST_PASS("lpr_via_cut path=%s copy=%d/%d source=%d/%d", qPrintable(vcutPath),
                qAlpha(vcutCopy.pixel(0, 0)), qAlpha(vcutCopy.pixel(3, 0)),
                qAlpha(vcutSrc.pixel(0, 0)), qAlpha(vcutSrc.pixel(3, 0)));
        if (!vcutOk) {
            return pictura::selfTest().fail(230, "layer via cut");
        }
        frame.closeDocument(vcutDoc, false);

        // lpr_via_refuse (231): with no active selection both commands return
        // empty and add no history.
        const bool vrCreated = frame.newDocument(QStringLiteral("ViaRefuseCtl"), 4, 4,
                                                 QStringLiteral("rgb"), 8,
                                                 QStringLiteral("white"));
        pictura::PictureView* vrView = frame.activeView();
        if (!vrCreated || !vrView) {
            return pictura::selfTest().fail(231, "via refuse fixture");
        }
        const int vrDoc = frame.activeDocumentIndex();
        const int vrBase = vrView->history_count();
        const int vrRows = vrView->layer_count();
        const QString vrCopy = vrView->layer_via_copy(QStringLiteral("0"));
        const QString vrCut = vrView->layer_via_cut(QStringLiteral("0"));
        const bool vrOk = vrCopy.isEmpty() && vrCut.isEmpty()
            && vrView->history_count() == vrBase && vrView->layer_count() == vrRows;
        ST_BEGIN("lpr_via_refuse");
        ST_PASS("lpr_via_refuse copy=%d cut=%d history=%d", vrCopy.isEmpty() ? 1 : 0,
                vrCut.isEmpty() ? 1 : 0, vrView->history_count() - vrBase);
        if (!vrOk) {
            return pictura::selfTest().fail(231, "layer via no selection");
        }
        frame.closeDocument(vrDoc, false);

        const auto rowForPath = [](pictura::PictureView* view, const QString& path) {
            for (int i = 0; i < pictura::layers_api::rowCount(*view); ++i) {
                if (pictura::layers_api::rowAt(*view, i).path == path) {
                    return i;
                }
            }
            return -1;
        };

        // lpr_select_similar (232): Select Similar returns every layer of the
        // active pixel kind and blend mode, topmost-first, excluding the
        // reference and the differently-blended layer. No history.
        const bool ssCreated = frame.newDocument(QStringLiteral("SelectSimilarCtl"), 4, 4,
                                                 QStringLiteral("rgb"), 8,
                                                 QStringLiteral("white"));
        pictura::PictureView* ssView = frame.activeView();
        if (!ssCreated || !ssView) {
            return pictura::selfTest().fail(232, "select similar fixture");
        }
        const int ssDoc = frame.activeDocumentIndex();
        ssView->add_layer(0);
        ssView->add_layer(1);
        ssView->add_layer(2);
        ssView->set_layer_blend(2, QStringLiteral("mul "));
        const int ssBase = ssView->history_count();
        const QStringList ssMatched = ssView->select_similar(QStringLiteral("3"));
        const bool ssOk = ssMatched.size() == 2 && ssMatched.first() == QStringLiteral("1")
            && ssMatched.contains(QStringLiteral("0"))
            && !ssMatched.contains(QStringLiteral("2"))
            && !ssMatched.contains(QStringLiteral("3"))
            && ssView->history_count() == ssBase;
        ST_BEGIN("lpr_select_similar");
        ST_PASS("lpr_select_similar n=%d first=%s", ssMatched.size(),
                qPrintable(ssMatched.isEmpty() ? QString() : ssMatched.first()));
        if (!ssOk) {
            return pictura::selfTest().fail(232, "select similar");
        }
        frame.closeDocument(ssDoc, false);

        // lpr_link_linked_unlink (233): linking two layers makes Select Linked
        // return both; unlinking one drops just that membership. View-only: no
        // history.
        const bool lkCreated = frame.newDocument(QStringLiteral("LinkCtl"), 4, 4,
                                                 QStringLiteral("rgb"), 8,
                                                 QStringLiteral("white"));
        pictura::PictureView* lkView = frame.activeView();
        if (!lkCreated || !lkView) {
            return pictura::selfTest().fail(233, "link fixture");
        }
        const int lkDoc = frame.activeDocumentIndex();
        lkView->add_layer(0);
        const int lkBase = lkView->history_count();
        const int lkLinked = lkView->link_layers(QStringList{QStringLiteral("0"),
                                                             QStringLiteral("1")}, true);
        const QStringList lkBoth = lkView->select_linked(QStringLiteral("0"));
        const int lkUnlinked = lkView->link_layers(QStringList{QStringLiteral("0")}, false);
        const QStringList lkOne = lkView->select_linked(QStringLiteral("1"));
        const QStringList lkGone = lkView->select_linked(QStringLiteral("0"));
        const bool lkOk = lkLinked == 2 && lkBoth.size() == 2
            && lkBoth.contains(QStringLiteral("0")) && lkBoth.contains(QStringLiteral("1"))
            && lkUnlinked == 1 && lkOne == QStringList{QStringLiteral("1")} && lkGone.isEmpty()
            && lkView->history_count() == lkBase;
        ST_BEGIN("lpr_link_linked_unlink");
        ST_PASS("lpr_link_linked_unlink linked=%d both=%d unlinked=%d one=%d",
                lkLinked, lkBoth.size(), lkUnlinked, lkOne.size());
        if (!lkOk) {
            return pictura::selfTest().fail(233, "link/linked/unlink");
        }
        frame.closeDocument(lkDoc, false);

        // lpr_delete_hidden (234): only the hidden layer is removed, in one undo
        // state; a second run with nothing hidden removes nothing and records
        // no history.
        const bool dhCreated = frame.newDocument(QStringLiteral("DeleteHiddenCtl"), 4, 4,
                                                 QStringLiteral("rgb"), 8,
                                                 QStringLiteral("white"));
        pictura::PictureView* dhView = frame.activeView();
        if (!dhCreated || !dhView) {
            return pictura::selfTest().fail(234, "delete hidden fixture");
        }
        const int dhDoc = frame.activeDocumentIndex();
        dhView->add_layer(0);
        dhView->add_layer(1);
        dhView->set_layer_name(1, QStringLiteral("Hidden"));
        dhView->set_layer_visible(1, false);
        const int dhBase = dhView->history_count();
        const int dhRemoved = dhView->delete_hidden_layers();
        bool dhGone = pictura::layers_api::rowCount(*dhView) == 2;
        for (int i = 0; i < pictura::layers_api::rowCount(*dhView); ++i) {
            if (pictura::layers_api::rowAt(*dhView, i).name == QStringLiteral("Hidden")) {
                dhGone = false;
            }
        }
        const bool dhRecorded = dhView->history_count() == dhBase + 1;
        const int dhBase2 = dhView->history_count();
        const int dhAgain = dhView->delete_hidden_layers();
        const bool dhOk = dhRemoved == 1 && dhGone && dhRecorded && dhAgain == 0
            && dhView->history_count() == dhBase2;
        ST_BEGIN("lpr_delete_hidden");
        ST_PASS("lpr_delete_hidden removed=%d gone=%d again=%d", dhRemoved, dhGone ? 1 : 0,
                dhAgain);
        if (!dhOk) {
            return pictura::selfTest().fail(234, "delete hidden layers");
        }
        frame.closeDocument(dhDoc, false);

        // lpr_hide_layers (235): Hide Layers hides the selected layer in one
        // undo state and leaves the unselected layer visible; hiding an
        // already-hidden layer records nothing.
        const bool hlCreated = frame.newDocument(QStringLiteral("HideLayersCtl"), 4, 4,
                                                 QStringLiteral("rgb"), 8,
                                                 QStringLiteral("white"));
        pictura::PictureView* hlView = frame.activeView();
        if (!hlCreated || !hlView) {
            return pictura::selfTest().fail(235, "hide layers fixture");
        }
        const int hlDoc = frame.activeDocumentIndex();
        hlView->add_layer(0);
        const int hlBase = hlView->history_count();
        const int hlChanged = hlView->hide_layers(QStringList{QStringLiteral("0")});
        const int hlRow0 = rowForPath(hlView, QStringLiteral("0"));
        const int hlRow1 = rowForPath(hlView, QStringLiteral("1"));
        const bool hlOk = hlChanged == 1 && hlRow0 >= 0 && hlRow1 >= 0
            && !pictura::layers_api::rowAt(*hlView, hlRow0).visible && pictura::layers_api::rowAt(*hlView, hlRow1).visible
            && hlView->history_count() == hlBase + 1;
        const int hlBase2 = hlView->history_count();
        const int hlAgain = hlView->hide_layers(QStringList{QStringLiteral("0")});
        const bool hlNoop = hlAgain == 0 && hlView->history_count() == hlBase2;
        ST_BEGIN("lpr_hide_layers");
        ST_PASS("lpr_hide_layers changed=%d hidden=%d again=%d", hlChanged,
                hlRow0 >= 0 ? (pictura::layers_api::rowAt(*hlView, hlRow0).visible ? 0 : 1) : -1, hlAgain);
        if (!hlOk || !hlNoop) {
            return pictura::selfTest().fail(235, "hide layers");
        }
        frame.closeDocument(hlDoc, false);

        // lpr_solid_fill (236): a solid fill layer is fill content, composites
        // its color over the stack, and records one undo state.
        const bool sfCreated = frame.newDocument(QStringLiteral("SolidFillCtl"), 4, 4,
                                                 QStringLiteral("rgb"), 8,
                                                 QStringLiteral("white"));
        pictura::PictureView* sfView = frame.activeView();
        if (!sfCreated || !sfView) {
            return pictura::selfTest().fail(236, "solid fill fixture");
        }
        const int sfDoc = frame.activeDocumentIndex();
        const int sfBase = sfView->history_count();
        const QString sfPath = sfView->add_solid_fill(0xff3366ffu);
        const bool sfOk = sfPath == QStringLiteral("1")
            && sfView->layer_is_fill_content(sfPath)
            && sfView->sample_argb(0, 0) == 0xff3366ffu
            && sfView->history_count() == sfBase + 1;
        ST_BEGIN("lpr_solid_fill");
        ST_PASS("lpr_solid_fill path=%s fill=%d pixel=%08x", qPrintable(sfPath),
                sfView->layer_is_fill_content(sfPath) ? 1 : 0, sfView->sample_argb(0, 0));
        if (!sfOk) {
            return pictura::selfTest().fail(236, "solid fill layer");
        }
        frame.closeDocument(sfDoc, false);

        // lpr_rasterize_fill (237): Rasterize Fill Content bakes the color into
        // the layer's pixels, clears the fill data, and refuses a second run
        // without adding history.
        const bool rfCreated = frame.newDocument(QStringLiteral("RasterFillCtl"), 4, 4,
                                                 QStringLiteral("rgb"), 8,
                                                 QStringLiteral("white"));
        pictura::PictureView* rfView = frame.activeView();
        if (!rfCreated || !rfView) {
            return pictura::selfTest().fail(237, "rasterize fill fixture");
        }
        const int rfDoc = frame.activeDocumentIndex();
        const QString rfPath = rfView->add_solid_fill(0xff2244aau);
        const int rfBase = rfView->history_count();
        const bool rfRaster = rfView->rasterize_fill_content(rfPath);
        const bool rfPixel = rfView->layer_kind(rfPath.toInt()) == QStringLiteral("pixel");
        const bool rfBaked = rfView->sample_argb(0, 0) == 0xff2244aau;
        const bool rfRecorded = rfView->history_count() == rfBase + 1;
        const int rfBase2 = rfView->history_count();
        const bool rfAgain = rfView->rasterize_fill_content(rfPath);
        const bool rfOk = rfRaster && rfPixel && rfBaked && rfRecorded && !rfAgain
            && rfView->history_count() == rfBase2;
        ST_BEGIN("lpr_rasterize_fill");
        ST_PASS("lpr_rasterize_fill raster=%d pixel=%d color=%08x again=%d",
                rfRaster ? 1 : 0, rfPixel ? 1 : 0, rfView->sample_argb(0, 0),
                rfAgain ? 1 : 0);
        if (!rfOk) {
            return pictura::selfTest().fail(237, "rasterize fill content");
        }
        frame.closeDocument(rfDoc, false);

        // lpr_rasterize_refuse (238): a plain layer is not rasterizable by the
        // generic, type, or all-layers commands, all without history; the
        // Rasterize variants with no matching current layer (Shape/Layer
        // Style/Video/3D) stay disabled.
        const bool rrCreated = frame.newDocument(QStringLiteral("RasterRefuseCtl"), 4, 4,
                                                 QStringLiteral("rgb"), 8,
                                                 QStringLiteral("white"));
        pictura::PictureView* rrView = frame.activeView();
        if (!rrCreated || !rrView) {
            return pictura::selfTest().fail(238, "rasterize refuse fixture");
        }
        const int rrDoc = frame.activeDocumentIndex();
        const int rrBase = rrView->history_count();
        const bool rrPlain = rrView->rasterize_layer(QStringLiteral("0"));
        const int rrAll = rrView->rasterize_all_layers();
        const bool rrType = pictura::type_api::rasterize(*rrView, QStringLiteral("0"));
        const bool rrHistory = rrView->history_count() == rrBase;
        bool kindlessDisabled = true;
        for (const QString& command : {QStringLiteral("layer.rasterize.shape"),
                                       QStringLiteral("layer.rasterize.layer.style"),
                                       QStringLiteral("layer.rasterize.video"),
                                       QStringLiteral("layer.rasterize.3d")}) {
            QAction* action = frame.registry()->action(command);
            if (!action || action->isEnabled()) {
                kindlessDisabled = false;
            }
        }
        const bool rrOk = !rrPlain && rrAll == 0 && !rrType && rrHistory && kindlessDisabled;
        ST_BEGIN("lpr_rasterize_refuse");
        ST_PASS("lpr_rasterize_refuse plain=%d all=%d type=%d history=%d kindless=%d",
                rrPlain ? 1 : 0, rrAll, rrType ? 1 : 0, rrView->history_count() - rrBase,
                kindlessDisabled ? 1 : 0);
        if (!rrOk) {
            return pictura::selfTest().fail(238, "rasterize refusal");
        }
        frame.closeDocument(rrDoc, false);

        if (const int lso = pictura::runLayersSmartObjectConvertChecks(frame); lso != 0) { return lso; }

        if (const int lso = pictura::runLayersSmartObjectRasterizeChecks(frame); lso != 0) { return lso; }

        // lpr_drop_out (239): a layer nested in a group, dropped on the empty
        // viewport (empty target, mode 0), reparents to the document root in
        // one undo step; the validator accepts the root target.
        const bool doCreated = frame.newDocument(QStringLiteral("DropOutCtl"), 8, 8,
                                                 QStringLiteral("rgb"), 8,
                                                 QStringLiteral("white"));
        pictura::PictureView* doView = frame.activeView();
        if (!doCreated || !doView) {
            return pictura::selfTest().fail(239, "drop out fixture");
        }
        const int doDoc = frame.activeDocumentIndex();
        lpcPanel->setView(doView);
        const QString doGroup = doView->add_group_in(QString());
        const QString doChild = doView->add_layer_in(doGroup);
        doView->set_layer_name_path(doChild, QStringLiteral("Out"));
        lpcPanel->refresh();
        const int doBase = doView->history_count();
        const bool doCan = lpcPanel->canMoveForTest(doChild, QString(), 0);
        const bool doMoved = lpcPanel->moveForTest(doChild, QString(), 0);
        QString doWhere;
        for (int i = 0; i < pictura::layers_api::rowCount(*doView); ++i) {
            if (pictura::layers_api::rowAt(*doView, i).name == QStringLiteral("Out")) {
                doWhere = pictura::layers_api::rowAt(*doView, i).path;
            }
        }
        const bool doOk = doCan && doMoved && doWhere == QStringLiteral("2")
            && doView->history_count() == doBase + 1;
        ST_BEGIN("lpr_drop_out");
        ST_PASS("lpr_drop_out can=%d moved=%d path=%s history=%d", doCan ? 1 : 0,
                doMoved ? 1 : 0, qPrintable(doWhere), doView->history_count() - doBase);
        if (!doOk) {
            return pictura::selfTest().fail(239, "drop out of group");
        }
        frame.closeDocument(doDoc, false);

        // lpr_drop_rules (240): the validator refuses a drop into a non-group,
        // onto a descendant, and onto self; a refused release changes nothing
        // and adds no history state.
        const bool drCreated = frame.newDocument(QStringLiteral("DropRulesCtl"), 8, 8,
                                                 QStringLiteral("rgb"), 8,
                                                 QStringLiteral("white"));
        pictura::PictureView* drView = frame.activeView();
        if (!drCreated || !drView) {
            return pictura::selfTest().fail(240, "drop rules fixture");
        }
        const int drDoc = frame.activeDocumentIndex();
        lpcPanel->setView(drView);
        const QString drGroup = drView->add_group_in(QString());
        const QString drChild = drView->add_layer_in(drGroup);
        const QString drTop = drView->add_layer_in(QString());
        const int drRowCount = pictura::layers_api::rowCount(*drView);
        lpcPanel->refresh();
        const int drBase = drView->history_count();
        const bool drDescend = !lpcPanel->canMoveForTest(drGroup, drChild, 0);
        const bool drIntoPlain = !lpcPanel->canMoveForTest(drTop, drChild, 2);
        const bool drSelf = !lpcPanel->canMoveForTest(drChild, drChild, 0);
        const bool drRefused = !lpcPanel->moveForTest(drTop, drChild, 2);
        const bool drOk = drDescend && drIntoPlain && drSelf && drRefused
            && drView->history_count() == drBase
            && pictura::layers_api::rowCount(*drView) == drRowCount;
        ST_BEGIN("lpr_drop_rules");
        ST_PASS("lpr_drop_rules descend=%d intoplain=%d self=%d history=%d",
                drDescend ? 1 : 0, drIntoPlain ? 1 : 0, drSelf ? 1 : 0,
                drView->history_count() - drBase);
        if (!drOk) {
            return pictura::selfTest().fail(240, "drop rules refusal");
        }
        frame.closeDocument(drDoc, false);

        // lpr_group_from_layers (241): the bridge wraps two pixel layers in one
        // group carrying the dialog's name/color/blend/opacity, in one undo step.
        const bool gflCreated = frame.newDocument(QStringLiteral("GroupFromLayersCtl"), 4, 4,
                                                  QStringLiteral("rgb"), 8,
                                                  QStringLiteral("white"));
        pictura::PictureView* gflView = frame.activeView();
        if (!gflCreated || !gflView) {
            return pictura::selfTest().fail(241, "group from layers fixture");
        }
        const int gflDoc = frame.activeDocumentIndex();
        const QString gflA = gflView->add_layer_in(QString());
        const QString gflB = gflView->add_layer_in(QString());
        gflView->set_layer_name_path(gflA, QStringLiteral("A"));
        gflView->set_layer_name_path(gflB, QStringLiteral("B"));
        const int gflBase = gflView->history_count();
        const QString gflGroup = gflView->group_from_layers_dialog(
            QStringList{gflA, gflB}, QStringLiteral("Grp"), 2, QStringLiteral("scrn"), 180);
        int gflRow = -1;
        for (int i = 0; i < pictura::layers_api::rowCount(*gflView); ++i) {
            if (pictura::layers_api::rowAt(*gflView, i).path == gflGroup) {
                gflRow = i;
            }
        }
        const bool gflOk = !gflGroup.isEmpty()
            && pictura::layers_api::rowAt(*gflView, gflRow).kind == QStringLiteral("group")
            && pictura::layers_api::rowAt(*gflView, gflRow).name == QStringLiteral("Grp")
            && pictura::layers_api::rowAt(*gflView, gflRow).color == 2
            && pictura::layers_api::rowAt(*gflView, gflRow).blend == QStringLiteral("scrn")
            && pictura::layers_api::rowAt(*gflView, gflRow).opacity == 180
            && gflView->history_count() == gflBase + 1;
        ST_BEGIN("lpr_group_from_layers");
        ST_PASS("lpr_group_from_layers path=%s name=%s mode=%s opacity=%d history=%d",
                qPrintable(gflGroup), qPrintable(pictura::layers_api::rowAt(*gflView, gflRow).name),
                qPrintable(pictura::layers_api::rowAt(*gflView, gflRow).blend), pictura::layers_api::rowAt(*gflView, gflRow).opacity,
                gflView->history_count() - gflBase);
        if (!gflOk) {
            return pictura::selfTest().fail(241, "group from layers");
        }
        frame.closeDocument(gflDoc, false);

        if (const int lso = pictura::runLayersPlaceSmartObjectChecks(frame); lso != 0) { return lso; }

        if (const int lso = pictura::runLayersSmartObjectReplaceChecks(frame); lso != 0) { return lso; }

        if (const int lso = pictura::runLayersOpenSmartObjectChecks(frame); lso != 0) { return lso; }

        if (const int lso = pictura::runLayersExportSmartObjectChecks(frame); lso != 0) { return lso; }

        if (const int lso = pictura::runLayersEditSmartObjectChecks(frame); lso != 0) { return lso; }

        if (const int lso = pictura::runLayersEditSmartObjectSessionChecks(frame); lso != 0) { return lso; }

        if (const int lso = pictura::runImageImportChecks(frame); lso != 0) { return lso; }

        if (const int ft = pictura::runFreeTransformChecks(frame); ft != 0) { return ft; }

        if (const int wp = pictura::runWarpPresetChecks(frame); wp != 0) { return wp; }

        if (const int ports = pictura::runPortChecks(frame); ports != 0) { return ports; }

        if (const int sts = pictura::runToolsSelectionChecks(frame); sts != 0) { return sts; }

        if (const int la = pictura::runLayersAdjustmentChecks(frame); la != 0) { return la; }

        // lpr_gradient_fill (286): adding a gradient fill layer succeeds, the
        // layer is fill content, and the composite becomes a black-to-white ramp.
        {
            const bool gfCreated = frame.newDocument(QStringLiteral("GradientFillCtl"), 4, 4,
                                                     QStringLiteral("rgb"), 8,
                                                     QStringLiteral("white"));
            pictura::PictureView* gfView = frame.activeView();
            if (!gfCreated || !gfView) {
                return pictura::selfTest().fail(286, "gradient fill fixture");
            }
            const int gfDoc = frame.activeDocumentIndex();
            const unsigned int gfBefore = gfView->sample_argb(0, 0);
            const int gfBase = gfView->history_count();
            const QString gfPath = gfView->add_gradient_fill();
            const unsigned int gfLeft = gfView->sample_argb(0, 0);
            const unsigned int gfRight = gfView->sample_argb(3, 0);
            const bool gfOk = gfPath == QStringLiteral("1")
                && gfView->layer_is_fill_content(gfPath)
                && ((gfLeft >> 16) & 0xffu) < 64u
                && ((gfRight >> 16) & 0xffu) > 192u
                && gfLeft != gfBefore
                && gfView->history_count() == gfBase + 1;
            ST_BEGIN("lpr_gradient_fill");
            ST_PASS("lpr_gradient_fill path=%s fill=%d left=%08x right=%08x history=%d",
                    qPrintable(gfPath), gfView->layer_is_fill_content(gfPath) ? 1 : 0, gfLeft,
                    gfRight, gfView->history_count() - gfBase);
            if (!gfOk) {
                return pictura::selfTest().fail(286, "gradient fill layer");
            }
            frame.closeDocument(gfDoc, false);
        }

        // lpr_gradient_fill_menu (287): the Layers-panel strip fill/adjustment
        // menu offers an enabled `Gradient…` entry wired to the gradient fill.
        {
            auto* gfButton =
                frame.findChild<QToolButton*>(QStringLiteral("layersStripFillAdjustment"));
            bool gfEntry = false;
            if (gfButton && gfButton->menu()) {
                for (QAction* action : gfButton->menu()->actions()) {
                    if (action->text() == QStringLiteral("Gradient…")) {
                        gfEntry = action->isEnabled();
                    }
                }
            }
            ST_BEGIN("lpr_gradient_fill_menu");
            ST_PASS("lpr_gradient_fill_menu entry=%d", gfEntry ? 1 : 0);
            if (!gfEntry) {
                return pictura::selfTest().fail(287, "gradient fill menu");
            }
        }

        if (const int fd = pictura::runFileDropChecks(frame); fd != 0) { return fd; }

        if (const int ld = pictura::runLayersDragChecks(frame); ld != 0) { return ld; }

        if (const int li = pictura::runLayersInteractionsChecks(frame); li != 0) { return li; }

        if (const int nf = pictura::runNumericFieldChecks(frame); nf != 0) { return nf; }

        if (const int cv = pictura::runCanvasViewChecks(frame); cv != 0) { return cv; }

        if (const int ll = pictura::runLayerLocksChecks(frame); ll != 0) { return ll; }

        if (const int al = pictura::runActiveLayerChecks(frame); al != 0) { return al; }

        if (const int ss = pictura::runSessionChecks(frame); ss != 0) { return ss; }

        if (const int up = pictura::runUiPersistenceChecks(frame); up != 0) { return up; }

        if (const int tc = pictura::runToolCanvasChecks(frame); tc != 0) { return tc; }

        if (const int pp = pictura::runPaintPerfChecks(frame); pp != 0) { return pp; }

        if (const int pl = pictura::runPaintLiveChecks(frame); pl != 0) { return pl; }

        if (const int ld = pictura::runLargeDocumentChecks(frame); ld != 0) { return ld; }

        if (const int sr = pictura::runShellRound3Checks(frame); sr != 0) { return sr; }

        if (const int sr4 = pictura::runShellRound4Checks(frame); sr4 != 0) { return sr4; }

        if (const int lr3 = pictura::runLayersRound3Checks(frame); lr3 != 0) { return lr3; }

        if (const int vv = pictura::runVisibilityChecks(frame); vv != 0) { return vv; }

        if (const int wi = pictura::runWorkspaceInputChecks(frame); wi != 0) { return wi; }

        // lpr_qt_text (525): the Qt font helper rasterizes text into a
        // layer-sized RGBA8888 buffer with some non-zero alpha coverage.
        {
            const ::rust::Vec<uint8_t> qt =
                render_text_rgba(::rust::String("DejaVu Sans"), 48.0, ::rust::String("Hi"), 0,
                                 0, 0, 0, 255, 200, 80);
            bool covered = false;
            for (std::size_t i = 3; i < qt.size(); i += 4) {
                if (qt[i] != 0) {
                    covered = true;
                    break;
                }
            }
            const bool qtOk =
                qt.size() == static_cast<std::size_t>(200 * 80 * 4) && covered;
            ST_BEGIN("lpr_qt_text");
            ST_PASS("lpr_qt_text bytes=%d covered=%d", static_cast<int>(qt.size()),
                    covered ? 1 : 0);
            if (!qtOk) {
                return pictura::selfTest().fail(525, "qt text helper");
            }
        }

    return 0;
}
