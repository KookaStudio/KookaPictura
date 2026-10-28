// The Layers snapshot crossing: `layers_api::snapshot` is field-for-field the
// legacy per-getter sequence (the equivalence half retires with the getters),
// and the fixture's known rows carry the expected typed values.

#include <QtTest/QtTest>

#include "api/layers_api.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/clipping.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/new_document.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layers_surface.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/vector_masks.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_style.cxxqt.h"
#include "pictura_app/src/cxxqt_object/shapes.cxxqt.h"

#include <QtCore/QTemporaryDir>
#include <QtGui/QImage>

#include <memory>

namespace {

using pictura::layers_api::LayerRow;
using pictura::layers_api::Snapshot;

// A built fixture's handles. Layer paths are positional and shift on every
// insert, so these are names; resolve them with pathOf at use time.
struct Fixture {
    const QString pix = QStringLiteral("Pix");
    const QString grp = QStringLiteral("Grp");
    const QString nested = QStringLiteral("Nested");
    const QString adjustment = QStringLiteral("Invert");
    const QString shape = QStringLiteral("Rectangle 1");
    const QString fill = QStringLiteral("Color Fill 1");
    const QString placed = QStringLiteral("Placed");
};

QString pathOf(pictura::PictureView& view, const QString& name)
{
    for (const LayerRow& row : pictura::layers_api::rows(view)) {
        if (row.name == name) {
            return row.path;
        }
    }
    return QString();
}

// Background + a pixel layer (smart object, style, Blend If, vector mask,
// label, lock) + a group with a masked child + a shape with a clipped colour
// fill + an adjustment + an embedded 8x8 place, with the first two linked.
bool buildFixture(std::unique_ptr<pictura::PictureView>& view, QString* why = nullptr)
{
    const auto fail = [why](const char* step) {
        if (why) {
            *why = QString::fromLatin1(step);
        }
        return false;
    };
    view = std::make_unique<pictura::PictureView>();
    if (!pictura::create_document(*view, 16, 16, QStringLiteral("rgb"), 8, 0xffffffu, false, 72.0,
                                  false)) {
        return fail("create_document");
    }
    const QString pix = view->add_layer_in(QString());
    const QString grp = view->add_group_in(QString());
    const QString nested = view->add_layer_in(grp);
    if (pix.isEmpty() || grp.isEmpty() || nested.isEmpty()
        || !view->set_layer_name_path(pix, QStringLiteral("Pix"))
        || !view->set_layer_name_path(grp, QStringLiteral("Grp"))
        || !view->set_layer_name_path(nested, QStringLiteral("Nested"))) {
        return fail("build rows");
    }
    if (!view->convert_to_smart_object(pix)) {
        return fail("convert smart");
    }
    view->set_active_layer(pix);
    pictura::ShapeSpec square{};
    square.kind = 0;
    square.boxed = true;
    square.x0 = 2;
    square.y0 = 2;
    square.x1 = 8;
    square.y1 = 8;
    const QString shape = pictura::shape_add_layer(*view, square, 0xffff0000u);
    const QString fill = view->add_solid_fill(0xff00ff00u);
    if (shape.isEmpty() || fill.isEmpty()
        || pictura::clipping_create(*view, QStringList{fill}) != 1) {
        return fail("shape/fill/clip");
    }
    if (!view->add_adjustment(QStringLiteral("invert"))) {
        return fail("add adjustment");
    }
    QTemporaryDir dir;
    if (!dir.isValid()) {
        return fail("temp dir");
    }
    const QString png = dir.filePath(QStringLiteral("placed.png"));
    QImage placedImage(8, 8, QImage::Format_ARGB32);
    placedImage.fill(0xff00ff00u);
    if (!placedImage.save(png)) {
        return fail("save png");
    }
    const QString placed = view->place_image(png);
    if (placed.isEmpty()
        || !view->set_layer_name_path(placed, QStringLiteral("Placed"))) {
        return fail("place image");
    }
    const QString pixPath = pathOf(*view, QStringLiteral("Pix"));
    const QString grpPath = pathOf(*view, QStringLiteral("Grp"));
    if (pixPath.isEmpty() || grpPath.isEmpty()
        || view->set_layers_opacity(QStringList{pixPath}, 128) != 1
        || view->set_layers_fill(QStringList{pixPath}, 100) != 1
        || view->set_layers_blend(QStringList{pixPath}, QStringLiteral("mul ")) != 1
        || view->set_layers_color(QStringList{pixPath}, 3) != 1
        || view->set_layers_lock(QStringList{pixPath}, QStringLiteral("transparency"), true) != 1
        || !pictura::layer_style_set(*view, pixPath, QStringLiteral("colorOverlay.on"), 1.0)
        || !pictura::layer_style_set_blend_if(*view, pixPath, 0, 40000)) {
        return fail("pix properties");
    }
    const QString nestedPath = pathOf(*view, QStringLiteral("Nested"));
    view->set_active_layer(nestedPath);
    if (!pictura::layer_mask_add(*view, QStringLiteral("reveal-all"))) {
        return fail("raster mask");
    }
    if (view->link_layers(QStringList{pixPath, grpPath}, true) != 2) {
        return fail("link layers");
    }
    view->set_active_layer(pixPath);
    if (!pictura::vector_mask_add(*view, QStringLiteral("reveal-all"))) {
        return fail("vector mask");
    }
    // The mask edit target belongs to the active layer, so it is set after the
    // last active-layer change.
    if (!pictura::mask_edit_target_set(*view, nestedPath)) {
        return fail("mask target");
    }
    return true;
}

const LayerRow& rowFor(const Snapshot& snap, pictura::PictureView& view,
                       const QString& name)
{
    static const LayerRow none;
    const QString path = pathOf(view, name);
    for (const LayerRow& row : snap.rows) {
        if (row.path == path) {
            return row;
        }
    }
    return none;
}

} // namespace

