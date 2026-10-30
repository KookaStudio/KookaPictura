// The Properties panel coalesces adjustment previews and commits the latest
// value, including a slider release that beats the preview timer.

#include <QtTest/QtTest>
#include <QtTest/QSignalSpy>

#include "api/layers_api.h"

#include "frame.h"
#include "panels/layers_panel.h"
#include "panels/properties_panel.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"

#include <QtCore/QCoreApplication>
#include <QtGui/QImage>
#include <QtWidgets/QSlider>

#include <memory>

#include "qt_test_support.h"

class PropertiesPreviewTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void adjustmentDragCoalescesPreview();
    void commitUsesTheLatestValue();
    void commitFlushesThePendingPreview();

private:
    bool setupAdjustment(const QString& kind);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::PropertiesPanel* panel_ = nullptr;
    pictura::LayersPanel* layers_ = nullptr;
    QString path_;
};

void PropertiesPreviewTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void PropertiesPreviewTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
    view_ = nullptr;
    panel_ = nullptr;
    layers_ = nullptr;
}

bool PropertiesPreviewTest::setupAdjustment(const QString& kind)
{
    if (!window_->newDocument(QStringLiteral("Preview"), 16, 16, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return false;
    }
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::PropertiesPanel*>(QStringLiteral("propertiesPanel"));
    layers_ = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    if (!view_ || !panel_ || !layers_ || !view_->add_adjustment(kind)) {
        return false;
    }
    path_.clear();
    for (int i = 0; i < pictura::layers_api::rowCount(*view_); ++i) {
        const auto row = pictura::layers_api::rowAt(*view_, i);
        if (row.hasAdjustment) {
            path_ = row.path;
        }
    }
    panel_->setView(view_);
    layers_->setView(view_);
    if (path_.isEmpty() || !layers_->selectRowForTest(path_)) {
        return false;
    }
    // The canvas would report this on its next paint; the test reports it
    // directly so the session has a level and rectangle to composite.
    view_->set_presented_view(0, 0, 0, 16, 16);
    panel_->refresh();
    return panel_->pathForTest() == path_;
}

void PropertiesPreviewTest::adjustmentDragCoalescesPreview()
{
    QVERIFY(setupAdjustment(QStringLiteral("brightness-contrast")));
    auto* brightness = qobject_cast<QSlider*>(panel_->controlForTest(QStringLiteral("brightness")));
    QVERIFY(brightness);
    QSignalSpy damaged(view_, &pictura::PictureView::regionDamaged);
    const int history = view_->history_count();

    brightness->setValue(80);
    brightness->setValue(100);
    brightness->setValue(60);
    QCoreApplication::processEvents();

    QCOMPARE(damaged.count(), 1);
    QCOMPARE(view_->history_count(), history);
}

void PropertiesPreviewTest::commitUsesTheLatestValue()
{
    QVERIFY(setupAdjustment(QStringLiteral("brightness-contrast")));
    auto* brightness = qobject_cast<QSlider*>(panel_->controlForTest(QStringLiteral("brightness")));
    QVERIFY(brightness);
    const int history = view_->history_count();

    brightness->setValue(100);
    QCoreApplication::processEvents();
    brightness->setValue(-100);
    QCoreApplication::processEvents();
    const QImage shown = view_->display_image(0, 0, 0, 16, 16);
    QVERIFY(!shown.isNull());
    const QRgb latest = shown.pixel(4, 4);

    panel_->commitForTest();
    QCOMPARE(view_->history_count(), history + 1);
    QCOMPARE(view_->composite_argb(4, 4), latest);
    QCOMPARE(view_->history_label(view_->history_index()),
             QStringLiteral("Modify Brightness/Contrast Layer"));
}

void PropertiesPreviewTest::commitFlushesThePendingPreview()
{
    QVERIFY(setupAdjustment(QStringLiteral("hue-saturation")));
    auto* lightness = qobject_cast<QSlider*>(panel_->controlForTest(QStringLiteral("lightness")));
    QVERIFY(lightness);
    const int history = view_->history_count();

    // No event-loop turn: the release must flush the pending value itself.
    lightness->setValue(-100);
    panel_->commitForTest();

    QCOMPARE(view_->history_count(), history + 1);
    QCOMPARE(qGray(view_->composite_argb(4, 4)), 0);
}

QTEST_MAIN(PropertiesPreviewTest)
#include "tst_properties_preview.moc"
