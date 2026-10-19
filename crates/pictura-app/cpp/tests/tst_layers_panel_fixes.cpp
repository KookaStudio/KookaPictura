// The Layers panel manual-test pass (#378): the Opacity label alignment, the
// active thumbnail after adding a mask, keeping a dropped-into group expanded,
// the group icon outline, the thumbnail's vertical fill, and the chain glyph.
// Also #439: the bottom action strip's fixed-gap, centred geometry.

#include <QtTest/QtTest>
#include "api/layers_api.h"
#include "api/type_api.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QList>
#include <QtCore/QMimeData>
#include <QtCore/QModelIndex>
#include <QtCore/QPoint>
#include <QtCore/QPointF>
#include <QtCore/QRect>
#include <QtCore/QStringList>
#include <QtGui/QDragEnterEvent>
#include <QtGui/QDragMoveEvent>
#include <QtGui/QDropEvent>
#include <QtGui/QImage>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLayout>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QTreeView>

#include "frame.h"
#include "panels/layers_panel.h"
#include "panels/layers_panel_internal.h"
#include "panels/percent_field.h"
#include "selftest_paint_fixture.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/clipping.cxxqt.h"
#include "pictura_app/src/cxxqt_object/shapes.cxxqt.h"

#include "qt_test_support.h"

