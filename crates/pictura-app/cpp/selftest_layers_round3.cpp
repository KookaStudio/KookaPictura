#include "selftest_layers_round3.h"
#include "selftest_report.h"
#include "api/layers_api.h"

#include "frame.h"
#include "layer_new_dialog.h"
#include "panels/layers_panel.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtGui/QColor>
#include <QtGui/QImage>
#include <QtGui/QRegion>

namespace pictura {

namespace {

int rowForPath(PictureView* view, const QString& path)
{
    for (int i = 0; i < pictura::layers_api::rowCount(*view); ++i) {
        if (pictura::layers_api::rowAt(*view, i).path == path) {
            return i;
        }
    }
    return -1;
}

bool nearWhite(const QColor& color)
{
    return color.red() > 0xF0 && color.green() > 0xF0 && color.blue() > 0xF0;
}

} // namespace

int runLayersRound3Checks(PicturaMainWindow& frame)
{
    auto* panel = frame.findChild<LayersPanel*>(QStringLiteral("layersPanel"));
    if (!panel) {
        return pictura::selfTest().fail(352, "round3 panel fixture");
    }

    // lpr_rename_band (352): a content-band double-click opens the editor; the
    // eye and chevron controls do not; a zero-width-name row still resolves.
    {
        const bool created = frame.newDocument(QStringLiteral("RenameBand"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        PictureView* view = frame.activeView();
        if (!created || !view) {
            return pictura::selfTest().fail(352, "rename band fixture");
        }
        const int doc = frame.activeDocumentIndex();
        panel->setView(view);
        panel->setOptionsForTest(2, 1, false);
        const QString namePath = view->add_layer_in(QString());
        view->set_layer_name_path(namePath, QStringLiteral("Rename"));
        const QString group = view->add_group_in(QString());
        view->add_layer_in(group);
        panel->refresh();
        const bool nameOpens = panel->doubleClickAtForTest(namePath, true);
        const bool eyeNo = !panel->doubleClickAtForTest(namePath, false);
        const bool chevronNo = !panel->doubleClickChevronForTest(group);
        panel->setOptionsForTest(0, 1, false);
        panel->refresh();
        const bool zeroWidth = panel->rowNameRectForTest(namePath).width() > 0;
        const bool zeroWidthOpens = panel->doubleClickAtForTest(namePath, true);
        ST_BEGIN("lpr_rename_band");
        ST_PASS("lpr_rename_band name=%d eye=%d chevron=%d zero=%d zeroopen=%d",
                nameOpens ? 1 : 0, eyeNo ? 1 : 0, chevronNo ? 1 : 0, zeroWidth ? 1 : 0,
                zeroWidthOpens ? 1 : 0);
        if (!nameOpens || !eyeNo || !chevronNo || !zeroWidth || !zeroWidthOpens) {
            return pictura::selfTest().fail(352, "rename content band");
        }
        frame.closeDocument(doc, false);
    }

    // lpr_drop_root (353): the invalid parent index is drop-enabled and a
    // top-level below-gap resolves to a real above/below indicator.
    {
        const bool created = frame.newDocument(QStringLiteral("DropRoot"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        PictureView* view = frame.activeView();
        if (!created || !view) {
            return pictura::selfTest().fail(353, "root drop fixture");
        }
        const int doc = frame.activeDocumentIndex();
        panel->setView(view);
        const QString a = view->add_layer_in(QString());
        const QString b = view->add_layer_in(QString());
        view->set_layer_name_path(a, QStringLiteral("A"));
        view->set_layer_name_path(b, QStringLiteral("B"));
        panel->refresh();
        const bool rootEnabled = panel->rootDropEnabledForTest();
        const int mode = panel->dragMoveModeAtForTest(a, b, false);
        const bool indicator = panel->dropIndicatorShownForTest();
        ST_BEGIN("lpr_drop_root");
        ST_PASS("lpr_drop_root enabled=%d mode=%d indicator=%d", rootEnabled ? 1 : 0, mode,
                indicator ? 1 : 0);
        if (!rootEnabled || (mode != 0 && mode != 1) || !indicator) {
            return pictura::selfTest().fail(353, "top-level drop indicator");
        }
        frame.closeDocument(doc, false);
    }

    // lpr_drag_cursor (354): a drag shows the closed hand and restores the
    // previous cursor when it ends.
    {
        const int before = panel->dragCursorShapeForTest();
        panel->beginDragCursorForTest();
        const int during = panel->dragCursorShapeForTest();
        panel->endDragCursorForTest();
        const int after = panel->dragCursorShapeForTest();
        const int closed = static_cast<int>(Qt::ClosedHandCursor);
        const bool ok = before != closed && during == closed && after == before;
        ST_BEGIN("lpr_drag_cursor");
        ST_PASS("lpr_drag_cursor before=%d during=%d after=%d", before, during, after);
        if (!ok) {
            return pictura::selfTest().fail(354, "drag closed-hand cursor");
        }
    }

    // lpr_background_dialog (355): the name-and-color factory is name/color only;
    // accepting the dialog converts in place to the next free `Layer N` with the
    // chosen label, in one undo state.
    {
        const bool created = frame.newDocument(QStringLiteral("BgConvertDialog"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("transparent"));
        PictureView* view = frame.activeView();
        if (!created || !view) {
            return pictura::selfTest().fail(355, "background dialog fixture");
        }
        const int doc = frame.activeDocumentIndex();
        panel->setView(view);
        view->background_from_layer(QStringLiteral("0"));
        panel->refresh();
        LayerNewDialog shape(QStringLiteral("Layer 1"), nullptr);
        const bool nameColorOnly = shape.nameColorOnlyForTest()
            && !shape.clippingVisibleForTest()
            && shape.spec().name == QStringLiteral("Layer 1") && shape.spec().opacity == 255;
        const QString expected = view->next_layer_name(QStringLiteral("Layer"));
        const bool wasBackground =
            pictura::layers_api::rowAt(*view, rowForPath(view, QStringLiteral("0"))).kind == QStringLiteral("background");
        const int base = view->history_count();
        panel->setBackgroundConvertForTest(true, expected, 3);
        panel->doubleClickAtForTest(QStringLiteral("0"), true);
        const int row = rowForPath(view, QStringLiteral("0"));
        const bool converted = row >= 0 && pictura::layers_api::rowAt(*view, row).kind == QStringLiteral("pixel");
        const bool named = row >= 0 && pictura::layers_api::rowAt(*view, row).name == expected;
        const bool labeled = row >= 0 && pictura::layers_api::rowAt(*view, row).color == 3;
        const bool unlocked = view->layer_lock(0) == 0;
        const bool oneStep = view->history_count() == base + 1;
        ST_BEGIN("lpr_background_dialog");
        ST_PASS("lpr_background_dialog factory=%d expected=%s kind=%s name=%s color=%d history=%d",
                nameColorOnly ? 1 : 0, qPrintable(expected),
                qPrintable(pictura::layers_api::rowAt(*view, row).kind), qPrintable(pictura::layers_api::rowAt(*view, row).name),
                pictura::layers_api::rowAt(*view, row).color, view->history_count() - base);
        if (!nameColorOnly || !wasBackground || !converted || !named || !labeled || !unlocked
            || !oneStep) {
            return pictura::selfTest().fail(355, "background conversion dialog");
        }
        frame.closeDocument(doc, false);
    }

    // lpr_background_cancel (356): cancelling the dialog leaves the Background
    // flag, lock, name, pixels, and history untouched.
    {
        const bool created = frame.newDocument(QStringLiteral("BgConvertCancel"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("transparent"));
        PictureView* view = frame.activeView();
        if (!created || !view) {
            return pictura::selfTest().fail(356, "background cancel fixture");
        }
        const int doc = frame.activeDocumentIndex();
        panel->setView(view);
        view->background_from_layer(QStringLiteral("0"));
        panel->refresh();
        const int row = rowForPath(view, QStringLiteral("0"));
        const QString beforeName = pictura::layers_api::rowAt(*view, row).name;
        const int beforeLock = view->layer_lock(0);
        const unsigned beforePixel = view->sample_argb(0, 0);
        const int base = view->history_count();
        panel->setBackgroundConvertForTest(false, QString(), 0);
        panel->doubleClickAtForTest(QStringLiteral("0"), true);
        const bool unchanged = pictura::layers_api::rowAt(*view, row).kind == QStringLiteral("background")
            && pictura::layers_api::rowAt(*view, row).name == beforeName && view->layer_lock(0) == beforeLock
            && view->sample_argb(0, 0) == beforePixel && view->history_count() == base;
        ST_BEGIN("lpr_background_cancel");
        ST_PASS("lpr_background_cancel kind=%s name=%s lock=%d history=%d",
                qPrintable(pictura::layers_api::rowAt(*view, row).kind), qPrintable(pictura::layers_api::rowAt(*view, row).name),
                view->layer_lock(0), view->history_count() - base);
        if (!unchanged) {
            return pictura::selfTest().fail(356, "background conversion cancel");
        }
        frame.closeDocument(doc, false);
    }

    // The color-label fill check (retired, was exit code 357) now lives in
    // the Qt Test suite (`tst_layers_panel::layerSurfaceCompleteness`), which
    // asserts the full visibility-column fill and no chip.

    // lpr_highlight_clip (358): the selection highlight changes the content but
    // not the eye column, which is repainted with the base colour.
    {
        const bool created = frame.newDocument(QStringLiteral("HighlightClip"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        PictureView* view = frame.activeView();
        if (!created || !view) {
            return pictura::selfTest().fail(358, "highlight clip fixture");
        }
        const int doc = frame.activeDocumentIndex();
        panel->setView(view);
        panel->setOptionsForTest(2, 1, false);
        const QString path = view->add_layer_in(QString());
        view->set_layer_name_path(path, QStringLiteral("Clip"));
        panel->refresh();
        panel->selectPaths(QStringList{path}, path);
        const QImage selected = panel->rowImageForTest(path);
        const QRect eye = panel->rowEyeRectForTest(path);
        panel->selectPaths(QStringList{}, QString());
        const QImage unselected = panel->rowImageForTest(path);
        bool contentDiffers = false;
        for (int y = 0; y < selected.height() && !contentDiffers; ++y) {
            for (int x = 0; x < selected.width(); ++x) {
                if (!eye.contains(x, y) && selected.pixel(x, y) != unselected.pixel(x, y)) {
                    contentDiffers = true;
                    break;
                }
            }
        }
        const bool eyeSame =
            selected.pixel(eye.left() + 2, eye.top() + 1)
            == unselected.pixel(eye.left() + 2, eye.top() + 1);
        ST_BEGIN("lpr_highlight_clip");
        ST_PASS("lpr_highlight_clip contentDiffers=%d eyeSame=%d", contentDiffers ? 1 : 0,
                eyeSame ? 1 : 0);
        if (!contentDiffers || !eyeSame) {
            return pictura::selfTest().fail(358, "selected-row eye clip");
        }
        frame.closeDocument(doc, false);
    }

    // lpr_thumb_frame (359): regular thumbnails show a two-tone checkerboard
    // under a 1 px black outline, the singular active layer adds white corner
    // brackets one pixel outside, and zero selection drops them; a group keeps
    // the folder glyph and draws no checkerboard.
    {
        const bool created = frame.newDocument(QStringLiteral("ThumbFrame"), 16, 16,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        PictureView* view = frame.activeView();
        if (!created || !view) {
            return pictura::selfTest().fail(359, "thumbnail frame fixture");
        }
        const int doc = frame.activeDocumentIndex();
        panel->setView(view);
        panel->setOptionsForTest(2, 1, false);
        const QString path = view->add_layer_in(QString());
        view->set_layer_name_path(path, QStringLiteral("Frame"));
        panel->refresh();
        panel->selectPaths(QStringList{path}, path);
        const QImage image = panel->rowImageForTest(path);
        const QRect thumb = panel->rowThumbRectForTest(path);
        const QColor checkerA = image.pixelColor(thumb.left() + 2, thumb.top() + 2);
        const QColor checkerB = image.pixelColor(thumb.left() + 5, thumb.top() + 2);
        const bool checker = checkerA.red() >= 0xB0 && checkerA.green() >= 0xB0
            && checkerB.red() >= 0xB0 && checkerB.green() >= 0xB0 && checkerA != checkerB;
        const QColor outline = image.pixelColor(thumb.left(), thumb.top() + thumb.height() / 2);
        const bool outlined = outline.red() < 0x40 && outline.green() < 0x40;
        const QColor bracket = image.pixelColor(thumb.left() + 2, thumb.top() - 1);
        const bool bracketed = nearWhite(bracket);
        panel->selectPaths(QStringList{}, QString());
        const QImage none = panel->rowImageForTest(path);
        const bool noBrackets = !nearWhite(none.pixelColor(thumb.left() + 2, thumb.top() - 1));
        const QString group = view->add_group_in(QString());
        panel->refresh();
        const QImage groupImage = panel->rowImageForTest(group);
        const QRect groupThumb = panel->rowThumbRectForTest(group);
        const bool groupNoChecker =
            !nearWhite(groupImage.pixelColor(groupThumb.left() + 2, groupThumb.top() + 2));
        ST_BEGIN("lpr_thumb_frame");
        ST_PASS("lpr_thumb_frame checker=%d outline=%d bracket=%d none=%d group=%d",
                checker ? 1 : 0, outlined ? 1 : 0, bracketed ? 1 : 0, noBrackets ? 1 : 0,
                groupNoChecker ? 1 : 0);
        if (!checker || !outlined || !bracketed || !noBrackets || !groupNoChecker) {
            return pictura::selfTest().fail(359, "thumbnail checker/frame/brackets");
        }
        frame.closeDocument(doc, false);
    }

    // lpr_row_fonts (360): the Background name is italic, ordinary names normal,
    // and a linked row is underlined through its role.
    {
        const bool created = frame.newDocument(QStringLiteral("RowFonts"), 8, 8,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("transparent"));
        PictureView* view = frame.activeView();
        if (!created || !view) {
            return pictura::selfTest().fail(360, "row fonts fixture");
        }
        const int doc = frame.activeDocumentIndex();
        panel->setView(view);
        view->background_from_layer(QStringLiteral("0"));
        panel->refresh();
        const bool bgItalic = panel->rowNameItalicForTest(QStringLiteral("0"));
        const bool bgNoUnderline = !panel->rowNameUnderlineForTest(QStringLiteral("0"));
        const QString plain = view->add_layer_in(QString());
        view->set_layer_name_path(plain, QStringLiteral("Plain"));
        const bool linked = view->link_layers(QStringList{plain}, true) > 0;
        panel->refresh();
        const bool plainNormal = !panel->rowNameItalicForTest(plain);
        const bool linkedRole = panel->rowLinkedForTest(plain);
        const bool linkedUnderline = panel->rowNameUnderlineForTest(plain);
        const bool placedRole = !panel->rowPlacedForTest(plain);
        ST_BEGIN("lpr_row_fonts");
        ST_PASS("lpr_row_fonts bgItalic=%d bgPlain=%d plainNormal=%d linked=%d underline=%d placed=%d",
                bgItalic ? 1 : 0, bgNoUnderline ? 1 : 0, plainNormal ? 1 : 0, linked ? 1 : 0,
                linkedUnderline ? 1 : 0, placedRole ? 1 : 0);
        if (!bgItalic || !bgNoUnderline || !plainNormal || !linked || !linkedRole
            || !linkedUnderline || !placedRole) {
            return pictura::selfTest().fail(360, "row typography roles");
        }
        frame.closeDocument(doc, false);
    }

    // lpr_row_height (361): the row height honours the named 28 px floor.
    {
        const int height = panel->rowHeightForTest();
        ST_BEGIN("lpr_row_height");
        ST_PASS("lpr_row_height height=%d", height);
        if (height < 28) {
            return pictura::selfTest().fail(361, "row height floor");
        }
    }

    // lpr_nesting_hidden (362): the panel hides the nesting-lock button while the
    // other lock controls remain.
    {
        const bool hidden = panel->lockNestingHiddenForTest();
        const bool others = panel->lockButtonCountForTest() == 5
            && panel->lockIconsPresentForTest();
        ST_BEGIN("lpr_nesting_hidden");
        ST_PASS("lpr_nesting_hidden hidden=%d others=%d", hidden ? 1 : 0, others ? 1 : 0);
        if (!hidden || !others) {
            return pictura::selfTest().fail(362, "nesting button hidden");
        }
    }

    // lpr_eye_gutter (387): the eye glyph is centred in its fixed gutter with
    // equal left/right padding, and a 1 px darker separator sits at the gutter's
    // right edge. lpr_thumb_gap (388): a non-expandable row's thumbnail starts
    // closer than a group's at the same depth. lpr_thumb_aspect (389): a wide
    // document's thumbnail is letterboxed, not stretched. lpr_drop_indicator
    // (390): a sibling gap yields a line and a drop-into a group an outline; an
    // invalid target yields none.
    {
        const bool created = frame.newDocument(QStringLiteral("FollowupCosmetics"), 20, 10,
                                               QStringLiteral("rgb"), 8,
                                               QStringLiteral("white"));
        PictureView* view = frame.activeView();
        if (!created || !view) {
            return pictura::selfTest().fail(387, "follow-up cosmetics fixture");
        }
        const int doc = frame.activeDocumentIndex();
        panel->setView(view);
        panel->setOptionsForTest(2, 1, false);
        const QString group = view->add_group_in(QString());
        view->add_layer_in(group);
        const QString a = view->add_layer_in(QString());
        const QString b = view->add_layer_in(QString());
        view->set_layer_name_path(a, QStringLiteral("A"));
        view->set_layer_name_path(b, QStringLiteral("B"));
        panel->refresh();

        const bool centred = panel->eyeGutterCentredForTest(a);
        const bool separator = panel->eyeSeparatorPresentForTest(a);
        ST_BEGIN("lpr_eye_gutter");
        ST_PASS("lpr_eye_gutter centred=%d separator=%d", centred ? 1 : 0, separator ? 1 : 0);
        if (!centred || !separator) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(387, "eye gutter centring/separator");
        }

        const int regularLeft = panel->rowThumbRectForTest(a).left();
        const int groupLeft = panel->rowThumbRectForTest(group).left();
        const bool closer = regularLeft >= 0 && groupLeft > regularLeft;
        ST_BEGIN("lpr_thumb_gap");
        ST_PASS("lpr_thumb_gap regular=%d group=%d", regularLeft, groupLeft);
        if (!closer) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(388, "non-group thumbnail gap");
        }

        const QRect thumb = panel->rowThumbRectForTest(a);
        const bool wide = thumb.width() > thumb.height() && thumb.height() > 0;
        const bool ratio = thumb.height() * 2 == thumb.width();
        ST_BEGIN("lpr_thumb_aspect");
        ST_PASS("lpr_thumb_aspect w=%d h=%d", thumb.width(), thumb.height());
        if (!wide || !ratio) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(389, "thumbnail aspect letterbox");
        }

        const int sibling = panel->dropIndicatorForTest(a, b, 1);
        const int into = panel->dropIndicatorForTest(a, group, 2);
        const int invalid = panel->dropIndicatorForTest(group, group, 2);
        ST_BEGIN("lpr_drop_indicator");
        ST_PASS("lpr_drop_indicator sibling=%d into=%d invalid=%d", sibling, into, invalid);
        if (sibling != 1 || into != 2 || invalid != 0) {
            frame.closeDocument(doc, false);
            return pictura::selfTest().fail(390, "custom drop indicator");
        }
        frame.closeDocument(doc, false);
    }

    // lpr_row_height_raised (391): a Medium-thumbnail row is at least 34 px tall,
    // from the single named floor shared by sizeHint and the delegate.
    {
        panel->setOptionsForTest(2, 1, false);
        const int height = panel->rowHeightForTest();
        ST_BEGIN("lpr_row_height_raised");
        ST_PASS("lpr_row_height_raised height=%d", height);
        if (height < 34) {
            return pictura::selfTest().fail(391, "raised row height");
        }
    }

    return 0;
}

} // namespace pictura
