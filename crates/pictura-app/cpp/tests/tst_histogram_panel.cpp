#include <QtTest/QtTest>

#include <QtGui/QColor>

#include "frame.h"
#include "panels/histogram_panel.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/annotations.cxxqt.h"

#include "qt_test_support.h"

class HistogramPanelTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void allChannelsIsDefaultWithRgbOverlay();
    void singleChannelDropsToItsCurve();

private:
    void openDocument(const QString& name);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::HistogramPanel* panel_ = nullptr;
    pictura::PictureView* view_ = nullptr;
};

void HistogramPanelTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void HistogramPanelTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
    panel_ = nullptr;
    view_ = nullptr;
}

void HistogramPanelTest::openDocument(const QString& name)
{
    const bool created = window_->newDocument(name, 16, 16, QStringLiteral("rgb"), 8,
                                              QStringLiteral("white"));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::HistogramPanel*>(QStringLiteral("histogramPanel"));
    QVERIFY2(created && view_ && panel_, "histogram fixture");
    panel_->setView(view_);
}

void HistogramPanelTest::allChannelsIsDefaultWithRgbOverlay()
{
    openDocument(QStringLiteral("AllChannels"));

    QCOMPARE(panel_->channelIndexForTest(), 0);
    QCOMPARE(panel_->channelLabelForTest(), QStringLiteral("All Channels"));
    QCOMPARE(panel_->seriesCountForTest(), 3);

    const QVector<QColor> colors = panel_->seriesColorsForTest();
    QCOMPARE(colors.size(), 3);
    QCOMPARE(colors.at(0).name(), QStringLiteral("#e65a5a"));
    QCOMPARE(colors.at(1).name(), QStringLiteral("#5adc5a"));
    QCOMPARE(colors.at(2).name(), QStringLiteral("#5a82f0"));
}

void HistogramPanelTest::singleChannelDropsToItsCurve()
{
    openDocument(QStringLiteral("SingleChannel"));

    panel_->setChannelIndexForTest(1); // Red
    QCOMPARE(panel_->seriesCountForTest(), 1);
    QCOMPARE(panel_->seriesColorsForTest().at(0).name(), QStringLiteral("#e65a5a"));

    panel_->setChannelIndexForTest(4); // Luminance
    QCOMPARE(panel_->seriesCountForTest(), 1);
    QCOMPARE(panel_->seriesColorsForTest().at(0).name(), QStringLiteral("#dcdcdc"));
}

QTEST_MAIN(HistogramPanelTest)
#include "tst_histogram_panel.moc"
