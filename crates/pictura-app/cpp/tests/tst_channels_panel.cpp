#include <QtTest/QtTest>

#include "channel_drag.h"
#include "commands.h"
#include "duplicate_channel_dialog.h"
#include "frame.h"
#include "image_view.h"
#include "panels/channels_panel.h"
#include "panels/panel_group.h"
#include "api/layers_bridge.h"
#include "api/selection_bridge.h"

#include <QtTest/QSignalSpy>
#include <QtCore/QTimer>
#include <QtCore/QTemporaryDir>
#include <QtGui/QAction>
#include <QtGui/QDragEnterEvent>
#include <QtGui/QDropEvent>
#include <QtGui/QPixmap>
#include <QtCore/QMimeData>
#include <QtWidgets/QApplication>
#include <QtWidgets/QDialog>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMenu>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QTabBar>
#include <QtWidgets/QToolButton>

#include <cstddef>
#include <cstdint>
#include <functional>

#include "qt_test_support.h"

namespace {

QMenu* livePanelMenu(pictura::PicturaMainWindow& window, const QString& panelName)
{
    for (pictura::PanelGroup* group : window.findChildren<pictura::PanelGroup*>()) {
        if (group->containsPanel(panelName)) {
            group->setCurrentPanel(panelName);
            break;
        }
    }
    auto* button =
        window.findChild<QToolButton*>(QStringLiteral("panelWidgetMenu_") + panelName);
    return button ? button->menu() : nullptr;
}

QAction* menuText(QMenu* menu, const QString& text)
{
    for (QAction* action : menu->actions()) {
        if (action->text() == text) {
            return action;
        }
        if (QMenu* submenu = action->menu()) {
            if (QAction* found = menuText(submenu, text)) {
                return found;
            }
        }
    }
    return nullptr;
}

bool applyDuplicateDialog(
    pictura::PicturaMainWindow& window,
    const std::function<void(pictura::DuplicateChannelDialog*)>& setup)
{
    window.show();
    QTest::qWait(0);
    bool seen = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = qobject_cast<pictura::DuplicateChannelDialog*>(
            QApplication::activeModalWidget());
        if (dialog) {
            seen = true;
            setup(dialog);
            dialog->accept();
        }
    });
    window.openDuplicateChannelDialog();
    return seen;
}

