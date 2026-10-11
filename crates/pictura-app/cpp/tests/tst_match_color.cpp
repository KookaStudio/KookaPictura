// Image > Adjustments > Match Color (#352): the menu row goes live, the
// dialog's statistics transfer commits one "Match Color" state, Cancel
// restores the layer exactly, a second document feeds the Source combo, and
// Save/Load Statistics round-trips.

#include <QtTest/QtTest>

#include "commands.h"
#include "frame.h"
#include "image_view.h"
#include "match_color_dialog.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QBuffer>
#include <QtCore/QByteArray>
#include <QtCore/QTemporaryDir>
#include <QtGui/QAction>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>

#include "qt_test_support.h"

class MatchColorTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void rowIsEnabledWithEllipsis();
    void cancelRestoresExactly();
    void neutralizeCommitsOneStateAndUndoRestores();
    void secondDocumentFeedsTheSource();
    void statisticsRoundTripThroughSaveLoad();

private:
    bool openImage(const QColor& color, const QString& name);
    QAction* leaf() const;
    QList<pictura::MatchColorDialog::Source> sources() const;

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    QTemporaryDir dir_;
    pictura::PictureView* view_ = nullptr;
};

void MatchColorTest::initTestCase()
{
    QVERIFY(stateHome_.isValid() && dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void MatchColorTest::cleanup()
{
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
    view_ = nullptr;
}

bool MatchColorTest::openImage(const QColor& color, const QString& name)
{
    QImage image(20, 20, QImage::Format_RGB32);
    image.fill(color);
    const QString path = dir_.filePath(name);
    if (!image.save(path) || !window_->openDocumentAtPath(path)) {
        return false;
    }
    QCoreApplication::processEvents();
    view_ = window_->activeView();
    return view_ != nullptr;
}

QAction* MatchColorTest::leaf() const
{
    return window_->registry()->action(pictura::commandIdForPath(
        {QStringLiteral("Image"), QStringLiteral("Adjustments"), QStringLiteral("Match Color")}));
}

QList<pictura::MatchColorDialog::Source> MatchColorTest::sources() const
{
    QList<pictura::MatchColorDialog::Source> out;
    for (int i = 0; i < window_->documentCount(); ++i) {
        if (window_->viewAt(i) && window_->viewAt(i) != view_) {
            out.append({window_->documentName(i), window_->viewAt(i)});
        }
    }
    return out;
}

void MatchColorTest::rowIsEnabledWithEllipsis()
{
    QAction* action = leaf();
    QVERIFY(action);
    window_->registry()->refresh();
    QVERIFY(!action->isEnabled());
    QVERIFY(openImage(QColor(100, 150, 200), QStringLiteral("target.png")));
    QVERIFY(action->isEnabled());
    QCOMPARE(action->text(), QStringLiteral("Match Color…"));
}

void MatchColorTest::cancelRestoresExactly()
{
    QVERIFY(openImage(QColor(200, 60, 60), QStringLiteral("cast.png")));
    const QRgb before = view_->composite_argb(5, 5);
    const int history = view_->history_count();
    {
        pictura::MatchColorDialog dialog(view_, window_->imageView(), sources());
        dialog.findChild<QCheckBox*>(QStringLiteral("matchColorNeutralize"))
            ->setChecked(true);
        QVERIFY(view_->composite_argb(5, 5) != before);
        dialog.reject();
    }
    QCOMPARE(view_->composite_argb(5, 5), before);
    QCOMPARE(view_->history_count(), history);
}

void MatchColorTest::neutralizeCommitsOneStateAndUndoRestores()
{
    QVERIFY(openImage(QColor(200, 60, 60), QStringLiteral("cast.png")));
    const QRgb before = view_->composite_argb(5, 5);
    const int history = view_->history_count();
    pictura::MatchColorDialog dialog(view_, window_->imageView(), sources());
    dialog.findChild<QCheckBox*>(QStringLiteral("matchColorNeutralize"))
        ->setChecked(true);
    dialog.accept();
    // A gray-world pull: the cast is gone, so the channels equalize.
    const QRgb after = view_->composite_argb(5, 5);
    QVERIFY(qRed(after) != qRed(before));
    QCOMPARE(qRed(after), qGreen(after));
    QCOMPARE(view_->history_count(), history + 1);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Match Color"));
    QVERIFY(view_->undo());
    QCOMPARE(view_->composite_argb(5, 5), before);
}

void MatchColorTest::secondDocumentFeedsTheSource()
{
    QVERIFY(openImage(QColor(200, 60, 60), QStringLiteral("red.png")));
    pictura::PictureView* red = view_;
    QVERIFY(openImage(QColor(60, 60, 220), QStringLiteral("blue.png")));
    const QRgb before = view_->composite_argb(5, 5);
    const int history = view_->history_count();

    pictura::MatchColorDialog dialog(view_, window_->imageView(), sources());
    auto* source = dialog.findChild<QComboBox*>(QStringLiteral("matchColorSource"));
    auto* layer = dialog.findChild<QComboBox*>(QStringLiteral("matchColorLayer"));
    QVERIFY(source && layer);
    // "None", "Loaded Statistics", then one entry per other open document.
    QCOMPARE(source->count(), 3);
    QCOMPARE(source->itemText(2), QStringLiteral("red.png"));
    source->setCurrentIndex(2);
    // A real source lists its layers, "Merged" first.
    QVERIFY(layer->isEnabled());
    QVERIFY(layer->count() >= 1);
    QCOMPARE(layer->itemText(0), QStringLiteral("Merged"));
    dialog.accept();

    // The blue target is pulled toward the red source's mean.
    const QRgb after = view_->composite_argb(5, 5);
    QVERIFY(qRed(after) > qRed(before));
    QVERIFY(qBlue(after) < qBlue(before));
    QCOMPARE(view_->history_count(), history + 1);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Match Color"));
    QVERIFY(view_->undo());
    QCOMPARE(view_->composite_argb(5, 5), before);
    QVERIFY(red);
}

void MatchColorTest::statisticsRoundTripThroughSaveLoad()
{
    const QList<double> stats = {1.5, -2.25, 3.125, 4.0, -5.75, 6.0625};
    QByteArray bytes;
    {
        QBuffer buffer(&bytes);
        buffer.open(QIODevice::WriteOnly);
        QVERIFY(pictura::MatchColorDialog::writeStatistics(&buffer, stats));
    }
    QList<double> read;
    {
        QBuffer buffer(&bytes);
        buffer.open(QIODevice::ReadOnly);
        read = pictura::MatchColorDialog::readStatistics(&buffer);
    }
    QCOMPARE(read.size(), 6);
    QCOMPARE(read, stats);
    // A wrong-size payload or foreign magic is refused.
    QByteArray foreign = QByteArrayLiteral("not stats");
    QBuffer foreignBuffer(&foreign);
    foreignBuffer.open(QIODevice::ReadOnly);
    QCOMPARE(pictura::MatchColorDialog::readStatistics(&foreignBuffer).size(), 0);
    QBuffer shortBuffer(&bytes);
    shortBuffer.open(QIODevice::ReadOnly);
    shortBuffer.read(10); // desynchronize past the header
    QCOMPARE(pictura::MatchColorDialog::readStatistics(&shortBuffer).size(), 0);
}

#include "tst_match_color.moc"

QTEST_MAIN(MatchColorTest)
