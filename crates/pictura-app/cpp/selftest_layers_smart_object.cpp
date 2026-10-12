#include "selftest_layers_smart_object.h"
#include "selftest_report.h"
#include "api/layers_api.h"

#include "api/transform_api.h"
#include "frame.h"
#include "image_view.h"
#include "panels/layers_panel.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_transform/bridge.cxxqt.h"

#include <cmath>

#include <QtCore/QCoreApplication>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QMimeData>
#include <QtCore/QRectF>
#include <QtCore/QUrl>
#include <QtGui/QColor>
#include <QtGui/QDragEnterEvent>
#include <QtGui/QDropEvent>
#include <QtGui/QImage>
#include <QtGui/QKeyEvent>
#include <QtWidgets/QMenuBar>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QTabWidget>
#include <QtWidgets/QToolBar>

int pictura::runLayersSmartObjectConvertChecks(pictura::PicturaMainWindow& frame)
{
    // lpr_smart_object_convert (277): converting a raster pixel layer
    // authors an embedded smart object while keeping its raster proxy, so
    // the composite is unchanged and the object survives save→load; a group
    // and the Background refuse without history.
    const bool socCreated = frame.newDocument(QStringLiteral("SmartObjectCtl"), 4, 4,
                                              QStringLiteral("rgb"), 8,
                                              QStringLiteral("white"));
    pictura::PictureView* socView = frame.activeView();
    if (!socCreated || !socView) {
        return pictura::selfTest().fail(277, "smart object fixture");
    }
    const int socDoc = frame.activeDocumentIndex();
    const QString socPath = socView->add_solid_fill(0xff2244aau);
    const bool socRaster = socView->rasterize_fill_content(socPath);
    const bool socCan = socView->layer_can_convert_to_smart_object(socPath);
    const int socBase = socView->history_count();
    const unsigned int socBefore = socView->sample_argb(0, 0);
    const bool socConverted = socView->convert_to_smart_object(socPath);
    const QString socState = socView->layer_smart_object_state(socPath);
    const bool socProxy = socView->sample_argb(0, 0) == socBefore;
    const bool socRecorded = socView->history_count() == socBase + 1;
    const bool socEmbedded = socState.startsWith(QStringLiteral("embedded:"))
        && socState.mid(9).toInt() > 0
        && !socView->layer_can_convert_to_smart_object(socPath);
    const QString socGroup = socView->add_group_in(QString());
    socView->background_from_layer(QStringLiteral("0"));
    const int socRefBase = socView->history_count();
    const bool socRefGroup = !socView->convert_to_smart_object(socGroup);
    const bool socRefBackground = !socView->convert_to_smart_object(QStringLiteral("0"));
    const bool socRefHistory = socView->history_count() == socRefBase;
    const QString socSavePath =
        QDir::tempPath() + QStringLiteral("/kooka-pictura-smart-object.psd");
    const bool socSaved = frame.saveActiveAs(socSavePath);
    const bool socReopened = frame.openPath(socSavePath);
    pictura::PictureView* socReload = frame.activeView();
    const int socReloadDoc = frame.activeDocumentIndex();
    const QString socReloadState =
        socReload ? socReload->layer_smart_object_state(socPath) : QString();
    const bool socRoundTrip = socReopened && socReload
        && socReloadState.startsWith(QStringLiteral("embedded:"))
        && socReloadState.mid(9).toInt() > 0;
    const bool socOk = socRaster && socCan && socConverted && socProxy && socRecorded
        && socEmbedded && socRefGroup && socRefBackground && socRefHistory && socSaved
        && socRoundTrip;
    ST_BEGIN("lpr_smart_object_convert");
    ST_PASS("lpr_smart_object_convert raster=%d can=%d convert=%d state=%s proxy=%d "
            "history=%d group=%d bg=%d saved=%d reload=%s",
            socRaster ? 1 : 0, socCan ? 1 : 0, socConverted ? 1 : 0, qPrintable(socState),
            socProxy ? 1 : 0, socRecorded ? 1 : 0, socRefGroup ? 1 : 0,
            socRefBackground ? 1 : 0, socSaved ? 1 : 0, qPrintable(socReloadState));
    if (!socOk) {
        return pictura::selfTest().fail(277, "smart object convert");
    }
    frame.closeDocument(socReloadDoc, false);
    frame.closeDocument(socDoc, false);
    return 0;
}

int pictura::runLayersSmartObjectRasterizeChecks(pictura::PicturaMainWindow& frame)
{
    // lpr_smart_object_rasterize (278): converting then rasterizing a raster
    // pixel layer restores a plain pixel layer with the same composite, no
    // smart object on save→load, and refuses a non-smart layer without a
    // history state.
    const bool rosCreated = frame.newDocument(QStringLiteral("SmartObjectRasterCtl"), 4, 4,
                                              QStringLiteral("rgb"), 8,
                                              QStringLiteral("white"));
    pictura::PictureView* rosView = frame.activeView();
    if (!rosCreated || !rosView) {
        return pictura::selfTest().fail(278, "smart object rasterize fixture");
    }
    const int rosDoc = frame.activeDocumentIndex();
    const QString rosPath = rosView->add_solid_fill(0xff2244aau);
    const bool rosRaster = rosView->rasterize_fill_content(rosPath);
    const unsigned int rosBefore = rosView->sample_argb(0, 0);
    const bool rosConverted = rosView->convert_to_smart_object(rosPath);
    const unsigned int rosAfterConvert = rosView->sample_argb(0, 0);
    const bool rosCanRaster = rosView->layer_can_rasterize_smart_object(rosPath);
    const int rosBase = rosView->history_count();
    const bool rosRasterized = rosView->rasterize_smart_object(rosPath);
    const QString rosState = rosView->layer_smart_object_state(rosPath);
    const unsigned int rosAfterRaster = rosView->sample_argb(0, 0);
    const bool rosRecorded = rosView->history_count() == rosBase + 1;
    const bool rosPixel = rosView->layer_kind(rosPath.toInt()) == QStringLiteral("pixel");
    const int rosBase2 = rosView->history_count();
    const bool rosRefuse = !rosView->rasterize_smart_object(rosPath);
    const bool rosRefuseHistory = rosView->history_count() == rosBase2;
    const QString rosSavePath =
        QDir::tempPath() + QStringLiteral("/kooka-pictura-smart-object-rasterize.psd");
    const bool rosSaved = frame.saveActiveAs(rosSavePath);
    const bool rosReopened = frame.openPath(rosSavePath);
    pictura::PictureView* rosReload = frame.activeView();
    const int rosReloadDoc = frame.activeDocumentIndex();
    const QString rosReloadState =
        rosReload ? rosReload->layer_smart_object_state(rosPath) : QString();
    const bool rosRoundTrip = rosReopened && rosReload && rosReloadState.isEmpty();
    const bool rosOk = rosRaster && rosConverted && rosCanRaster && rosRasterized
        && rosBefore == rosAfterConvert && rosBefore == rosAfterRaster && rosState.isEmpty()
        && rosRecorded && rosPixel && rosRefuse && rosRefuseHistory && rosSaved && rosRoundTrip;
    ST_BEGIN("lpr_smart_object_rasterize");
    ST_PASS("lpr_smart_object_rasterize raster=%d convert=%d can=%d rasterize=%d proxy=%d "
            "state=%s history=%d pixel=%d refuse=%d saved=%d reload=%s",
            rosRaster ? 1 : 0, rosConverted ? 1 : 0, rosCanRaster ? 1 : 0,
            rosRasterized ? 1 : 0, rosBefore == rosAfterRaster ? 1 : 0, qPrintable(rosState),
            rosRecorded ? 1 : 0, rosPixel ? 1 : 0, rosRefuse ? 1 : 0, rosSaved ? 1 : 0,
            qPrintable(rosReloadState));
    if (!rosOk) {
        return pictura::selfTest().fail(278, "smart object rasterize");
    }
    frame.closeDocument(rosReloadDoc, false);
    frame.closeDocument(rosDoc, false);
    return 0;
}

