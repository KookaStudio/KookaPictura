#include <QtTest/QtTest>

#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>

#include "commands.h"
#include "filter_commands.h"
#include "frame.h"
#include "lens_flare_dialog.h"
#include "api/filters_bridge.h"

#include "qt_test_support.h"

namespace {

const QStringList kLensFlarePath = {QStringLiteral("Filter"), QStringLiteral("Render"),
                                    QStringLiteral("Lens Flare")};

const pictura::FilterCommandSpec& lensFlareSpec()
{
    return *pictura::filterCommandForPath(kLensFlarePath);
}

pictura::LensFlareDialog* openLensFlareDialog()
{
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (auto* dialog = qobject_cast<pictura::LensFlareDialog*>(widget)) {
            return dialog;
        }
    }
    return nullptr;
}

int luma(const QImage& image, double fx, double fy)
{
    const QColor c = image.pixelColor(static_cast<int>(fx * (image.width() - 1)),
                                      static_cast<int>(fy * (image.height() - 1)));
    return c.red() + c.green() + c.blue();
}

} // namespace

class LensFlareDialogTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void layoutFollowsCs6();
    void padShowsTheFlareWhereTheCrosshairIs();
    void brightnessSpinAndSliderStayInStep();
    void sliderDragDefersCanvasPreviewUntilRelease();
    void rejectRestoresPixels();
    void menuOpensItAndCommitsTheFlare();
    void lastFilterSettingsReopensItPrefilled();

private:
    pictura::test::ScopedStateHome stateHome_;
    QTemporaryDir dir_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void LensFlareDialogTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    QVERIFY(dir_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    // A flare on white is all white; a dark opaque picture shows where it lands.
    QImage dark(120, 80, QImage::Format_RGB32);
    dark.fill(QColor(20, 24, 30));
    const QString path = dir_.filePath(QStringLiteral("dark.png"));
    QVERIFY(dark.save(path) && window_->openDocumentAtPath(path));
    QVERIFY(window_->activeView() != nullptr);
    QVERIFY(pictura::filterCommandForPath(kLensFlarePath) != nullptr);
}

void LensFlareDialogTest::layoutFollowsCs6()
{
    pictura::LensFlareDialog dialog(window_->activeView(), lensFlareSpec());
    QCOMPARE(dialog.windowTitle(), QStringLiteral("Lens Flare"));
    QCOMPARE(dialog.values(), QList<double>({100.0, 0.5, 0.5, 0.0}));

    auto* spin = dialog.findChild<QSpinBox*>(QStringLiteral("lensFlareBrightness"));
    QVERIFY(spin != nullptr);
    QCOMPARE(spin->minimum(), 10);
    QCOMPARE(spin->maximum(), 300);
    QCOMPARE(spin->suffix(), QStringLiteral("%"));

    auto* group = dialog.findChild<QGroupBox*>();
    QVERIFY(group != nullptr);
    QCOMPARE(group->title(), QStringLiteral("Lens Type"));
    const QList<QRadioButton*> radios = group->findChildren<QRadioButton*>();
    QStringList labels;
    for (QRadioButton* radio : radios) {
        labels.append(radio->text());
    }
    QCOMPARE(labels, QStringList({QStringLiteral("50-300mm Zoom"), QStringLiteral("35mm Prime"),
                                  QStringLiteral("105mm Prime"), QStringLiteral("Movie Prime")}));
    QVERIFY(radios.first()->isChecked());

    auto* preview = dialog.findChild<QCheckBox*>(QStringLiteral("filterPreview"));
    QVERIFY(preview != nullptr);
    QVERIFY(preview->isChecked());

    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    auto* pad = dialog.findChild<QWidget*>(QStringLiteral("lensFlarePad"));
    QVERIFY(pad != nullptr);
    const QRect padRect(pad->mapTo(&dialog, QPoint(0, 0)), pad->size());
    // OK / Cancel / Preview stand to the right of the preview; Brightness and
    // the lens group sit under it.
    for (QPushButton* button : dialog.findChildren<QPushButton*>()) {
        QVERIFY2(button->mapTo(&dialog, QPoint(0, 0)).x() > padRect.right(),
                 qPrintable(button->text()));
    }
    QVERIFY(spin->mapTo(&dialog, QPoint(0, 0)).y() > padRect.bottom());
    QVERIFY(group->mapTo(&dialog, QPoint(0, 0)).y() > spin->mapTo(&dialog, QPoint(0, 0)).y());
    dialog.reject();
}

