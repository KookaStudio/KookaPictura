// Unit C: paint_api::Stroke owns the begin → dab/present → finish ordering so a
// stroke cannot be left open on an early return, and closes exactly once.

#include <QtTest/QtTest>

#include "api/document_api.h"
#include "api/paint_api.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

using namespace pictura;

namespace {

constexpr quint32 kRed = 0xFFFF0000u;
constexpr quint32 kWhite = 0xFFFFFFFFu;

bool beginRed(PictureView& view)
{
    return view.begin_paint(kRed, kWhite, 8, 100, 100, 0, 100, 100, 25,
                            QStringLiteral("normal"), false, false);
}

void newDoc(PictureView& view)
{
    view.set_active_layer(QStringLiteral("0"));
}

} // namespace

class PaintApiTest : public QObject {
    Q_OBJECT

private slots:
    void destructorClosesExactlyOnceAndMatchesExplicitBeginEnd();
    void explicitFinishDoesNotDoubleCommit();
    void midStrokePresentKeepsStrokeOpen();
};

void PaintApiTest::destructorClosesExactlyOnceAndMatchesExplicitBeginEnd()
{
    PictureView explicitView;
    QVERIFY(document_api::newDocument(explicitView, 32, 32, QStringLiteral("rgb"), 8,
                                      QStringLiteral("white")));
    newDoc(explicitView);
    const int explicitBase = explicitView.history_count();
    QVERIFY(beginRed(explicitView));
    QVERIFY(explicitView.paint_dab(8.0, 8.0, 1.0));
    QVERIFY(explicitView.end_paint());
    const int explicitStates = explicitView.history_count() - explicitBase;
    const QString explicitLabel = explicitView.history_label(explicitBase);
    QVERIFY(explicitStates > 0);

    PictureView view;
    QVERIFY(document_api::newDocument(view, 32, 32, QStringLiteral("rgb"), 8,
                                      QStringLiteral("white")));
    newDoc(view);
    const int base = view.history_count();
    {
        paint_api::Stroke stroke(view, [&] { return beginRed(view); });
        QVERIFY(stroke.open());
        QVERIFY(stroke.dab(8.0, 8.0, 1.0));
        // No finish(): the destructor must commit, exactly once.
    }
    QVERIFY(!view.is_painting());
    QCOMPARE(view.history_count() - base, explicitStates);
    QCOMPARE(view.history_label(base), explicitLabel);
}

void PaintApiTest::explicitFinishDoesNotDoubleCommit()
{
    PictureView view;
    QVERIFY(document_api::newDocument(view, 32, 32, QStringLiteral("rgb"), 8,
                                      QStringLiteral("white")));
    newDoc(view);
    const int base = view.history_count();
    {
        paint_api::Stroke stroke(view, [&] { return beginRed(view); });
        QVERIFY(stroke.open());
        QVERIFY(stroke.dab(8.0, 8.0, 1.0));
        QVERIFY(stroke.finish());
        QVERIFY(!stroke.finish());
    }
    QCOMPARE(view.history_count(), base + 1);
    QVERIFY(!view.is_painting());
}

void PaintApiTest::midStrokePresentKeepsStrokeOpen()
{
    PictureView view;
    QVERIFY(document_api::newDocument(view, 32, 32, QStringLiteral("rgb"), 8,
                                      QStringLiteral("white")));
    newDoc(view);
    paint_api::Stroke stroke(view, [&] { return beginRed(view); });
    QVERIFY(stroke.open());
    QVERIFY(stroke.dab(8.0, 8.0, 1.0));
    stroke.present();
    QVERIFY(view.is_painting());
    QVERIFY(stroke.dab(12.0, 12.0, 1.0));
    QVERIFY(stroke.finish());
    QVERIFY(!view.is_painting());
}

QTEST_MAIN(PaintApiTest)
#include "tst_paint_api.moc"
