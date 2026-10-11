#include <QtTest/QtTest>

#include <QtCore/QStringList>
#include <QtCore/QTemporaryDir>
#include <QtGui/QImage>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QLabel>
#include <QtWidgets/QRadioButton>

#include "commands.h"
#include "filter_commands.h"
#include "filter_preview_dialog.h"
#include "frame.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"

#include "qt_test_support.h"

namespace {

const QStringList kDisplacePath = {QStringLiteral("Filter"), QStringLiteral("Distort"),
                                   QStringLiteral("Displace")};

QImage flat(QColor color)
{
    QImage image(32, 32, QImage::Format_RGB32);
    image.fill(color);
    return image;
}

} // namespace

class DisplaceTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void dialogFollowsCs6();
    void aLoadedMapMovesThePictureAndLastFilterReusesIt();
    void unusableMapsAreRefused();

private:
    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    QString mapPath_;
};

void DisplaceTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);

    // The map: full red pulls everything 12.7 px sideways at 10%; green and
    // blue at mid grey leave the rows where they are. Saved as the PSD the
    // filter asks for.
    const QString mapPng = dir_.filePath(QStringLiteral("map.png"));
    mapPath_ = dir_.filePath(QStringLiteral("map.psd"));
    QVERIFY(flat(QColor(255, 128, 128)).save(mapPng) && window_->openDocumentAtPath(mapPng));
    QVERIFY(window_->activeView()->save(mapPath_));

    QImage pattern(64, 64, QImage::Format_RGB32);
    for (int y = 0; y < 64; ++y) {
        for (int x = 0; x < 64; ++x) {
            pattern.setPixelColor(x, y, QColor((x * 4) % 256, (y * 4) % 256, (x * 9) % 256));
        }
    }
    const QString path = dir_.filePath(QStringLiteral("pattern.png"));
    QVERIFY(pattern.save(path) && window_->openDocumentAtPath(path));
    QVERIFY(pictura::filterCommandForPath(kDisplacePath) != nullptr);
}

// Two scales and two radio groups, no preview: the map is chosen after OK.
void DisplaceTest::dialogFollowsCs6()
{
    const pictura::FilterCommandSpec& spec = *pictura::filterCommandForPath(kDisplacePath);
    QVERIFY(!spec.livePreview);
    pictura::FilterPreviewDialog dialog(window_->activeView(), spec);
    QCOMPARE(dialog.windowTitle(), QStringLiteral("Displace"));
    QVERIFY(dialog.findChild<QCheckBox*>(QStringLiteral("filterPreview")) == nullptr);
    QVERIFY(dialog.findChild<QLabel*>(QStringLiteral("filterThumbnail")) == nullptr);
    // 10%, 10%, Stretch To Fit, Repeat Edge Pixels, no map yet.
    QCOMPARE(dialog.values(), QList<double>({10.0, 10.0, 0.0, 1.0, 0.0}));

    QStringList labels;
    for (QRadioButton* radio : dialog.findChildren<QRadioButton*>()) {
        labels.append(radio->text());
    }
    QCOMPARE(labels, QStringList({QStringLiteral("Stretch To Fit"), QStringLiteral("Tile"),
                                  QStringLiteral("Wrap Around"),
                                  QStringLiteral("Repeat Edge Pixels")}));

    // Last Filter Settings reopens with the old map id carried, untouched.
    dialog.applyInitial({25.0, -5.0, 1.0, 0.0, 7.0});
    QCOMPARE(dialog.values(), QList<double>({25.0, -5.0, 1.0, 0.0, 7.0}));
    dialog.reject();
}

void DisplaceTest::aLoadedMapMovesThePictureAndLastFilterReusesIt()
{
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const int id = pictura::filter_displace_map_load(mapPath_);
    QVERIFY(id > 0);

    const QImage before = view->image();
    const QList<double> values = {10.0, 10.0, 0.0, 1.0, static_cast<double>(id)};
    QVERIFY(pictura::apply_filter_params(*view, QStringLiteral("displace"), values));
    const QImage once = view->image();
    QVERIFY2(once != before, "the map moved nothing");
    // A red-only map moves the picture sideways: each pixel now shows the
    // colour from about 12.7 px to its right, so the rows keep their green.
    QCOMPARE(once.pixelColor(10, 20).green(), before.pixelColor(10, 20).green());
    QVERIFY(qAbs(once.pixelColor(10, 20).red() - before.pixelColor(23, 20).red()) <= 4);

    pictura::CommandRegistry* registry = window_->registry();
    registry->refresh();
    QAction* last = registry->action(QString::fromLatin1(pictura::command_ids::FilterLastFilter));
    QVERIFY(last != nullptr && last->isEnabled());
    QCOMPARE(last->text(), QStringLiteral("Displace"));
    const int states = view->history_count();
    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::FilterLastFilter)));
    QCOMPARE(view->history_count(), states + 1);
    QVERIFY2(view->image() != once, "Last Filter did not displace again");
}

void DisplaceTest::unusableMapsAreRefused()
{
    QCOMPARE(pictura::filter_displace_map_load(dir_.filePath(QStringLiteral("missing.psd"))), 0);
    const QString png = dir_.filePath(QStringLiteral("map.png"));
    QCOMPARE(pictura::filter_displace_map_load(png), 0);

    // A map id the store never handed out is refused without touching pixels.
    pictura::PictureView* view = window_->activeView();
    const QImage before = view->image();
    QVERIFY(!pictura::apply_filter_params(*view, QStringLiteral("displace"),
                                          {10.0, 10.0, 0.0, 1.0, 1.0e9}));
    QCOMPARE(view->image(), before);
}

QTEST_MAIN(DisplaceTest)

#include "tst_displace.moc"