int pictura::runLayersPlaceSmartObjectChecks(pictura::PicturaMainWindow& frame)
{
    // lpr_place_smart_object (279): placing a PSD file appends a channel-less
    // embedded smart-object layer that renders the source, in one undo state;
    // a malformed file refuses without changing the document.
    const bool plSrcCreated = frame.newDocument(QStringLiteral("PlaceSource"), 4, 4,
                                                QStringLiteral("rgb"), 8,
                                                QStringLiteral("white"));
    pictura::PictureView* plSrcView = frame.activeView();
    if (!plSrcCreated || !plSrcView) {
        return pictura::selfTest().fail(279, "place source fixture");
    }
    const int plSrcDoc = frame.activeDocumentIndex();
    const QString plSourcePath =
        QDir::tempPath() + QStringLiteral("/kooka-pictura-place-source.psd");
    const bool plSourceSaved = frame.saveActiveAs(plSourcePath);
    frame.closeDocument(plSrcDoc, false);

    const bool plCreated = frame.newDocument(QStringLiteral("PlaceCtl"), 4, 4,
                                             QStringLiteral("rgb"), 8,
                                             QStringLiteral("transparent"));
    pictura::PictureView* plView = frame.activeView();
    if (!plCreated || !plView) {
        return pictura::selfTest().fail(279, "place fixture");
    }
    const int plDoc = frame.activeDocumentIndex();
    const int plBase = plView->history_count();
    const int plRows = pictura::layers_api::rowCount(*plView);
    const unsigned int plBefore = plView->sample_argb(0, 0);
    const QString plPath = plView->place_smart_object(plSourcePath);
    const bool plPlaced = !plPath.isEmpty() && pictura::layers_api::rowCount(*plView) == plRows + 1
        && plView->history_count() == plBase + 1
        && plView->sample_argb(0, 0) == 0xffffffffu
        && plView->sample_argb(0, 0) != plBefore
        && plView->layer_smart_object_state(plPath).startsWith(QStringLiteral("embedded:"));
    const QString plBadPath =
        QDir::tempPath() + QStringLiteral("/kooka-pictura-place-bad.psd");
    QFile plBad(plBadPath);
    if (plBad.open(QIODevice::WriteOnly)) {
        plBad.write("not a psd");
    }
    plBad.close();
    const int plBadBase = plView->history_count();
    const QString plRefused = plView->place_smart_object(plBadPath);
    const bool plRefusedOk = plRefused.isEmpty()
        && plView->history_count() == plBadBase
        && pictura::layers_api::rowCount(*plView) == plRows + 1;
    ST_BEGIN("lpr_place_smart_object");
    ST_PASS("lpr_place_smart_object saved=%d path=%s pixel=%08x before=%08x refuse=%d",
            plSourceSaved ? 1 : 0, qPrintable(plPath), plView->sample_argb(0, 0), plBefore,
            plRefusedOk ? 1 : 0);
    if (!plSourceSaved || !plPlaced || !plRefusedOk) {
        return pictura::selfTest().fail(279, "place smart object");
    }
    QFile::remove(plSourcePath);
    QFile::remove(plBadPath);
    frame.closeDocument(plDoc, false);
    return 0;
}

int pictura::runLayersSmartObjectReplaceChecks(pictura::PicturaMainWindow& frame)
{
    // lpr_smart_object_replace (280): replacing a smart object's embedded
    // contents swaps the composite to the new source in one "Replace Contents"
    // history state; a malformed file refuses without adding a state.
    frame.newDocument(QStringLiteral("ReplaceSource"), 2, 2, QStringLiteral("rgb"), 8, QStringLiteral("white"));
    pictura::PictureView* rpSrcView = frame.activeView();
    if (!rpSrcView) { return pictura::selfTest().fail(280, "replace source fixture"); }
    const QString rpSource = QDir::tempPath() + QStringLiteral("/kooka-pictura-replace-source.psd");
    rpSrcView->rasterize_fill_content(rpSrcView->add_solid_fill(0xff22cc44u));
    const bool rpSaved = frame.saveActiveAs(rpSource);
    frame.closeDocument(frame.activeDocumentIndex(), false);
    frame.newDocument(QStringLiteral("SmartObjectReplaceCtl"), 4, 4, QStringLiteral("rgb"), 8, QStringLiteral("white"));
    pictura::PictureView* rpView = frame.activeView();
    if (!rpView) { return pictura::selfTest().fail(280, "replace fixture"); }
    const int rpDoc = frame.activeDocumentIndex();
    const QString rpPath = rpView->add_solid_fill(0xff2244aau);
    rpView->rasterize_fill_content(rpPath);
    rpView->convert_to_smart_object(rpPath);
    const bool rpCan = rpView->layer_can_replace_smart_object_contents(rpPath);
    const unsigned int rpBefore = rpView->sample_argb(0, 0);
    const int rpBase = rpView->history_count();
    const bool rpReplaced = rpView->replace_smart_object_contents(rpPath, rpSource);
    const bool rpChanged = rpView->sample_argb(0, 0) == 0xff22cc44u && rpView->sample_argb(0, 0) != rpBefore;
    const QString rpState = rpView->layer_smart_object_state(rpPath);
    const bool rpSmart = rpState.startsWith(QStringLiteral("embedded:")) && rpState.mid(9).toInt() > 0;
    const bool rpLabelOk = rpView->history_label(rpBase) == QStringLiteral("Replace Contents");
    const QString rpBad = QDir::tempPath() + QStringLiteral("/kooka-pictura-replace-bad.psd");
    QFile rpBadFile(rpBad); if (rpBadFile.open(QIODevice::WriteOnly)) { rpBadFile.write("not a psd"); } rpBadFile.close();
    const int rpBadBase = rpView->history_count();
    const bool rpRefused = !rpView->replace_smart_object_contents(rpPath, rpBad) && rpView->history_count() == rpBadBase;
    const bool rpOk = rpSaved && rpCan && rpReplaced && rpChanged && rpView->history_count() == rpBase + 1 && rpSmart && rpLabelOk && rpRefused;
    ST_BEGIN("lpr_smart_object_replace");
    ST_PASS("lpr_smart_object_replace can=%d replace=%d pixel=%08x before=%08x state=%s history=%d smart=%d refuse=%d",
            rpCan ? 1 : 0, rpReplaced ? 1 : 0, rpView->sample_argb(0, 0), rpBefore, qPrintable(rpState),
            rpView->history_count() - rpBase, rpSmart ? 1 : 0, rpRefused ? 1 : 0);
    if (!rpOk) { return pictura::selfTest().fail(280, "replace smart object"); }
    QFile::remove(rpSource);
    QFile::remove(rpBad);
    frame.closeDocument(rpDoc, false);
    return 0;
}

int pictura::runLayersOpenSmartObjectChecks(pictura::PicturaMainWindow& frame)
{
    // lpr_open_as_smart_object (281): opening a written PSD as a smart object
    // adds one untitled tab holding exactly one embedded smart-object layer
    // whose composite is the source colour, in one "Open As Smart Object"
    // history state; a malformed file refuses and adds no tab.
    const bool ossSourceCreated = frame.newDocument(QStringLiteral("OpenSmartSource"), 4, 4,
                                                    QStringLiteral("rgb"), 8,
                                                    QStringLiteral("white"));
    pictura::PictureView* ossSrcView = frame.activeView();
    if (!ossSourceCreated || !ossSrcView) {
        return pictura::selfTest().fail(281, "open smart object source fixture");
    }
    ossSrcView->rasterize_fill_content(ossSrcView->add_solid_fill(0xff2244aau));
    const QString ossSourcePath =
        QDir::tempPath() + QStringLiteral("/kooka-pictura-open-smart-source.psd");
    const bool ossSourceSaved = frame.saveActiveAs(ossSourcePath);
    frame.closeDocument(frame.activeDocumentIndex(), false);

    const int ossBase = frame.documentCount();
    const bool ossOpened = frame.openAsSmartObjectPath(ossSourcePath);
    pictura::PictureView* ossView = frame.activeView();
    const bool ossAdded = ossOpened && frame.documentCount() == ossBase + 1 && ossView;
    const bool ossUntitled = ossView && ossView->file_path().isEmpty();
    const bool ossOneLayer = ossView && pictura::layers_api::rowCount(*ossView) == 1;
    const QString ossState =
        ossView ? ossView->layer_smart_object_state(QStringLiteral("0")) : QString();
    const bool ossSmart = ossState.startsWith(QStringLiteral("embedded:"))
        && ossState.mid(9).toInt() > 0;
    const bool ossPixel = ossView && ossView->sample_argb(0, 0) == 0xff2244aau;
    const bool ossHistory = ossView && ossView->history_count() == 1
        && ossView->history_label(0) == QStringLiteral("Open As Smart Object");

    const QString ossBadPath =
        QDir::tempPath() + QStringLiteral("/kooka-pictura-open-smart-bad.psd");
    QFile ossBad(ossBadPath);
    if (ossBad.open(QIODevice::WriteOnly)) {
        ossBad.write("not a psd");
    }
    ossBad.close();
    const int ossBadBase = frame.documentCount();
    const bool ossRefused = !frame.openAsSmartObjectPath(ossBadPath);
    const bool ossNoTab = frame.documentCount() == ossBadBase;

    const bool ossOk = ossSourceSaved && ossAdded && ossUntitled && ossOneLayer && ossSmart
        && ossPixel && ossHistory && ossRefused && ossNoTab;
    ST_BEGIN("lpr_open_as_smart_object");
    ST_PASS("lpr_open_as_smart_object opened=%d added=%d untitled=%d layers=%d state=%s "
            "pixel=%08x history=%d refused=%d notab=%d",
            ossOpened ? 1 : 0, ossAdded ? 1 : 0, ossUntitled ? 1 : 0,
            ossView ? pictura::layers_api::rowCount(*ossView) : -1, qPrintable(ossState),
            ossView ? ossView->sample_argb(0, 0) : 0, ossView ? ossView->history_count() : -1,
            ossRefused ? 1 : 0, ossNoTab ? 1 : 0);
    if (!ossOk) {
        return pictura::selfTest().fail(281, "open as smart object");
    }
    QFile::remove(ossSourcePath);
    QFile::remove(ossBadPath);
    frame.closeDocument(frame.activeDocumentIndex(), false);
    return 0;
}

