#include <QtTest/QtTest>

#include "commands.h"
#include "frame.h"
#include "image_view.h"
#include "panels/channels_panel.h"
#include "panels/panel_group.h"
#include "api/layers_bridge.h"
#include "api/selection_bridge.h"

#include <QtCore/QFile>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QImage>
#include <QtGui/QPixmap>
#include <QtWidgets/QApplication>
#include <QtWidgets/QDialog>
#include <QtWidgets/QMenu>
#include <QtWidgets/QRadioButton>
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

QByteArray coverageBytes(const ::rust::Vec<std::uint8_t>& coverage)
{
    return QByteArray(reinterpret_cast<const char*>(coverage.data()), int(coverage.size()));
}

QByteArray alphaPlaneBytes(pictura::PictureView* view, int index)
{
    int width = 0;
    int height = 0;
    const ::rust::Vec<std::uint8_t> plane =
        pictura::channels_alpha_thumbnail(*view, index, 4096, width, height);
    return coverageBytes(plane);
}

bool applyChannelOptions(pictura::PicturaMainWindow& window, bool selectedAreas, bool accept)
{
    window.show();
    QTest::qWait(0);
    bool seen = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = qobject_cast<QDialog*>(QApplication::activeModalWidget());
        if (!dialog || dialog->objectName() != QStringLiteral("channelOptionsDialog")) {
            return;
        }
        seen = true;
        if (auto* radio = dialog->findChild<QRadioButton*>(
                QStringLiteral("channelOptionsSelectedAreas"))) {
            radio->setChecked(selectedAreas);
        }
        if (auto* radio = dialog->findChild<QRadioButton*>(
                QStringLiteral("channelOptionsMaskedAreas"))) {
            radio->setChecked(!selectedAreas);
        }
        if (accept) {
            dialog->accept();
        } else {
            dialog->reject();
        }
    });
    window.openChannelOptionsDialog();
    return seen;
}

QString spotFixture(QTemporaryDir& dir)
{
    // A minimal 2x2 Grayscale PSD with one spot extra channel: the synthetic
    // input for the "spot row is not an alpha channel" case.
    static const char kHex[] =
        "3842505300010000000000000002000000020000000200080001000000000000001e3842494d04350000000000110000000100000000000000000000003202000000000000000000000000000000";
    const QByteArray bytes = QByteArray::fromHex(kHex);
    const QString path = dir.filePath(QStringLiteral("spot_channel.psd"));
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly) || file.write(bytes) != bytes.size()) {
        return QString();
    }
    return path;
}

void addNamedAlpha(pictura::PictureView* view, const QString& name, int x, int y, int w, int h)
{
    view->select_rect(x, y, w, h, QStringLiteral("new"), 0.0);
    pictura::save_selection_to(*view, QStringLiteral("new"), name, QStringLiteral("replace"));
    view->deselect();
}

} // namespace

class ChannelOptionsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void channelOptionsMenuRow();
    void channelOptionsDoubleClick();
    void channelOptionsCancel();
    void channelOptionsSameSense();
    void channelOptionsApply();
    void channelOptionsThumbnail();
    void channelOptionsOverlay();
    void channelOptionsOverlayRefresh();
    void duplicateSelectedChannelNormalizesToMasked();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
    pictura::PictureView* view_ = nullptr;
    pictura::ChannelsPanel* panel_ = nullptr;
};

void ChannelOptionsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void ChannelOptionsTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
    view_ = nullptr;
    panel_ = nullptr;
}

