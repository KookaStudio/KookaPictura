// Image > Mode > 32 -> 16/8 HDR Conversion: the method dialog, the four-method
// bridge, and the one-state history contract.

#include <QtTest/QtTest>

#include <QtWidgets/QAbstractSpinBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QStackedWidget>

#include "commands.h"
#include "frame.h"
#include "hdr_conversion_dialog.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/image_adjust/image_mode.cxxqt.h"
#include "pictura_app/src/cxxqt_object/hdr_conversion.cxxqt.h"
#include "pictura_app/src/cxxqt_object/paint_tools/fills.cxxqt.h"

#include "qt_test_support.h"

class HdrConversionTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void non32BitConvertsWithoutDialog();
    void methodComboGatesFields();
    void defaultsToLocalAdaptation();
    void acceptConvertsWithSelectedMethod();
    void failedConversionRecordsNoHistory();

private:
    pictura::PictureView* newThirtyTwoBit();
    void closeActive()
    {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void HdrConversionTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void HdrConversionTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
}

pictura::PictureView* HdrConversionTest::newThirtyTwoBit()
{
    if (!window_->newDocument(QStringLiteral("HDR"), 4, 4, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return nullptr;
    }
    pictura::PictureView* view = window_->activeView();
    view->select_all();
    pictura::edit_fill(*view, qRgba(200, 100, 50, 255), -1, QStringLiteral("normal"), 100,
                       false);
    view->deselect();
    if (!pictura::convert_image_depth(*view, 32)) {
        return nullptr;
    }
    return view;
}

void HdrConversionTest::non32BitConvertsWithoutDialog()
{
    QVERIFY(window_->newDocument(QStringLiteral("LDR"), 4, 4, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    window_->registry()->refresh();
    QAction* to16 = window_->registry()->action(pictura::command_ids::ImageMode16Bits);
    QVERIFY(to16);
    to16->trigger();
    QCOMPARE(pictura::image_depth_bits(*view), 16);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Convert Mode"));
}

void HdrConversionTest::methodComboGatesFields()
{
    pictura::HdrConversionDialog dialog;
    auto* method = qobject_cast<QComboBox*>(dialog.controlForTest(QStringLiteral("hdrConversionMethod")));
    auto* pages = qobject_cast<QStackedWidget*>(dialog.controlForTest(QStringLiteral("hdrConversionPages")));
    QVERIFY(method && pages);
    QCOMPARE(method->count(), 4);
    for (int i = 0; i < 4; ++i) {
        method->setCurrentIndex(i);
        QCOMPARE(pages->currentIndex(), i);
    }
    // Local Adaptation shows the nine numeric rows; Exposure and Gamma shows
    // its two; the two automatic pages show no parameter fields.
    QCOMPARE(pages->widget(0)->findChildren<QAbstractSpinBox*>().size(), 9);
    QCOMPARE(pages->widget(2)->findChildren<QAbstractSpinBox*>().size(), 2);
    QCOMPARE(pages->widget(1)->findChildren<QAbstractSpinBox*>().size(), 0);
    QCOMPARE(pages->widget(3)->findChildren<QAbstractSpinBox*>().size(), 0);
    QVERIFY(dialog.controlForTest(QStringLiteral("hdrRadius")));
    QVERIFY(dialog.controlForTest(QStringLiteral("hdrConversionExposure")));
    QVERIFY(dialog.controlForTest(QStringLiteral("hdrConversionGamma")));
}

void HdrConversionTest::defaultsToLocalAdaptation()
{
    pictura::HdrConversionDialog dialog;
    auto* method = qobject_cast<QComboBox*>(dialog.controlForTest(QStringLiteral("hdrConversionMethod")));
    QVERIFY(method);
    QCOMPARE(method->currentText(), QStringLiteral("Local Adaptation"));
    const pictura::HdrConversionSpec spec = dialog.spec();
    QCOMPARE(int(spec.method), int(pictura::HdrConversionMethod::LocalAdaptation));
    QCOMPARE(spec.radius, 187.0);
    QCOMPARE(spec.strength, 4.0);
    QCOMPARE(spec.local_gamma, 0.99);
    QCOMPARE(spec.local_exposure, 0.0);
    QCOMPARE(spec.detail, 30);
    QCOMPARE(spec.shadow, 0);
    QCOMPARE(spec.highlight, 0);
    QCOMPARE(spec.vibrance, 0);
    QCOMPARE(spec.saturation, 20);
}

void HdrConversionTest::acceptConvertsWithSelectedMethod()
{
    pictura::PictureView* view = newThirtyTwoBit();
    QVERIFY(view);
    const int before = view->history_count();

    pictura::HdrConversionDialog dialog;
    auto* method = qobject_cast<QComboBox*>(dialog.controlForTest(QStringLiteral("hdrConversionMethod")));
    auto* exposure = qobject_cast<QDoubleSpinBox*>(
        dialog.controlForTest(QStringLiteral("hdrConversionExposure")));
    QVERIFY(method && exposure);
    method->setCurrentIndex(int(pictura::HdrConversionMethod::ExposureGamma));
    exposure->setValue(-2.0);
    const pictura::HdrConversionSpec spec = dialog.spec();
    QCOMPARE(int(spec.method), int(pictura::HdrConversionMethod::ExposureGamma));

    QVERIFY(pictura::convert_depth_method(
        *view, 8, int(spec.method), spec.exposure_ev, spec.gamma, spec.radius, spec.strength,
        spec.local_gamma, spec.local_exposure, spec.detail, spec.shadow, spec.highlight,
        spec.vibrance, spec.saturation));
    QCOMPARE(pictura::image_depth_bits(*view), 8);
    QCOMPARE(qRed(view->composite_argb(2, 2)), 50);   // 200 * 2^-2
    QCOMPARE(qGreen(view->composite_argb(2, 2)), 25); // 100 * 2^-2
    QCOMPARE(qBlue(view->composite_argb(2, 2)), 12);  // 50 * 2^-2, truncated
    QCOMPARE(view->history_count(), before + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("HDR Conversion"));
    // The tone map runs on the merged composite; the converted document is
    // flattened so the next recomposite cannot restore the old layer.
    QCOMPARE(view->layer_count(), 1);
    QCOMPARE(view->layer_name(0), QStringLiteral("Background"));
    closeActive();


}

void HdrConversionTest::failedConversionRecordsNoHistory()
{
    pictura::PictureView* view = newThirtyTwoBit();
    QVERIFY(view);
    const int before = view->history_count();
    // Radius 0 is outside the Local Adaptation range, so the engine refuses.
    QVERIFY(!pictura::convert_depth_method(*view, 8, 0, 0.0, 1.0, 0.0, 4.0, 0.99, 0.0, 30, 0, 0,
                                           0, 20));
    QCOMPARE(pictura::image_depth_bits(*view), 32);
    QCOMPARE(view->history_count(), before);
}

QTEST_MAIN(HdrConversionTest)
#include "tst_hdr_conversion.moc"
