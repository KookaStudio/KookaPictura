#include <QtTest/QtTest>

#include "frame.h"
#include "image_view.h"
#include "panels/channels_panel.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"

#include <QtCore/QTemporaryDir>
#include <QtWidgets/QToolButton>

#include "qt_test_support.h"

class ChannelsPanelTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void rowsAndThumbnails();
    void eyesHideChannelsOnTheCanvas();
    void alphaChannelsAndFooter();
    void ctrlClickLoadsChannelSelection();
    void refreshKeepsRows();
    void grayscaleDocumentShowsSingleGrayRow();
    void thumbnailsTrackDocument();
    void maskChannelRow();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::ChannelsPanel* panel_ = nullptr;
};

void ChannelsPanelTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void ChannelsPanelTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
    view_ = nullptr;
    panel_ = nullptr;
}

void ChannelsPanelTest::rowsAndThumbnails()
{
    const bool created = window_->newDocument(QStringLiteral("Channels"), 16, 16,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY2(created && view_ && panel_, "channels fixture");

    panel_->setView(view_);
    panel_->refresh();
    QCOMPARE(panel_->channelCountForTest(), 4);
    QCOMPARE(panel_->channelNameForTest(0), QStringLiteral("RGB"));
    QCOMPARE(panel_->channelNameForTest(1), QStringLiteral("Red"));
    QCOMPARE(panel_->channelNameForTest(2), QStringLiteral("Green"));
    QCOMPARE(panel_->channelNameForTest(3), QStringLiteral("Blue"));
    for (int i = 0; i < panel_->channelCountForTest(); ++i) {
        QVERIFY2(!panel_->channelThumbnailForTest(i).isNull(), "thumbnail pixmap");
    }
}

// The RGB eyes hide channels on the canvas (#71): one hidden shows the other
// two in colour, two hidden show the last as greyscale, the last cannot be
// hidden, and the composite eye (on only when all are) shows them all again.
void ChannelsPanelTest::eyesHideChannelsOnTheCanvas()
{
    QTemporaryDir dir;
    QImage image(40, 40, QImage::Format_RGB32);
    image.fill(QColor(100, 150, 200));
    const QString path = dir.filePath(QStringLiteral("photo.png"));
    QVERIFY(image.save(path) && window_->openDocumentAtPath(path));
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    pictura::ImageView* canvas = window_->imageView();
    QVERIFY(view_ && panel_ && canvas);
    panel_->setView(view_);
    QCOMPARE(panel_->channelShortcutForTest(0), QStringLiteral("Ctrl+2"));
    QCOMPARE(panel_->channelShortcutForTest(3), QStringLiteral("Ctrl+5"));

    const auto centre = [canvas]() {
        const QPointF p = canvas->imageToWidget(QPointF(20, 20));
        return QColor(canvas->grab().toImage().pixel(p.toPoint()));
    };
    QCOMPARE(centre(), QColor(100, 150, 200));

    panel_->toggleChannelForTest(1);
    QVERIFY(!panel_->channelVisibleForTest(1) && !panel_->channelVisibleForTest(0));
    QCOMPARE(canvas->channelMask(), 0x6);
    QCOMPARE(centre(), QColor(0, 150, 200));

    panel_->toggleChannelForTest(2);
    QCOMPARE(canvas->channelMask(), 0x4);
    QCOMPARE(centre(), QColor(200, 200, 200));
    panel_->toggleChannelForTest(3);
    QVERIFY2(panel_->channelVisibleForTest(3), "the last visible channel stays");
    QCOMPARE(canvas->channelMask(), 0x4);

    panel_->toggleChannelForTest(0);
    QCOMPARE(canvas->channelMask(), 0x7);
    for (int i = 0; i < 4; ++i) {
        QVERIFY(panel_->channelVisibleForTest(i));
    }
    QCOMPARE(centre(), QColor(100, 150, 200));

    // Another document shows all its channels.
    panel_->toggleChannelForTest(1);
    QVERIFY(window_->newDocument(QStringLiteral("Other"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    panel_->setView(window_->activeView());
    QVERIFY(panel_->channelVisibleForTest(1));
    QCOMPARE(window_->imageView()->channelMask(), 0x7);
}

// Alpha channels: New Channel (black) and Delete, Save Selection as Channel,
// and Load Channel as Selection, each from the footer.
void ChannelsPanelTest::alphaChannelsAndFooter()
{
    QVERIFY(window_->newDocument(QStringLiteral("Alpha"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    const auto button = [this](const char* name) {
        return panel_->findChild<QToolButton*>(QLatin1String(name));
    };
    QToolButton* add = button("channelsNew");
    QToolButton* remove = button("channelsDelete");
    QToolButton* save = button("channelsSaveSelection");
    QToolButton* load = button("channelsLoadSelection");
    QVERIFY(add && remove && save && load);
    QVERIFY(add->isEnabled() && !remove->isEnabled() && !save->isEnabled());

    add->click();
    QCOMPARE(panel_->channelCountForTest(), 5);
    QCOMPARE(panel_->channelNameForTest(4), QStringLiteral("Alpha 1"));
    QCOMPARE(panel_->channelShortcutForTest(4), QStringLiteral("Ctrl+6"));
    QVERIFY(!panel_->channelEyeEnabledForTest(4));
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("New Channel"));
    QVERIFY(remove->isEnabled());
    remove->click();
    QCOMPARE(panel_->channelCountForTest(), 4);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Delete Channel"));

    QVERIFY(view_->select_rect(2, 2, 6, 6, QStringLiteral("new"), 0.0));
    panel_->refresh();
    QVERIFY(save->isEnabled());
    save->click();
    QCOMPARE(panel_->channelNameForTest(4), QStringLiteral("Alpha 1"));
    view_->deselect();
    panel_->selectChannelForTest(4);
    QVERIFY(load->isEnabled());
    load->click();
    QVERIFY(view_->has_selection());
    QCOMPARE(view_->selection_coverage(4, 4), 255);
    QCOMPARE(view_->selection_coverage(12, 12), 0);

    // A Ctrl-click on the alpha row loads it through the same path.
    view_->deselect();
    panel_->clickChannelForTest(4, Qt::ControlModifier);
    QVERIFY(view_->has_selection());
    QCOMPARE(view_->selection_coverage(4, 4), 255);
    QCOMPARE(view_->selection_coverage(12, 12), 0);
}

// Ctrl-clicking a colour row loads that channel's pixels as the selection: the
// composite row its luminance over the visible channels, each colour row its
// own plane; a plain click only moves the current row (#438).
void ChannelsPanelTest::ctrlClickLoadsChannelSelection()
{
    QTemporaryDir dir;
    QImage image(16, 16, QImage::Format_RGB32);
    image.fill(QColor(100, 150, 200));
    const QString path = dir.filePath(QStringLiteral("flat.png"));
    QVERIFY(image.save(path) && window_->openDocumentAtPath(path));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);

    panel_->clickChannelForTest(1, Qt::ControlModifier);
    QVERIFY(view_->has_selection());
    QCOMPARE(view_->selection_coverage(8, 8), 100);
    panel_->clickChannelForTest(2, Qt::ControlModifier);
    QCOMPARE(view_->selection_coverage(8, 8), 150);
    panel_->clickChannelForTest(3, Qt::ControlModifier);
    QCOMPARE(view_->selection_coverage(8, 8), 200);
    panel_->clickChannelForTest(0, Qt::ControlModifier);
    QCOMPARE(view_->selection_coverage(8, 8), 140);

    // The composite follows the visible channels: hiding Red loads (0, 150, 200).
    panel_->toggleChannelForTest(1);
    panel_->clickChannelForTest(0, Qt::ControlModifier);
    QCOMPARE(view_->selection_coverage(8, 8), 106);
    panel_->toggleChannelForTest(1);

    // A plain click moves the current row but loads no selection.
    view_->deselect();
    panel_->selectChannelForTest(1);
    panel_->clickChannelForTest(3, Qt::NoModifier);
    QCOMPARE(panel_->currentChannelForTest(), 3);
    QVERIFY(!view_->has_selection());
}

void ChannelsPanelTest::refreshKeepsRows()
{
    const bool created = window_->newDocument(QStringLiteral("Stable"), 16, 16,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY2(created && view_ && panel_, "stable fixture");
    panel_->setView(view_);
    panel_->refresh();
    const int before = panel_->channelCountForTest();
    panel_->refresh();
    QCOMPARE(panel_->channelCountForTest(), before);
}

void ChannelsPanelTest::grayscaleDocumentShowsSingleGrayRow()
{
    const bool created = window_->newDocument(QStringLiteral("Gray"), 16, 16,
                                              QStringLiteral("grayscale"), 8,
                                              QStringLiteral("white"));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY2(created && view_ && panel_, "grayscale fixture");
    panel_->setView(view_);
    QCOMPARE(panel_->channelCountForTest(), 1);
    QCOMPARE(panel_->channelNameForTest(0), QStringLiteral("Gray"));
}

void ChannelsPanelTest::thumbnailsTrackDocument()
{
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(panel_ != nullptr);
    panel_->setView(nullptr);
    QVERIFY2(panel_->channelThumbnailForTest(0).isNull(), "no document -> no thumbnail");

    const bool created = window_->newDocument(QStringLiteral("Thumbs"), 16, 16,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    view_ = window_->activeView();
    QVERIFY2(created && view_, "thumbs fixture");
    panel_->setView(view_);
    QVERIFY2(!panel_->channelThumbnailForTest(0).isNull(), "document -> thumbnail");
}

// The active layer's raster mask is listed as a channel after the colour rows,
// with a Ctrl+\ label; selecting it shows the mask on the canvas (view-only
// solo), and selecting a colour row restores the composite.
void ChannelsPanelTest::maskChannelRow()
{
    const bool created = window_->newDocument(QStringLiteral("MaskChannel"), 32, 32,
                                              QStringLiteral("rgb"), 8, QStringLiteral("white"));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY2(created && view_ && panel_, "mask channel fixture");
    panel_->setView(view_);
    panel_->refresh();
    QCOMPARE(panel_->maskRowIndexForTest(), -1);

    const QString layer = view_->add_solid_fill(0xff2244aau);
    if (!layer.isEmpty()) {
        pictura::layer_mask_delete_path(*view_, layer);
    }
    QVERIFY(!layer.isEmpty());
    QVERIFY(view_->rasterize_fill_content(layer));
    window_->selectLayerPath(layer);
    QVERIFY2(pictura::layer_mask_add(*view_, QStringLiteral("reveal-all")), "add mask");
    panel_->setView(view_);
    panel_->refresh();

    const int maskRow = panel_->maskRowIndexForTest();
    QVERIFY2(maskRow >= 0, "the mask row is present");
    QVERIFY(panel_->channelNameForTest(maskRow).endsWith(QStringLiteral("Mask")));
    QCOMPARE(panel_->channelShortcutForTest(maskRow), QStringLiteral("Ctrl+\\"));
    QVERIFY2(!panel_->channelThumbnailForTest(maskRow).isNull(), "mask thumbnail");

    // Selecting the mask row shows the mask (view-only solo), with no history.
    const int base = view_->history_count();
    panel_->selectChannelForTest(maskRow);
    QCoreApplication::processEvents();
    QCOMPARE(pictura::mask_solo_get(*view_), layer);
    QCOMPARE(view_->history_count(), base);

    // Selecting a colour row restores the composite.
    panel_->selectChannelForTest(0);
    QCoreApplication::processEvents();
    QVERIFY(pictura::mask_solo_get(*view_).isEmpty());
}

QTEST_MAIN(ChannelsPanelTest)
#include "tst_channels_panel.moc"