void ChannelOptionsTest::channelOptionsMenuRow()
{
    QVERIFY(window_->newDocument(QStringLiteral("Options"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    addNamedAlpha(view_, QStringLiteral("Hair"), 2, 2, 8, 8);
    panel_->refresh();
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));

    QMenu* menu = livePanelMenu(*window_, QStringLiteral("channelsPanel"));
    QVERIFY(menu);
    QAction* action = menuText(menu, QStringLiteral("Channel Options…"));
    QVERIFY2(action, "Channel Options… must be in the panel menu");

    panel_->selectChannelForTest(4);
    window_->registry()->applyState(action);
    QVERIFY2(action->isEnabled(), "an alpha row enables Channel Options");

    panel_->selectChannelForTest(1);
    window_->registry()->applyState(action);
    QVERIFY2(!action->isEnabled(), "a colour row disables Channel Options");

    const QString layer = view_->add_solid_fill(0xff2244aau);
    if (!layer.isEmpty()) {
        pictura::layer_mask_delete_path(*view_, layer);
    }
    QVERIFY(!layer.isEmpty());
    QVERIFY(view_->rasterize_fill_content(layer));
    window_->selectLayerPath(layer);
    QVERIFY(pictura::layer_mask_add(*view_, QStringLiteral("reveal-all")));
    panel_->setView(view_);
    panel_->refresh();
    const int maskRow = panel_->maskRowIndexForTest();
    QVERIFY(maskRow >= 0);
    panel_->selectChannelForTest(maskRow);
    window_->registry()->applyState(action);
    QVERIFY2(!action->isEnabled(), "the layer-mask row disables Channel Options");

    QVERIFY(pictura::quick_mask_set(*view_, true));
    panel_->refresh();
    const int quickRow = panel_->channelCountForTest() - 1;
    panel_->selectChannelForTest(quickRow);
    window_->registry()->applyState(action);
    QVERIFY2(!action->isEnabled(), "the Quick Mask row disables Channel Options");
    QVERIFY(pictura::quick_mask_set(*view_, false));

    QTemporaryDir spotDir;
    QVERIFY(spotDir.isValid());
    const QString fixture = spotFixture(spotDir);
    QVERIFY2(!fixture.isEmpty(), "spot fixture");
    QVERIFY(window_->openDocumentAtPath(fixture));
    view_ = window_->activeView();
    panel_->setView(view_);
    panel_->refresh();
    QCOMPARE(panel_->channelCountForTest(), 2);
    panel_->selectChannelForTest(1);
    menu = livePanelMenu(*window_, QStringLiteral("channelsPanel"));
    QVERIFY(menu);
    action = menuText(menu, QStringLiteral("Channel Options…"));
    QVERIFY(action);
    window_->registry()->applyState(action);
    QVERIFY2(!action->isEnabled(), "a spot row disables Channel Options");
}

void ChannelOptionsTest::channelOptionsDoubleClick()
{
    QVERIFY(window_->newDocument(QStringLiteral("Double"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    addNamedAlpha(view_, QStringLiteral("Hair"), 2, 2, 8, 8);
    panel_->refresh();
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));

    QWidget* row = panel_->rowWidgetForTest(4);
    QVERIFY(row);
    const bool selectedBefore = pictura::channels_selected_areas(*view_, 0);
    bool seen = false;
    bool senseOk = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = qobject_cast<QDialog*>(QApplication::activeModalWidget());
        if (dialog && dialog->objectName() == QStringLiteral("channelOptionsDialog")) {
            seen = true;
            auto* selectedRadio = dialog->findChild<QRadioButton*>(
                QStringLiteral("channelOptionsSelectedAreas"));
            auto* maskedRadio = dialog->findChild<QRadioButton*>(
                QStringLiteral("channelOptionsMaskedAreas"));
            senseOk = selectedRadio && maskedRadio
                      && selectedRadio->isChecked() == selectedBefore
                      && maskedRadio->isChecked() == !selectedBefore;
            dialog->reject();
        }
    });
    QTest::mouseDClick(row, Qt::LeftButton, Qt::NoModifier, QPoint(10, 10));
    QVERIFY2(seen, "double-clicking an alpha row opens Channel Options");
    QVERIFY2(senseOk, "the dialog opens with the current sense selected");
}