namespace {

pictura::layers_api::LayerRow rowNamed(pictura::PictureView& view, const QString& name)
{
    for (int i = 0; i < pictura::layers_api::rowCount(view); ++i) {
        const auto row = pictura::layers_api::rowAt(view, i);
        if (row.name == name) {
            return row;
        }
    }
    return {};
}

bool dropOnViewport(QWidget* viewport, const void* document, const QString& source,
                    const QPoint& pos)
{
    QMimeData mime;
    mime.setData(pictura::kLayerMimeType, source.toUtf8());
    mime.setData(pictura::kLayerSourceMimeType,
                 QByteArray::number(reinterpret_cast<quintptr>(document)));
    QDragEnterEvent enter(pos, Qt::MoveAction, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(viewport, &enter);
    QDragMoveEvent move(pos, Qt::MoveAction, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(viewport, &move);
    QDropEvent drop(QPointF(pos), Qt::MoveAction, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(viewport, &drop);
    return drop.isAccepted();
}

QList<QRect> actionStripRects(const pictura::LayersPanel& panel)
{
    const QStringList names = {
        QStringLiteral("layersStripLink"),
        QStringLiteral("layersStripFx"),
        QStringLiteral("layersStripMask"),
        QStringLiteral("layersStripFillAdjustment"),
        QStringLiteral("layersStripGroup"),
        QStringLiteral("layersStripNewLayer"),
        QStringLiteral("layersStripDelete"),
    };
    QList<QRect> rects;
    rects.reserve(names.size());
    for (const QString& name : names) {
        auto* button = panel.findChild<QToolButton*>(name);
        if (!button) {
            return {};
        }
        rects.push_back(button->geometry());
    }
    return rects;
}

bool actionStripGapsAreFixed(const QList<QRect>& rects)
{
    for (int i = 1; i < rects.size(); ++i) {
        const QRect& previous = rects.at(i - 1);
        const QRect& current = rects.at(i);
        if (current.x() - (previous.x() + previous.width()) != 2) {
            return false;
        }
    }
    return true;
}

bool actionStripIsCentred(const QList<QRect>& rects, int panelWidth)
{
    return !rects.isEmpty()
        && qAbs(rects.first().x()
                - (panelWidth - (rects.last().x() + rects.last().width())))
               <= 1;
}

} // namespace

class LayersPanelFixesTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void opacityLabelRightAligned();
    void dropIntoOpenGroupKeepsExpanded();
    void addingLayerKeepsGroupExpanded();
    void renameKeepsGroupExpanded();
    void duplicateGroupNamesStayDistinct();
    void groupIconHasNoOutline();
    void typeThumbnailIsSquareWithoutOutline();
    void labelGlyphTintFollowsFill();
    void groupIconIsLarger();
    void blendComboArrowAndKeys();
    void blendChangeHistoryLabel();
    void thumbnailFillsRow();
    void tallThumbnailKeepsDocumentAspect();
    void linkGlyphRotatedAndLarger();
    void dragBelowBaseReleasesClip();
    void legacyMoveLayerShiftsAndRefuses();
    void actionStripStaysFixedAndCentred();
    void bottomStripDisabledWithoutDocument();

private:
    bool setupNest();

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::LayersPanel* panel_ = nullptr;
    QString group_;
    QString a_;
};

void LayersPanelFixesTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void LayersPanelFixesTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
    view_ = nullptr;
    panel_ = nullptr;
    group_.clear();
    a_.clear();
}

bool LayersPanelFixesTest::setupNest()
{
    const bool created = window_->newDocument(QStringLiteral("Fixes"), 16, 16,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    if (!created || !view_ || !panel_) {
        return false;
    }
    group_ = view_->add_group_in(QString());
    a_ = view_->add_layer_in(group_);
    view_->set_layer_name_path(a_, QStringLiteral("A"));
    return true;
}

// The Opacity field's label shares its slack with the value box, so it is
// right-aligned against that box rather than floating at the row's left edge.
void LayersPanelFixesTest::opacityLabelRightAligned()
{
    QVERIFY2(setupNest(), "opacity label fixture");
    panel_->setView(view_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);
    QVERIFY2((panel_->opacityLabelAlignmentForTest() & Qt::AlignRight) != 0,
             "the Opacity label is right-aligned");
    auto* field = panel_->findChild<pictura::PercentField*>(QStringLiteral("layersOpacityField"));
    QVERIFY(field != nullptr);
    auto* edit = field->findChild<QLineEdit*>(QStringLiteral("percentEdit"));
    QVERIFY(edit != nullptr);
    const int labelRight = panel_->opacityLabelRightForTest();
    const int editLeft = edit->mapTo(field, QPoint(0, 0)).x();
    QVERIFY2(labelRight > 0 && editLeft - labelRight >= 0 && editLeft - labelRight <= 6,
             "the label text sits against the value box");
}

// Dropping a layer into an open group must leave the group expanded, even when
// the source's removal shifts the group's own index-based path.
void LayersPanelFixesTest::dropIntoOpenGroupKeepsExpanded()
{
    const bool created = window_->newDocument(QStringLiteral("DropInto"), 16, 16,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY2(created && view_ && panel_, "drop into fixture");
    const QString layer = view_->add_layer_in(QString());
    const QString group = view_->add_group_in(QString());
    view_->set_layer_name_path(layer, QStringLiteral("L"));
    view_->set_layer_name_path(group, QStringLiteral("G"));
    panel_->setView(view_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);
    panel_->expandForTest(group);
    QCoreApplication::processEvents();
    QTreeView* tree = panel_->findChild<QTreeView*>();
    QVERIFY(tree != nullptr);

    const int base = view_->history_count();
    const QRect row = panel_->rowViewportRectForTest(group);
    QVERIFY2(dropOnViewport(tree->viewport(), view_, layer, row.center()),
             "drop into the open group accepted");
    QCOMPARE(view_->history_count(), base + 1);
    QCoreApplication::processEvents();
    panel_->refresh();
    QCoreApplication::processEvents();

    // The group's path shifted when L left the root; find it and confirm it is
    // still expanded with the dropped layer as a visible child.
    QString groupNew;
    for (int i = 0; i < pictura::layers_api::rowCount(*view_); ++i) {
        const pictura::layers_api::LayerRow row = pictura::layers_api::rowAt(*view_, i);
        if (row.name == QStringLiteral("G")) {
            groupNew = row.path;
            break;
        }
    }
    QVERIFY2(!groupNew.isEmpty(), "the group still exists");
    const QStringList visible = panel_->visiblePathsForTest();
    QVERIFY2(visible.contains(groupNew), "the group is still displayed");
    bool childVisible = false;
    for (const QString& path : visible) {
        if (path.startsWith(groupNew + QLatin1Char('/'))) {
            childVisible = true;
            break;
        }
    }
    QVERIFY2(childVisible, "the group stayed expanded with the dropped layer visible");
}

// A structural change that shifts an expanded group's index path (a layer added
// outside the group) must leave the group expanded.
void LayersPanelFixesTest::addingLayerKeepsGroupExpanded()
{
    const bool created = window_->newDocument(QStringLiteral("AddKeep"), 16, 16,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY2(created && view_ && panel_, "add keep fixture");
    const QString group = view_->add_group_in(QString());
    const QString child = view_->add_layer_in(group);
    view_->set_layer_name_path(group, QStringLiteral("G"));
    view_->set_layer_name_path(child, QStringLiteral("C"));
    panel_->setView(view_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);
    panel_->expandForTest(group);
    QCoreApplication::processEvents();

    // Add a layer above the bottom-most root layer, which sits below the group,
    // so the group's index shifts.
    QString bottom;
    for (int i = 0; i < pictura::layers_api::rowCount(*view_); ++i) {
        const pictura::layers_api::LayerRow row = pictura::layers_api::rowAt(*view_, i);
        if (!row.path.contains(QLatin1Char('/'))) {
            bottom = row.path;
        }
    }
    QVERIFY2(!bottom.isEmpty(), "a root layer found");
    QVERIFY(!view_->add_layer_in(bottom).isEmpty());
    QCoreApplication::processEvents();
    panel_->refresh();
    QCoreApplication::processEvents();

    QString groupNew;
    for (int i = 0; i < pictura::layers_api::rowCount(*view_); ++i) {
        const pictura::layers_api::LayerRow row = pictura::layers_api::rowAt(*view_, i);
        if (row.name == QStringLiteral("G")) {
            groupNew = row.path;
            break;
        }
    }
    QVERIFY2(!groupNew.isEmpty(), "the group still exists");
    QVERIFY2(groupNew != group, "the group's index path shifted");
    const QStringList visible = panel_->visiblePathsForTest();
    bool childVisible = false;
    for (const QString& path : visible) {
        if (path.startsWith(groupNew + QLatin1Char('/'))) {
            childVisible = true;
            break;
        }
    }
    QVERIFY2(childVisible, "the group stayed expanded after the sibling was added");
}

// Renaming an expanded group keeps it expanded: a rename leaves the index path
// intact, so the panel's name-chain guard must not treat it as a shifted path.
void LayersPanelFixesTest::renameKeepsGroupExpanded()
{
    QVERIFY2(setupNest(), "rename fixture");
    panel_->setView(view_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);
    panel_->expandForTest(group_);
    QCoreApplication::processEvents();
    auto* tree = panel_->findChild<QTreeView*>();
    QVERIFY(tree != nullptr);

    // Rename through the inline editor, i.e. the model's setData path.
    QVERIFY2(panel_->beginRenameForTest(group_), "inline editor opens");
    QLineEdit* editor = tree->viewport()->findChild<QLineEdit*>();
    QVERIFY(editor != nullptr);
    editor->setText(QStringLiteral("Renamed"));
    QTest::keyClick(editor, Qt::Key_Return);
    QCoreApplication::processEvents();
    panel_->refresh();
    QCoreApplication::processEvents();

    QString renamed;
    for (int i = 0; i < pictura::layers_api::rowCount(*view_); ++i) {
        const pictura::layers_api::LayerRow row = pictura::layers_api::rowAt(*view_, i);
        if (row.name == QStringLiteral("Renamed")) {
            renamed = row.path;
            break;
        }
    }
    QVERIFY2(!renamed.isEmpty(), "the group was renamed");
    QCOMPARE(renamed, group_);
    const QStringList visible = panel_->visiblePathsForTest();
    bool childVisible = false;
    for (const QString& path : visible) {
        if (path.startsWith(group_ + QLatin1Char('/'))) {
            childVisible = true;
            break;
        }
    }
    QVERIFY2(childVisible, "the renamed group stayed expanded");
}

// Two sibling groups may share a user name; a path shift must re-find the one
// that was expanded, not merely the first name match.
void LayersPanelFixesTest::duplicateGroupNamesStayDistinct()
{
    const bool created = window_->newDocument(QStringLiteral("Dup"), 16, 16,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY2(created && view_ && panel_, "duplicate name fixture");
    const QString first = view_->add_group_in(QString());
    const QString second = view_->add_group_in(QString());
    view_->set_layer_name_path(first, QStringLiteral("G"));
    view_->set_layer_name_path(second, QStringLiteral("G"));
    const QString firstChild = view_->add_layer_in(first);
    const QString secondChild = view_->add_layer_in(second);
    view_->set_layer_name_path(firstChild, QStringLiteral("C1"));
    view_->set_layer_name_path(secondChild, QStringLiteral("C2"));
    panel_->setView(view_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);
    panel_->expandForTest(second);
    QCoreApplication::processEvents();

    // Add a layer below both groups so their index paths shift.
    QString bottom;
    for (int i = 0; i < pictura::layers_api::rowCount(*view_); ++i) {
        const pictura::layers_api::LayerRow row = pictura::layers_api::rowAt(*view_, i);
        if (!row.path.contains(QLatin1Char('/'))) {
            bottom = row.path;
        }
    }
    QVERIFY2(!bottom.isEmpty(), "a root layer found");
    QVERIFY(!view_->add_layer_in(bottom).isEmpty());
    QCoreApplication::processEvents();
    panel_->refresh();
    QCoreApplication::processEvents();

    QString child1;
    QString child2;
    for (int i = 0; i < pictura::layers_api::rowCount(*view_); ++i) {
        const pictura::layers_api::LayerRow row = pictura::layers_api::rowAt(*view_, i);
        if (row.name == QStringLiteral("C1")) {
            child1 = row.path;
        } else if (row.name == QStringLiteral("C2")) {
            child2 = row.path;
        }
    }
    QVERIFY(!child1.isEmpty() && !child2.isEmpty());
    const QStringList visible = panel_->visiblePathsForTest();
    QVERIFY2(visible.contains(child2), "the expanded duplicate-named group stayed expanded");
    QVERIFY2(!visible.contains(child1), "the other duplicate-named group stayed collapsed");
}

// A group's folder glyph carries no black outline; a regular layer's thumbnail
// keeps its outline.
void LayersPanelFixesTest::groupIconHasNoOutline()
{
    QVERIFY2(setupNest(), "group outline fixture");
    panel_->setView(view_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);
    panel_->expandForTest(group_);
    QCoreApplication::processEvents();

    const QImage layerImage = panel_->rowImageForTest(a_);
    const QRect layerThumb = panel_->rowThumbRectForTest(a_);
    QVERIFY(!layerImage.isNull() && !layerThumb.isEmpty());
    const QColor layerBorder = layerImage.pixelColor(layerThumb.center().x(), layerThumb.top());
    QVERIFY2(layerBorder.red() < 80 && layerBorder.green() < 80 && layerBorder.blue() < 80,
             "a layer thumbnail keeps its black outline");

    const QImage groupImage = panel_->rowImageForTest(group_);
    const QRect groupThumb = panel_->rowThumbRectForTest(group_);
    QVERIFY(!groupImage.isNull() && !groupThumb.isEmpty());
    const QColor groupBorder = groupImage.pixelColor(groupThumb.center().x(), groupThumb.top());
    QVERIFY2(groupBorder.red() > 100 && groupBorder.green() > 100 && groupBorder.blue() > 100,
             "a group's folder glyph has no black outline");
}

// A type layer's thumbnail is a square (1:1) "T" card with no black outline for
// any document aspect ratio, and the hit rectangle is that same square card; a
// regular layer on the same portrait document stays letterboxed and outlined.
void LayersPanelFixesTest::typeThumbnailIsSquareWithoutOutline()
{
    QImage seed(80, 160, QImage::Format_RGB32);
    seed.fill(Qt::white);
    paint_fixture::Fixture f(*window_, seed, QStringLiteral("pictura_type_thumb"));
    QVERIFY2(f.ok(), "type thumbnail fixture");
    window_->setActiveTool(pictura::ToolId::HorizontalType);
    f.drag({QPointF(10, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("Hi"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    window_->setActiveTool(pictura::ToolId::Move);

    QString typePath;
    QString pixelPath;
    for (int i = 0; i < f.view->layer_count(); ++i) {
        const QString path = QString::number(i);
        if (pictura::type_api::isType(*f.view, path)) {
            typePath = path;
        } else if (pixelPath.isEmpty()) {
            pixelPath = path;
        }
    }
    QVERIFY2(!typePath.isEmpty(), "a type layer");
    QVERIFY2(!pixelPath.isEmpty(), "a regular layer");

    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel != nullptr);
    panel->setView(f.view);
    panel->refresh();
    window_->show();
    QTest::qWait(50);

    // The portrait document letterboxes a regular thumbnail and outlines it...
    const QRect pixelThumb = panel->rowThumbRectForTest(pixelPath);
    QVERIFY2(!pixelThumb.isEmpty(), "regular thumbnail laid out");
    QVERIFY2(pixelThumb.height() > pixelThumb.width(), "a regular thumbnail letterboxes");
    const QImage pixelImage = panel->rowImageForTest(pixelPath);
    QVERIFY(!pixelImage.isNull());
    const QColor pixelBorder = pixelImage.pixelColor(pixelThumb.center().x(), pixelThumb.top());
    QVERIFY2(pixelBorder.red() < 80 && pixelBorder.green() < 80 && pixelBorder.blue() < 80,
             "a regular thumbnail keeps its black outline");

    // ...but the type card is square, and its hit rect is that square card.
    const QRect typeThumb = panel->rowThumbRectForTest(typePath);
    QVERIFY2(!typeThumb.isEmpty(), "type thumbnail laid out");
    QCOMPARE(typeThumb.width(), typeThumb.height());

    // No black outline: the card's top edge at its centre is the white card.
    const QImage image = panel->rowImageForTest(typePath);
    QVERIFY(!image.isNull());
    const QColor border = image.pixelColor(typeThumb.center().x(), typeThumb.top());
    QVERIFY2(border.red() > 100 && border.green() > 100 && border.blue() > 100,
             "the type card has no black outline");
}

// The eye glyph flips to a contrasting tint at the sheet-specific qGray split;
// a None label keeps the theme glyph.
void LayersPanelFixesTest::labelGlyphTintFollowsFill()
{
    for (int label = 1; label <= 7; ++label) {
        const QColor fill = pictura::layerLabelColor(label);
        const QColor tint = pictura::LayerRowDelegate::labelGlyphTint(fill);
        QVERIFY2(tint.isValid(), "a labeled row tints the glyph");
        const bool light = tint == QColor(240, 240, 240);
        QCOMPARE(light, label == 6);
        QVERIFY2(tint == QColor(20, 20, 20) || light,
                 "the glyph tint is one of the two contrast colors");
    }
    QVERIFY2(!pictura::LayerRowDelegate::labelGlyphTint(QColor()).isValid(),
             "a None label leaves the theme glyph");
}

// The folder glyph is larger than the old half-thumbnail size, close to a layer
// thumbnail.
void LayersPanelFixesTest::groupIconIsLarger()
{
    QVERIFY2(setupNest(), "group icon fixture");
    panel_->setView(view_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);
    auto* delegate = static_cast<pictura::LayerRowDelegate*>(panel_->itemDelegateForTest());
    QVERIFY(delegate != nullptr);
    const int side = delegate->groupIconSide();
    QVERIFY2(side > delegate->thumbSize() / 2, "the folder glyph is larger than half");
    QVERIFY2(side >= (delegate->thumbSize() * 3) / 4,
             "the folder glyph is about three quarters of the thumbnail");
}

// A tall document's thumbnail fills the row's vertical extent rather than
// floating in a square box.
void LayersPanelFixesTest::thumbnailFillsRow()
{
    const bool created = window_->newDocument(QStringLiteral("Portrait"), 10, 20,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY2(created && view_ && panel_, "portrait fixture");
    panel_->setView(view_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);

    const QString layer = pictura::layers_api::rowAt(*view_, 0).path;
    const QRect thumb = panel_->rowThumbRectForTest(layer);
    QVERIFY2(!thumb.isEmpty(), "portrait thumbnail laid out");
    QVERIFY2(thumb.height() > thumb.width(), "a tall thumbnail fills vertically");
    QCOMPARE(thumb.height(),
             panel_->rowHeightForTest() - 2 * pictura::LayerRowDelegate::kThumbVerticalPad);
}

// A tall document's Entire Document thumbnail keeps the document aspect, so a
// full-width layer reaches the right edge instead of leaving a transparent
// column that the delegate would then squash into its letterboxed rect.
void LayersPanelFixesTest::tallThumbnailKeepsDocumentAspect()
{
    const bool created = window_->newDocument(QStringLiteral("TallAspect"), 10, 20,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    view_ = window_->activeView();
    QVERIFY2(created && view_, "tall fixture");

    const pictura::layers_api::Snapshot snap =
        pictura::layers_api::snapshot(*view_, 24, true);
    QVERIFY2(!snap.rows.isEmpty(), "positioned rows");
    const QImage thumb = snap.rows.first().thumbnail;
    QVERIFY2(!thumb.isNull(), "positioned thumbnail");
    QCOMPARE(thumb.width() * 2, thumb.height());
    QCOMPARE(thumb.pixelColor(thumb.width() - 1, thumb.height() / 2).alpha(), 255);
    window_->closeDocument(window_->activeDocumentIndex(), false);
}

// The mask chain glyph is drawn two pixels larger than its slot and rotated a
// quarter turn so it reads vertically between the thumbnails.
void LayersPanelFixesTest::linkGlyphRotatedAndLarger()
{
    const bool created = window_->newDocument(QStringLiteral("LinkGlyph"), 16, 16,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY2(created && view_ && panel_, "link glyph fixture");
    const QString layer = pictura::layers_api::rowAt(*view_, 0).path;
    view_->set_active_layer(layer);
    QVERIFY2(pictura::layer_mask_add(*view_, QStringLiteral("reveal-all")), "add mask");
    panel_->setView(view_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);

    QCOMPARE(panel_->linkIconSideForTest(), pictura::LayerRowDelegate::linkSide(24) + 2);

    const QRect slot = panel_->rowLinkGlyphRectForTest(layer);
    QVERIFY2(!slot.isEmpty(), "mask link slot laid out");
    const QImage image = panel_->rowImageForTest(layer);
    QVERIFY(!image.isNull());
    const QColor surface = image.pixelColor(slot.left() - 2, slot.center().y());
    int minX = slot.right();
    int maxX = slot.left();
    int minY = slot.bottom();
    int maxY = slot.top();
    bool any = false;
    for (int y = slot.top() - 2; y <= slot.bottom() + 2; ++y) {
        for (int x = slot.left() - 2; x <= slot.right() + 2; ++x) {
            if (x < 0 || y < 0 || x >= image.width() || y >= image.height()) {
                continue;
            }
            const QColor c = image.pixelColor(x, y);
            const int diff = qAbs(c.red() - surface.red()) + qAbs(c.green() - surface.green())
                + qAbs(c.blue() - surface.blue());
            if (diff > 60) {
                any = true;
                minX = qMin(minX, x);
                maxX = qMax(maxX, x);
                minY = qMin(minY, y);
                maxY = qMax(maxY, y);
            }
        }
    }
    QVERIFY2(any, "chain glyph ink found");
    QVERIFY2(maxY - minY > maxX - minX, "the chain glyph is vertical (rotated 90 degrees)");
}

// A focused blend-mode field cycles its options with Up/Down, and the global
// stylesheet gives every combo a drop-down arrow.
void LayersPanelFixesTest::blendComboArrowAndKeys()
{
    QVERIFY2(setupNest(), "blend combo fixture");
    panel_->setView(view_);
    panel_->selectPaths({a_}, a_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);
    auto* combo = panel_->findChild<QComboBox*>(QStringLiteral("layersBlendCombo"));
    QVERIFY(combo != nullptr);
    QVERIFY2(combo->isEnabled(), "the blend combo is enabled for the selected layer");

    // The drop-down strip carries the chevron: ink sits in a narrow band inside
    // the frame, and the lowest row is the narrowest (the glyph converges
    // downward). Frame/border rows are excluded so only the glyph is measured.
    const QImage shot = combo->grab().toImage();
    QVERIFY(!shot.isNull());
    const QColor surface = shot.pixelColor(2, shot.height() / 2);
    const auto inkAt = [&](int y) {
        int count = 0;
        for (int x = qMax(0, shot.width() - 16); x < shot.width() - 1; ++x) {
            const QColor c = shot.pixelColor(x, y);
            const int diff = qAbs(c.red() - surface.red()) + qAbs(c.green() - surface.green())
                + qAbs(c.blue() - surface.blue());
            if (diff > 60) {
                ++count;
            }
        }
        return count;
    };
    int minY = shot.height();
    int maxY = -1;
    QVector<int> widths(shot.height(), 0);
    for (int y = 2; y < shot.height() - 2; ++y) {
        widths[y] = inkAt(y);
        if (widths[y] > 0) {
            minY = qMin(minY, y);
            maxY = qMax(maxY, y);
        }
    }
    QVERIFY2(maxY >= minY, "the drop-down chevron is visible in the strip");
    int widest = minY;
    for (int y = minY; y <= maxY; ++y) {
        if (widths[y] > widths[widest]) {
            widest = y;
        }
    }
    QVERIFY2(widths[maxY] < widths[widest], "the drop-down chevron points down");

    combo->setFocus();
    QCoreApplication::processEvents();
    QVERIFY2(combo->hasFocus(), "the blend field takes focus");
    QCOMPARE(combo->currentIndex(), 0);
    QTest::keyClick(combo, Qt::Key_Down);
    QCoreApplication::processEvents();
    QCOMPARE(combo->currentIndex(), 1);
    // A blend change refreshes the panel; the field must keep focus so the next
    // arrow key still cycles it.
    QVERIFY2(combo->hasFocus(), "the field keeps focus after a blend change");
    QTest::keyClick(combo, Qt::Key_Down);
    QCoreApplication::processEvents();
    QCOMPARE(combo->currentIndex(), 2);
    QVERIFY2(combo->hasFocus(), "the field keeps focus after a second change");
    QTest::keyClick(combo, Qt::Key_Up);
    QCoreApplication::processEvents();
    QCOMPARE(combo->currentIndex(), 1);
}

// A blend-mode change records its mode name in the history label.
void LayersPanelFixesTest::blendChangeHistoryLabel()
{
    QVERIFY2(setupNest(), "blend history fixture");
    panel_->setView(view_);
    panel_->selectPaths({a_}, a_);
    panel_->refresh();
    window_->show();
    QTest::qWait(50);
    auto* combo = panel_->findChild<QComboBox*>(QStringLiteral("layersBlendCombo"));
    QVERIFY(combo != nullptr);

    const int burn = combo->findData(QStringLiteral("idiv"));
    QVERIFY(burn >= 0);
    const int base = view_->history_count();
    combo->setCurrentIndex(burn);
    QCoreApplication::processEvents();
    QCOMPARE(view_->history_count(), base + 1);
    QCOMPARE(view_->history_label(view_->history_index()),
             QStringLiteral("Blending Change: Color Burn"));

    const int lum = combo->findData(QStringLiteral("lum "));
    QVERIFY(lum >= 0);
    combo->setCurrentIndex(lum);
    QCoreApplication::processEvents();
    QCOMPARE(view_->history_label(view_->history_index()),
             QStringLiteral("Blending Change: Luminosity"));
}

// Issue #658: dragging a clipped layer below its base releases the clip, and
// the panel glyph follows the flag.
void LayersPanelFixesTest::dragBelowBaseReleasesClip()
{
    QVERIFY(window_->newDocument(QStringLiteral("DragClip"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view_ && panel_);

    pictura::ShapeSpec square{};
    square.kind = 0;
    square.boxed = true;
    square.x0 = 2;
    square.y0 = 2;
    square.x1 = 8;
    square.y1 = 8;
    const QString base = pictura::shape_add_layer(*view_, square, 0xffff0000u);
    const QString fill = view_->add_solid_fill(0xff00ff00u);
    QVERIFY(!base.isEmpty() && !fill.isEmpty());
    view_->set_layer_name_path(base, QStringLiteral("Base"));
    view_->set_layer_name_path(fill, QStringLiteral("Fill"));

    const QRgb green = 0xff00ff00u;
    const QRgb white = 0xffffffffu;
    QVERIFY(pictura::clipping_toggle(*view_, fill));
    QCoreApplication::processEvents();
    panel_->refresh();
    QVERIFY(rowNamed(*view_, QStringLiteral("Fill")).clipping);
    QCOMPARE(view_->sample_argb(12, 12), white);
    QCOMPARE(view_->sample_argb(4, 4), green);

    const QString fillPath = rowNamed(*view_, QStringLiteral("Fill")).path;
    const QString basePath = rowNamed(*view_, QStringLiteral("Base")).path;
    QVERIFY(!fillPath.isEmpty() && !basePath.isEmpty());
    QVERIFY(view_->move_layer_to(fillPath, basePath, 1));
    QCoreApplication::processEvents();
    panel_->refresh();

    const auto movedFill = rowNamed(*view_, QStringLiteral("Fill"));
    const auto movedBase = rowNamed(*view_, QStringLiteral("Base"));
    QVERIFY(!movedFill.clipping);
    QVERIFY(!panel_->rowClippingForTest(movedFill.path));
    QVERIFY(!panel_->rowClipBaseForTest(movedBase.path));
    QCOMPARE(view_->sample_argb(12, 12), green);
}

// The legacy top-level move bridge shifts by delta, releases a run when its
// base leaves, and refuses the Background and fully locked layers.
void LayersPanelFixesTest::legacyMoveLayerShiftsAndRefuses()
{
    QVERIFY(window_->newDocument(QStringLiteral("LegacyMove1"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    QVERIFY(view_);
    QVERIFY(!view_->move_layer(0, 1));
    const QString a = view_->add_layer_in(QString());
    const QString b = view_->add_layer_in(QString());
    view_->set_layer_name_path(a, QStringLiteral("A"));
    view_->set_layer_name_path(b, QStringLiteral("B"));
    QVERIFY(pictura::clipping_toggle(*view_, b));
    QVERIFY(view_->move_layer(2, -1));
    QCoreApplication::processEvents();
    QCOMPARE(rowNamed(*view_, QStringLiteral("B")).path, QStringLiteral("1"));
    QVERIFY(!rowNamed(*view_, QStringLiteral("B")).clipping);

    QVERIFY(window_->newDocument(QStringLiteral("LegacyMove2"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    QVERIFY(view_);
    const QString a2 = view_->add_layer_in(QString());
    const QString b2 = view_->add_layer_in(QString());
    const QString c2 = view_->add_layer_in(QString());
    view_->set_layer_name_path(a2, QStringLiteral("A"));
    view_->set_layer_name_path(b2, QStringLiteral("B"));
    view_->set_layer_name_path(c2, QStringLiteral("C"));
    QVERIFY(pictura::clipping_toggle(*view_, b2));
    QVERIFY(view_->move_layer(1, 2));
    QCoreApplication::processEvents();
    QCOMPARE(rowNamed(*view_, QStringLiteral("A")).path, QStringLiteral("3"));
    QVERIFY(!rowNamed(*view_, QStringLiteral("B")).clipping);

    QVERIFY(view_->set_layer_lock(1, QStringLiteral("all"), true));
    QVERIFY(!view_->move_layer(1, -1));
}

// The bottom action strip keeps 2 px gaps and stays centred when the panel is
// widened, instead of distributing the surplus width between its buttons.
void LayersPanelFixesTest::actionStripStaysFixedAndCentred()
{
    pictura::LayersPanel panel;
    panel.ensurePolished();
    const int narrow = panel.minimumSizeHint().width() + 80;
    const int wide = narrow + 240;
    panel.resize(narrow, 420);
    panel.show();
    QTest::qWait(50);
    QCOMPARE(panel.width(), narrow);

    const QList<QRect> narrowRects = actionStripRects(panel);
    QCOMPARE(narrowRects.size(), 7);
    QVERIFY2(actionStripGapsAreFixed(narrowRects), "the narrow strip has 2 px gaps");
    QVERIFY2(actionStripIsCentred(narrowRects, panel.width()),
             "the narrow strip is centred");

    panel.resize(wide, panel.height());
    QVERIFY(panel.layout() != nullptr);
    panel.layout()->activate();
    QTest::qWait(50);
    QCOMPARE(panel.width(), wide);

    const QList<QRect> wideRects = actionStripRects(panel);
    QCOMPARE(wideRects.size(), 7);
    QVERIFY2(actionStripGapsAreFixed(wideRects), "the wide strip has 2 px gaps");
    QVERIFY2(actionStripIsCentred(wideRects, panel.width()),
             "the wide strip stays centred");
    for (int i = 0; i < wideRects.size(); ++i) {
        QCOMPARE(wideRects.at(i).width(), narrowRects.at(i).width());
    }
}

// With no document open, every action-strip button is inert.
void LayersPanelFixesTest::bottomStripDisabledWithoutDocument()
{
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel != nullptr);
    panel->setView(nullptr);

    const QStringList names = {
        QStringLiteral("layersStripLink"),
        QStringLiteral("layersStripFx"),
        QStringLiteral("layersStripMask"),
        QStringLiteral("layersStripFillAdjustment"),
        QStringLiteral("layersStripGroup"),
        QStringLiteral("layersStripNewLayer"),
        QStringLiteral("layersStripDelete"),
    };
    for (const QString& name : names) {
        auto* button = panel->findChild<QToolButton*>(name);
        QVERIFY2(button != nullptr, qPrintable(name));
        QVERIFY2(!button->isEnabled(), qPrintable(name));
    }
}

QTEST_MAIN(LayersPanelFixesTest)
#include "tst_layers_panel_fixes.moc"