bool dragMimeOver(QWidget* target, const QMimeData* mime, const QPoint& pos, bool drop)
{
    const auto actions = Qt::MoveAction | Qt::CopyAction;
    QDragEnterEvent enter(pos, actions, mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(target, &enter);
    QDragMoveEvent move(pos, actions, mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(target, &move);
    if (!drop) {
        return move.isAccepted();
    }
    QDropEvent release(QPointF(pos), actions, mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(target, &release);
    return release.isAccepted();
}

QByteArray coverageBytes(const ::rust::Vec<std::uint8_t>& coverage)
{
    return QByteArray(reinterpret_cast<const char*>(coverage.data()), int(coverage.size()));
}

void addNamedAlpha(pictura::PictureView* view, const QString& name, int x, int y, int w, int h)
{
    view->select_rect(x, y, w, h, QStringLiteral("new"), 0.0);
    pictura::save_selection_to(*view, QStringLiteral("new"), name, QStringLiteral("replace"));
    view->deselect();
}

} // namespace

class ChannelsPanelTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void rowsAndThumbnails();
    void rgbSelectionAndVisibility();
    void rgbAdjustmentAvailability();
    void eyesHideChannelsOnTheCanvas();
    void channelShortcutsToggleFromTheCanvas();
    void alphaChannelsAndFooter();
    void newChannelDisabledAtCap();
    void ctrlClickLoadsChannelSelection();
    void refreshKeepsRows();
    void grayscaleDocumentShowsSingleGrayRow();
    void thumbnailsTrackDocument();
    void maskChannelRow();
    void ctrlShiftAdds();
    void ctrlAltSubtracts();
    void ctrlShiftAltIntersects();
    void dragLoadsChannel();
    void closedChannelSourceDropsIgnored();
    void altClickSaveOpensDialog();
    void alphaRowShowsStoredName();
    void modifierClickRecordsOneState();
    void clickChannelRowSelects();
    void clickChannelNameHitsRow();
    void clickAlphaEyeKeepsSelectedRow();
    void alphaEyeShowsChannel();
    void alphaEyesShowMultiple();
    void alphaEyesAccumulateRedness();
    void deletingShownAlphaClearsView();
    void duplicateChannelMenu();
    void duplicateChannelColorRow();
    void duplicateChannelSameDocument();
    void duplicateChannelCrossDocument();
    void duplicateChannelNewDocument();
    void duplicateChannelInvert();
    void duplicateChannelOntoNewButton();
    void crossDocumentNewButtonDrop();
    void dragChannelOntoAnotherDocument();
    void dragChannelToDifferentSizeFits();
    void dragChannelInvalidSourceIgnored();
    void duplicateChannelAtCapRefused();

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

void ChannelsPanelTest::rgbSelectionAndVisibility()
{
    QVERIFY(window_->newDocument(QStringLiteral("RGB rows"), 16, 16,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    auto* row = panel_->rowWidgetForTest(0);
    auto* thumb = row->findChild<QLabel*>(QStringLiteral("channelsRowThumb"));
    QVERIFY(thumb);
    QCOMPARE(thumb->geometry().top(), 2);
    QCOMPARE(row->height() - thumb->geometry().bottom() - 1, 2);
    QCOMPARE(panel_->eyeForTest(0)->width(), 30);
    panel_->clickChannelForTest(1, Qt::NoModifier);
    QCOMPARE(panel_->currentChannelForTest(), 1);
    QCOMPARE(panel_->channelMask(), 1);
    panel_->clickChannelForTest(0, Qt::NoModifier);
    QCOMPARE(panel_->channelMask(), 7);
    QCOMPARE(panel_->rowSurfaceForTest(0), panel_->rowSurfaceForTest(1));
    QCOMPARE(panel_->rowSurfaceForTest(0), panel_->rowSurfaceForTest(2));
    QCOMPARE(panel_->rowSurfaceForTest(0), panel_->rowSurfaceForTest(3));
    panel_->eyeForTest(3)->click();
    QCOMPARE(panel_->currentChannelForTest(), 0);
    QCOMPARE(panel_->channelMask(), 3);
    QVERIFY(!panel_->channelVisibleForTest(3));
    panel_->clickChannelForTest(1, Qt::NoModifier);
    QCOMPARE(panel_->currentChannelForTest(), 1);
    QCOMPARE(panel_->channelMask(), 3);
    panel_->clickChannelForTest(3, Qt::NoModifier);
    QCOMPARE(panel_->currentChannelForTest(), 3);
    QCOMPARE(panel_->channelMask(), 4);
    QVERIFY(panel_->channelVisibleForTest(3));
    QVERIFY(!panel_->channelVisibleForTest(1));
    panel_->clickChannelForTest(0, Qt::NoModifier);
    QCOMPARE(panel_->channelMask(), 7);
    for (int i = 0; i < 4; ++i) {
        QVERIFY(panel_->channelVisibleForTest(i));
    }
}

void ChannelsPanelTest::rgbAdjustmentAvailability()
{
    QVERIFY(window_->newDocument(QStringLiteral("RGB adjustments"), 16, 16,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    const auto action = [this](const char* name) {
        return window_->registry()->action(pictura::commandIdForPath(
            {QStringLiteral("Image"), QStringLiteral("Adjustments"), QString::fromUtf8(name)}));
    };
    window_->registry()->refresh();
    const auto check = [&](bool composite) {
        for (const char* name : {"Exposure", "Vibrance", "Hue/Saturation", "Color Balance",
                                 "Black & White", "Photo Filter", "Channel Mixer", "Color Lookup",
                                 "Gradient Map", "Selective Color", "Desaturate", "Match Color"}) {
            QVERIFY2(action(name), name);
            QCOMPARE(action(name)->isEnabled(), composite);
        }
        for (const char* name : {"Brightness/Contrast", "Levels", "Curves", "Invert",
                                 "Posterize", "Threshold", "Shadows/Highlights", "HDR Toning",
                                 "Replace Color", "Equalize"}) {
            QVERIFY2(action(name) && action(name)->isEnabled(), name);
        }
    };
    check(true);
    panel_->eyeForTest(3)->click();
    check(false);
    panel_->clickChannelForTest(1, Qt::NoModifier);
    check(false);
    panel_->clickChannelForTest(0, Qt::NoModifier);
    check(true);
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

// #1553: the rows' Ctrl+2..5 labels work from the canvas, not only while the
// Channels panel has focus. Ctrl+3/4/5 flip Red/Green/Blue, and Ctrl+2 restores
// every channel.
void ChannelsPanelTest::channelShortcutsToggleFromTheCanvas()
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
    canvas->setFocus();
    QCOMPARE(canvas->channelMask(), 0x7);

    QTest::keyClick(canvas, Qt::Key_3, Qt::ControlModifier);
    QVERIFY(!panel_->channelVisibleForTest(1));
    QCOMPARE(canvas->channelMask(), 0x6);
    QTest::keyClick(canvas, Qt::Key_3, Qt::ControlModifier);
    QVERIFY(panel_->channelVisibleForTest(1));
    QCOMPARE(canvas->channelMask(), 0x7);

    QTest::keyClick(canvas, Qt::Key_4, Qt::ControlModifier);
    QCOMPARE(canvas->channelMask(), 0x5);
    QTest::keyClick(canvas, Qt::Key_4, Qt::ControlModifier);
    QCOMPARE(canvas->channelMask(), 0x7);

    QTest::keyClick(canvas, Qt::Key_5, Qt::ControlModifier);
    QCOMPARE(canvas->channelMask(), 0x3);
    QTest::keyClick(canvas, Qt::Key_5, Qt::ControlModifier);
    QCOMPARE(canvas->channelMask(), 0x7);

    QTest::keyClick(canvas, Qt::Key_3, Qt::ControlModifier);
    QCOMPARE(canvas->channelMask(), 0x6);
    QTest::keyClick(canvas, Qt::Key_2, Qt::ControlModifier);
    QCOMPARE(canvas->channelMask(), 0x7);
    for (int i = 0; i < 4; ++i) {
        QVERIFY(panel_->channelVisibleForTest(i));
    }
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
    QVERIFY(panel_->channelEyeEnabledForTest(4));
    QVERIFY(!panel_->channelVisibleForTest(4));
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

// The 56-channel budget disables New Channel and refuses the extra add.
void ChannelsPanelTest::newChannelDisabledAtCap()
{
    QVERIFY(window_->newDocument(QStringLiteral("Cap"), 8, 8, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    for (int i = 0; i < 53; ++i) {
        QCOMPARE(pictura::channels_new_alpha(*view_), i);
    }
    QCOMPARE(view_->selection_channel_count(), 53);
    const int history = view_->history_count();
    QCOMPARE(pictura::channels_new_alpha(*view_), -1);
    QCOMPARE(view_->selection_channel_count(), 53);
    QCOMPARE(view_->history_count(), history);
    panel_->refresh();
    auto* add = panel_->findChild<QToolButton*>(QStringLiteral("channelsNew"));
    QVERIFY(add);
    QVERIFY(!add->isEnabled());
    QVERIFY(add->toolTip().contains(QString::number(pictura::channels_max())));
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


void ChannelsPanelTest::ctrlShiftAdds()
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
    const QByteArray base(16 * 16, 64);
    QVERIFY(pictura::load_selection_coverage(*view_,
        ::rust::Slice<const std::uint8_t>(
            reinterpret_cast<const std::uint8_t*>(base.constData()), base.size()),
        16, 16, false, QStringLiteral("new")));
    panel_->clickChannelForTest(1, Qt::ControlModifier | Qt::ShiftModifier);
    QCOMPARE(view_->selection_coverage(8, 8), 100);
}

void ChannelsPanelTest::ctrlAltSubtracts()
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
    view_->select_all();
    panel_->clickChannelForTest(1, Qt::ControlModifier | Qt::AltModifier);
    QCOMPARE(view_->selection_coverage(8, 8), 155);
}

void ChannelsPanelTest::ctrlShiftAltIntersects()
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
    view_->select_all();
    panel_->clickChannelForTest(1, Qt::ControlModifier | Qt::ShiftModifier | Qt::AltModifier);
    QCOMPARE(view_->selection_coverage(8, 8), 100);
}

void ChannelsPanelTest::dragLoadsChannel()
{
    QVERIFY(window_->newDocument(QStringLiteral("Drag"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    view_->select_all();
    QVERIFY(pictura::save_selection_to(*view_, QStringLiteral("new"), QStringLiteral("Hair"),
                                     QStringLiteral("replace")));
    view_->deselect();
    panel_->refresh();
    QToolButton* load = panel_->findChild<QToolButton*>(QStringLiteral("channelsLoadSelection"));
    QVERIFY(load);
    std::unique_ptr<QMimeData> mime(pictura::makeChannelDragMime(
        view_, 4, 0, 0x7, QStringLiteral("Hair")));
    QVERIFY(mime);
    QDragEnterEvent enter(QPoint(0, 0), Qt::CopyAction, mime.get(), Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(load, &enter);
    QVERIFY(enter.isAccepted());
    QDropEvent drop(QPointF(0, 0), Qt::CopyAction, mime.get(), Qt::LeftButton, Qt::NoModifier);
    const int before = view_->history_count();
    QCoreApplication::sendEvent(load, &drop);
    QVERIFY2(drop.isAccepted(), "the Load button did not accept the channel drop");
    QCOMPARE(view_->history_count(), before + 1);
    QVERIFY(view_->has_selection());
    QCOMPARE(view_->selection_coverage(8, 8), 255);
}

void ChannelsPanelTest::closedChannelSourceDropsIgnored()
{
    QVERIFY(window_->newDocument(QStringLiteral("ClosedSource"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* source = window_->activeView();
    QVERIFY(source);
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(panel_);
    panel_->setView(source);
    panel_->refresh();
    panel_->selectChannelForTest(1);
    std::unique_ptr<QMimeData> mime(panel_->channelDragMime(1));
    QVERIFY(mime);

    QVERIFY(window_->newDocument(QStringLiteral("ClosedTarget"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* target = window_->activeView();
    QVERIFY(target);
    view_ = target;
    panel_->setView(target);
    panel_->refresh();
    auto* add = panel_->findChild<QToolButton*>(QStringLiteral("channelsNew"));
    auto* load = panel_->findChild<QToolButton*>(QStringLiteral("channelsLoadSelection"));
    QVERIFY(add && load);
    const int channels = target->selection_channel_count();
    const int history = target->history_count();
    QVERIFY(!target->has_selection());
    QVERIFY(window_->closeDocument(0, false));

    QVERIFY2(!dragMimeOver(add, mime.get(), QPoint(1, 1), true),
             "a closed source is ignored by New Channel");
    QVERIFY2(!dragMimeOver(load, mime.get(), QPoint(1, 1), true),
             "a closed source is ignored by Load Selection");
    QCOMPARE(target->selection_channel_count(), channels);
    QCOMPARE(target->history_count(), history);
    QVERIFY(!target->has_selection());
}

void ChannelsPanelTest::altClickSaveOpensDialog()
{
    QVERIFY(window_->newDocument(QStringLiteral("AltSave"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    QToolButton* save = panel_->findChild<QToolButton*>(QStringLiteral("channelsSaveSelection"));
    QVERIFY(save);
    const int channels = view_->selection_channel_count();
    const int history = view_->history_count();
    QSignalSpy spy(panel_, &pictura::ChannelsPanel::saveOptionsRequested);
    QTest::mouseClick(save, Qt::LeftButton, Qt::AltModifier);
    QCOMPARE(spy.count(), 1);
    QCOMPARE(view_->selection_channel_count(), channels);
    QCOMPARE(view_->history_count(), history);
}

void ChannelsPanelTest::alphaRowShowsStoredName()
{
    QVERIFY(window_->newDocument(QStringLiteral("Names"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    view_->select_all();
    QVERIFY(pictura::save_selection_to(*view_, QStringLiteral("new"), QStringLiteral("Hair"),
                                     QStringLiteral("replace")));
    panel_->refresh();
    QCOMPARE(panel_->channelNameForTest(4), QStringLiteral("Hair"));
    QCOMPARE(panel_->channelShortcutForTest(4), QStringLiteral("Ctrl+6"));
}

void ChannelsPanelTest::modifierClickRecordsOneState()
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
    const QList<Qt::KeyboardModifiers> modes {
        Qt::ControlModifier,
        Qt::ControlModifier | Qt::ShiftModifier,
        Qt::ControlModifier | Qt::AltModifier,
        Qt::ControlModifier | Qt::ShiftModifier | Qt::AltModifier,
    };
    for (Qt::KeyboardModifiers mods : modes) {
        view_->select_all();
        const int before = view_->history_count();
        panel_->clickChannelForTest(1, mods);
        QCOMPARE(view_->history_count(), before + 1);
    }
}


void ChannelsPanelTest::clickChannelRowSelects()
{
    QVERIFY(window_->newDocument(QStringLiteral("RowClick"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    addNamedAlpha(view_, QStringLiteral("Hair"), 2, 2, 8, 8);
    panel_->refresh();
    panel_->clickChannelForTest(4, Qt::NoModifier);
    QCOMPARE(panel_->currentChannelForTest(), 4);
}

void ChannelsPanelTest::clickChannelNameHitsRow()
{
    QVERIFY(window_->newDocument(QStringLiteral("NameHit"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    addNamedAlpha(view_, QStringLiteral("Hair"), 2, 2, 8, 8);
    panel_->refresh();
    for (pictura::PanelGroup* group : window_->findChildren<pictura::PanelGroup*>()) {
        if (group->containsPanel(QStringLiteral("channelsPanel"))) {
            group->setCurrentPanel(QStringLiteral("channelsPanel"));
            break;
        }
    }
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    QLabel* alpha = nullptr;
    for (QLabel* label : panel_->findChildren<QLabel*>(QStringLiteral("channelsRowName"))) {
        if (label->text() == QStringLiteral("Hair")) {
            alpha = label;
            break;
        }
    }
    QVERIFY(alpha);
    const QPoint global = alpha->mapToGlobal(alpha->rect().center());
    QWidget* hit = window_->childAt(window_->mapFromGlobal(global));
    QVERIFY(hit);
    QTest::mouseClick(hit, Qt::LeftButton, Qt::NoModifier, hit->mapFromGlobal(global));
    QCOMPARE(panel_->currentChannelForTest(), 4);
}

void ChannelsPanelTest::clickAlphaEyeKeepsSelectedRow()
{
    QVERIFY(window_->newDocument(QStringLiteral("EyeHit"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    addNamedAlpha(view_, QStringLiteral("Hair"), 2, 2, 8, 8);
    panel_->refresh();
    for (pictura::PanelGroup* group : window_->findChildren<pictura::PanelGroup*>()) {
        if (group->containsPanel(QStringLiteral("channelsPanel"))) {
            group->setCurrentPanel(QStringLiteral("channelsPanel"));
            break;
        }
    }
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    QToolButton* eye = panel_->eyeForTest(4);
    QVERIFY(eye && eye->isEnabled());
    QTest::mouseClick(eye, Qt::LeftButton, Qt::NoModifier, eye->rect().center());
    QCOMPARE(panel_->currentChannelForTest(), 0);
    QVERIFY(panel_->shownAlphas().contains(0));
}

void ChannelsPanelTest::alphaEyeShowsChannel()
{
    QVERIFY(window_->newDocument(QStringLiteral("AlphaEye"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    addNamedAlpha(view_, QStringLiteral("Hair"), 2, 2, 8, 8);
    panel_->refresh();
    QToolButton* eye = panel_->eyeForTest(4);
    QVERIFY(eye && eye->isEnabled());
    panel_->toggleChannelForTest(4);
    QVERIFY(panel_->shownAlphas().contains(0));
    QVERIFY(eye->isChecked());
    pictura::ImageView* canvas = window_->imageView();
    QVERIFY(canvas);
    const QImage overlay = canvas->maskOverlayForTest();
    QVERIFY(!overlay.isNull());
    const QColor masked = overlay.pixelColor(0, 0);
    QCOMPARE(masked.red(), 255);
    QCOMPARE(masked.green(), 0);
    QCOMPARE(masked.blue(), 0);
    QCOMPARE(masked.alpha(), 128);
    QCOMPARE(overlay.pixelColor(4, 4).alpha(), 0);
    panel_->toggleChannelForTest(4);
    QVERIFY(panel_->shownAlphas().isEmpty());
    QVERIFY(canvas->maskOverlayForTest().isNull());
    panel_->toggleChannelForTest(4);
    QVERIFY(panel_->shownAlphas().contains(0));
    panel_->toggleChannelForTest(0);
    QVERIFY(panel_->shownAlphas().isEmpty());
    QVERIFY(canvas->maskOverlayForTest().isNull());
}

void ChannelsPanelTest::alphaEyesShowMultiple()
{
    QVERIFY(window_->newDocument(QStringLiteral("AlphaMulti"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    addNamedAlpha(view_, QStringLiteral("Hair"), 2, 2, 8, 8);
    addNamedAlpha(view_, QStringLiteral("Skin"), 10, 10, 4, 4);
    panel_->refresh();
    panel_->toggleChannelForTest(4);
    panel_->toggleChannelForTest(5);
    QCOMPARE(panel_->shownAlphas().size(), 2);
    QVERIFY(panel_->eyeForTest(4)->isChecked());
    QVERIFY(panel_->eyeForTest(5)->isChecked());
    pictura::ImageView* canvas = window_->imageView();
    QVERIFY(canvas);
    QImage overlay = canvas->maskOverlayForTest();
    QVERIFY(!overlay.isNull());
    QCOMPARE(overlay.pixelColor(0, 0).alpha(), 192);
    QCOMPARE(overlay.pixelColor(4, 4).alpha(), 128);
    QCOMPARE(overlay.pixelColor(11, 11).alpha(), 128);
    QCOMPARE(overlay.pixelColor(4, 4).red(), 255);
    QCOMPARE(overlay.pixelColor(4, 4).green(), 0);
    panel_->toggleChannelForTest(4);
    QCOMPARE(panel_->shownAlphas().size(), 1);
    overlay = canvas->maskOverlayForTest();
    QVERIFY(!overlay.isNull());
    QCOMPARE(overlay.pixelColor(0, 0).alpha(), 128);
    QCOMPARE(overlay.pixelColor(4, 4).alpha(), 128);
    QCOMPARE(overlay.pixelColor(11, 11).alpha(), 0);
}

void ChannelsPanelTest::alphaEyesAccumulateRedness()
{
    QVERIFY(window_->newDocument(QStringLiteral("AlphaRed"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    QCOMPARE(pictura::channels_new_alpha(*view_), 0);
    QCOMPARE(pictura::channels_new_alpha(*view_), 1);
    QCOMPARE(pictura::channels_new_alpha(*view_), 2);
    panel_->refresh();
    pictura::ImageView* canvas = window_->imageView();
    QVERIFY(canvas);
    panel_->toggleChannelForTest(4);
    QCOMPARE(canvas->maskOverlayForTest().pixelColor(0, 0).alpha(), 128);
    panel_->toggleChannelForTest(5);
    QCOMPARE(canvas->maskOverlayForTest().pixelColor(0, 0).alpha(), 192);
    panel_->toggleChannelForTest(6);
    const QImage overlay = canvas->maskOverlayForTest();
    const QColor red = overlay.pixelColor(0, 0);
    QCOMPARE(red.red(), 255);
    QCOMPARE(red.green(), 0);
    QCOMPARE(red.blue(), 0);
    QCOMPARE(red.alpha(), 224);
    panel_->toggleChannelForTest(6);
    QCOMPARE(canvas->maskOverlayForTest().pixelColor(0, 0).alpha(), 192);
}

void ChannelsPanelTest::deletingShownAlphaClearsView()
{
    QVERIFY(window_->newDocument(QStringLiteral("AlphaDelete"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    addNamedAlpha(view_, QStringLiteral("Hair"), 2, 2, 8, 8);
    panel_->refresh();
    panel_->selectChannelForTest(4);
    panel_->toggleChannelForTest(4);
    QVERIFY(panel_->shownAlphas().contains(0));
    auto* remove = panel_->findChild<QToolButton*>(QStringLiteral("channelsDelete"));
    QVERIFY(remove && remove->isEnabled());
    remove->click();
    QVERIFY(panel_->shownAlphas().isEmpty());
    QCOMPARE(view_->selection_channel_count(), 0);
}

void ChannelsPanelTest::duplicateChannelMenu()
{
    QVERIFY(window_->newDocument(QStringLiteral("Menu"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    addNamedAlpha(view_, QStringLiteral("Hair"), 2, 2, 8, 8);
    panel_->refresh();
    panel_->clickChannelForTest(4, Qt::NoModifier);
    QCOMPARE(panel_->currentChannelForTest(), 4);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));

    QMenu* menu = livePanelMenu(*window_, QStringLiteral("channelsPanel"));
    QVERIFY(menu);
    QAction* action = menuText(menu, QStringLiteral("Duplicate Channel…"));
    QVERIFY2(action && action->isEnabled(), "Duplicate Channel… must be live");
    bool seen = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = qobject_cast<pictura::DuplicateChannelDialog*>(
            QApplication::activeModalWidget());
        if (dialog) {
            seen = true;
            dialog->setNameForTest(QStringLiteral("MenuCopy"));
            dialog->accept();
        }
    });
    action->trigger();
    QVERIFY(seen);
    QCOMPARE(view_->selection_channel_count(), 2);
    QCOMPARE(QString(pictura::channels_alpha_name(*view_, 1)), QStringLiteral("MenuCopy"));
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Duplicate Channel"));
}

void ChannelsPanelTest::duplicateChannelColorRow()
{
    QVERIFY(window_->newDocument(QStringLiteral("ColorRow"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    panel_->selectChannelForTest(1);
    QVERIFY(applyDuplicateDialog(*window_, [](pictura::DuplicateChannelDialog* dialog) {
        dialog->setNameForTest(QStringLiteral("Red copy"));
    }));
    QCOMPARE(view_->selection_channel_count(), 1);
    QCOMPARE(QString(pictura::channels_alpha_name(*view_, 0)), QStringLiteral("Red copy"));
    const QByteArray coverage =
        coverageBytes(pictura::channel_coverage(*view_, QStringLiteral("alpha:0")));
    QCOMPARE(coverage.size(), 16 * 16);
    QCOMPARE(static_cast<unsigned char>(coverage.at(0)), 255);
}

void ChannelsPanelTest::duplicateChannelSameDocument()
{
    QVERIFY(window_->newDocument(QStringLiteral("Same"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    addNamedAlpha(view_, QStringLiteral("Hair"), 2, 2, 8, 8);
    panel_->refresh();
    panel_->selectChannelForTest(4);
    const QByteArray source = coverageBytes(pictura::channel_coverage(*view_, QStringLiteral("alpha:0")));
    const int before = view_->history_count();
    QVERIFY(applyDuplicateDialog(*window_, [](pictura::DuplicateChannelDialog* dialog) {
        dialog->setNameForTest(QStringLiteral("Copy"));
    }));
    QCOMPARE(view_->selection_channel_count(), 2);
    QCOMPARE(QString(pictura::channels_alpha_name(*view_, 1)), QStringLiteral("Copy"));
    QCOMPARE(coverageBytes(pictura::channel_coverage(*view_, QStringLiteral("alpha:1"))), source);
    QCOMPARE(view_->history_count(), before + 1);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Duplicate Channel"));
}

void ChannelsPanelTest::duplicateChannelCrossDocument()
{
    QVERIFY(window_->newDocument(QStringLiteral("Source"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* source = window_->activeView();
    QVERIFY(source);
    addNamedAlpha(source, QStringLiteral("Hair"), 2, 2, 8, 8);
    QVERIFY(window_->newDocument(QStringLiteral("Target"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* target = window_->activeView();
    QVERIFY(target);
    window_->setActiveDocumentIndex(0);
    view_ = source;
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(panel_);
    panel_->setView(source);
    panel_->refresh();
    panel_->selectChannelForTest(4);
    const int sourceHistory = source->history_count();
    const int targetHistory = target->history_count();
    QVERIFY(applyDuplicateDialog(*window_, [](pictura::DuplicateChannelDialog* dialog) {
        dialog->setDocumentIndexForTest(1);
        dialog->setNameForTest(QStringLiteral("Cross"));
    }));
    QCOMPARE(source->selection_channel_count(), 1);
    QCOMPARE(source->history_count(), sourceHistory);
    QCOMPARE(target->selection_channel_count(), 1);
    QCOMPARE(QString(pictura::channels_alpha_name(*target, 0)), QStringLiteral("Cross"));
    QCOMPARE(target->history_count(), targetHistory + 1);
    QCOMPARE(target->history_label(target->history_index()), QStringLiteral("Duplicate Channel"));
}

void ChannelsPanelTest::duplicateChannelNewDocument()
{
    QVERIFY(window_->newDocument(QStringLiteral("NewSource"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    addNamedAlpha(view_, QStringLiteral("Hair"), 2, 2, 8, 8);
    panel_->refresh();
    panel_->selectChannelForTest(4);
    const int before = window_->documentCount();
    QVERIFY(applyDuplicateDialog(*window_, [](pictura::DuplicateChannelDialog* dialog) {
        dialog->setDocumentIndexForTest(dialog->documentCountForTest() - 1);
        dialog->setNameForTest(QStringLiteral("NewCopy"));
    }));
    QCOMPARE(window_->documentCount(), before + 1);
    pictura::PictureView* created = window_->activeView();
    QVERIFY(created && created != view_);
    QCOMPARE(QString(pictura::channels_mode(*created)), QStringLiteral("grayscale"));
    QCOMPARE(created->selection_channel_count(), 1);
    QCOMPARE(QString(pictura::channels_alpha_name(*created, 0)), QStringLiteral("NewCopy"));
}

void ChannelsPanelTest::duplicateChannelInvert()
{
    QVERIFY(window_->newDocument(QStringLiteral("Invert"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    const QByteArray flat(16 * 16, char(64));
    QVERIFY(pictura::load_selection_coverage(
        *view_,
        ::rust::Slice<const std::uint8_t>(
            reinterpret_cast<const std::uint8_t*>(flat.constData()), flat.size()),
        16, 16, false, QStringLiteral("new")));
    QVERIFY(pictura::save_selection_to(*view_, QStringLiteral("new"), QStringLiteral("Src"),
                                       QStringLiteral("replace")));
    panel_->refresh();
    panel_->selectChannelForTest(4);
    QVERIFY(applyDuplicateDialog(*window_, [](pictura::DuplicateChannelDialog* dialog) {
        dialog->setNameForTest(QStringLiteral("Inv"));
        dialog->setInvertForTest(true);
    }));
    const QByteArray inverted = coverageBytes(pictura::channel_coverage(*view_, QStringLiteral("alpha:1")));
    QCOMPARE(inverted.size(), 16 * 16);
    QCOMPARE(static_cast<unsigned char>(inverted.at(0)), 191);
    QCOMPARE(static_cast<unsigned char>(inverted.at(16 * 16 - 1)), 191);
}

void ChannelsPanelTest::duplicateChannelOntoNewButton()
{
    QVERIFY(window_->newDocument(QStringLiteral("Button"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    addNamedAlpha(view_, QStringLiteral("Hair"), 2, 2, 8, 8);
    panel_->refresh();
    panel_->selectChannelForTest(4);
    std::unique_ptr<QMimeData> mime(panel_->channelDragMime(4));
    QVERIFY(mime);
    auto* add = panel_->findChild<QToolButton*>(QStringLiteral("channelsNew"));
    QVERIFY(add);
    const int before = view_->history_count();
    QVERIFY2(dragMimeOver(add, mime.get(), QPoint(1, 1), true), "New Channel drop");
    QCOMPARE(view_->selection_channel_count(), 2);
    QCOMPARE(QString(pictura::channels_alpha_name(*view_, 1)), QStringLiteral("Hair copy"));
    QCOMPARE(view_->history_count(), before + 1);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Duplicate Channel"));
}

void ChannelsPanelTest::crossDocumentNewButtonDrop()
{
    QVERIFY(window_->newDocument(QStringLiteral("DropSource"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* source = window_->activeView();
    QVERIFY(source);
    addNamedAlpha(source, QStringLiteral("Hair"), 2, 2, 8, 8);
    QVERIFY(window_->newDocument(QStringLiteral("DropTarget"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* target = window_->activeView();
    QVERIFY(target);
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(panel_);
    panel_->setView(source);
    panel_->refresh();
    panel_->selectChannelForTest(4);
    std::unique_ptr<QMimeData> mime(panel_->channelDragMime(4));
    QVERIFY(mime);
    const QByteArray sourceCoverage =
        coverageBytes(pictura::channel_coverage(*source, QStringLiteral("alpha:0")));
    const int sourceHistory = source->history_count();

    panel_->setView(target);
    panel_->refresh();
    auto* add = panel_->findChild<QToolButton*>(QStringLiteral("channelsNew"));
    QVERIFY(add);
    const int before = target->history_count();
    QVERIFY2(dragMimeOver(add, mime.get(), QPoint(1, 1), true),
             "cross-document New Channel drop");
    QCOMPARE(target->selection_channel_count(), 1);
    QCOMPARE(QString(pictura::channels_alpha_name(*target, 0)), QStringLiteral("Hair copy"));
    QCOMPARE(coverageBytes(pictura::channel_coverage(*target, QStringLiteral("alpha:0"))),
             sourceCoverage);
    QCOMPARE(target->history_count(), before + 1);
    QCOMPARE(source->selection_channel_count(), 1);
    QCOMPARE(source->history_count(), sourceHistory);
}

void ChannelsPanelTest::dragChannelOntoAnotherDocument()
{
    QVERIFY(window_->newDocument(QStringLiteral("DragSource"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* source = window_->activeView();
    QVERIFY(source);
    addNamedAlpha(source, QStringLiteral("Hair"), 2, 2, 8, 8);
    QVERIFY(window_->newDocument(QStringLiteral("DragTarget"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* target = window_->activeView();
    QVERIFY(target);
    window_->setActiveDocumentIndex(0);
    view_ = source;
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(panel_);
    panel_->setView(source);
    panel_->refresh();
    panel_->selectChannelForTest(4);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    auto* bar = window_->findChild<QTabBar*>(QStringLiteral("documentTabBar"));
    QVERIFY(bar && bar->count() == 2);
    const QByteArray sourceCoverage =
        coverageBytes(pictura::channel_coverage(*source, QStringLiteral("alpha:0")));
    const int sourceHistory = source->history_count();
    const int targetHistory = target->history_count();
    std::unique_ptr<QMimeData> mime(panel_->channelDragMime(4));
    QVERIFY(mime);
    QVERIFY2(dragMimeOver(bar, mime.get(), bar->tabRect(1).center(), true), "tab drop");
    QCOMPARE(target->selection_channel_count(), 1);
    QCOMPARE(QString(pictura::channels_alpha_name(*target, 0)), QStringLiteral("Hair copy"));
    QCOMPARE(coverageBytes(pictura::channel_coverage(*target, QStringLiteral("alpha:0"))),
             sourceCoverage);
    QCOMPARE(target->history_count(), targetHistory + 1);
    QCOMPARE(source->selection_channel_count(), 1);
    QCOMPARE(coverageBytes(pictura::channel_coverage(*source, QStringLiteral("alpha:0"))),
             sourceCoverage);
    QCOMPARE(source->history_count(), sourceHistory);
}

void ChannelsPanelTest::dragChannelToDifferentSizeFits()
{
    QVERIFY(window_->newDocument(QStringLiteral("FitSource"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* source = window_->activeView();
    QVERIFY(source);
    addNamedAlpha(source, QStringLiteral("Hair"), 4, 4, 8, 8);
    QVERIFY(window_->newDocument(QStringLiteral("FitTarget"), 32, 32, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* target = window_->activeView();
    QVERIFY(target);
    window_->setActiveDocumentIndex(0);
    view_ = source;
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(panel_);
    panel_->setView(source);
    panel_->refresh();
    panel_->selectChannelForTest(4);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    auto* bar = window_->findChild<QTabBar*>(QStringLiteral("documentTabBar"));
    QVERIFY(bar && bar->count() == 2);
    std::unique_ptr<QMimeData> mime(panel_->channelDragMime(4));
    QVERIFY(mime);
    QVERIFY2(dragMimeOver(bar, mime.get(), bar->tabRect(1).center(), true), "tab drop");
    QCOMPARE(target->selection_channel_count(), 1);
    const QByteArray fitted =
        coverageBytes(pictura::channel_coverage(*target, QStringLiteral("alpha:0")));
    QCOMPARE(fitted.size(), 32 * 32);
    QCOMPARE(static_cast<unsigned char>(fitted.at(16 * 32 + 16)), 255);
    QCOMPARE(static_cast<unsigned char>(fitted.at(0)), 0);
    QCOMPARE(static_cast<unsigned char>(fitted.at(32 * 32 - 1)), 0);
}

void ChannelsPanelTest::dragChannelInvalidSourceIgnored()
{
    QVERIFY(window_->newDocument(QStringLiteral("BadChannelSource"), 16, 16, QStringLiteral("rgb"),
                                 8, QStringLiteral("white")));
    pictura::PictureView* source = window_->activeView();
    QVERIFY(source);
    addNamedAlpha(source, QStringLiteral("Hair"), 2, 2, 8, 8);
    QVERIFY(window_->newDocument(QStringLiteral("BadChannelTarget"), 16, 16, QStringLiteral("rgb"),
                                 8, QStringLiteral("white")));
    pictura::PictureView* target = window_->activeView();
    QVERIFY(target);
    window_->setActiveDocumentIndex(0);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    auto* bar = window_->findChild<QTabBar*>(QStringLiteral("documentTabBar"));
    QVERIFY(bar && bar->count() == 2);
    const int targetChannels = target->selection_channel_count();
    const int targetHistory = target->history_count();
    QMimeData mime;
    mime.setData(pictura::kChannelRowMime, QByteArray::number(4));
    mime.setData(pictura::kChannelSourceMime, QByteArray::number(quintptr(1)));
    QVERIFY2(!dragMimeOver(bar, &mime, bar->tabRect(1).center(), true),
             "an unknown source is ignored");
    QCOMPARE(window_->activeView(), source);
    QCOMPARE(target->selection_channel_count(), targetChannels);
    QCOMPARE(target->history_count(), targetHistory);
}

void ChannelsPanelTest::duplicateChannelAtCapRefused()
{
    QVERIFY(window_->newDocument(QStringLiteral("CapSource"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* source = window_->activeView();
    QVERIFY(source);
    addNamedAlpha(source, QStringLiteral("Hair"), 2, 2, 8, 8);
    QVERIFY(window_->newDocument(QStringLiteral("CapTarget"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* target = window_->activeView();
    QVERIFY(target);
    for (int i = 0; i < 53; ++i) {
        QCOMPARE(pictura::channels_new_alpha(*target), i);
    }
    const int before = target->history_count();
    QVERIFY(!pictura::transfer_duplicate_channel(*target, *source, QStringLiteral("alpha:0"), 0,
                                                 QStringLiteral("Refused"), false));
    QCOMPARE(target->selection_channel_count(), 53);
    QCOMPARE(target->history_count(), before);
}


PICTURA_QT_TEST_MAIN(ChannelsPanelTest)
#include "tst_channels_panel.moc"
