// Crop straighten (#282): committing a crop with a straighten angle rotates the
// document about the box centre and crops to the box (rotate_document_in +
// crop_document), recording one "Crop" state.

#include <QtTest/QtTest>

#include "frame.h"
#include "qt_test_support.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/crop_group.cxxqt.h"

class CropStraightenTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void straightenCommitCropsToTheBox();
    void angleZeroIsAnAxisAlignedCrop();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void CropStraightenTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void CropStraightenTest::straightenCommitCropsToTheBox()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("Straighten"), 40, 30, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();
    const int before = view->history_count();

    QVERIFY(pictura::straighten_crop(*view, 5, 4, 20, 10, 30.0, 15.0, 9.0, true, -1));
    QCOMPARE(view->document_width(), 20);
    QCOMPARE(view->document_height(), 10);
    QCOMPARE(view->history_count(), before + 1);
    QCOMPARE(view->history_label(view->history_count() - 1), QStringLiteral("Crop"));

    frame.closeDocument(doc, false);
}

void CropStraightenTest::angleZeroIsAnAxisAlignedCrop()
{
    pictura::PicturaMainWindow& frame = *window_;
    QVERIFY(frame.newDocument(QStringLiteral("NoAngle"), 24, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("white")));
    pictura::PictureView* view = frame.activeView();
    QVERIFY(view);
    const int doc = frame.activeDocumentIndex();

    QVERIFY(pictura::straighten_crop(*view, 2, 3, 10, 8, 0.0, 7.0, 7.0, true, -1));
    QCOMPARE(view->document_width(), 10);
    QCOMPARE(view->document_height(), 8);

    frame.closeDocument(doc, false);
}

QTEST_MAIN(CropStraightenTest)
#include "tst_crop_straighten.moc"
