// Unit B: selection_api as the typed façade over the selection geometry
// (SelectionBounds / SelectionContour), replacing the string re-parse at each
// consumer.

#include <QtTest/QtTest>

#include "api/document_api.h"
#include "api/selection_api.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

using namespace pictura;

class SelectionApiTest : public QObject {
    Q_OBJECT

private slots:
    void emptySelectionIsInvalidAndEmpty();
    void typedBoundsMatchesTheLegacyString();
    void typedContourMatchesTheLegacyString();
};

void SelectionApiTest::emptySelectionIsInvalidAndEmpty()
{
    PictureView view;
    QVERIFY(document_api::newDocument(view, 32, 32, QStringLiteral("rgb"), 8,
                                      QStringLiteral("white")));
    QVERIFY(!selection_api::bounds(view).valid);
    QVERIFY(selection_api::bounds(view).toString().isEmpty());
    QVERIFY(selection_api::contour(view).isEmpty());
}

void SelectionApiTest::typedBoundsMatchesTheLegacyString()
{
    PictureView view;
    QVERIFY(document_api::newDocument(view, 32, 32, QStringLiteral("rgb"), 8,
                                      QStringLiteral("white")));
    QVERIFY(view.select_rect(4, 4, 5, 5, QStringLiteral("new"), 0.0));

    const selection_api::SelectionBounds bounds = selection_api::bounds(view);
    QVERIFY(bounds.valid);
    QCOMPARE(bounds.x, 4);
    QCOMPARE(bounds.y, 4);
    QCOMPARE(bounds.width, 5);
    QCOMPARE(bounds.height, 5);
    // Equivalence: the typed value reproduces the legacy `"x y w h"` string.
    QCOMPARE(bounds.toString(), QStringLiteral("4 4 5 5"));
}

void SelectionApiTest::typedContourMatchesTheLegacyString()
{
    PictureView view;
    QVERIFY(document_api::newDocument(view, 32, 32, QStringLiteral("rgb"), 8,
                                      QStringLiteral("white")));
    QVERIFY(view.select_rect(4, 4, 5, 5, QStringLiteral("new"), 0.0));

    const selection_api::SelectionContour contour = selection_api::contour(view);
    QCOMPARE(contour.loopCount(), 1);
    // Equivalence: the typed loops carry the integer corners the legacy string
    // encoded as `"x,y x,y ..."` (a 5x5 rect selects pixels 4..8, so the outline
    // runs corner to exclusive boundary 9).
    QVERIFY(contour.hasVertex(4, 4));
    QVERIFY(contour.hasVertex(9, 9));
}

QTEST_MAIN(SelectionApiTest)
#include "tst_selection_api.moc"
