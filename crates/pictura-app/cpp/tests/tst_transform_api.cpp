// Unit C: transform_api as the typed façade over layer_rect, transform_quad,
// and the preview matrix, replacing the string re-parse at each consumer.

#include <QtTest/QtTest>

#include "api/document_api.h"
#include "api/transform_api.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

using namespace pictura;

class TransformApiTest : public QObject {
    Q_OBJECT

private slots:
    void layerRectIsTypedAndMatchesTheLegacyString();
    void quadAndPreviewMatrixMatchTheLegacyFormat();
};

void TransformApiTest::layerRectIsTypedAndMatchesTheLegacyString()
{
    PictureView view;
    QVERIFY(document_api::newDocument(view, 800, 600, QStringLiteral("rgb"), 8,
                                      QStringLiteral("white")));

    const transform_api::LayerRect rect = transform_api::layerRect(view, QStringLiteral("0"));
    QVERIFY(rect.valid);
    QCOMPARE(rect.left, 0);
    QCOMPARE(rect.top, 0);
    QCOMPARE(rect.right, 800);
    QCOMPARE(rect.bottom, 600);
    QCOMPARE(rect.width(), 800);
    QCOMPARE(rect.height(), 600);
    // Equivalence: the typed value reproduces the legacy `"l t r b"` string.
    QCOMPARE(rect.toString(), QStringLiteral("0 0 800 600"));

    QVERIFY(!transform_api::layerRect(view, QStringLiteral("99")).valid);
}

void TransformApiTest::quadAndPreviewMatrixMatchTheLegacyFormat()
{
    PictureView view;
    QVERIFY(document_api::newDocument(view, 800, 600, QStringLiteral("rgb"), 8,
                                      QStringLiteral("white")));
    QVERIFY(view.begin_free_transform(QStringLiteral("0")));

    const transform_api::Quad quad = transform_api::quad(view);
    QCOMPARE(quad.corners.size(), 4);
    // Equivalence: the typed corners reproduce the legacy `"x,y …"` string.
    QCOMPARE(quad.toString(),
             QStringLiteral("0.00,0.00 800.00,0.00 800.00,600.00 0.00,600.00"));

    const transform_api::Matrix matrix = transform_api::previewMatrix(view);
    QVERIFY(matrix.valid);
    const double identity[9] = {1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0};
    for (int i = 0; i < 9; ++i) {
        QCOMPARE(matrix.coefficients[size_t(i)], identity[i]);
    }

    view.cancel_transform();
    QVERIFY(transform_api::quad(view).isEmpty());
    QVERIFY(!transform_api::previewMatrix(view).valid);
}

QTEST_MAIN(TransformApiTest)
#include "tst_transform_api.moc"
