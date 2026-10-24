#include <QtTest/QtTest>

#include <QtCore/QTemporaryDir>
#include <QtGui/QImageWriter>

#include <cmath>
#include <functional>
#include <memory>

#include "frame.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools/vanishing_point.cxxqt.h"
#include "vp_dialog.h"

#include "qt_test_support.h"

using pictura::VanishingPointDialog;

namespace {

// Two planes inside the 400×300 test documents.
const double kSquare[] = {100, 60, 300, 60, 300, 240, 100, 240};
const double kRight[] = {320, 40, 390, 70, 390, 230, 320, 260};

rust::Slice<const double> quad(const double (&points)[8])
{
    return rust::Slice<const double>(points, 8);
}

QList<double> corners(int plane)
{
    const rust::Vec<double> c = pictura::vp_plane_corners(plane);
    return QList<double>(c.begin(), c.end());
}

bool near(const QList<double>& a, const QList<double>& b)
{
    if (a.size() != b.size()) {
        return false;
    }
    for (qsizetype i = 0; i < a.size(); ++i) {
        if (std::abs(a[i] - b[i]) > 1e-9) {
            return false;
        }
    }
    return true;
}

} // namespace

class VanishingPointFilesTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void planesReopenFromSavedFiles();
    void planeOnlyOkMarksTheDocumentModified();

private:
    // A white 400×300 8-bit RGB document, active and unmodified.
    pictura::PictureView* fresh(const QString& name);
    // Run a session on `view`: `body` acts on it, then OK (or Cancel).
    void session(pictura::PictureView* view, const std::function<void()>& body, bool ok = true);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    QTemporaryDir dir_;
};

void VanishingPointFilesTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

pictura::PictureView* VanishingPointFilesTest::fresh(const QString& name)
{
    if (!window_->newDocument(name, 400, 300, QStringLiteral("rgb"), 8, QStringLiteral("white"))) {
        return nullptr;
    }
    return window_->activeView();
}

void VanishingPointFilesTest::session(pictura::PictureView* view, const std::function<void()>& body,
                                      bool ok)
{
    VanishingPointDialog dialog(view, QStringLiteral("files"));
    QVERIFY(dialog.ready());
    body();
    if (ok) {
        dialog.accept();
    } else {
        dialog.reject();
    }
}

void VanishingPointFilesTest::planesReopenFromSavedFiles()
{
    pictura::PictureView* view = fresh(QStringLiteral("saved"));
    QVERIFY(view);
    QList<double> square;
    QList<double> right;
    session(view, [&] {
        QCOMPARE(pictura::vp_create(quad(kSquare)), 0);
        pictura::vp_set_grid_size(40);
        QCOMPARE(pictura::vp_create(quad(kRight)), 1);
        square = corners(0);
        right = corners(1);
    });

    for (const char* suffix : {"psd", "psb", "tif", "jpg", "png"}) {
        // Qt's TIFF writer ships in the optional imageformats module; the
        // codec tests cover the TIFF block without it.
        if (QLatin1String(suffix) == QLatin1String("tif")
            && !QImageWriter::supportedImageFormats().contains("tif")) {
            qInfo("no Qt TIFF writer: TIFF skipped");
            continue;
        }
        const QString path = dir_.filePath(QStringLiteral("planes.%1").arg(QLatin1String(suffix)));
        QVERIFY2(view->save(path), suffix);
        QVERIFY2(!view->is_dirty(), suffix);
        pictura::PictureView reader;
        const bool native = QLatin1String(suffix).startsWith(QLatin1String("ps"));
        QVERIFY2(native ? reader.open(path) : reader.open_image(path), suffix);
        const bool kept = QLatin1String(suffix) != QLatin1String("png");
        session(&reader, [&] {
            QCOMPARE(pictura::vp_plane_count(), kept ? 2 : 0);
            if (kept) {
                QVERIFY2(near(corners(0), square), suffix);
                QVERIFY2(near(corners(1), right), suffix);
                pictura::vp_set_active(0);
                QCOMPARE(pictura::vp_grid_size(), 40.0);
            }
        }, false);
    }
}

void VanishingPointFilesTest::planeOnlyOkMarksTheDocumentModified()
{
    pictura::PictureView* view = fresh(QStringLiteral("modified"));
    QVERIFY(view);
    QVERIFY(!view->is_dirty());
    const int history = view->history_count();
    session(view, [] { pictura::vp_create(quad(kSquare)); }, false);
    QVERIFY2(!view->is_dirty(), "Cancel leaves the document as it was");
    session(view, [] {});
    QVERIFY2(!view->is_dirty(), "OK with nothing changed");

    session(view, [] { pictura::vp_create(quad(kSquare)); });
    QVERIFY(view->is_dirty());
    QCOMPARE(view->history_count(), history);
    const QString path = dir_.filePath(QStringLiteral("modified.psd"));
    QVERIFY(view->save(path));
    QVERIFY(!view->is_dirty());
    session(view, [] {});
    QVERIFY2(!view->is_dirty(), "reopening on the same planes changes nothing");

    session(view, [] {
        pictura::vp_set_active(0);
        QVERIFY(pictura::vp_delete_active());
    });
    QVERIFY2(view->is_dirty(), "deleting the last plane is a change");
    QVERIFY(view->save(path));
    pictura::PictureView reader;
    QVERIFY(reader.open(path));
    session(&reader, [] { QCOMPARE(pictura::vp_plane_count(), 0); }, false);
}

QTEST_MAIN(VanishingPointFilesTest)
#include "tst_vanishing_point_files.moc"