class LayersSnapshotTest : public QObject {
    Q_OBJECT

private slots:
    void fixtureExpectations();
    void entireDocumentPlacement();
    void adjustmentThumbnailKindIcon();
    void fillThumbnailPreviews();
    void fillThumbnailIsSquare();
};

void LayersSnapshotTest::fixtureExpectations()
{
    std::unique_ptr<pictura::PictureView> view;
    QString why;
    QVERIFY2(buildFixture(view, &why), qPrintable(why));
    const Fixture fx;

    const Snapshot snap = pictura::layers_api::snapshot(*view, 24, false);
    QCOMPARE(snap.documentWidth, 16);
    QCOMPARE(snap.documentHeight, 16);
    QCOMPARE(snap.thumbSize, 24);
    QCOMPARE(snap.maskTargetPath, pathOf(*view, fx.nested));

    const LayerRow& background
        = rowFor(snap, *view, QStringLiteral("Background"));
    QCOMPARE(background.kind, QStringLiteral("background"));
    QCOMPARE(background.depth, 0);
    QVERIFY(background.visible);
    QCOMPARE(background.blend, QStringLiteral("norm"));
    QCOMPARE(background.opacity, 255);
    QCOMPARE(background.fill, 255);
    QCOMPARE(background.lockBits, 0x05);
    QVERIFY(!background.hasMask);
    QVERIFY(!background.smartObject);
    QVERIFY(!background.thumbnail.isNull());
    QVERIFY(background.maskThumbnail.isNull());

    const LayerRow& pix = rowFor(snap, *view, fx.pix);
    QCOMPARE(pix.kind, QStringLiteral("pixel"));
    QCOMPARE(pix.depth, 0);
    QCOMPARE(pix.blend, QStringLiteral("mul "));
    QCOMPARE(pix.opacity, 128);
    QCOMPARE(pix.fill, 100);
    QCOMPARE(pix.lockBits, 0x01);
    QCOMPARE(pix.color, 3);
    QVERIFY(pix.visible);
    QVERIFY(pix.smartObject);
    QVERIFY(!pix.placed);
    QVERIFY(pix.hasStyle);
    QCOMPARE(pix.styleEffects, QStringList{QStringLiteral("colorOverlay")});
    QCOMPARE(pix.styleEffectsOn, QStringList{QStringLiteral("colorOverlay")});
    QVERIFY(pix.styleVisible);
    QVERIFY(pix.hasBlendIf);
    QVERIFY(pix.hasVectorMask);
    QVERIFY(pix.linked);
    QVERIFY(!pix.thumbnail.isNull());
    QVERIFY(!pix.vectorMaskThumbnail.isNull());

    const LayerRow& grp = rowFor(snap, *view, fx.grp);
    QCOMPARE(grp.kind, QStringLiteral("group"));
    QVERIFY(grp.expandable);
    QCOMPARE(grp.childCount, 1);
    QVERIFY(grp.linked);
    QVERIFY(grp.thumbnail.isNull());

    const LayerRow& nested = rowFor(snap, *view, fx.nested);
    QCOMPARE(nested.depth, 1);
    QCOMPARE(nested.kind, QStringLiteral("pixel"));
    QVERIFY(nested.hasMask);
    QVERIFY(nested.maskLinked);
    QVERIFY(!nested.maskDisabled);
    QVERIFY(!nested.maskThumbnail.isNull());

    const LayerRow& shape = rowFor(snap, *view, fx.shape);
    QVERIFY(shape.shape);
    QCOMPARE(shape.lockBits, 0x03);
    QVERIFY(!shape.thumbnail.isNull());

    const LayerRow& fill = rowFor(snap, *view, fx.fill);
    QVERIFY(fill.clipping);
    QVERIFY(fill.hasAdjustment);

    const LayerRow& adjustment = rowFor(snap, *view, fx.adjustment);
    QCOMPARE(adjustment.kind, QStringLiteral("adjustment"));
    QVERIFY(adjustment.hasAdjustment);
    QVERIFY(!adjustment.expandable);
    QVERIFY(adjustment.thumbnail.isNull());

    const LayerRow& placedRow = rowFor(snap, *view, fx.placed);
    QVERIFY(placedRow.smartObject);
    QVERIFY(!placedRow.placed);
    QCOMPARE(placedRow.depth, 0);
}

