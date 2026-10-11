#include <QtTest/QtTest>

#include <QtCore/QStringList>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
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

const QStringList kLensBlurPath = {QStringLiteral("Filter"), QStringLiteral("Blur"),
                                   QStringLiteral("Lens Blur")};

const pictura::FilterCommandSpec& lensBlurSpec()
{
    return *pictura::filterCommandForPath(kLensBlurPath);
}

pictura::FilterPreviewDialog* activeFilterDialog()
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (auto* dialog = qobject_cast<pictura::FilterPreviewDialog*>(widget)) {
            return dialog;
        }
    }
    return nullptr;
}

QImage pattern()
{
    QImage image(64, 64, QImage::Format_RGB32);
    for (int y = 0; y < 64; ++y) {
        for (int x = 0; x < 64; ++x) {
            image.setPixelColor(x, y, QColor((x * 4) % 256, (y * 4) % 256, (x * 9) % 256));
        }
    }
    return image;
}

// Opaque at the top, transparent at the bottom, so transparency is a depth ramp.
QImage alphaRamp()
{
    QImage image(64, 64, QImage::Format_ARGB32);
    for (int y = 0; y < 64; ++y) {
        for (int x = 0; x < 64; ++x) {
            image.setPixelColor(x, y,
                                QColor((x * 4) % 256, (y * 4) % 256, 128, 255 - y * 4));
        }
    }
    return image;
}

// The 15 slots: Preview, Source, Blur Focal Distance, Invert, Shape, Radius,
// Blade Curvature, Rotation, Specular Brightness, Specular Threshold, Noise
// Amount, Distribution, Monochromatic, map id, seed. CS6's dialog values
// (#338), Source None, and no depth map yet.
QList<double> defaultSlots()
{
    return {0.0, 0.0, 126.0, 0.0, 0.0, 50.0, 33.0, 191.0, 44.0, 93.0, 47.0, 1.0, 0.0, 0.0, 1.0};
}

} // namespace

class LensBlurDialogTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void dialogFollowsCs6();
    void aTransparencySourceBlursOutOfFocus();
    void anUnavailableSourceRefusesTheCommit();
    void fasterStillCommitsMoreAccurate();
    void rejectRestoresPixels();
    void lastFilterReusesTheSettings();

private:
    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    QString patternPath_;
    QString alphaPath_;
};

void LensBlurDialogTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);

    patternPath_ = dir_.filePath(QStringLiteral("pattern.png"));
    QVERIFY(pattern().save(patternPath_) && window_->openDocumentAtPath(patternPath_));
    alphaPath_ = dir_.filePath(QStringLiteral("alpha.png"));
    QVERIFY(alphaRamp().save(alphaPath_));
    QVERIFY(pictura::filterCommandForPath(kLensBlurPath) != nullptr);
}

void LensBlurDialogTest::dialogFollowsCs6()
{
    const pictura::FilterCommandSpec& spec = lensBlurSpec();
    QVERIFY(spec.livePreview);
    QVERIFY(spec.previewPane);
    pictura::FilterPreviewDialog dialog(window_->activeView(), spec);
    QCOMPARE(dialog.windowTitle(), QStringLiteral("Lens Blur"));
    QVERIFY(dialog.findChild<QLabel*>(QStringLiteral("filterThumbnail")) != nullptr);
    QCOMPARE(dialog.values(), defaultSlots());

    QStringList radios;
    for (QRadioButton* radio : dialog.findChildren<QRadioButton*>()) {
        radios.append(radio->text());
    }
    QCOMPARE(radios, QStringList({QStringLiteral("Faster"), QStringLiteral("More Accurate"),
                                  QStringLiteral("Uniform"), QStringLiteral("Gaussian")}));
    const QList<QRadioButton*> widgets = dialog.findChildren<QRadioButton*>();
    QVERIFY(widgets.at(0)->isChecked()); // Faster
    QVERIFY(widgets.at(3)->isChecked()); // Gaussian
    dialog.reject();
}

void LensBlurDialogTest::aTransparencySourceBlursOutOfFocus()
{
    QVERIFY(window_->openDocumentAtPath(alphaPath_));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const int id = pictura::filter_lens_blur_depth(*view, 1);
    QVERIFY(id > 0);

    const QImage before = view->image();
    QList<double> values = defaultSlots();
    values[1] = 1.0; // Source: Transparency
    values[13] = id;
    QVERIFY(pictura::apply_filter_params(*view, QStringLiteral("lens-blur"), values));
    QVERIFY2(view->image() != before, "the transparency source blurred nothing");
}

void LensBlurDialogTest::anUnavailableSourceRefusesTheCommit()
{
    QVERIFY(window_->openDocumentAtPath(patternPath_));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    QCOMPARE(pictura::filter_lens_blur_depth(*view, 2), 0); // no layer mask
    QCOMPARE(pictura::filter_lens_blur_depth(*view, 3), 0); // no selection

    const QImage before = view->image();
    QList<double> values = defaultSlots();
    values[1] = 2.0; // Source: Layer Mask
    values[13] = 0.0;
    QVERIFY(!pictura::apply_filter_params(*view, QStringLiteral("lens-blur"), values));
    QCOMPARE(view->image(), before);
    QVERIFY(!pictura::filter_last_error(*view).isEmpty());
}

void LensBlurDialogTest::fasterStillCommitsMoreAccurate()
{
    QVERIFY(window_->openDocumentAtPath(patternPath_));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const QImage before = view->image();
    const QList<double> values = defaultSlots(); // slot 0 = Faster
    QVERIFY(pictura::apply_filter_params(*view, QStringLiteral("lens-blur"), values));
    QVERIFY(view->image() != before);
    // The commit always stores the More Accurate path.
    QCOMPARE(pictura::filter_last_params(*view).at(0), 1.0);
}

void LensBlurDialogTest::rejectRestoresPixels()
{
    QVERIFY(window_->openDocumentAtPath(patternPath_));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    const QImage before = view->image();
    pictura::FilterPreviewDialog dialog(view, lensBlurSpec());
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    QTRY_VERIFY2(view->image() != before, "the canvas preview did not run");
    dialog.reject();
    QCOMPARE(view->image(), before);
}

void LensBlurDialogTest::lastFilterReusesTheSettings()
{
    QVERIFY(window_->openDocumentAtPath(patternPath_));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view != nullptr);
    QList<double> values = defaultSlots();
    values[0] = 1.0; // More Accurate
    QVERIFY(pictura::apply_filter_params(*view, QStringLiteral("lens-blur"), values));

    pictura::CommandRegistry* registry = window_->registry();
    registry->refresh();
    QAction* last = registry->action(QString::fromLatin1(pictura::command_ids::FilterLastFilter));
    QVERIFY(last != nullptr && last->isEnabled());
    QCOMPARE(last->text(), QStringLiteral("Lens Blur"));
    const int states = view->history_count();
    const QImage once = view->image();
    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::FilterLastFilter)));
    QCOMPARE(view->history_count(), states + 1);
    QVERIFY2(view->image() != once, "Last Filter did not blur again");

    // Last Filter Settings reopens prefilled; the depth id is re-resolved.
    QList<double> seen;
    QTimer::singleShot(0, [&seen] {
        if (auto* dialog = activeFilterDialog()) {
            seen = dialog->values();
            dialog->reject();
        }
    });
    QVERIFY(registry->dispatch(
        QString::fromLatin1(pictura::command_ids::FilterLastFilterSettings)));
    QCOMPARE(seen, values);
}

QTEST_MAIN(LensBlurDialogTest)

#include "tst_lens_blur_dialog.moc"
