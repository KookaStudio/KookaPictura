// Image > Image Rotation > Arbitrary (#282): rotates the document about its
// centre by an arbitrary angle through pictura_render::rotate_document_in.

#include <QtTest/QtTest>

#include <limits>

#include "frame.h"
#include "qt_test_support.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_ops.cxxqt.h"

class ImageRotateTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void arbitraryRotatesAndRecords();
    void invalidAngleIsRefused();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void ImageRotateTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void ImageRotateTest::arbitraryRotatesAndRecords()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Rot"), 20, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();
    const int before = view->history_count();

    QVERIFY((pictura::rotate_document_arbitrary(*view, 30.0)));
    // The 20x16 canvas grows to the rotated bounding box.
    QVERIFY(view->document_width() != 20 || view->document_height() != 16);
    QCOMPARE(view->history_count(), before + 1);
    QCOMPARE(view->history_label(view->history_count() - 1), QStringLiteral("Rotate"));

    // Undo restores the original dimensions.
    QVERIFY(view->undo());
    QCOMPARE(view->document_width(), 20);
    QCOMPARE(view->document_height(), 16);

    frame.closeDocument(doc, false);
}

void ImageRotateTest::invalidAngleIsRefused()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("RotBad"), 12, 12, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();

    const int before = view->history_count();
    QVERIFY(!(pictura::rotate_document_arbitrary(*view, 0.0)));
    QVERIFY(!(pictura::rotate_document_arbitrary(*view, std::numeric_limits<double>::quiet_NaN())));
    QVERIFY(!(pictura::rotate_document_arbitrary(*view, 360.0)));
    QCOMPARE(view->history_count(), before);
    QCOMPARE(view->document_width(), 12);
    QCOMPARE(view->document_height(), 12);

    frame.closeDocument(doc, false);
}

QTEST_MAIN(ImageRotateTest)
#include "tst_image_rotate.moc"