void LayersSnapshotTest::entireDocumentPlacement()
{
    std::unique_ptr<pictura::PictureView> view;
    QString why;
    QVERIFY2(buildFixture(view, &why), qPrintable(why));
    const Fixture fx;

    const Snapshot bounded = pictura::layers_api::snapshot(*view, 24, false);
    const Snapshot positioned = pictura::layers_api::snapshot(*view, 24, true);
    QCOMPARE(positioned.rows.size(), bounded.rows.size());
    for (int i = 0; i < bounded.rows.size(); ++i) {
        const LayerRow& row = bounded.rows.at(i);
        const LayerRow& wide = positioned.rows.at(i);
        QCOMPARE(wide.path, row.path);
        if (row.kind == QStringLiteral("group")
            || row.kind == QStringLiteral("adjustment")) {
            QCOMPARE(wide.thumbnail, row.thumbnail);
        }
    }
    QVERIFY2(rowFor(positioned, *view, fx.placed).thumbnail
            != rowFor(bounded, *view, fx.placed).thumbnail,
        "the 8x8 place is positioned in the 16x16 canvas, not bounds-filling");
}

void LayersSnapshotTest::adjustmentThumbnailKindIcon()
{
    std::unique_ptr<pictura::PictureView> view;
    QString why;
    QVERIFY2(buildFixture(view, &why), qPrintable(why));
    const Fixture fx;
    const Snapshot snap = pictura::layers_api::snapshot(*view, 24, false);
    const LayerRow& adjustment = rowFor(snap, *view, fx.adjustment);
    QCOMPARE(adjustment.adjustmentKind, QStringLiteral("invert"));
    QVERIFY2(adjustment.thumbnail.isNull(), "the delegate paints the kind icon");
    const LayerRow& fill = rowFor(snap, *view, fx.fill);
    QCOMPARE(fill.adjustmentKind, QStringLiteral("solid-color"));
    QVERIFY2(!fill.thumbnail.isNull(), "a fill row carries a content preview");
}

void LayersSnapshotTest::fillThumbnailPreviews()
{
    std::unique_ptr<pictura::PictureView> view;
    QString why;
    QVERIFY2(buildFixture(view, &why), qPrintable(why));
    const Snapshot snap = pictura::layers_api::snapshot(*view, 24, false);
    const LayerRow& solid = rowFor(snap, *view, Fixture().fill);
    QVERIFY(!solid.thumbnail.isNull());
    const QColor swatch =
        solid.thumbnail.pixelColor(solid.thumbnail.width() / 2, solid.thumbnail.height() / 2);
    QVERIFY2(swatch.green() > 200 && swatch.red() < 60, "the solid preview is the fill colour");

    const QString gradient = view->add_gradient_fill();
    QVERIFY(!gradient.isEmpty());
    const Snapshot withGradient = pictura::layers_api::snapshot(*view, 24, false);
    pictura::layers_api::LayerRow gradientRow;
    for (const auto& row : withGradient.rows) {
        if (row.path == gradient) {
            gradientRow = row;
        }
    }
    QCOMPARE(gradientRow.adjustmentKind, QStringLiteral("gradient"));
    QVERIFY(!gradientRow.thumbnail.isNull());
}

// A fill row's preview image is square (1:1) for a non-square document,
// matching the panel's square fill card (#982).
void LayersSnapshotTest::fillThumbnailIsSquare()
{
    auto view = std::make_unique<pictura::PictureView>();
    QVERIFY(pictura::create_document(*view, 20, 10, QStringLiteral("rgb"), 8, 0xffffffu, false,
                                     72.0, false));
    const QString fill = view->add_solid_fill(0xff00ff00u);
    QVERIFY(!fill.isEmpty());
    for (const bool entireDocument : {false, true}) {
        const Snapshot snap = pictura::layers_api::snapshot(*view, 24, entireDocument);
        const LayerRow& row = rowFor(snap, *view, QStringLiteral("Color Fill 1"));
        QVERIFY2(!row.thumbnail.isNull(), "the fill preview is present");
        QCOMPARE(row.thumbnail.width(), row.thumbnail.height());
    }
}

QTEST_MAIN(LayersSnapshotTest)
#include "tst_layers_snapshot.moc"
