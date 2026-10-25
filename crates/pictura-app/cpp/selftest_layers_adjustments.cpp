#include "selftest_layers_adjustments.h"
#include "selftest_report.h"

#include "commands.h"
#include "color_settings_dialog.h"
#include "file_info_dialog.h"
#include "frame.h"
#include "panels/panel_column.h"
#include "profile_dialog.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QFile>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtCore/QTemporaryDir>
#include <QtGui/QAction>
#include <QtGui/QImage>
#include <QtWidgets/QPushButton>

#include <cstring>

int pictura::runLayersAdjustmentChecks(pictura::PicturaMainWindow& frame)
{
        // lpr_photo_filter (283): a photo-filter adjustment layer is reported
        // as an adjustment and warms the composite (red exceeds blue at a
        // sampled pixel).
        {
            const bool pflCreated = frame.newDocument(QStringLiteral("PhotoFilterCtl"), 4, 4,
                                                      QStringLiteral("rgb"), 8,
                                                      QStringLiteral("white"));
            pictura::PictureView* pflView = frame.activeView();
            if (!pflCreated || !pflView) {
                return pictura::selfTest().fail(283, "photo filter fixture");
            }
            const int pflDoc = frame.activeDocumentIndex();
            const unsigned int pflBefore = pflView->sample_argb(1, 1);
            const bool pflAdded = pflView->add_adjustment(QStringLiteral("photo-filter"));
            const bool pflAdjustment =
                pflView->layer_kind(pflView->layer_count() - 1) == QStringLiteral("adjustment");
            const unsigned int pflAfter = pflView->sample_argb(1, 1);
            const bool pflWarm = qRed(pflAfter) > qBlue(pflAfter);
            const bool pflChanged = pflAfter != pflBefore;
            ST_BEGIN("lpr_photo_filter");
            ST_PASS("lpr_photo_filter added=%d adjustment=%d before=%08x after=%08x warm=%d",
                    pflAdded ? 1 : 0, pflAdjustment ? 1 : 0, pflBefore, pflAfter,
                    pflWarm ? 1 : 0);
            if (!pflAdded || !pflAdjustment || !pflChanged || !pflWarm) {
                return pictura::selfTest().fail(283, "photo filter adjustment");
            }
            frame.closeDocument(pflDoc, false);
        }

        // adjustments_photo_filter_menu (284): the Adjustments panel menu offers
        // the `Photo Filter` row wired to `adjustment:photo-filter`.
        const pictura::PanelColumn* adjColumn = frame.panelColumn();
        const bool adjPF = adjColumn
            && adjColumn->widgetMenuTextsForTest(QStringLiteral("adjustmentsPanel"))
                   .contains(QStringLiteral("Photo Filter"));
        ST_BEGIN("adjustments_photo_filter_menu");
        ST_PASS("adjustments_photo_filter_menu row=%d", adjPF ? 1 : 0);
        if (!adjPF) {
            return pictura::selfTest().fail(284, "adjustments photo filter menu");
        }

        // lpr_gradient_map (285): a gradient-map adjustment layer is reported as
        // an adjustment, maps a non-uniform backdrop's luminance through the
        // black-to-white gradient, and the Adjustments panel menu offers it.
        {
            const bool gmCreated = frame.newDocument(QStringLiteral("GradientMapCtl"), 8, 8,
                                                     QStringLiteral("rgb"), 8,
                                                     QStringLiteral("white"));
            pictura::PictureView* gmView = frame.activeView();
            if (!gmCreated || !gmView) {
                return pictura::selfTest().fail(285, "gradient map fixture");
            }
            const int gmDoc = frame.activeDocumentIndex();
            bool gmPainted = gmView->begin_paint(0xFFFF0000u, 0xFFFFFFFFu, 4, 100, 100, 0,
                                                 100, 100, 25, QStringLiteral("normal"), false,
                                                 false);
            gmPainted = gmPainted && gmView->paint_dab(2, 2, 1.0) && gmView->paint_dab(5, 5, 1.0)
                && gmView->end_paint();
            const unsigned int gmBefore = gmView->sample_argb(2, 2);
            const bool gmAdded = gmView->add_adjustment(QStringLiteral("gradient-map"));
            const bool gmAdjustment =
                gmView->layer_kind(gmView->layer_count() - 1) == QStringLiteral("adjustment");
            const unsigned int gmAfter = gmView->sample_argb(2, 2);
            const pictura::PanelColumn* gmColumn = frame.panelColumn();
            const bool gmMenu = gmColumn
                && gmColumn->widgetMenuTextsForTest(QStringLiteral("adjustmentsPanel"))
                       .contains(QStringLiteral("Gradient Map"));
            ST_BEGIN("lpr_gradient_map");
            ST_PASS("lpr_gradient_map added=%d adjustment=%d changed=%d menu=%d before=%08x "
                    "after=%08x",
                    gmAdded ? 1 : 0, gmAdjustment ? 1 : 0, gmAfter != gmBefore ? 1 : 0,
                    gmMenu ? 1 : 0, gmBefore, gmAfter);
            if (!gmPainted || !gmAdded || !gmAdjustment || gmAfter == gmBefore || !gmMenu) {
                return pictura::selfTest().fail(285, "gradient map adjustment");
            }
            frame.closeDocument(gmDoc, false);
        }

        // lpr_color_balance (293): the neutral `color-balance` kind becomes an
        // adjustment layer that leaves the composite unchanged, and the
        // Adjustments panel offers a `Color Balance` row.
        {
            const bool cbCreated = frame.newDocument(QStringLiteral("ColorBalanceCtl"), 4, 4,
                                                     QStringLiteral("rgb"), 8,
                                                     QStringLiteral("white"));
            pictura::PictureView* cbView = frame.activeView();
            if (!cbCreated || !cbView) {
                return pictura::selfTest().fail(293, "color balance fixture");
            }
            const int cbDoc = frame.activeDocumentIndex();
            const unsigned int cbBefore = cbView->sample_argb(1, 1);
            const bool cbAdded = cbView->add_adjustment(QStringLiteral("color-balance"));
            const bool cbAdjustment =
                cbView->layer_kind(cbView->layer_count() - 1) == QStringLiteral("adjustment");
            const unsigned int cbAfter = cbView->sample_argb(1, 1);
            const bool cbNeutral = cbAfter == cbBefore;
            const pictura::PanelColumn* cbColumn = frame.panelColumn();
            const bool cbMenu = cbColumn
                && cbColumn->widgetMenuTextsForTest(QStringLiteral("adjustmentsPanel"))
                       .contains(QStringLiteral("Color Balance"));
            ST_BEGIN("lpr_color_balance");
            ST_PASS("lpr_color_balance added=%d adjustment=%d neutral=%d menu=%d",
                    cbAdded ? 1 : 0, cbAdjustment ? 1 : 0, cbNeutral ? 1 : 0, cbMenu ? 1 : 0);
            if (!cbAdded || !cbAdjustment || !cbNeutral || !cbMenu) {
                return pictura::selfTest().fail(293, "color balance adjustment");
            }
            frame.closeDocument(cbDoc, false);
        }

        // lpr_channel_mixer (294): the neutral `channel-mixer` kind becomes an
        // adjustment layer that leaves the composite unchanged, and the
        // Adjustments panel offers a `Channel Mixer` row.
        {
            const bool cmCreated = frame.newDocument(QStringLiteral("ChannelMixerCtl"), 4, 4,
                                                     QStringLiteral("rgb"), 8,
                                                     QStringLiteral("white"));
            pictura::PictureView* cmView = frame.activeView();
            if (!cmCreated || !cmView) {
                return pictura::selfTest().fail(294, "channel mixer fixture");
            }
            const int cmDoc = frame.activeDocumentIndex();
            const unsigned int cmBefore = cmView->sample_argb(1, 1);
            const bool cmAdded = cmView->add_adjustment(QStringLiteral("channel-mixer"));
            const bool cmAdjustment =
                cmView->layer_kind(cmView->layer_count() - 1) == QStringLiteral("adjustment");
            const unsigned int cmAfter = cmView->sample_argb(1, 1);
            const bool cmNeutral = cmAfter == cmBefore;
            const pictura::PanelColumn* cmColumn = frame.panelColumn();
            const bool cmMenu = cmColumn
                && cmColumn->widgetMenuTextsForTest(QStringLiteral("adjustmentsPanel"))
                       .contains(QStringLiteral("Channel Mixer"));
            ST_BEGIN("lpr_channel_mixer");
            ST_PASS("lpr_channel_mixer added=%d adjustment=%d neutral=%d menu=%d",
                    cmAdded ? 1 : 0, cmAdjustment ? 1 : 0, cmNeutral ? 1 : 0, cmMenu ? 1 : 0);
            if (!cmAdded || !cmAdjustment || !cmNeutral || !cmMenu) {
                return pictura::selfTest().fail(294, "channel mixer adjustment");
            }
            frame.closeDocument(cmDoc, false);
        }

        // lpr_selective_color (296): the neutral `selective-color` kind becomes
        // an adjustment layer that leaves the composite unchanged, and the
        // Adjustments panel offers a `Selective Color` row.
        {
            const bool selCreated = frame.newDocument(QStringLiteral("SelectiveColorCtl"), 4, 4,
                                                      QStringLiteral("rgb"), 8,
                                                      QStringLiteral("white"));
            pictura::PictureView* selView = frame.activeView();
            if (!selCreated || !selView) {
                return pictura::selfTest().fail(296, "selective color fixture");
            }
            const int selDoc = frame.activeDocumentIndex();
            const unsigned int selBefore = selView->sample_argb(1, 1);
            const bool selAdded = selView->add_adjustment(QStringLiteral("selective-color"));
            const bool selAdjustment =
                selView->layer_kind(selView->layer_count() - 1) == QStringLiteral("adjustment");
            const unsigned int selAfter = selView->sample_argb(1, 1);
            const bool selNeutral = selAfter == selBefore;
            const pictura::PanelColumn* selColumn = frame.panelColumn();
            const bool selMenu = selColumn
                && selColumn->widgetMenuTextsForTest(QStringLiteral("adjustmentsPanel"))
                       .contains(QStringLiteral("Selective Color"));
            ST_BEGIN("lpr_selective_color");
            ST_PASS("lpr_selective_color added=%d adjustment=%d neutral=%d menu=%d",
                    selAdded ? 1 : 0, selAdjustment ? 1 : 0, selNeutral ? 1 : 0,
                    selMenu ? 1 : 0);
            if (!selAdded || !selAdjustment || !selNeutral || !selMenu) {
                return pictura::selfTest().fail(296, "selective color adjustment");
            }
            frame.closeDocument(selDoc, false);
        }

        // lpr_color_lookup (455): the neutral `color-lookup` kind becomes an
        // adjustment layer carrying an identity cube that leaves the composite
        // unchanged, and the Adjustments panel offers a `Color Lookup` row.
        {
            const bool clCreated = frame.newDocument(QStringLiteral("ColorLookupCtl"), 4, 4,
                                                     QStringLiteral("rgb"), 8,
                                                     QStringLiteral("white"));
            pictura::PictureView* clView = frame.activeView();
            if (!clCreated || !clView) {
                return pictura::selfTest().fail(455, "color lookup fixture");
            }
            const int clDoc = frame.activeDocumentIndex();
            const unsigned int clBefore = clView->sample_argb(1, 1);
            const bool clAdded = clView->add_adjustment(QStringLiteral("color-lookup"));
            const bool clAdjustment =
                clView->layer_kind(clView->layer_count() - 1) == QStringLiteral("adjustment");
            const unsigned int clAfter = clView->sample_argb(1, 1);
            const bool clNeutral = clAfter == clBefore;
            const pictura::PanelColumn* clColumn = frame.panelColumn();
            const bool clMenu = clColumn
                && clColumn->widgetMenuTextsForTest(QStringLiteral("adjustmentsPanel"))
                       .contains(QStringLiteral("Color Lookup"));
            ST_BEGIN("lpr_color_lookup");
            ST_PASS("lpr_color_lookup added=%d adjustment=%d neutral=%d menu=%d",
                    clAdded ? 1 : 0, clAdjustment ? 1 : 0, clNeutral ? 1 : 0, clMenu ? 1 : 0);
            if (!clAdded || !clAdjustment || !clNeutral || !clMenu) {
                return pictura::selfTest().fail(455, "color lookup adjustment");
            }
            frame.closeDocument(clDoc, false);
        }

        // color_mode_open (297): a minimal flat CMYK PSD opens as a normalized
        // RGB document and the view reports the CMYK conversion notice.
        {
            QTemporaryDir cmDir;
            const QString cmPath = cmDir.filePath(QStringLiteral("cmyk.psd"));
            QByteArray cmBytes;
            const auto cmAppend16 = [&cmBytes](unsigned short value) {
                cmBytes.append(char((value >> 8) & 0xff));
                cmBytes.append(char(value & 0xff));
            };
            const auto cmAppend32 = [&cmBytes](unsigned int value) {
                cmBytes.append(char((value >> 24) & 0xff));
                cmBytes.append(char((value >> 16) & 0xff));
                cmBytes.append(char((value >> 8) & 0xff));
                cmBytes.append(char(value & 0xff));
            };
            cmBytes.append("8BPS", 4);
            cmAppend16(1);              // version
            cmBytes.append(6, char(0)); // reserved
            cmAppend16(4);              // channels
            cmAppend32(1);              // height
            cmAppend32(1);              // width
            cmAppend16(8);              // depth
            cmAppend16(4);              // color mode CMYK
            cmAppend32(0);              // color mode data
            cmAppend32(0);              // image resources
            cmAppend32(0);              // layer/mask section
            cmAppend16(0);              // raw compression
            cmBytes.append(char(128));  // C
            cmBytes.append(char(64));   // M
            cmBytes.append(char(32));   // Y
            cmBytes.append(char(200));  // K
            QFile cmFile(cmPath);
            const bool cmWritten = cmFile.open(QIODevice::WriteOnly)
                && cmFile.write(cmBytes) == cmBytes.size();
            cmFile.close();

            const int cmDocs = frame.documentCount();
            const bool cmOpened = cmWritten && frame.openPath(cmPath);
            pictura::PictureView* cmView = frame.activeView();
            const bool cmOk = cmOpened && frame.documentCount() == cmDocs + 1 && cmView
                && cmView->open_notice() == QStringLiteral("Converted from CMYK; saved as CMYK")
                && cmView->document_mode() == QStringLiteral("rgb")
                && cmView->sample_argb(0, 0) == 0xff141d25u;
            ST_BEGIN("color_mode_open");
            ST_PASS("color_mode_open open=%d notice=%s mode=%s pixel=%08x",
                    cmOpened ? 1 : 0, cmView ? qPrintable(cmView->open_notice()) : "-",
                    cmView ? qPrintable(cmView->document_mode()) : "-",
                    cmView ? cmView->sample_argb(0, 0) : 0u);
            if (cmView) {
                frame.closeDocument(frame.activeDocumentIndex(), false);
            }
            if (!cmOk) {
                return pictura::selfTest().fail(297, "color mode open");
            }
        }

        // lab_mode_open (518): a minimal flat Lab PSD opens as a normalized RGB
        // document; the view reports a Lab-preserving save notice rather than an
        // RGB one.
        {
            QTemporaryDir labDir;
            const QString labPath = labDir.filePath(QStringLiteral("lab.psd"));
            QByteArray labBytes;
            const auto labAppend16 = [&labBytes](unsigned short value) {
                labBytes.append(char((value >> 8) & 0xff));
                labBytes.append(char(value & 0xff));
            };
            const auto labAppend32 = [&labBytes](unsigned int value) {
                labBytes.append(char((value >> 24) & 0xff));
                labBytes.append(char((value >> 16) & 0xff));
                labBytes.append(char((value >> 8) & 0xff));
                labBytes.append(char(value & 0xff));
            };
            labBytes.append("8BPS", 4);
            labAppend16(1);              // version
            labBytes.append(6, char(0)); // reserved
            labAppend16(3);              // channels
            labAppend32(1);              // height
            labAppend32(1);              // width
            labAppend16(8);              // depth
            labAppend16(9);              // color mode Lab
            labAppend32(0);              // color mode data
            labAppend32(0);              // image resources
            labAppend32(0);              // layer/mask section
            labAppend16(0);              // raw compression
            labBytes.append(char(225));  // L
            labBytes.append(char(82));   // a
            labBytes.append(char(114));  // b
            QFile labFile(labPath);
            const bool labWritten = labFile.open(QIODevice::WriteOnly)
                && labFile.write(labBytes) == labBytes.size();
            labFile.close();

            const int labDocs = frame.documentCount();
            const bool labOpened = labWritten && frame.openPath(labPath);
            pictura::PictureView* labView = frame.activeView();
            const bool labOk = labOpened && frame.documentCount() == labDocs + 1 && labView
                && labView->open_notice() == QStringLiteral("Converted from Lab; saved as Lab")
                && labView->document_mode() == QStringLiteral("rgb");
            ST_BEGIN("lab_mode_open");
            ST_PASS("lab_mode_open open=%d notice=%s mode=%s",
                    labOpened ? 1 : 0, labView ? qPrintable(labView->open_notice()) : "-",
                    labView ? qPrintable(labView->document_mode()) : "-");
            if (labView) {
                frame.closeDocument(frame.activeDocumentIndex(), false);
            }
            if (!labOk) {
                return pictura::selfTest().fail(518, "Lab mode open");
            }
        }

        // lab16_mode_open (519): a 16-bit Lab PSD normalizes to 8-bit RGB but
        // retains its native samples, so the view reports a Lab-preserving save
        // notice at the source depth.
        {
            QTemporaryDir lab16Dir;
            const QString lab16Path = lab16Dir.filePath(QStringLiteral("lab16.psd"));
            QByteArray lab16Bytes;
            const auto lab16Append16 = [&lab16Bytes](unsigned short value) {
                lab16Bytes.append(char((value >> 8) & 0xff));
                lab16Bytes.append(char(value & 0xff));
            };
            const auto lab16Append32 = [&lab16Bytes](unsigned int value) {
                lab16Bytes.append(char((value >> 24) & 0xff));
                lab16Bytes.append(char((value >> 16) & 0xff));
                lab16Bytes.append(char((value >> 8) & 0xff));
                lab16Bytes.append(char(value & 0xff));
            };
            lab16Bytes.append("8BPS", 4);
            lab16Append16(1);              // version
            lab16Bytes.append(6, char(0)); // reserved
            lab16Append16(3);              // channels
            lab16Append32(1);              // height
            lab16Append32(1);              // width
            lab16Append16(16);             // depth
            lab16Append16(9);              // color mode Lab
            lab16Append32(0);              // color mode data
            lab16Append32(0);              // image resources
            lab16Append32(0);              // layer/mask section
            lab16Append16(0);              // raw compression
            for (int c = 0; c < 3; ++c) {
                lab16Append16(0x8000);     // one 16-bit sample per plane
            }
            QFile lab16File(lab16Path);
            const bool lab16Written = lab16File.open(QIODevice::WriteOnly)
                && lab16File.write(lab16Bytes) == lab16Bytes.size();
            lab16File.close();

            const int lab16Docs = frame.documentCount();
            const bool lab16Opened = lab16Written && frame.openPath(lab16Path);
            pictura::PictureView* lab16View = frame.activeView();
            const bool lab16Ok = lab16Opened && frame.documentCount() == lab16Docs + 1 && lab16View
                && lab16View->open_notice()
                    == QStringLiteral("Converted from Lab; saved as Lab; "
                                      "Converted from 16-bit to 8-bit for editing; saved at 16-bit")
                && lab16View->document_mode() == QStringLiteral("rgb");
            ST_BEGIN("lab16_mode_open");
            ST_PASS("lab16_mode_open open=%d notice=%s mode=%s",
                    lab16Opened ? 1 : 0, lab16View ? qPrintable(lab16View->open_notice()) : "-",
                    lab16View ? qPrintable(lab16View->document_mode()) : "-");
            if (lab16View) {
                frame.closeDocument(frame.activeDocumentIndex(), false);
            }
            if (!lab16Ok) {
                return pictura::selfTest().fail(519, "16-bit Lab mode open");
            }
        }

        // indexed_mode_open (520): a minimal flat Indexed PSD opens as a
        // normalized RGB document and the view reports an Indexed-preserving
        // save notice.
        {
            QTemporaryDir idxDir;
            const QString idxPath = idxDir.filePath(QStringLiteral("indexed.psd"));
            QByteArray idxBytes;
            const auto idxAppend16 = [&idxBytes](unsigned short value) {
                idxBytes.append(char((value >> 8) & 0xff));
                idxBytes.append(char(value & 0xff));
            };
            const auto idxAppend32 = [&idxBytes](unsigned int value) {
                idxBytes.append(char((value >> 24) & 0xff));
                idxBytes.append(char((value >> 16) & 0xff));
                idxBytes.append(char((value >> 8) & 0xff));
                idxBytes.append(char(value & 0xff));
            };
            idxBytes.append("8BPS", 4);
            idxAppend16(1);              // version
            idxBytes.append(6, char(0)); // reserved
            idxAppend16(1);              // channels
            idxAppend32(1);              // height
            idxAppend32(1);              // width
            idxAppend16(8);              // depth
            idxAppend16(2);              // color mode Indexed
            idxAppend32(768);            // color mode data length
            QByteArray idxPalette(768, char(0));
            for (int i = 0; i < 256; ++i) {
                idxPalette[i] = char(i);
                idxPalette[256 + i] = char(255 - i);
                idxPalette[512 + i] = char((i * 7) % 256);
            }
            idxBytes.append(idxPalette);
            idxAppend32(0);              // image resources
            idxAppend32(0);              // layer/mask section
            idxAppend16(0);              // raw compression
            idxBytes.append(char(2));    // index 2 -> RGB (2, 253, 14)
            QFile idxFile(idxPath);
            const bool idxWritten = idxFile.open(QIODevice::WriteOnly)
                && idxFile.write(idxBytes) == idxBytes.size();
            idxFile.close();

            const int idxDocs = frame.documentCount();
            const bool idxOpened = idxWritten && frame.openPath(idxPath);
            pictura::PictureView* idxView = frame.activeView();
            const bool idxOk = idxOpened && frame.documentCount() == idxDocs + 1 && idxView
                && idxView->open_notice() == QStringLiteral("Converted from Indexed; saved as Indexed")
                && idxView->document_mode() == QStringLiteral("rgb")
                && idxView->sample_argb(0, 0) == 0xff02fd0eu;
            ST_BEGIN("indexed_mode_open");
            ST_PASS("indexed_mode_open open=%d notice=%s mode=%s pixel=%08x",
                    idxOpened ? 1 : 0, idxView ? qPrintable(idxView->open_notice()) : "-",
                    idxView ? qPrintable(idxView->document_mode()) : "-",
                    idxView ? idxView->sample_argb(0, 0) : 0u);
            if (idxView) {
                frame.closeDocument(frame.activeDocumentIndex(), false);
            }
            if (!idxOk) {
                return pictura::selfTest().fail(520, "Indexed mode open");
            }
        }

        // bitmap_mode_open (521): a minimal flat depth-1 Bitmap PSD opens as a
        // normalized RGB document and the view reports a Bitmap-preserving save
        // notice.
        {
            QTemporaryDir bmpDir;
            const QString bmpPath = bmpDir.filePath(QStringLiteral("bitmap.psd"));
            QByteArray bmpBytes;
            const auto bmpAppend16 = [&bmpBytes](unsigned short value) {
                bmpBytes.append(char((value >> 8) & 0xff));
                bmpBytes.append(char(value & 0xff));
            };
            const auto bmpAppend32 = [&bmpBytes](unsigned int value) {
                bmpBytes.append(char((value >> 24) & 0xff));
                bmpBytes.append(char((value >> 16) & 0xff));
                bmpBytes.append(char((value >> 8) & 0xff));
                bmpBytes.append(char(value & 0xff));
            };
            bmpBytes.append("8BPS", 4);
            bmpAppend16(1);              // version
            bmpBytes.append(6, char(0)); // reserved
            bmpAppend16(1);              // channels
            bmpAppend32(1);              // height
            bmpAppend32(1);              // width
            bmpAppend16(1);              // depth (1-bit)
            bmpAppend16(0);              // color mode Bitmap
            bmpAppend32(0);              // color mode data
            bmpAppend32(0);              // image resources
            bmpAppend32(0);              // layer/mask section
            bmpAppend16(0);              // raw compression
            bmpBytes.append(char(0x80)); // one row byte, MSB set -> pixel 0
            QFile bmpFile(bmpPath);
            const bool bmpWritten = bmpFile.open(QIODevice::WriteOnly)
                && bmpFile.write(bmpBytes) == bmpBytes.size();
            bmpFile.close();

            const int bmpDocs = frame.documentCount();
            const bool bmpOpened = bmpWritten && frame.openPath(bmpPath);
            pictura::PictureView* bmpView = frame.activeView();
            const bool bmpOk = bmpOpened && frame.documentCount() == bmpDocs + 1
                && bmpView
                && bmpView->open_notice() == QStringLiteral("Converted from Bitmap; saved as Bitmap")
                && bmpView->document_mode() == QStringLiteral("rgb")
                && bmpView->sample_argb(0, 0) == 0xff000000u;
            ST_BEGIN("bitmap_mode_open");
            ST_PASS("bitmap_mode_open open=%d notice=%s mode=%s pixel=%08x",
                    bmpOpened ? 1 : 0, bmpView ? qPrintable(bmpView->open_notice()) : "-",
                    bmpView ? qPrintable(bmpView->document_mode()) : "-",
                    bmpView ? bmpView->sample_argb(0, 0) : 0u);
            if (bmpView) {
                frame.closeDocument(frame.activeDocumentIndex(), false);
            }
            if (!bmpOk) {
                return pictura::selfTest().fail(521, "Bitmap mode open");
            }
        }

        // depth_open (298): a minimal flat depth-16 RGB PSD opens as a
        // normalized 8-bit RGB document, the view reports the 16-bit conversion
        // notice, and a composite pixel is the `v >> 8` narrowing.
        {
            QTemporaryDir depthDir;
            const QString depthPath = depthDir.filePath(QStringLiteral("depth16.psd"));
            QByteArray depthBytes;
            const auto depthAppend16 = [&depthBytes](unsigned short value) {
                depthBytes.append(char((value >> 8) & 0xff));
                depthBytes.append(char(value & 0xff));
            };
            const auto depthAppend32 = [&depthBytes](unsigned int value) {
                depthBytes.append(char((value >> 24) & 0xff));
                depthBytes.append(char((value >> 16) & 0xff));
                depthBytes.append(char((value >> 8) & 0xff));
                depthBytes.append(char(value & 0xff));
            };
            depthBytes.append("8BPS", 4);
            depthAppend16(1);               // version
            depthBytes.append(6, char(0));  // reserved
            depthAppend16(3);               // channels
            depthAppend32(1);               // height
            depthAppend32(1);               // width
            depthAppend16(16);              // depth
            depthAppend16(3);               // color mode RGB
            depthAppend32(0);               // color mode data
            depthAppend32(0);               // image resources
            depthAppend32(0);               // layer/mask section
            depthAppend16(0);               // raw compression
            depthAppend16(0x1234);          // R narrows to 0x12
            depthAppend16(0x5678);          // G narrows to 0x56
            depthAppend16(0x9abc);          // B narrows to 0x9a
            QFile depthFile(depthPath);
            const bool depthWritten = depthFile.open(QIODevice::WriteOnly)
                && depthFile.write(depthBytes) == depthBytes.size();
            depthFile.close();

            const int depthDocs = frame.documentCount();
            const bool depthOpened = depthWritten && frame.openPath(depthPath);
            pictura::PictureView* depthView = frame.activeView();
            const bool depthOk = depthOpened && frame.documentCount() == depthDocs + 1
                && depthView
                && depthView->open_notice() == QStringLiteral("Converted from 16-bit to 8-bit for editing; saved at 16-bit")
                && depthView->document_mode() == QStringLiteral("rgb")
                && depthView->sample_argb(0, 0) == 0xff12569au;
            ST_BEGIN("depth_open");
            ST_PASS("depth_open open=%d notice=%s mode=%s pixel=%08x",
                    depthOpened ? 1 : 0,
                    depthView ? qPrintable(depthView->open_notice()) : "-",
                    depthView ? qPrintable(depthView->document_mode()) : "-",
                    depthView ? depthView->sample_argb(0, 0) : 0u);
            if (depthView) {
                frame.closeDocument(frame.activeDocumentIndex(), false);
            }
            if (!depthOk) {
                return pictura::selfTest().fail(298, "depth open");
            }
        }

        // hdr_conversion (526): a 32-bit document reports depth 32 and the
        // bridge converts it to 16 (the working model stays 8-bit; the reported
        // and save depth becomes 16).
        {
            QTemporaryDir hdrDir;
            const QString hdrPath = hdrDir.filePath(QStringLiteral("hdr32.psd"));
            QByteArray hdrBytes;
            const auto hdr16 = [&hdrBytes](unsigned short value) {
                hdrBytes.append(char((value >> 8) & 0xff));
                hdrBytes.append(char(value & 0xff));
            };
            const auto hdr32 = [&hdrBytes](unsigned int value) {
                hdrBytes.append(char((value >> 24) & 0xff));
                hdrBytes.append(char((value >> 16) & 0xff));
                hdrBytes.append(char((value >> 8) & 0xff));
                hdrBytes.append(char(value & 0xff));
            };
            const auto hdrF32 = [&hdrBytes](float value) {
                unsigned int bits = 0;
                std::memcpy(&bits, &value, sizeof(bits));
                hdrBytes.append(char((bits >> 24) & 0xff));
                hdrBytes.append(char((bits >> 16) & 0xff));
                hdrBytes.append(char((bits >> 8) & 0xff));
                hdrBytes.append(char(bits & 0xff));
            };
            hdrBytes.append("8BPS", 4);
            hdr16(1);               // version
            hdrBytes.append(6, char(0));  // reserved
            hdr16(3);               // channels
            hdr32(1);               // height
            hdr32(1);               // width
            hdr16(32);              // depth
            hdr16(3);               // color mode RGB
            hdr32(0);               // color mode data
            hdr32(0);               // image resources
            hdr32(0);               // layer/mask section
            hdr16(0);               // raw compression
            hdrF32(0.25f);          // R
            hdrF32(0.5f);           // G
            hdrF32(0.75f);          // B
            QFile hdrFile(hdrPath);
            const bool hdrWritten = hdrFile.open(QIODevice::WriteOnly)
                && hdrFile.write(hdrBytes) == hdrBytes.size();
            hdrFile.close();

            const int hdrDocs = frame.documentCount();
            const bool hdrOpened = hdrWritten && frame.openPath(hdrPath);
            pictura::PictureView* hdrView = frame.activeView();
            const bool hdrIs32 = hdrView && hdrView->document_depth_bits() == 32;
            const bool hdrConverted = hdrView && hdrView->convert_depth(16, 0.0, 1.0);
            const bool hdrIs16 = hdrView && hdrView->document_depth_bits() == 16;
            ST_BEGIN("hdr_conversion");
            ST_PASS("hdr_conversion docs=%d open=%d is32=%d convert=%d is16=%d",
                    frame.documentCount() - hdrDocs, hdrOpened ? 1 : 0, hdrIs32 ? 1 : 0,
                    hdrConverted ? 1 : 0, hdrIs16 ? 1 : 0);
            if (hdrView) {
                frame.closeDocument(frame.activeDocumentIndex(), false);
            }
            if (!hdrOpened || !hdrIs32 || !hdrConverted || !hdrIs16) {
                return pictura::selfTest().fail(526, "hdr conversion");
            }
        }

        // file_info_metadata (456): a PSD carrying EXIF, IPTC, and XMP image
        // resources decodes into File Info rows, and the read-only dialog shows
        // the three categories with those values.
        {
            QTemporaryDir fiDir;
            const QString fiPath = fiDir.filePath(QStringLiteral("metadata.psd"));
            QByteArray fiRes;
            const auto fiRes16 = [&fiRes](unsigned short value) {
                fiRes.append(char((value >> 8) & 0xff));
                fiRes.append(char(value & 0xff));
            };
            const auto fiRes32 = [&fiRes](unsigned int value) {
                fiRes.append(char((value >> 24) & 0xff));
                fiRes.append(char((value >> 16) & 0xff));
                fiRes.append(char((value >> 8) & 0xff));
                fiRes.append(char(value & 0xff));
            };
            const auto fiResource = [&](unsigned short id, const QByteArray& data) {
                fiRes.append("8BIM", 4);
                fiRes16(id);
                fiRes.append(char(0)); // empty Pascal name
                fiRes.append(char(0)); // name pad to even
                fiRes32(unsigned(data.size()));
                fiRes.append(data);
                if (data.size() % 2 != 0) {
                    fiRes.append(char(0));
                }
            };
            // Little-endian TIFF with IFD0 Make = "ACME" at offset 0x1a.
            const QByteArray fiExif = QByteArrayLiteral(
                "II\x2a\x00\x08\x00\x00\x00"
                "\x01\x00"
                "\x0f\x01\x02\x00\x05\x00\x00\x00\x1a\x00\x00\x00"
                "\x00\x00\x00\x00"
                "ACME\x00");
            // IPTC-IIM: Object Name (2:5) = "Hi".
            const QByteArray fiIptc = QByteArrayLiteral("\x1c\x02\x05\x00\x02Hi");
            // XMP: Element-form dc:title = "Hi" plus a property in an unknown
            // namespace that must survive an edit.
            const QByteArray fiXmp = QByteArrayLiteral(
                "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">"
                "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">"
                "<rdf:Description rdf:about=\"\""
                " xmlns:dc=\"http://purl.org/dc/elements/1.1/\""
                " xmlns:my=\"http://example.com/ns/\">"
                "<dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">Hi</rdf:li></rdf:Alt>"
                "</dc:title>"
                "<my:Custom>keep</my:Custom>"
                "</rdf:Description></rdf:RDF></x:xmpmeta>");
            fiResource(1058, fiExif);
            fiResource(1028, fiIptc);
            fiResource(1060, fiXmp);

            QByteArray fi;
            const auto fi16 = [&fi](unsigned short value) {
                fi.append(char((value >> 8) & 0xff));
                fi.append(char(value & 0xff));
            };
            const auto fi32 = [&fi](unsigned int value) {
                fi.append(char((value >> 24) & 0xff));
                fi.append(char((value >> 16) & 0xff));
                fi.append(char((value >> 8) & 0xff));
                fi.append(char(value & 0xff));
            };
            fi.append("8BPS", 4);
            fi16(1);
            fi.append(6, char(0));
            fi16(3); // channels
            fi32(1); // height
            fi32(1); // width
            fi16(8); // depth
            fi16(3); // RGB
            fi32(0); // color mode data
            fi32(unsigned(fiRes.size()));
            fi.append(fiRes);
            fi32(0); // layer/mask section
            fi16(0); // raw compression
            fi.append(char(10));
            fi.append(char(20));
            fi.append(char(30));
            QFile fiFile(fiPath);
            const bool fiWritten = fiFile.open(QIODevice::WriteOnly)
                && fiFile.write(fi) == fi.size();
            fiFile.close();

            const int fiDocs = frame.documentCount();
            const bool fiOpened = fiWritten && frame.openPath(fiPath);
            pictura::PictureView* fiView = frame.activeView();
            const QStringList fiExifRows = fiView ? fiView->exif_rows() : QStringList();
            const QStringList fiEditFields =
                fiView ? fiView->iptc_edit_fields() : QStringList();
            const QStringList fiOtherRows = fiView ? fiView->iptc_rows() : QStringList();
            const QStringList fiXmpRows = fiView ? fiView->xmp_rows() : QStringList();
            const QString fiXmpText = fiView ? fiView->xmp_packet() : QString();
            const bool fiExifOk = fiExifRows.contains(QStringLiteral("Make\tACME"));
            const bool fiIptcOk =
                fiEditFields.contains(QStringLiteral("2:5\tObject Name\tHi"));
            const bool fiXmpRowsOk = fiXmpRows.contains(QStringLiteral("Title\tHi"));
            const bool fiXmpOk = fiXmpText.contains(QStringLiteral("dc:title"));

            pictura::FileInfoDialog fiDialog(fiExifRows, fiXmpRows, fiEditFields, fiOtherRows,
                                             fiXmpText);
            const bool fiCategories =
                fiDialog.categoriesForTest()
                == QStringList({QStringLiteral("Camera Data"), QStringLiteral("Description"),
                                QStringLiteral("IPTC"), QStringLiteral("Raw Data")});
            const bool fiDialogRows = fiDialog.rowsForTest(QStringLiteral("Camera Data"))
                                          .contains(QStringLiteral("Make\tACME"));
            const bool fiDescriptionRows = fiDialog.rowsForTest(QStringLiteral("Description"))
                                               .contains(QStringLiteral("Title\tHi"));
            const bool fiDialogXmp = fiDialog.xmpForTest() == fiXmpText;
            const bool fiFieldShown = fiDialog.fieldForTest(QStringLiteral("2:5"))
                == QStringLiteral("Hi");
            const bool fiEditsEmpty = fiDialog.edits().isEmpty();
            fiDialog.setFieldForTest(QStringLiteral("2:5"), QStringLiteral("FromDialog"));
            const bool fiEditsRow = fiDialog.edits().size() == 1
                && fiDialog.edits().contains(QStringLiteral("2:5\tFromDialog"));
            QAction* fiAction = frame.registry()->action(
                QString::fromLatin1(pictura::command_ids::FileInfo));
            const bool fiEnabled = fiAction && fiAction->isEnabled();

            const bool fiOk = fiOpened && frame.documentCount() == fiDocs + 1 && fiExifOk
                && fiIptcOk && fiXmpRowsOk && fiXmpOk && fiCategories && fiDialogRows
                && fiDescriptionRows && fiDialogXmp && fiFieldShown && fiEditsEmpty && fiEditsRow
                && fiEnabled;
            ST_BEGIN("file_info_metadata");
            ST_PASS("file_info_metadata open=%d exif=%d iptc=%d xmpr=%d xmp=%d categories=%d "
                    "rows=%d desc=%d",
                    fiOpened ? 1 : 0, fiExifOk ? 1 : 0, fiIptcOk ? 1 : 0, fiXmpRowsOk ? 1 : 0,
                    fiXmpOk ? 1 : 0, fiCategories ? 1 : 0, fiDialogRows ? 1 : 0,
                    fiDescriptionRows ? 1 : 0);
            if (!fiOk) {
                return pictura::selfTest().fail(456, "file info metadata");
            }

            // file_info_iptc_edit (457): applying a metadata edit through the
            // bridge changes both the IIM record and the parsed XMP, marks the
            // document dirty, records one undo state, and undo restores both;
            // re-applying the same value is a no-op.
            const bool fiNoop = !fiView->apply_metadata_edits(
                QStringList({QStringLiteral("2:5\tHi")}));
            const int fiStates = fiView->history_count();
            const bool fiEdited =
                fiView->apply_metadata_edits(QStringList({QStringLiteral("2:5\tEdited")}));
            const bool fiApplied =
                fiView->iptc_edit_fields().contains(QStringLiteral("2:5\tObject Name\tEdited"));
            const bool fiXmpApplied = fiView->xmp_rows().contains(QStringLiteral("Title\tEdited"));
            const bool fiDirty = fiView->is_dirty();
            const bool fiOneState = fiView->history_count() == fiStates + 1
                && fiView->history_label(fiView->history_count() - 1)
                    == QStringLiteral("File Info");
            const bool fiUndone =
                fiView->undo()
                && fiView->iptc_edit_fields().contains(QStringLiteral("2:5\tObject Name\tHi"))
                && fiView->xmp_rows().contains(QStringLiteral("Title\tHi"));
            ST_BEGIN("file_info_iptc_edit");
            ST_PASS("file_info_iptc_edit noop=%d edited=%d applied=%d xmp=%d dirty=%d state=%d "
                    "undone=%d",
                    fiNoop ? 1 : 0, fiEdited ? 1 : 0, fiApplied ? 1 : 0, fiXmpApplied ? 1 : 0,
                    fiDirty ? 1 : 0, fiOneState ? 1 : 0, fiUndone ? 1 : 0);

            // file_info_xmp_sync (459): one apply writes the IIM record and the
            // XMP title together as a single undo state, and undo restores both.
            const int syncBase = fiView->history_index();
            const bool syncApplied = fiView->apply_metadata_edits(
                QStringList({QStringLiteral("2:5\tSynced")}));
            const bool syncXmp = fiView->xmp_rows().contains(QStringLiteral("Title\tSynced"));
            const bool syncIim = fiView->iptc_edit_fields().contains(
                QStringLiteral("2:5\tObject Name\tSynced"));
            const bool syncOneState = fiView->history_index() == syncBase + 1
                && fiView->history_count() == fiView->history_index() + 1
                && fiView->history_label(fiView->history_index()) == QStringLiteral("File Info");
            const bool syncUndone = fiView->undo()
                && fiView->xmp_rows().contains(QStringLiteral("Title\tHi"))
                && fiView->iptc_edit_fields().contains(QStringLiteral("2:5\tObject Name\tHi"));
            ST_BEGIN("file_info_xmp_sync");
            ST_PASS("file_info_xmp_sync applied=%d xmp=%d iim=%d state=%d undone=%d",
                    syncApplied ? 1 : 0, syncXmp ? 1 : 0, syncIim ? 1 : 0, syncOneState ? 1 : 0,
                    syncUndone ? 1 : 0);

            if (fiView) {
                frame.closeDocument(frame.activeDocumentIndex(), false);
            }
            if (!fiNoop || !fiEdited || !fiApplied || !fiXmpApplied || !fiDirty || !fiOneState
                || !fiUndone) {
                return pictura::selfTest().fail(457, "file info iptc edit");
            }
            if (!syncApplied || !syncXmp || !syncIim || !syncOneState || !syncUndone) {
                return pictura::selfTest().fail(459, "file info xmp sync");
            }
        }

        // profile_assign_convert (458): Assign Profile retags the active RGB
        // document as exactly one undoable dirty state, undo restores the pre-
        // assign cursor, a bad index is refused without a state, and Convert to
        // Profile pushes one more state. Strictly the public bridge API.
        {
            const bool pcfCreated = frame.newDocument(
                QStringLiteral("ProfileCtl"), 4, 4, QStringLiteral("rgb"), 8,
                QStringLiteral("white"));
            pictura::PictureView* pcfView = frame.activeView();
            ST_BEGIN("profile_assign_convert");
            if (!pcfCreated || !pcfView) {
                return pictura::selfTest().fail(458, "profile fixture");
            }
            const int pcfDoc = frame.activeDocumentIndex();
            const int pcfBase = pcfView->history_count();
            const int pcfBaseIndex = pcfView->history_index();
            const QString pcfBaseLabel = pcfView->history_label(pcfBaseIndex);
            const bool pcfAssign = pcfView->assign_profile(1);
            const bool pcfOneState = pcfView->history_count() == pcfBase + 1
                && pcfView->history_label(pcfView->history_count() - 1)
                    == QStringLiteral("Assign Profile");
            const bool pcfDirty = pcfView->is_dirty();
            const bool pcfRestored = pcfView->undo()
                && pcfView->history_index() == pcfBaseIndex
                && pcfView->history_label(pcfBaseIndex) == pcfBaseLabel;
            const bool pcfBadIndex = !pcfView->assign_profile(99)
                && pcfView->history_count() == pcfBase + 1;
            const int pcfConvertBase = pcfView->history_index();
            const bool pcfConvert = pcfView->convert_profile(1);
            const bool pcfConvertState = pcfView->history_index() == pcfConvertBase + 1
                && pcfView->history_label(pcfView->history_index())
                    == QStringLiteral("Convert to Profile");
            const bool pcfConvertUndone = pcfView->undo()
                && pcfView->history_index() == pcfConvertBase
                && pcfView->history_label(pcfConvertBase) == pcfBaseLabel;

            pictura::ProfileDialog pcfAssignDialog(false, nullptr);
            const QStringList pcfAssignChoices = pcfAssignDialog.choicesForTest();
            pcfAssignDialog.setChoiceForTest(0);
            const bool pcfAssignMapping =
                pcfAssignChoices.contains(QStringLiteral("Don't Color Manage"))
                && pcfAssignDialog.profileIndex() == 0;
            pictura::ProfileDialog pcfConvertDialog(true, nullptr);
            const QStringList pcfConvertChoices = pcfConvertDialog.choicesForTest();
            pcfConvertDialog.setChoiceForTest(1);
            const bool pcfConvertMapping =
                !pcfConvertChoices.contains(QStringLiteral("Don't Color Manage"))
                && pcfConvertDialog.profileIndex() == 1;

            ST_PASS("profile_assign_convert assign=%d state=%d dirty=%d restored=%d bad=%d "
                    "convert=%d convert_state=%d convert_undo=%d map_a=%d map_c=%d",
                    pcfAssign ? 1 : 0, pcfOneState ? 1 : 0, pcfDirty ? 1 : 0,
                    pcfRestored ? 1 : 0, pcfBadIndex ? 1 : 0, pcfConvert ? 1 : 0,
                    pcfConvertState ? 1 : 0, pcfConvertUndone ? 1 : 0,
                    pcfAssignMapping ? 1 : 0, pcfConvertMapping ? 1 : 0);
            frame.closeDocument(pcfDoc, false);
            if (!pcfAssign || !pcfOneState || !pcfDirty || !pcfRestored || !pcfBadIndex
                || !pcfConvert || !pcfConvertState || !pcfConvertUndone || !pcfAssignMapping
                || !pcfConvertMapping) {
                return pictura::selfTest().fail(458, "profile assign convert");
            }
        }

        // metadata_template_apply (460): exporting the active document's managed
        // metadata writes a standalone XMP template without mutating the
        // document or its history, and applying it with each merge mode has that
        // mode's effect as one undoable state.
        {
            QTemporaryDir mtDir;
            const QString mtPath = mtDir.filePath(QStringLiteral("t.xmp"));
            const bool mtCreated = frame.newDocument(QStringLiteral("TemplateCtl"), 4, 4,
                                                     QStringLiteral("rgb"), 8,
                                                     QStringLiteral("white"));
            pictura::PictureView* mtSource = frame.activeView();
            if (!mtCreated || !mtSource) {
                return pictura::selfTest().fail(460, "metadata template fixture");
            }
            const int mtSourceDoc = frame.activeDocumentIndex();
            mtSource->apply_metadata_edits(QStringList(
                {QStringLiteral("2:5\tTpl"), QStringLiteral("2:110\tTplCred")}));
            const QStringList mtBefore = mtSource->xmp_rows();
            const int mtHistory = mtSource->history_count();
            const bool mtDirtyBefore = mtSource->is_dirty();
            const bool mtExported = mtSource->export_metadata_template(mtPath);
            const bool mtFileOk = QFile::exists(mtPath) && QFile(mtPath).size() > 0;
            const bool mtUnchanged = mtSource->history_count() == mtHistory
                && mtSource->xmp_rows() == mtBefore
                && mtSource->is_dirty() == mtDirtyBefore;
            frame.closeDocument(mtSourceDoc, false);

            frame.newDocument(QStringLiteral("TemplateApplyCtl"), 4, 4, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"));
            pictura::PictureView* mtView = frame.activeView();
            const bool mtBase = mtView
                && mtView->apply_metadata_edits(QStringList(
                       {QStringLiteral("2:5\tOld"), QStringLiteral("2:120\tDocDesc")}));
            const QString mtApplyPath = mtDir.filePath(QStringLiteral("apply.psd"));
            const bool mtClean = mtView && mtView->save(mtApplyPath) && !mtView->is_dirty();

            const int mtAppendBase = mtView ? mtView->history_index() : 0;
            const bool mtAppend = mtView && mtView->apply_metadata_template(mtPath, 0);
            const bool mtAppendDirty = mtView && mtView->is_dirty();
            const bool mtAppendState = mtView && mtView->history_index() == mtAppendBase + 1
                && mtView->history_label(mtView->history_index())
                    == QStringLiteral("Metadata Template");
            const bool mtAppendEffect = mtView
                && mtView->xmp_rows().contains(QStringLiteral("Title\tOld"))
                && mtView->xmp_rows().contains(QStringLiteral("Credit\tTplCred"));
            const bool mtAppendUndo = mtView && mtView->undo()
                && mtView->xmp_rows().contains(QStringLiteral("Title\tOld"))
                && !mtView->xmp_rows().contains(QStringLiteral("Credit\tTplCred"));

            const int mtReplaceBase = mtView ? mtView->history_index() : 0;
            const bool mtReplace = mtView && mtView->apply_metadata_template(mtPath, 1);
            const bool mtReplaceState = mtView && mtView->history_index() == mtReplaceBase + 1
                && mtView->history_label(mtView->history_index())
                    == QStringLiteral("Metadata Template");
            const bool mtReplaceEffect = mtView
                && mtView->xmp_rows().contains(QStringLiteral("Title\tTpl"))
                && mtView->xmp_rows().contains(QStringLiteral("Credit\tTplCred"))
                && !mtView->xmp_rows().contains(QStringLiteral("Description\tDocDesc"));
            const bool mtReplaceUndo = mtView && mtView->undo()
                && mtView->xmp_rows().contains(QStringLiteral("Title\tOld"))
                && mtView->xmp_rows().contains(QStringLiteral("Description\tDocDesc"));

            const int mtKeepBase = mtView ? mtView->history_index() : 0;
            const bool mtKeep = mtView && mtView->apply_metadata_template(mtPath, 2);
            const bool mtKeepState = mtView && mtView->history_index() == mtKeepBase + 1
                && mtView->history_label(mtView->history_index())
                    == QStringLiteral("Metadata Template");
            const bool mtKeepEffect = mtView
                && mtView->xmp_rows().contains(QStringLiteral("Title\tTpl"))
                && mtView->xmp_rows().contains(QStringLiteral("Credit\tTplCred"))
                && mtView->xmp_rows().contains(QStringLiteral("Description\tDocDesc"));
            const bool mtKeepUndo = mtView && mtView->undo()
                && mtView->xmp_rows().contains(QStringLiteral("Title\tOld"))
                && mtView->xmp_rows().contains(QStringLiteral("Description\tDocDesc"));

            pictura::FileInfoDialog mtDialog({}, {}, {}, {}, QString());
            int mtCaptured = -1;
            mtDialog.onApplyTemplate = [&mtCaptured](int mode) { mtCaptured = mode; };
            QPushButton* mtApplyButton =
                mtDialog.findChild<QPushButton*>(QStringLiteral("fileInfoApplyTemplate"));
            mtDialog.setMergeModeForTest(2);
            const bool mtControls = mtDialog.hasTemplateControlsForTest()
                && mtDialog.mergeModeForTest() == 2 && mtDialog.templateMode() == 2;
            if (mtApplyButton) {
                mtApplyButton->click();
            }
            const bool mtCallback = mtCaptured == 2;

            const bool mtOk = mtExported && mtFileOk && mtUnchanged && mtBase && mtClean
                && mtAppend && mtAppendDirty && mtAppendState && mtAppendEffect && mtAppendUndo
                && mtReplace && mtReplaceState && mtReplaceEffect && mtReplaceUndo && mtKeep
                && mtKeepState && mtKeepEffect && mtKeepUndo && mtControls && mtCallback;
            ST_BEGIN("metadata_template_apply");
            ST_PASS("metadata_template_apply export=%d file=%d unchanged=%d clean=%d dirty=%d "
                    "append=%d replace=%d keep=%d state=%d undo=%d controls=%d callback=%d",
                    mtExported ? 1 : 0, mtFileOk ? 1 : 0, mtUnchanged ? 1 : 0,
                    mtClean ? 1 : 0, mtAppendDirty ? 1 : 0,
                    mtAppend && mtAppendEffect ? 1 : 0,
                    mtReplace && mtReplaceEffect ? 1 : 0, mtKeep && mtKeepEffect ? 1 : 0,
                    mtAppendState && mtReplaceState && mtKeepState ? 1 : 0,
                    mtAppendUndo && mtReplaceUndo && mtKeepUndo ? 1 : 0, mtControls ? 1 : 0,
                    mtCallback ? 1 : 0);
            if (mtView) {
                frame.closeDocument(frame.activeDocumentIndex(), false);
            }
            if (!mtOk) {
                return pictura::selfTest().fail(460, "metadata template apply");
            }
        }

        // color_settings_policy (461): a fresh view defaults to Preserve; the
        // setter/getter round-trip the three codes and ignore an unknown one;
        // the Color Settings dialog maps each choice to its code, shows the
        // sRGB working space, and a rejected dialog changes nothing.
        {
            pictura::PictureView cspView;
            const bool cspDefault = cspView.color_policy() == 0;
            bool cspRoundTrip = true;
            for (const int code : {0, 1, 2}) {
                cspView.set_color_policy(code);
                cspRoundTrip = cspRoundTrip && cspView.color_policy() == code;
            }
            cspView.set_color_policy(1);
            cspView.set_color_policy(99);
            const bool cspRejectsInvalid = cspView.color_policy() == 1;

            pictura::ColorSettingsDialog cspDialog(1);
            const QStringList cspChoices = cspDialog.policyChoicesForTest();
            const bool cspWorkingSpace =
                cspDialog.workingSpaceForTest() == QStringLiteral("sRGB IEC61966-2.1");
            const bool cspDefaultStored = cspDialog.policyCode() == 1;
            cspDialog.setPolicyForTest(0);
            const bool cspOff = cspDialog.policyCode() == 2;
            cspDialog.setPolicyForTest(1);
            const bool cspPreserve = cspDialog.policyCode() == 0;
            cspDialog.setPolicyForTest(2);
            const bool cspConvert = cspDialog.policyCode() == 1;
            const bool cspLabels = cspChoices.contains(QStringLiteral("Off"))
                && cspChoices.contains(QStringLiteral("Preserve Embedded Profiles"))
                && cspChoices.contains(QStringLiteral("Convert to Working RGB"));
            cspDialog.setPolicyForTest(0);
            cspDialog.reject();
            const bool cspCancel =
                cspDialog.result() != QDialog::Accepted && cspView.color_policy() == 1;

            ST_BEGIN("color_settings_policy");
            ST_PASS("color_settings_policy default=%d round_trip=%d invalid=%d "
                    "working_space=%d stored=%d off=%d preserve=%d convert=%d labels=%d "
                    "cancel=%d",
                    cspDefault ? 1 : 0, cspRoundTrip ? 1 : 0, cspRejectsInvalid ? 1 : 0,
                    cspWorkingSpace ? 1 : 0, cspDefaultStored ? 1 : 0, cspOff ? 1 : 0,
                    cspPreserve ? 1 : 0, cspConvert ? 1 : 0, cspLabels ? 1 : 0,
                    cspCancel ? 1 : 0);
            if (!cspDefault || !cspRoundTrip || !cspRejectsInvalid || !cspWorkingSpace
                || !cspDefaultStored || !cspOff || !cspPreserve || !cspConvert || !cspLabels
                || !cspCancel) {
                return pictura::selfTest().fail(461, "color settings policy");
            }
        }

    return 0;
}