int pictura::runLayersExportSmartObjectChecks(pictura::PicturaMainWindow& frame)
{
    // lpr_export_smart_object_contents (282): exporting a smart object's
    // embedded source writes the payload bytes to the chosen file and adds no
    // history state; a non-smart layer refuses and writes nothing.
    const bool expCreated = frame.newDocument(QStringLiteral("ExportSmartCtl"), 4, 4,
                                              QStringLiteral("rgb"), 8,
                                              QStringLiteral("white"));
    pictura::PictureView* expView = frame.activeView();
    if (!expCreated || !expView) {
        return pictura::selfTest().fail(282, "export smart object fixture");
    }
    const int expDoc = frame.activeDocumentIndex();
    const QString expPath = expView->add_solid_fill(0xff2244aau);
    expView->rasterize_fill_content(expPath);
    const bool expConverted = expView->convert_to_smart_object(expPath);
    const bool expCan = expView->layer_can_export_smart_object_contents(expPath);
    const int expBase = expView->history_count();
    const QString expDest =
        QDir::tempPath() + QStringLiteral("/kooka-pictura-smart-object-export.psd");
    const bool expExported = expView->export_smart_object_contents(expPath, expDest);
    const int expHistoryDelta = expView->history_count() - expBase;
    QFile expFile(expDest);
    bool expRead = expFile.open(QIODevice::ReadOnly);
    const QByteArray expBytes = expRead ? expFile.readAll() : QByteArray();
    expFile.close();
    const bool expPsd = expBytes.size() > 4 && expBytes.startsWith("8BPS");
    const bool expHistory = expHistoryDelta == 0;

    const QString expPlain = expView->add_solid_fill(0xff22cc44u);
    const bool expPlainCan = !expView->layer_can_export_smart_object_contents(expPlain);
    const int expPlainBase = expView->history_count();
    const QString expPlainDest =
        QDir::tempPath() + QStringLiteral("/kooka-pictura-smart-object-export-plain.psd");
    const bool expRefused = !expView->export_smart_object_contents(expPlain, expPlainDest)
        && !QFile::exists(expPlainDest) && expView->history_count() == expPlainBase;

    const bool expOk = expConverted && expCan && expExported && expPsd && expHistory
        && expPlainCan && expRefused;
    ST_BEGIN("lpr_export_smart_object_contents");
    ST_PASS("lpr_export_smart_object_contents convert=%d can=%d export=%d psd=%d bytes=%d "
            "history=%d plain=%d refuse=%d",
            expConverted ? 1 : 0, expCan ? 1 : 0, expExported ? 1 : 0, expPsd ? 1 : 0,
            expBytes.size(), expHistoryDelta, expPlainCan ? 1 : 0,
            expRefused ? 1 : 0);
    if (!expOk) {
        return pictura::selfTest().fail(282, "export smart object contents");
    }
    QFile::remove(expDest);
    QFile::remove(expPlainDest);
    frame.closeDocument(expDoc, false);
    return 0;
}