void ChannelOptionsTest::channelOptionsCancel()
{
    QVERIFY(window_->newDocument(QStringLiteral("Cancel"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    addNamedAlpha(view_, QStringLiteral("Hair"), 2, 2, 8, 8);
    panel_->refresh();
    panel_->selectChannelForTest(4);

    const QByteArray before = alphaPlaneBytes(view_, 0);
    const bool beforeSelected = pictura::channels_selected_areas(*view_, 0);
    const int history = view_->history_count();
    QVERIFY(applyChannelOptions(*window_, !beforeSelected, false));
    QCOMPARE(alphaPlaneBytes(view_, 0), before);
    QCOMPARE(pictura::channels_selected_areas(*view_, 0), beforeSelected);
    QCOMPARE(view_->history_count(), history);
}

void ChannelOptionsTest::channelOptionsSameSense()
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

    const QByteArray before = alphaPlaneBytes(view_, 0);
    const bool current = pictura::channels_selected_areas(*view_, 0);
    const int history = view_->history_count();
    QVERIFY(applyChannelOptions(*window_, current, true));
    QCOMPARE(alphaPlaneBytes(view_, 0), before);
    QCOMPARE(view_->history_count(), history);
}

void ChannelOptionsTest::channelOptionsApply()
{
    QVERIFY(window_->newDocument(QStringLiteral("Apply"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    addNamedAlpha(view_, QStringLiteral("Hair"), 2, 2, 8, 8);
    panel_->refresh();
    panel_->selectChannelForTest(4);

    const QByteArray planeBefore = alphaPlaneBytes(view_, 0);
    const QByteArray coverageBefore =
        coverageBytes(pictura::channel_coverage(*view_, QStringLiteral("alpha:0")));
    const int history = view_->history_count();
    QVERIFY(applyChannelOptions(*window_, true, true));
    QVERIFY(pictura::channels_selected_areas(*view_, 0));

    const QByteArray planeAfter = alphaPlaneBytes(view_, 0);
    QCOMPARE(planeAfter.size(), planeBefore.size());
    for (int i = 0; i < planeBefore.size(); ++i) {
        QCOMPARE(quint8(planeAfter.at(i)), quint8(255 - quint8(planeBefore.at(i))));
    }
    QCOMPARE(coverageBytes(pictura::channel_coverage(*view_, QStringLiteral("alpha:0"))),
             coverageBefore);
    QCOMPARE(view_->history_count(), history + 1);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Channel Options"));
}

void ChannelOptionsTest::channelOptionsThumbnail()
{
    QVERIFY(window_->newDocument(QStringLiteral("Thumb"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    addNamedAlpha(view_, QStringLiteral("Hair"), 2, 2, 8, 8);
    panel_->refresh();
    panel_->selectChannelForTest(4);

    const QByteArray before = alphaPlaneBytes(view_, 0);
    const QImage beforeThumb = panel_->channelThumbnailForTest(4).toImage();
    QVERIFY(applyChannelOptions(*window_, true, true));
    const QByteArray after = alphaPlaneBytes(view_, 0);
    for (int i = 0; i < before.size(); ++i) {
        QCOMPARE(quint8(after.at(i)), quint8(255 - quint8(before.at(i))));
    }
    QVERIFY(beforeThumb != panel_->channelThumbnailForTest(4).toImage());
}

void ChannelOptionsTest::channelOptionsOverlay()
{
    QVERIFY(window_->newDocument(QStringLiteral("Overlay"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    addNamedAlpha(view_, QStringLiteral("Hair"), 2, 2, 8, 8);
    panel_->refresh();
    panel_->selectChannelForTest(4);
    QVERIFY(applyChannelOptions(*window_, true, true));

    panel_->toggleChannelForTest(4);
    QVERIFY(panel_->shownAlphas().contains(0));
    pictura::ImageView* canvas = window_->imageView();
    QVERIFY(canvas);
    const QImage overlay = canvas->maskOverlayForTest();
    QVERIFY(!overlay.isNull());
    QCOMPARE(overlay.pixelColor(4, 4).alpha(), 128);
    QCOMPARE(overlay.pixelColor(4, 4).red(), 255);
    QCOMPARE(overlay.pixelColor(0, 0).alpha(), 0);
}

void ChannelOptionsTest::channelOptionsOverlayRefresh()
{
    QVERIFY(window_->newDocument(QStringLiteral("OverlayRefresh"), 16, 16,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    addNamedAlpha(view_, QStringLiteral("Hair"), 2, 2, 8, 8);
    panel_->refresh();
    panel_->selectChannelForTest(4);

    panel_->toggleChannelForTest(4);
    pictura::ImageView* canvas = window_->imageView();
    QVERIFY(canvas);
    QImage overlay = canvas->maskOverlayForTest();
    QVERIFY(!overlay.isNull());
    QCOMPARE(overlay.pixelColor(0, 0).alpha(), 128);
    QCOMPARE(overlay.pixelColor(4, 4).alpha(), 0);

    QVERIFY(applyChannelOptions(*window_, true, true));
    QVERIFY(panel_->shownAlphas().contains(0));
    overlay = canvas->maskOverlayForTest();
    QVERIFY(!overlay.isNull());
    QCOMPARE(overlay.pixelColor(4, 4).alpha(), 128);
    QCOMPARE(overlay.pixelColor(0, 0).alpha(), 0);
}

void ChannelOptionsTest::duplicateSelectedChannelNormalizesToMasked()
{
    QVERIFY(window_->newDocument(QStringLiteral("SelectedSource"), 16, 16,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    view_ = window_->activeView();
    panel_ = window_->findChild<pictura::ChannelsPanel*>(QStringLiteral("channelsPanel"));
    QVERIFY(view_ && panel_);
    panel_->setView(view_);
    addNamedAlpha(view_, QStringLiteral("Hair"), 2, 2, 8, 8);
    QVERIFY(pictura::channels_set_sense(*view_, 0, true));
    QVERIFY(pictura::channels_selected_areas(*view_, 0));
    const QByteArray source =
        coverageBytes(pictura::channel_coverage(*view_, QStringLiteral("alpha:0")));
    const QByteArray sourcePlane = alphaPlaneBytes(view_, 0);
    const int sourceHistory = view_->history_count();

    QVERIFY(pictura::duplicate_channel_in_view(*view_, QStringLiteral("alpha:0"), 0,
                                               QStringLiteral("Copy"), false));
    QCOMPARE(view_->selection_channel_count(), 2);
    QVERIFY(!pictura::channels_selected_areas(*view_, 1));
    QCOMPARE(coverageBytes(pictura::channel_coverage(*view_, QStringLiteral("alpha:1"))),
             source);
    QCOMPARE(alphaPlaneBytes(view_, 0), sourcePlane);
    QCOMPARE(view_->history_count(), sourceHistory + 1);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Duplicate Channel"));

    QVERIFY(window_->newDocument(QStringLiteral("SelectedTarget"), 16, 16,
                                 QStringLiteral("rgb"), 8, QStringLiteral("white")));
    pictura::PictureView* target = window_->activeView();
    QVERIFY(target);
    const int targetHistory = target->history_count();
    QVERIFY(pictura::transfer_duplicate_channel(*target, *view_, QStringLiteral("alpha:0"), 0,
                                                QStringLiteral("Cross"), false));
    QCOMPARE(target->selection_channel_count(), 1);
    QVERIFY(!pictura::channels_selected_areas(*target, 0));
    QCOMPARE(coverageBytes(pictura::channel_coverage(*target, QStringLiteral("alpha:0"))),
             source);
    QCOMPARE(target->history_count(), targetHistory + 1);
    QCOMPARE(target->history_label(target->history_index()),
             QStringLiteral("Duplicate Channel"));
}

QTEST_MAIN(ChannelOptionsTest)
#include "tst_channel_options.moc"