void LensFlareDialogTest::padShowsTheFlareWhereTheCrosshairIs()
{
    pictura::LensFlareDialog dialog(window_->activeView(), lensFlareSpec());
    dialog.setCenter(QPointF(0.2, 0.3));
    QCOMPARE(dialog.values().mid(1, 2), QList<double>({0.2, 0.3}));
    QImage pad = dialog.padImage();
    QVERIFY(!pad.isNull());
    // The proxy keeps the picture's shape.
    QCOMPARE(pad.size(), QSize(120, 80).scaled(250, 250, Qt::KeepAspectRatio));
    QVERIFY2(luma(pad, 0.2, 0.3) > luma(pad, 0.8, 0.1) + 300, "flare not at the crosshair");

    dialog.setCenter(QPointF(0.8, 0.1));
    pad = dialog.padImage();
    QVERIFY2(luma(pad, 0.8, 0.1) > luma(pad, 0.2, 0.3) + 300, "flare did not follow");
    dialog.reject();
}

void LensFlareDialogTest::brightnessSpinAndSliderStayInStep()
{
    pictura::LensFlareDialog dialog(window_->activeView(), lensFlareSpec());
    auto* spin = dialog.findChild<QSpinBox*>(QStringLiteral("lensFlareBrightness"));
    auto* slider = dialog.findChild<QSlider*>();
    QVERIFY(spin != nullptr && slider != nullptr);
    spin->setValue(250);
    QCOMPARE(slider->value(), 250);
    slider->setValue(40);
    QCOMPARE(spin->value(), 40);
    QCOMPARE(dialog.values().first(), 40.0);

    auto* movie = dialog.findChildren<QRadioButton*>().last();
    movie->setChecked(true);
    QCOMPARE(dialog.values().last(), 3.0);
    dialog.reject();
}

void LensFlareDialogTest::sliderDragDefersCanvasPreviewUntilRelease()
{
    pictura::PictureView* view = window_->activeView();
    const QImage before = view->image();
    pictura::LensFlareDialog dialog(view, lensFlareSpec());
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    QTRY_VERIFY(view->image() != before); // the opening preview has run
    const QImage opened = view->image();
    auto* slider = dialog.findChild<QSlider*>();
    QVERIFY(slider != nullptr);
    const QImage padBefore = dialog.padImage();
    slider->setSliderDown(true);
    slider->setValue(300);
    QCOMPARE(view->image(), opened);
    QVERIFY(dialog.padImage() != padBefore);
    slider->setSliderDown(false);
    emit slider->sliderReleased();
    QVERIFY(view->image() != opened);
    dialog.reject();
}

void LensFlareDialogTest::rejectRestoresPixels()
{
    pictura::PictureView* view = window_->activeView();
    const QImage before = view->image();
    pictura::LensFlareDialog dialog(view, lensFlareSpec());
    dialog.show();
    QVERIFY(QTest::qWaitForWindowExposed(&dialog));
    dialog.setCenter(QPointF(0.3, 0.6));
    QVERIFY(view->image() != before);
    dialog.reject();
    QCOMPARE(view->image(), before);
}

void LensFlareDialogTest::menuOpensItAndCommitsTheFlare()
{
    pictura::PictureView* view = window_->activeView();
    const int before = view->history_count();
    bool opened = false;
    QTimer::singleShot(0, [&opened] {
        if (auto* dialog = openLensFlareDialog()) {
            opened = true;
            dialog->setCenter(QPointF(0.25, 0.75));
            dialog->accept();
        }
    });
    QVERIFY(window_->registry()->dispatch(pictura::commandIdForPath(kLensFlarePath)));
    QVERIFY(opened);
    QCOMPARE(view->history_count(), before + 1);
    QCOMPARE(filter_last_kind(*view), QStringLiteral("lens-flare"));
    QCOMPARE(filter_last_params(*view), QList<double>({100.0, 0.25, 0.75, 0.0}));
    const QImage image = view->image();
    QVERIFY(luma(image, 0.25, 0.75) > luma(image, 0.9, 0.1) + 300);
}

void LensFlareDialogTest::lastFilterSettingsReopensItPrefilled()
{
    pictura::PictureView* view = window_->activeView();
    QVERIFY(apply_filter_params(*view, QStringLiteral("lens-flare"), {180.0, 0.6, 0.4, 2.0}));
    QList<double> seen;
    QTimer::singleShot(0, [&seen] {
        if (auto* dialog = openLensFlareDialog()) {
            seen = dialog->values();
            dialog->reject();
        }
    });
    QVERIFY(window_->registry()->dispatch(
        QString::fromLatin1(pictura::command_ids::FilterLastFilterSettings)));
    QCOMPARE(seen, QList<double>({180.0, 0.6, 0.4, 2.0}));
}

QTEST_MAIN(LensFlareDialogTest)
#include "tst_lens_flare_dialog.moc"