int pictura::runLayersEditSmartObjectChecks(pictura::PicturaMainWindow& frame)
{
    // lpr_edit_smart_object_contents (288): Edit Contents opens an editable
    // smart object's source as a new untitled editor tab without touching the
    // origin; saving that tab commits the edited source in exactly one
    // "Edit Contents" history state and leaves the editor open and clean;
    // discarding a second editor leaves the origin unchanged; a non-editable
    // target refuses and adds no tab.
    const bool editCreated = frame.newDocument(QStringLiteral("EditSmartCtl"), 4, 4,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
    pictura::PictureView* editView = frame.activeView();
    if (!editCreated || !editView) {
        return pictura::selfTest().fail(288, "edit smart object fixture");
    }
    const int editOriginDoc = frame.activeDocumentIndex();
    const QString editPath = editView->add_solid_fill(0xff2244aau);
    editView->rasterize_fill_content(editPath);
    const bool editConverted = editView->convert_to_smart_object(editPath);
    const bool editCan = editView->layer_can_edit_smart_object_contents(editPath);
    const int editOriginHistory = editView->history_count();
    const unsigned int editOriginPixel = editView->sample_argb(0, 0);
    const int editDocsBase = frame.documentCount();

    const bool editOpened = frame.editSmartObjectContents(editPath);
    pictura::PictureView* editEditor = frame.activeView();
    const bool editAdded = editOpened && frame.documentCount() == editDocsBase + 1
        && editEditor && editEditor != editView;
    const bool editUntitled = editEditor && frame.documentPath(editDocsBase).isEmpty();
    const bool editDecoded = editEditor && editEditor->has_document()
        && pictura::layers_api::rowCount(*editEditor) == 1
        && editEditor->layer_smart_object_state(QStringLiteral("0")).isEmpty();
    const bool editOriginUntouched = editView->history_count() == editOriginHistory
        && editView->sample_argb(0, 0) == editOriginPixel;

    const QString editFill = editEditor ? editEditor->add_solid_fill(0xff22cc44u) : QString();
    const bool editModified = editEditor && !editFill.isEmpty()
        && editEditor->rasterize_fill_content(editFill);
    const bool editSaved = frame.saveActive();
    const unsigned int editCommitPixel = editView->sample_argb(0, 0);
    const bool editCommitted = editCommitPixel == 0xff22cc44u;
    const bool editOneState = editView->history_count() == editOriginHistory + 1;
    const bool editLabel = editView->history_label(editOriginHistory)
        == QStringLiteral("Edit Contents");
    const bool editClean = editEditor && !editEditor->is_dirty();
    const bool editStillOpen = frame.documentCount() == editDocsBase + 1;

    frame.setActiveDocumentIndex(editOriginDoc);
    const int editHistoryAfterSave = editView->history_count();
    const unsigned int editPixelAfterSave = editView->sample_argb(0, 0);
    const bool editOpened2 = frame.editSmartObjectContents(editPath);
    pictura::PictureView* editEditor2 = frame.activeView();
    if (editOpened2 && editEditor2) {
        const QString editFill2 = editEditor2->add_solid_fill(0xff000000u);
        editEditor2->rasterize_fill_content(editFill2);
    }
    const bool editDiscarded = frame.closeDocument(frame.activeDocumentIndex(), false);
    const bool editDiscardNoop = editView->history_count() == editHistoryAfterSave
        && editView->sample_argb(0, 0) == editPixelAfterSave;

    frame.setActiveDocumentIndex(editOriginDoc);
    const QString editPlain = editView->add_solid_fill(0xffffffffu);
    const bool editPlainCan = !editView->layer_can_edit_smart_object_contents(editPlain);
    const int editRefuseDocs = frame.documentCount();
    const bool editRefused = !frame.editSmartObjectContents(editPlain);
    const bool editNoTab = frame.documentCount() == editRefuseDocs;

    const bool editOk = editConverted && editCan && editOpened && editAdded && editUntitled
        && editDecoded && editOriginUntouched && editModified && editSaved && editCommitted
        && editOneState && editLabel && editClean && editStillOpen && editDiscarded
        && editDiscardNoop && editPlainCan && editRefused && editNoTab;
    ST_BEGIN("lpr_edit_smart_object_contents");
    ST_PASS("lpr_edit_smart_object_contents can=%d open=%d added=%d untitled=%d decoded=%d "
            "origin=%d saved=%d pixel=%08x state=%d label=%s clean=%d discard=%d plain=%d "
            "refuse=%d notab=%d",
            editCan ? 1 : 0, editOpened ? 1 : 0, editAdded ? 1 : 0, editUntitled ? 1 : 0,
            editDecoded ? 1 : 0, editOriginUntouched ? 1 : 0, editSaved ? 1 : 0,
            editCommitPixel, editOneState ? 1 : 0,
            qPrintable(editView->history_label(editOriginHistory)), editClean ? 1 : 0,
            editDiscardNoop ? 1 : 0, editPlainCan ? 1 : 0, editRefused ? 1 : 0,
            editNoTab ? 1 : 0);
    if (!editOk) {
        return pictura::selfTest().fail(288, "edit smart object contents");
    }
    frame.closeDocument(editDocsBase, false);
    frame.closeDocument(editOriginDoc, false);
    return 0;
}

int pictura::runLayersEditSmartObjectSessionChecks(pictura::PicturaMainWindow& frame)
{
    // lpr_edit_smart_object_session (289): closing an Edit Contents editor
    // removes its session and temporary file; closing the origin first drops the
    // session temp and leaves the orphaned editor open as an untitled tab.
    const bool sesCreated = frame.newDocument(QStringLiteral("EditSessionCtl"), 4, 4,
                                              QStringLiteral("rgb"), 8,
                                              QStringLiteral("white"));
    pictura::PictureView* sesOrigin = frame.activeView();
    if (!sesCreated || !sesOrigin) {
        return pictura::selfTest().fail(289, "edit session fixture");
    }
    const int sesOriginDoc = frame.activeDocumentIndex();
    const QString sesPath = sesOrigin->add_solid_fill(0xff2244aau);
    sesOrigin->rasterize_fill_content(sesPath);
    const bool sesConverted = sesOrigin->convert_to_smart_object(sesPath);

    // Closing the editor drops its session and temp file.
    const int sesDocs = frame.documentCount();
    const bool sesOpened = frame.editSmartObjectContents(sesPath);
    pictura::PictureView* sesEditor = frame.activeView();
    const int sesEditorDoc = frame.activeDocumentIndex();
    const QString sesTemp = sesEditor ? sesEditor->file_path() : QString();
    const bool sesTempLive = !sesTemp.isEmpty() && QFile::exists(sesTemp);
    const bool sesClosed = frame.closeDocument(sesEditorDoc, false);
    const bool sesTempGone = !sesTemp.isEmpty() && !QFile::exists(sesTemp);
    const bool sesDocsBack = frame.documentCount() == sesDocs;

    // Closing the origin drops the session temp and orphans the editor as an
    // untitled tab with an empty document path.
    const bool sesOpened2 = frame.editSmartObjectContents(sesPath);
    pictura::PictureView* sesOrphan = frame.activeView();
    const QString sesOrphanTemp = sesOrphan ? sesOrphan->file_path() : QString();
    const bool sesOrphanUntitled =
        sesOrphan && frame.documentPath(frame.activeDocumentIndex()).isEmpty();
    const bool sesOrphanTempLive = !sesOrphanTemp.isEmpty() && QFile::exists(sesOrphanTemp);
    frame.setActiveDocumentIndex(sesOriginDoc);
    const bool sesOriginClosed = frame.closeDocument(sesOriginDoc, false);
    const int sesOrphanDoc = sesDocs - 1;
    const bool sesOrphanKept = frame.documentCount() == sesDocs
        && frame.viewAt(sesOrphanDoc) == sesOrphan
        && frame.documentPath(sesOrphanDoc).isEmpty();
    const bool sesOrphanTempGone = !sesOrphanTemp.isEmpty() && !QFile::exists(sesOrphanTemp);
    const bool sesOrphanClosed = frame.closeDocument(sesOrphanDoc, false);

    const bool sesOk = sesConverted && sesOpened && sesTempLive && sesClosed && sesTempGone
        && sesDocsBack && sesOpened2 && sesOrphanUntitled && sesOrphanTempLive
        && sesOriginClosed && sesOrphanKept && sesOrphanTempGone && sesOrphanClosed;
    ST_BEGIN("lpr_edit_smart_object_session");
    ST_PASS("lpr_edit_smart_object_session convert=%d open=%d temp=%d closed=%d tempgone=%d "
            "docsback=%d open2=%d untitled=%d originclosed=%d kept=%d orphantempgone=%d "
            "closed2=%d",
            sesConverted ? 1 : 0, sesOpened ? 1 : 0, sesTempLive ? 1 : 0, sesClosed ? 1 : 0,
            sesTempGone ? 1 : 0, sesDocsBack ? 1 : 0, sesOpened2 ? 1 : 0,
            sesOrphanUntitled ? 1 : 0, sesOriginClosed ? 1 : 0, sesOrphanKept ? 1 : 0,
            sesOrphanTempGone ? 1 : 0, sesOrphanClosed ? 1 : 0);
    if (!sesOk) {
        return pictura::selfTest().fail(289, "edit smart object session");
    }
    return 0;
}

int pictura::runImageImportChecks(pictura::PicturaMainWindow& frame)
{
    // lpr_image_import (290): File > Open decodes a raster image into an
    // untitled one-layer document in exactly one "Open" state; Place appends it
    // as a topmost smart-object layer in exactly one "Place" state; an
    // unrecognized and an over-budget file refuse without adding a state.
    const QString pngPath = QDir::tempPath() + QStringLiteral("/kooka-pictura-import.png");
    QImage png(2, 2, QImage::Format_RGBA8888);
    png.fill(QColor(255, 0, 0, 255));
    const bool pngSaved = png.save(pngPath, "PNG");
    if (!pngSaved) {
        return pictura::selfTest().fail(290, "image import fixture");
    }

    const int openDocsBase = frame.documentCount();
    const bool opened = frame.openImagePath(pngPath);
    pictura::PictureView* view = frame.activeView();
    const bool openAdded = opened && frame.documentCount() == openDocsBase + 1 && view;
    const bool openSize = view && view->document_width() == 2 && view->document_height() == 2;
    const bool openOneLayer = view && pictura::layers_api::rowCount(*view) == 1;
    const bool openPixel = view && view->sample_argb(0, 0) == 0xffff0000u;
    // An all-opaque import becomes the locked Background (issue 4 / D1).
    const bool openBackground =
        view && pictura::layers_api::rowAt(*view, 0).kind == QStringLiteral("background");
    const bool openLocked = view && pictura::layers_api::rowAt(*view, 0).lockBits == 0x05;
    const bool openHistory =
        view && view->history_count() == 1 && view->history_label(0) == QStringLiteral("Open");
    const bool openUntitled = view && view->file_path().isEmpty();
    const bool openClean = view && !view->is_dirty();

    const int placeBase = view ? view->history_count() : -1;
    const int placeRows = view ? pictura::layers_api::rowCount(*view) : -1;
    const QString placePath = view ? view->place_image(pngPath) : QString();
    const bool placePlaced = !placePath.isEmpty() && view
        && pictura::layers_api::rowCount(*view) == placeRows + 1 && view->history_count() == placeBase + 1
        && view->history_label(placeBase) == QStringLiteral("Place")
        && view->layer_smart_object_state(placePath).startsWith(QStringLiteral("embedded:"));

    const QString badPath = QDir::tempPath() + QStringLiteral("/kooka-pictura-import-bad.png");
    QFile bad(badPath);
    if (bad.open(QIODevice::WriteOnly)) {
        bad.write("not an image");
    }
    bad.close();
    const int badDocs = frame.documentCount();
    const bool badOpen = !frame.openImagePath(badPath);
    const bool badOpenDocs = frame.documentCount() == badDocs;
    const int badHistory = view ? view->history_count() : -1;
    const int badRows = view ? pictura::layers_api::rowCount(*view) : -1;
    const bool badPlace = view && view->place_image(badPath).isEmpty()
        && view->history_count() == badHistory && pictura::layers_api::rowCount(*view) == badRows;

    // A PNG IHDR declaring 40000x40000 is over the 30000 px dimension budget,
    // so the probe refuses it before Qt ever decodes.
    const QString bigPath = QDir::tempPath() + QStringLiteral("/kooka-pictura-import-big.png");
    QByteArray big;
    big += QByteArray::fromHex("89504e470d0a1a0a");
    big += QByteArray::fromHex("0000000d49484452");
    big += QByteArray::fromHex("00009c4000009c40");
    big += QByteArray::fromHex("0806000000");
    QFile bigFile(bigPath);
    if (bigFile.open(QIODevice::WriteOnly)) {
        bigFile.write(big);
    }
    bigFile.close();
    const int bigDocs = frame.documentCount();
    const bool bigOpen = !frame.openImagePath(bigPath);
    const bool bigOpenDocs = frame.documentCount() == bigDocs;
    const int bigHistory = view ? view->history_count() : -1;
    const bool bigPlace = view && view->place_image(bigPath).isEmpty()
        && view->history_count() == bigHistory;

    // A P3 PPM is decoded by Qt's built-in ppm handler but is not one of the
    // containers the probe sniffs, so the decode edge must still attempt Qt.
    const QString ppmPath = QDir::tempPath() + QStringLiteral("/kooka-pictura-import.ppm");
    QFile ppm(ppmPath);
    if (ppm.open(QIODevice::WriteOnly)) {
        ppm.write("P3\n2 1\n255\n255 0 0 0 255 0\n");
    }
    ppm.close();
    const int ppmDocsBase = frame.documentCount();
    const bool ppmOpened = frame.openImagePath(ppmPath);
    pictura::PictureView* ppmView = frame.activeView();
    const bool ppmOk = ppmOpened && frame.documentCount() == ppmDocsBase + 1 && ppmView
        && ppmView->document_width() == 2 && ppmView->document_height() == 1
        && ppmView->sample_argb(0, 0) == 0xffff0000u
        && ppmView->sample_argb(1, 0) == 0xff00ff00u;
    if (ppmOpened) {
        frame.closeDocument(frame.activeDocumentIndex(), false);
    }
    QFile::remove(ppmPath);

    // Suffix routing: `.psd`/`.psb` (case-insensitively, and only as the final
    // suffix) go native; anything else goes through the Qt image edge.
    const bool routing = PicturaMainWindow::isNativeDocumentPath(QStringLiteral("a.psd"))
        && PicturaMainWindow::isNativeDocumentPath(QStringLiteral("a.PSB"))
        && !PicturaMainWindow::isNativeDocumentPath(QStringLiteral("a.png"))
        && !PicturaMainWindow::isNativeDocumentPath(QStringLiteral("a.psd.png"));

    const bool ok = openAdded && openSize && openOneLayer && openPixel && openHistory
        && openUntitled && openClean && openBackground && openLocked && placePlaced && badOpen
        && badOpenDocs && badPlace
        && bigOpen && bigOpenDocs && bigPlace && ppmOk && routing;
    ST_BEGIN("lpr_image_import");
    ST_PASS("lpr_image_import saved=%d open=%d size=%dx%d layers=%d pixel=%08x history=%s "
            "untitled=%d clean=%d background=%d locked=%d place=%s placed=%d bad=%d big=%d "
            "ppm=%d routing=%d",
            pngSaved ? 1 : 0, opened ? 1 : 0, view ? view->document_width() : -1,
            view ? view->document_height() : -1, view ? pictura::layers_api::rowCount(*view) : -1,
            view ? view->sample_argb(0, 0) : 0u, view ? qPrintable(view->history_label(0)) : "-",
            openUntitled ? 1 : 0, openClean ? 1 : 0, openBackground ? 1 : 0, openLocked ? 1 : 0,
            qPrintable(placePath), placePlaced ? 1 : 0,
            (badOpen && badOpenDocs && badPlace) ? 1 : 0,
            (bigOpen && bigOpenDocs && bigPlace) ? 1 : 0, ppmOk ? 1 : 0, routing ? 1 : 0);
    if (!ok) {
        return pictura::selfTest().fail(290, "image import");
    }
    QFile::remove(pngPath);
    QFile::remove(badPath);
    QFile::remove(bigPath);
    frame.closeDocument(frame.activeDocumentIndex(), false);
    return 0;
}

namespace {

struct DropResult {
    bool enter = false;
    bool drop = false;
};

// Send a synthesized DragEnter/Drop pair to `target`. The router is installed as
// an event filter, so a direct send runs it without a platform drag.
DropResult sendDrop(QWidget* target, QMimeData& mime)
{
    const QPointF pos(4, 4);
    QDragEnterEvent enter(pos.toPoint(), Qt::CopyAction, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(target, &enter);
    QDropEvent drop(pos, Qt::CopyAction, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(target, &drop);
    return DropResult{enter.isAccepted(), drop.isAccepted()};
}

} // namespace

int pictura::runFileDropChecks(pictura::PicturaMainWindow& frame)
{
    // lpr_file_drop (291): a file drag on a document canvas places one topmost
    // object per file with one "Place" state each; the same drag on the tab
    // strip, menu bar, or options bar opens one tab per file; a URL-less or
    // all-directory drag is never consumed and a regular but undecodable file
    // changes nothing; with no document open the document area opens tabs.
    const QString pngA = QDir::tempPath() + QStringLiteral("/kooka-pictura-drop-a.png");
    const QString pngB = QDir::tempPath() + QStringLiteral("/kooka-pictura-drop-b.png");
    const QString badPath = QDir::tempPath() + QStringLiteral("/kooka-pictura-drop-bad.png");
    const QString psdPath = QDir::tempPath() + QStringLiteral("/kooka-pictura-drop.psd");
    const QString psdExport =
        QDir::tempPath() + QStringLiteral("/kooka-pictura-drop-export.psd");
    QImage dropA(2, 2, QImage::Format_RGBA8888);
    dropA.fill(QColor(255, 0, 0, 255));
    QImage dropB(2, 2, QImage::Format_RGBA8888);
    dropB.fill(QColor(0, 0, 255, 255));
    const bool saved = dropA.save(pngA, "PNG") && dropB.save(pngB, "PNG");
    QFile badFile(badPath);
    if (badFile.open(QIODevice::WriteOnly)) {
        badFile.write("not an image");
    }
    badFile.close();
    if (!saved) {
        return pictura::selfTest().fail(291, "file drop fixture");
    }

    // A small native PSD fixture: both routes must send it to the native
    // place_smart_object/openPath branch, not the Qt image edge.
    const bool psdSrcCreated = frame.newDocument(QStringLiteral("FileDropPsdSrc"), 2, 2,
                                                 QStringLiteral("rgb"), 8,
                                                 QStringLiteral("white"));
    pictura::PictureView* psdSrcView = frame.activeView();
    if (!psdSrcCreated || !psdSrcView) {
        return pictura::selfTest().fail(291, "file drop psd fixture");
    }
    psdSrcView->rasterize_fill_content(psdSrcView->add_solid_fill(0xff3366ccu));
    const bool psdSaved = frame.saveActiveAs(psdPath);
    frame.closeDocument(frame.activeDocumentIndex(), false);
    QFile psdFile(psdPath);
    const bool psdRead = psdFile.open(QIODevice::ReadOnly);
    const QByteArray psdBytes = psdRead ? psdFile.readAll() : QByteArray();
    psdFile.close();
    if (!psdSaved || psdBytes.isEmpty()) {
        return pictura::selfTest().fail(291, "file drop psd fixture");
    }

    const bool placeCreated = frame.newDocument(QStringLiteral("FileDropPlace"), 4, 4,
                                                QStringLiteral("rgb"), 8,
                                                QStringLiteral("white"));
    pictura::PictureView* placeView = frame.activeView();
    pictura::ImageView* placeCanvas = frame.imageView();
    if (!placeCreated || !placeView || !placeCanvas) {
        return pictura::selfTest().fail(291, "file drop canvas fixture");
    }
    const int placeRows = pictura::layers_api::rowCount(*placeView);
    const int placeBase = placeView->history_count();
    QMimeData placeMime;
    placeMime.setUrls({QUrl::fromLocalFile(pngA), QUrl::fromLocalFile(pngB)});
    const DropResult place = sendDrop(placeCanvas, placeMime);
    const QString placeTop = pictura::layers_api::rowAt(*placeView, 0).path;
    const QString placeNext = pictura::layers_api::rowCount(*placeView) > 1 ? pictura::layers_api::rowAt(*placeView, 1).path
                                                               : QString();
    const bool placed = place.enter && place.drop
        && pictura::layers_api::rowCount(*placeView) == placeRows + 2
        && placeView->history_count() == placeBase + 2
        && placeView->history_label(placeBase) == QStringLiteral("Place")
        && placeView->history_label(placeBase + 1) == QStringLiteral("Place")
        && !placeNext.isEmpty()
        && placeView->layer_smart_object_state(placeTop).startsWith(QStringLiteral("embedded:"))
        && placeView->layer_smart_object_state(placeNext).startsWith(QStringLiteral("embedded:"));

    // One supported file plus one undecodable file: the good file places, the bad
    // file records nothing and leaves the rest of the document untouched.
    const int mixedRows = pictura::layers_api::rowCount(*placeView);
    const int mixedBase = placeView->history_count();
    QMimeData mixedMime;
    mixedMime.setUrls({QUrl::fromLocalFile(pngA), QUrl::fromLocalFile(badPath)});
    const DropResult mixed = sendDrop(placeCanvas, mixedMime);
    const bool mixedPlaced = mixed.enter && mixed.drop
        && pictura::layers_api::rowCount(*placeView) == mixedRows + 1
        && placeView->history_count() == mixedBase + 1
        && placeView->history_label(mixedBase) == QStringLiteral("Place");

    // A PSD on the canvas stays native: place_smart_object embeds the original
    // file bytes verbatim, so the exported payload must equal the source file.
    const int psdRows = pictura::layers_api::rowCount(*placeView);
    const int psdBase = placeView->history_count();
    QMimeData psdMime;
    psdMime.setUrls({QUrl::fromLocalFile(psdPath)});
    const DropResult psdPlace = sendDrop(placeCanvas, psdMime);
    const QString psdLayer = psdPlace.drop ? pictura::layers_api::rowAt(*placeView, 0).path : QString();
    const bool psdPlaced = psdPlace.enter && psdPlace.drop && !psdLayer.isEmpty()
        && pictura::layers_api::rowCount(*placeView) == psdRows + 1
        && placeView->history_count() == psdBase + 1
        && placeView->history_label(psdBase) == QStringLiteral("Place")
        && placeView->layer_smart_object_state(psdLayer).startsWith(QStringLiteral("embedded:"));
    const bool psdExported = psdPlaced
        && placeView->export_smart_object_contents(psdLayer, psdExport);
    QFile psdExportFile(psdExport);
    const bool psdExportRead = psdExportFile.open(QIODevice::ReadOnly);
    const QByteArray psdPayload = psdExportRead ? psdExportFile.readAll() : QByteArray();
    psdExportFile.close();
    const bool psdNative = psdExported && psdPayload == psdBytes;

    auto* bar = frame.findChild<QTabBar*>(QStringLiteral("documentTabBar"));
    auto* optionsBar = frame.findChild<QToolBar*>(QStringLiteral("optionsBar"));
    const int openBase = frame.documentCount();
    QMimeData barMime;
    barMime.setUrls({QUrl::fromLocalFile(pngA), QUrl::fromLocalFile(pngB)});
    QMimeData menuMime;
    menuMime.setUrls({QUrl::fromLocalFile(pngB)});
    QMimeData optionsMime;
    optionsMime.setUrls({QUrl::fromLocalFile(pngA)});
    const DropResult barDrop = bar ? sendDrop(bar, barMime) : DropResult{};
    const DropResult menuDrop = sendDrop(frame.menuBar(), menuMime);
    const DropResult optionsDrop = optionsBar ? sendDrop(optionsBar, optionsMime) : DropResult{};
    const bool opened = bar && optionsBar && barDrop.enter && barDrop.drop && menuDrop.enter
        && menuDrop.drop && optionsDrop.enter && optionsDrop.drop
        && frame.documentCount() == openBase + 4;

    // A PSD on the tab strip stays native: openPath keeps the file path on the
    // tab, while the Qt image edge would leave it untitled.
    const int psdOpenBase = frame.documentCount();
    QMimeData psdBarMime;
    psdBarMime.setUrls({QUrl::fromLocalFile(psdPath)});
    const DropResult psdBarDrop = bar ? sendDrop(bar, psdBarMime) : DropResult{};
    const bool psdOpened = bar && psdBarDrop.enter && psdBarDrop.drop
        && frame.documentCount() == psdOpenBase + 1
        && frame.documentPath(frame.activeDocumentIndex()) == psdPath;

    const int guardDocs = frame.documentCount();
    QMimeData internalMime;
    internalMime.setData(QStringLiteral("application/x-pictura-internal"),
                         QByteArrayLiteral("x"));
    QMimeData dirMime;
    dirMime.setUrls({QUrl::fromLocalFile(QDir::tempPath())});
    QMimeData badMime;
    badMime.setUrls({QUrl::fromLocalFile(badPath)});
    const DropResult internalDrop = sendDrop(bar, internalMime);
    const DropResult dirDrop = sendDrop(bar, dirMime);
    const DropResult badDrop = sendDrop(bar, badMime);
    pictura::PictureView* guardView = frame.activeView();
    const int guardRows = guardView ? pictura::layers_api::rowCount(*guardView) : -1;
    const int guardHistory = guardView ? guardView->history_count() : -1;
    QMimeData badPlaceMime;
    badPlaceMime.setUrls({QUrl::fromLocalFile(badPath)});
    const DropResult badPlace = sendDrop(frame.imageView(), badPlaceMime);
    const bool ignored = !internalDrop.enter && !internalDrop.drop && !dirDrop.enter
        && !dirDrop.drop && badDrop.enter && badDrop.drop && badPlace.enter && badPlace.drop
        && frame.documentCount() == guardDocs && guardView
        && pictura::layers_api::rowCount(*guardView) == guardRows
        && guardView->history_count() == guardHistory;

    while (frame.documentCount() > 0) {
        frame.closeDocument(0, false);
    }
    auto* tabs = frame.findChild<QTabWidget*>(QStringLiteral("documentTabs"));
    QMimeData noDocMime;
    noDocMime.setUrls({QUrl::fromLocalFile(pngA), QUrl::fromLocalFile(pngB)});
    const DropResult noDoc = tabs ? sendDrop(tabs, noDocMime) : DropResult{};
    const bool fallback = tabs && noDoc.enter && noDoc.drop && frame.documentCount() == 2;

    const bool ok = placed && mixedPlaced && psdNative && opened && psdOpened && ignored
        && fallback;
    const int endDocs = frame.documentCount();
    while (frame.documentCount() > 0) {
        frame.closeDocument(0, false);
    }
    QFile::remove(pngA);
    QFile::remove(pngB);
    QFile::remove(badPath);
    QFile::remove(psdPath);
    QFile::remove(psdExport);
    ST_BEGIN("lpr_file_drop");
    ST_PASS("lpr_file_drop place=%d mixed=%d psd_native=%d open=%d psd_open=%d ignore=%d "
            "fallback=%d docs=%d",
            placed ? 1 : 0, mixedPlaced ? 1 : 0, psdNative ? 1 : 0, opened ? 1 : 0,
            psdOpened ? 1 : 0, ignored ? 1 : 0, fallback ? 1 : 0, endDocs);
    if (!ok) {
        return pictura::selfTest().fail(291, "file drop routing");
    }
    return 0;
}

int pictura::runFreeTransformChecks(pictura::PicturaMainWindow& frame)
{
    // lpr_free_transform (292): a canvas drop places a raster image and enters a
    // Free Transform session on the new layer; a corner-scale gesture commits in
    // exactly one "Free Transform" state and changes the layer rect; a second
    // session's rotation cancels byte-identically with history unchanged; a
    // group, an adjustment layer, and the Background refuse both the predicate
    // and a session begin.
    const QString png = QDir::tempPath() + QStringLiteral("/kooka-pictura-free-transform.png");
    QImage image(4, 4, QImage::Format_RGBA8888);
    image.fill(QColor(255, 0, 0, 255));
    if (!image.save(png, "PNG")) {
        return pictura::selfTest().fail(292, "free transform fixture");
    }

    const bool created = frame.newDocument(QStringLiteral("FreeTransformCtl"), 8, 8,
                                           QStringLiteral("rgb"), 8,
                                           QStringLiteral("transparent"));
    pictura::PictureView* view = frame.activeView();
    pictura::ImageView* canvas = frame.imageView();
    if (!created || !view || !canvas) {
        QFile::remove(png);
        return pictura::selfTest().fail(292, "free transform canvas fixture");
    }
    const int doc = frame.activeDocumentIndex();
    const int base = view->history_count();
    QMimeData mime;
    mime.setUrls({QUrl::fromLocalFile(png)});
    const DropResult drop = sendDrop(canvas, mime);
    const QString path = view->transform_session_path();
    const bool entered = drop.enter && drop.drop && view->transform_session_active()
        && !path.isEmpty() && view->history_label(base) == QStringLiteral("Place");

    // A successful place selects the newly placed layer in the Layers panel.
    auto* layersPanel = frame.findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    const bool placedSelected = layersPanel && !path.isEmpty()
        && layersPanel->currentPath() == path;

    // Begin-while-active: the same path is a no-op; a different path cancels
    // the active session and starts a new one on the new target.
    const QString otherPath = pictura::layers_api::rowAt(*view, pictura::layers_api::rowCount(*view) - 1).path;
    const bool samePathNoop = view->begin_free_transform(path)
        && view->transform_session_active() && view->transform_session_path() == path;
    const bool otherPathSwitched = otherPath != path
        && view->begin_free_transform(otherPath) && view->transform_session_path() == otherPath;
    const bool restoredPath =
        view->begin_free_transform(path) && view->transform_session_path() == path;
    // The pivot defaults to the box centre; these fixtures are a few pixels
    // wide, so hide it and let the presses hit the box handles.
    pictura::transform_set_show_pivot(*view, false);

    const QString beforeRect = transform_api::layerRect(*view, path).toString();

    // Scale about the top-left corner: press the handle, drag outward, commit.
    const int hit = view->transform_press(0.0, 0.0, 1.0, false, false);
    const bool moved = view->transform_move(-4.0, -4.0, 1.0, false, false);

    // Preview mapping must match commit: the painter transform composes as
    // `c + R·S·(u − c) + d` (the offset is the outermost translate), not
    // `c + R·S·(u − c + d)`. The gesture (sx=sy=2, dx=dy=−2) must map the
    // dragged corner (0,0) to (−4,−4) and the anchor (4,4) to (4,4).
    canvas->setTransformPreview(2.0, 2.0, 0.0, -2.0, -2.0);
    const QTransform previewMatrix = canvas->transformPreviewMatrix();
    const QPointF previewDragged = previewMatrix.map(QPointF(0.0, 0.0));
    const QPointF previewAnchor = previewMatrix.map(QPointF(4.0, 4.0));
    const bool previewMatchesCommit =
        std::abs(previewDragged.x() + 4.0) < 1e-6 && std::abs(previewDragged.y() + 4.0) < 1e-6
        && std::abs(previewAnchor.x() - 4.0) < 1e-6 && std::abs(previewAnchor.y() - 4.0) < 1e-6;

    view->transform_release();
    const bool committed = view->commit_transform();
    frame.imageView()->clearTransformPreview();
    const QString afterRect = transform_api::layerRect(*view, path).toString();
    const bool oneState = view->history_count() == base + 2
        && view->history_label(base + 1) == QStringLiteral("Free Transform");
    const bool rectChanged = !beforeRect.isEmpty() && afterRect != beforeRect;
    const bool previewMatchesCommitRect = previewMatchesCommit
        && afterRect == QStringLiteral("-4 -4 4 4");

    // Save the committed state, then rotate a second session and cancel: the
    // document must be byte-identical and the history count unchanged.
    const QString saveA = QDir::tempPath() + QStringLiteral("/kooka-pictura-ft-a.psd");
    const QString saveB = QDir::tempPath() + QStringLiteral("/kooka-pictura-ft-b.psd");
    const bool savedA = view->save(saveA);
    const int cancelBase = view->history_count();
    const bool reentered = view->begin_free_transform(path);
    pictura::transform_set_show_pivot(*view, false);
    const int rotHit = view->transform_press(-10.0, -10.0, 1.0, false, false);
    const bool rotated = rotHit == 8
        && view->transform_move(8.0, -2.0, 1.0, false, false)
        && view->transform_angle() != 0.0;
    view->cancel_transform();
    frame.imageView()->clearTransformPreview();
    const bool cancelled = !view->transform_session_active();
    const bool savedB = view->save(saveB);
    QFile fileA(saveA);
    QFile fileB(saveB);
    const bool readA = fileA.open(QIODevice::ReadOnly);
    const bool readB = fileB.open(QIODevice::ReadOnly);
    const bool identical = readA && readB && fileA.readAll() == fileB.readAll();
    // Close the handles: Windows refuses to overwrite a file that still has an
    // open read handle, so the later re-saves to saveA/saveB would fail.
    fileA.close();
    fileB.close();
    const bool historyKept = view->history_count() == cancelBase;

    // An identity commit records nothing and leaves the document unchanged.
    const int identityBase = view->history_count();
    const bool identityBegin = view->begin_free_transform(path);
    const bool identityCommitted = view->commit_transform();
    const bool identityNoState =
        !identityCommitted && view->history_count() == identityBase;

    // Enter and Escape reach commit/cancel through the real ImageView key path.
    const int keyCommitBase = view->history_count();
    const bool keyBegin = frame.beginFreeTransform(path);
    pictura::transform_set_show_pivot(*view, false);
    const bool keyHit = view->transform_press(4.0, 4.0, 1.0, false, false) == 2;
    const bool keyMoved = view->transform_move(6.0, 6.0, 1.0, false, false);
    QKeyEvent enterEvent(QEvent::KeyPress, Qt::Key_Return, Qt::NoModifier);
    QCoreApplication::sendEvent(canvas, &enterEvent);
    const bool keyCommitted = enterEvent.isAccepted() && keyBegin && keyHit && keyMoved
        && !view->transform_session_active() && view->history_count() == keyCommitBase + 1
        && view->history_label(keyCommitBase) == QStringLiteral("Free Transform");

    const int keyCancelBase = view->history_count();
    const bool keyReenter = frame.beginFreeTransform(path);
    QKeyEvent escapeEvent(QEvent::KeyPress, Qt::Key_Escape, Qt::NoModifier);
    QCoreApplication::sendEvent(canvas, &escapeEvent);
    const bool keyCancelled = escapeEvent.isAccepted() && keyReenter
        && !view->transform_session_active() && view->history_count() == keyCancelBase;

    // Refusals: a group, an adjustment layer, and the Background.
    const QString group = view->add_group_in(QString());
    const bool groupCan = !group.isEmpty() && !view->layer_can_free_transform(group)
        && !view->begin_free_transform(group);
    const bool adjAdded = view->add_adjustment(QStringLiteral("invert"));
    const QString adjPath = adjAdded ? pictura::layers_api::rowAt(*view, 0).path : QString();
    const bool adjCan = adjAdded && !adjPath.isEmpty()
        && !view->layer_can_free_transform(adjPath)
        && !view->begin_free_transform(adjPath);
    const bool bgCreated = frame.newDocument(QStringLiteral("FreeTransformBg"), 4, 4,
                                             QStringLiteral("rgb"), 8,
                                             QStringLiteral("white"));
    pictura::PictureView* bgView = frame.activeView();
    const bool bgFlagged =
        bgCreated && bgView && bgView->background_from_layer(QStringLiteral("0"));
    const bool bgCan = bgFlagged && !bgView->layer_can_free_transform(QStringLiteral("0"))
        && !bgView->begin_free_transform(QStringLiteral("0"));
    const int bgDoc = bgFlagged ? frame.activeDocumentIndex() : -1;

    // Projective modes: Distort commits the dragged corner and one state,
    // Perspective mirrors the opposite corner, Skew slides one edge, and Escape
    // in a projective mode restores the document byte-identically.
    auto rectOf = [](pictura::PictureView* v, const QString& p) {
        const QStringList parts = transform_api::layerRect(*v, p).toString().split(QLatin1Char(' '), Qt::SkipEmptyParts);
        return QRectF(parts.value(0).toDouble(), parts.value(1).toDouble(),
                      parts.value(2).toDouble() - parts.value(0).toDouble(),
                      parts.value(3).toDouble() - parts.value(1).toDouble());
    };
    auto quadOf = [](pictura::PictureView* v) {
        QList<QPointF> pts;
        const QStringList toks =
            transform_api::quad(*v).toString().split(QLatin1Char(' '), Qt::SkipEmptyParts);
        for (const QString& tok : toks) {
            const QStringList xy = tok.split(QLatin1Char(','));
            if (xy.size() == 2) {
                pts << QPointF(xy.at(0).toDouble(), xy.at(1).toDouble());
            }
        }
        return pts;
    };

    // (a) Distort drags corner 0 and commits exactly one state.
    const QRectF dRect = rectOf(view, path);
    const QString beforeDistort = transform_api::layerRect(*view, path).toString();
    const int distortBase = view->history_count();
    const bool distortBegin = view->begin_transform_mode(path, QStringLiteral("distort"));
    pictura::transform_set_show_pivot(*view, false);
    const bool distortHit =
        view->transform_press(dRect.left(), dRect.top(), 1.0, false, false) == 0;
    const bool distortMoved =
        view->transform_move(dRect.left() - 3.0, dRect.top() - 2.0, 1.0, false, false);
    view->transform_release();
    const bool distortCommitted = view->commit_transform();
    frame.imageView()->clearTransformPreview();
    const bool distortOneState = view->history_count() == distortBase + 1
        && view->history_label(distortBase) == QStringLiteral("Free Transform");
    const bool distortChanged = transform_api::layerRect(*view, path).toString() != beforeDistort;

    // (b) Perspective moves the opposite corner by the negated delta.
    const QRectF pRect = rectOf(view, path);
    const bool perspBegin = view->begin_transform_mode(path, QStringLiteral("perspective"));
    pictura::transform_set_show_pivot(*view, false);
    const QList<QPointF> perspBefore = quadOf(view);
    const bool perspHit =
        view->transform_press(pRect.left(), pRect.top(), 1.0, false, false) == 0;
    const bool perspMoved =
        view->transform_move(pRect.left() + 5.0, pRect.top() + 4.0, 1.0, false, false);
    const QList<QPointF> perspAfter = quadOf(view);
    view->transform_release();
    view->cancel_transform();
    frame.imageView()->clearTransformPreview();
    const bool perspOpposite = perspBefore.size() == 4 && perspAfter.size() == 4
        && std::abs(perspAfter.at(0).x() - (pRect.left() + 5.0)) < 1e-6
        && std::abs(perspAfter.at(0).y() - (pRect.top() + 4.0)) < 1e-6
        && std::abs(perspAfter.at(2).x() - (perspBefore.at(2).x() - 5.0)) < 1e-6
        && std::abs(perspAfter.at(2).y() - (perspBefore.at(2).y() - 4.0)) < 1e-6;

    // (c) Skew slides the top edge and leaves the bottom edge fixed.
    const QRectF sRect = rectOf(view, path);
    const double topMidX = (sRect.left() + sRect.right()) / 2.0;
    const bool skewBegin = view->begin_transform_mode(path, QStringLiteral("skew"));
    pictura::transform_set_show_pivot(*view, false);
    const bool skewHit = view->transform_press(topMidX, sRect.top(), 1.0, false, false) == 4;
    const bool skewMoved =
        view->transform_move(topMidX + 2.0, sRect.top() + 3.0, 1.0, false, false);
    const QList<QPointF> skewAfter = quadOf(view);
    view->transform_release();
    view->cancel_transform();
    frame.imageView()->clearTransformPreview();
    const bool skewEdge = skewAfter.size() == 4
        && std::abs(skewAfter.at(0).x() - sRect.left() - 2.0) < 1e-6
        && std::abs(skewAfter.at(1).x() - sRect.right() - 2.0) < 1e-6
        && std::abs(skewAfter.at(1).y() - sRect.top() - 3.0) < 1e-6
        && std::abs(skewAfter.at(2).y() - sRect.bottom()) < 1e-6
        && std::abs(skewAfter.at(3).y() - sRect.bottom()) < 1e-6;

    // (d) Escape in a projective mode restores the document and adds no state.
    const int escBase = view->history_count();
    const bool escSavedA = view->save(saveA);
    const bool escBegin = view->begin_transform_mode(path, QStringLiteral("distort"));
    pictura::transform_set_show_pivot(*view, false);
    const bool escHit =
        view->transform_press(sRect.left(), sRect.top(), 1.0, false, false) >= 0;
    const bool escMoved =
        view->transform_move(sRect.left() - 6.0, sRect.top() - 6.0, 1.0, false, false);
    view->cancel_transform();
    frame.imageView()->clearTransformPreview();
    const bool escCancelled =
        !view->transform_session_active() && view->history_count() == escBase;
    const bool escSavedB = view->save(saveB);
    QFile escFileA(saveA);
    QFile escFileB(saveB);
    const bool escRead = escFileA.open(QIODevice::ReadOnly) && escFileB.open(QIODevice::ReadOnly);
    const bool escIdentical = escRead && escFileA.readAll() == escFileB.readAll();
    escFileA.close();
    escFileB.close();

    // A second begin on the same path switches modes (not a no-op): a Distort
    // session followed by a Skew begin accepts an edge drag.
    const bool switchDistort = view->begin_transform_mode(path, QStringLiteral("distort"));
    const bool switchSkew = view->begin_transform_mode(path, QStringLiteral("skew"));
    pictura::transform_set_show_pivot(*view, false);
    const QRectF swRect = rectOf(view, path);
    const double swMid = (swRect.left() + swRect.right()) / 2.0;
    const bool swHit = view->transform_press(swMid, swRect.top(), 1.0, false, false) == 4;
    const bool swMoved = view->transform_move(swMid + 1.0, swRect.top() + 1.0, 1.0, false, false);
    const QList<QPointF> swAfter = quadOf(view);
    view->cancel_transform();
    frame.imageView()->clearTransformPreview();
    const bool switchOk = switchDistort && switchSkew && swHit && swMoved && swAfter.size() == 4
        && std::abs(swAfter.at(0).x() - swRect.left() - 1.0) < 1e-6;

    const bool distortOk = distortBegin && distortHit && distortMoved && distortCommitted
        && distortOneState && distortChanged;
    const bool perspOk = perspBegin && perspHit && perspMoved && perspOpposite;
    const bool skewOk = skewBegin && skewHit && skewMoved && skewEdge;
    const bool escOk = escBegin && escHit && escMoved && escCancelled && escSavedA && escSavedB
        && escIdentical;

    const bool ok = entered && placedSelected && samePathNoop && otherPathSwitched
        && restoredPath && hit == 0 && moved && previewMatchesCommitRect && committed && oneState
        && rectChanged && savedA && reentered && rotHit == 8 && rotated && cancelled && savedB
        && identical && historyKept && identityBegin && identityNoState && keyBegin && keyCommitted
        && keyReenter && keyCancelled && groupCan && adjCan && bgCan && distortOk && perspOk
        && skewOk && escOk && switchOk;
    ST_BEGIN("lpr_free_transform");
    ST_PASS("lpr_free_transform entered=%d select=%d begin=%d preview=%d hit=%d moved=%d "
            "commit=%d states=%d rect=%s->%s keys=%d rotate=%d cancel=%d identical=%d "
            "history=%d identity=%d group=%d adj=%d bg=%d distort=%d persp=%d skew=%d esc=%d "
            "switch=%d",
            entered ? 1 : 0, placedSelected ? 1 : 0,
            (samePathNoop && otherPathSwitched && restoredPath) ? 1 : 0,
            previewMatchesCommitRect ? 1 : 0, hit, moved ? 1 : 0, committed ? 1 : 0,
            view->history_count() - base, qPrintable(beforeRect), qPrintable(afterRect),
            (keyCommitted && keyCancelled) ? 1 : 0, rotated ? 1 : 0, cancelled ? 1 : 0,
            identical ? 1 : 0, historyKept ? 1 : 0, identityNoState ? 1 : 0, groupCan ? 1 : 0,
            adjCan ? 1 : 0, bgCan ? 1 : 0, distortOk ? 1 : 0, perspOk ? 1 : 0, skewOk ? 1 : 0,
            escOk ? 1 : 0, switchOk ? 1 : 0);
    if (!ok) {
        return pictura::selfTest().fail(292, "free transform");
    }
    QFile::remove(png);
    QFile::remove(saveA);
    QFile::remove(saveB);
    if (bgDoc >= 0) {
        frame.closeDocument(bgDoc, false);
    }
    frame.closeDocument(doc, false);
    return 0;
}
