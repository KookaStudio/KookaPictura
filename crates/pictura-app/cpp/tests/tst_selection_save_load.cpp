#include <QtTest/QtTest>

#include "command_ids.h"
#include "commands.h"
#include "frame.h"
#include "image_view.h"
#include "save_load_selection_dialog.h"
#include "selection_drag.h"
#include "tools.h"
#include "api/document_api.h"
#include "api/document_bridge.h"
#include "api/layers_bridge.h"
#include "api/selection_bridge.h"

#include "qt_test_support.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QFile>
#include <QtCore/QMimeData>
#include <QtCore/QTemporaryDir>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtGui/QColor>
#include <QtGui/QDragEnterEvent>
#include <QtGui/QDropEvent>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>
#include <QtWidgets/QTabBar>

namespace {

::rust::Slice<const std::uint8_t> coverageSlice(const QByteArray& bytes)
{
    return ::rust::Slice<const std::uint8_t>(
        reinterpret_cast<const std::uint8_t*>(bytes.constData()), bytes.size());
}

bool dragSelectionOver(QWidget* target, const void* source, const QPoint& pos, bool drop)
{
    QMimeData mime;
    mime.setData(pictura::kSelectionSourceMime,
                 QByteArray::number(reinterpret_cast<quintptr>(source)));
    const auto actions = Qt::MoveAction | Qt::CopyAction;
    QDragEnterEvent enter(pos, actions, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(target, &enter);
    QDragMoveEvent move(pos, actions, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(target, &move);
    if (!drop) {
        return move.isAccepted();
    }
    QDropEvent release(QPointF(pos), actions, &mime, Qt::LeftButton, Qt::NoModifier);
    QCoreApplication::sendEvent(target, &release);
    return release.isAccepted();
}

// A 16 x 16 RGB document filled with one colour, opened from a temporary PNG so
// the colour-row derivation has a real picture to stream.
pictura::PictureView* openFlatImage(pictura::PicturaMainWindow& window, const QColor& colour)
{
    QTemporaryDir dir;
    QImage image(16, 16, QImage::Format_RGB32);
    image.fill(colour);
    const QString path = dir.filePath(QStringLiteral("flat.png"));
    if (!image.save(path) || !window.openDocumentAtPath(path)) {
        return nullptr;
    }
    return window.activeView();
}

} // namespace

class SelectionSaveLoadTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void saveDialogDefaults();
    void dialogDefaultsToActiveDocument();
    void saveDialogNewChannelName();
    void saveDialogExistingChannelOps();
    void saveDialogLayerMaskOps();
    void saveDialogExcludesColourRows();
    void documentListSameSize();
    void saveDialogNewImage();
    void loadDialogDefaults();
    void loadDialogDefaultsToExtraChannel();
    void loadDialogListsColourRows();
    void loadDialogWithNoSavedChannels();
    void loadDialogGrayscaleCompositeRow();
    void loadDialogModeRows();
    void multichannelLoadsStoredPlates();
    void multichannelSpotSelectionRefreshesAndUndoes();
    void loadDialogColourRowTarget();
    void loadColourRowCoverage();
    void loadColourRowCombines();
    void loadColourRowCrossDocument();
    void loadColourRowThroughFrame();
    void saveDialogThroughFrame();
    void saveDialogCrossDocument();
    void loadDialogThroughFrame();
    void selectLoadEnabledWithoutChannels();
    void loadDialogInvertAndOps();
    void loadDialogCrossDocument();
    void channelLimitRefusal();
    void wrongLengthRefused();
    void selectionDragSameSize();
    void selectionDragDifferentSizeFits();
    void selectionDragSourceUnchanged();
    void selectionDragOntoSourceIgnored();
    void selectionDragInvalidSourceIgnored();
    void selectionDragPreparationRestoresSource();

private:
    pictura::PictureView* newDoc(int width, int height);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

pictura::PictureView* SelectionSaveLoadTest::newDoc(int width, int height)
{
    if (!window_->newDocument(QStringLiteral("Selection"), width, height, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return nullptr;
    }
    return window_->activeView();
}

void SelectionSaveLoadTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void SelectionSaveLoadTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
}

void SelectionSaveLoadTest::saveDialogDefaults()
{
    pictura::PictureView* view = newDoc(16, 16);
    QVERIFY(view);
    view->select_all();
    const QList<pictura::SelectionDocument> documents {{view, QStringLiteral("Doc"), true}};
    pictura::SaveLoadSelectionDialog dialog(pictura::SaveLoadSelectionDialog::Mode::Save, documents,
                                            window_.get());
    QCOMPARE(dialog.documentCountForTest(), 2);
    QCOMPARE(dialog.channelCountForTest(), 1);
    QCOMPARE(dialog.channelTextForTest(0), QStringLiteral("New Channel"));
    QCOMPARE(dialog.operationTextForTest(0), QStringLiteral("Replace Channel"));
    QVERIFY(dialog.okEnabledForTest());
    const pictura::SelectionSaveRequest request = dialog.saveRequest();
    QCOMPARE(request.document, view);
    QCOMPARE(request.target, QStringLiteral("new"));
    QCOMPARE(request.mode, QStringLiteral("replace"));
    QVERIFY(!request.newDocument);
}

void SelectionSaveLoadTest::dialogDefaultsToActiveDocument()
{
    pictura::PictureView* first = newDoc(16, 16);
    QVERIFY(first);
    pictura::PictureView* active = newDoc(16, 16);
    QVERIFY(active);
    QVERIFY(first != active);
    const QList<pictura::SelectionDocument> documents {
        {first, QStringLiteral("First"), false},
        {active, QStringLiteral("Active"), true},
    };
    pictura::SaveLoadSelectionDialog save(pictura::SaveLoadSelectionDialog::Mode::Save, documents,
                                          window_.get());
    QCOMPARE(save.currentDocumentForTest(), 1);
    QCOMPARE(save.saveRequest().document, active);
    pictura::SaveLoadSelectionDialog load(pictura::SaveLoadSelectionDialog::Mode::Load, documents,
                                          window_.get());
    QCOMPARE(load.currentDocumentForTest(), 1);
    QCOMPARE(load.loadRequest().document, active);
}

void SelectionSaveLoadTest::saveDialogNewChannelName()
{
    pictura::PictureView* view = newDoc(16, 16);
    QVERIFY(view);
    view->select_all();
    const QList<pictura::SelectionDocument> documents {{view, QStringLiteral("Doc"), true}};
    pictura::SaveLoadSelectionDialog dialog(pictura::SaveLoadSelectionDialog::Mode::Save, documents,
                                            window_.get());
    dialog.setNameForTest(QStringLiteral("Hair"));
    const pictura::SelectionSaveRequest request = dialog.saveRequest();
    QCOMPARE(request.target, QStringLiteral("new"));
    QCOMPARE(request.name, QStringLiteral("Hair"));
    QVERIFY(pictura::save_selection_to(*view, request.target, request.name, request.mode));
    QVERIFY(channels_alpha_names(*view).contains(QStringLiteral("Hair")));
}

void SelectionSaveLoadTest::saveDialogExistingChannelOps()
{
    pictura::PictureView* view = newDoc(16, 16);
    QVERIFY(view);
    QVERIFY(channels_new_alpha(*view) >= 0);
    const QByteArray base(16 * 16, char(128));
    const QByteArray incoming(16 * 16, char(64));
    const QList<QPair<QString, int>> cases {
        {QStringLiteral("add"), 128},
        {QStringLiteral("subtract"), 64},
        {QStringLiteral("intersect"), 32},
    };
    for (const auto& [mode, expected] : cases) {
        QVERIFY(pictura::load_selection_coverage(*view, coverageSlice(base), 16, 16, false,
                                                QStringLiteral("new")));
        QVERIFY(pictura::save_selection_to(*view, QStringLiteral("alpha:0"), QString(),
                                           QStringLiteral("replace")));
        QVERIFY(pictura::load_selection_coverage(*view, coverageSlice(incoming), 16, 16, false,
                                                QStringLiteral("new")));
        const int before = view->history_index();
        QVERIFY(pictura::save_selection_to(*view, QStringLiteral("alpha:0"), QString(), mode));
        QCOMPARE(view->history_index(), before + 1);
        const auto got = pictura::channel_coverage(*view, QStringLiteral("alpha:0"));
        QCOMPARE(int(got.size()), 16 * 16);
        for (int i = 0; i < int(got.size()); ++i) {
            QCOMPARE(int(got[i]), expected);
        }
    }
    const QList<pictura::SelectionDocument> documents {{view, QStringLiteral("Doc"), true}};
    pictura::SaveLoadSelectionDialog dialog(pictura::SaveLoadSelectionDialog::Mode::Save, documents,
                                            window_.get());
    QCOMPARE(dialog.channelTextForTest(1), QStringLiteral("Alpha 1"));
    const QStringList modes {QStringLiteral("add"), QStringLiteral("subtract"),
                             QStringLiteral("intersect")};
    for (int i = 0; i < modes.size(); ++i) {
        dialog.setChannelIndexForTest(1);
        dialog.setOperationIndexForTest(i + 1);
        const pictura::SelectionSaveRequest request = dialog.saveRequest();
        QCOMPARE(request.target, QStringLiteral("alpha:0"));
        QCOMPARE(request.mode, modes.at(i));
    }
}

void SelectionSaveLoadTest::saveDialogLayerMaskOps()
{
    pictura::PictureView* view = newDoc(16, 16);
    QVERIFY(view);
    const QString layer = view->add_layer_in(QString());
    QVERIFY(!layer.isEmpty());
    QVERIFY(pictura::layer_mask_add_path(*view, layer, QStringLiteral("reveal-all")));
    QVERIFY(view->select_rect(4, 4, 8, 8, QStringLiteral("new"), 0.0));
    const QList<pictura::SelectionDocument> documents {{view, QStringLiteral("Doc"), true}};
    pictura::SaveLoadSelectionDialog dialog(pictura::SaveLoadSelectionDialog::Mode::Save, documents,
                                            window_.get());
    int maskIndex = -1;
    for (int i = 0; i < dialog.channelCountForTest(); ++i) {
        if (dialog.channelTextForTest(i).contains(QStringLiteral("Mask"))) {
            maskIndex = i;
            break;
        }
    }
    QVERIFY(maskIndex >= 0);
    dialog.setChannelIndexForTest(maskIndex);
    const pictura::SelectionSaveRequest request = dialog.saveRequest();
    QVERIFY(request.target.startsWith(QStringLiteral("mask:")));
    QVERIFY(pictura::save_selection_to(*view, request.target, QString(), request.mode));
    QVERIFY(view->mask_selection(layer, QStringLiteral("new")));
    QCOMPARE(view->selection_coverage(5, 5), 255);
    QCOMPARE(view->selection_coverage(1, 1), 0);
}

void SelectionSaveLoadTest::saveDialogExcludesColourRows()
{
    pictura::PictureView* view = newDoc(16, 16);
    QVERIFY(view);
    QVERIFY(channels_new_alpha(*view) >= 0);
    const QString layer = view->add_layer_in(QString());
    QVERIFY(!layer.isEmpty());
    QVERIFY(pictura::layer_mask_add_path(*view, layer, QStringLiteral("reveal-all")));
    const QList<pictura::SelectionDocument> documents {{view, QStringLiteral("Doc"), true}};
    pictura::SaveLoadSelectionDialog dialog(pictura::SaveLoadSelectionDialog::Mode::Save, documents,
                                            window_.get());
    QCOMPARE(dialog.channelCountForTest(), 3);
    QCOMPARE(dialog.channelTextForTest(0), QStringLiteral("New Channel"));
    for (int i = 0; i < dialog.channelCountForTest(); ++i) {
        const QString text = dialog.channelTextForTest(i);
        QVERIFY(text != QStringLiteral("RGB"));
        QVERIFY(text != QStringLiteral("Red"));
        QVERIFY(text != QStringLiteral("Green"));
        QVERIFY(text != QStringLiteral("Blue"));
    }
}

void SelectionSaveLoadTest::documentListSameSize()
{
    QVERIFY(newDoc(32, 16));
    QVERIFY(newDoc(16, 16));
    QVERIFY(newDoc(16, 16));
    const QStringList labels = window_->selectionDocumentLabelsForTest();
    QCOMPARE(labels.size(), 2);
}

void SelectionSaveLoadTest::saveDialogNewImage()
{
    pictura::PictureView* view = newDoc(16, 16);
    QVERIFY(view);
    view->select_all();
    const auto coverage = pictura::selection_coverage_bytes(*view);
    const int sourceHistory = view->history_count();
    const int sourceChannels = view->selection_channel_count();
    const int documents = window_->documentCount();
    window_->show();
    QTest::qWait(0);
    bool seen = false;
    QTimer driver;
    driver.setInterval(0);
    QObject::connect(&driver, &QTimer::timeout, [&] {
        auto* dialog = qobject_cast<pictura::SaveLoadSelectionDialog*>(
            QApplication::activeModalWidget());
        if (!dialog) {
            return;
        }
        driver.stop();
        seen = true;
        dialog->setDocumentIndexForTest(dialog->documentCountForTest() - 1);
        dialog->setNameForTest(QStringLiteral("Hair"));
        dialog->accept();
    });
    driver.start();
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(registry);
    registry->refresh();
    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::SelectSave)));
    driver.stop();
    QVERIFY(seen);
    QCOMPARE(window_->documentCount(), documents + 1);
    pictura::PictureView* created = window_->activeView();
    QVERIFY(created && created != view);
    QCOMPARE(created->document_width(), 16);
    QCOMPARE(created->document_height(), 16);
    QCOMPARE(pictura::channels_mode(*created), QStringLiteral("grayscale"));
    QCOMPARE(created->selection_channel_count(), 1);
    QCOMPARE(created->history_count(), 1);
    QCOMPARE(created->history_label(created->history_index()), QStringLiteral("New"));
    const auto channel = pictura::channel_coverage(*created, QStringLiteral("alpha:0"));
    QCOMPARE(int(channel.size()), int(coverage.size()));
    for (int i = 0; i < int(channel.size()); ++i) {
        QCOMPARE(int(channel[i]), int(coverage[i]));
    }
    QCOMPARE(pictura::channels_alpha_names(*created).size(), 1);
    QCOMPARE(pictura::channels_alpha_names(*created).at(0), QStringLiteral("Hair"));
    QCOMPARE(view->selection_channel_count(), sourceChannels);
    QCOMPARE(view->history_count(), sourceHistory);
    const auto sourceAfter = pictura::selection_coverage_bytes(*view);
    QCOMPARE(int(sourceAfter.size()), int(coverage.size()));
    for (int i = 0; i < int(sourceAfter.size()); ++i) {
        QCOMPARE(int(sourceAfter[i]), int(coverage[i]));
    }

    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("selection.psd"));
    QVERIFY(pictura::document_api::save(*created, path));

    QVERIFY(window_->openDocumentAtPath(path));
    pictura::PictureView* reopened = window_->activeView();
    QVERIFY(reopened);
    QCOMPARE(pictura::channels_mode(*reopened), QStringLiteral("grayscale"));
    QCOMPARE(reopened->selection_channel_count(), 1);
    QCOMPARE(pictura::channels_alpha_names(*reopened).size(), 1);
    QCOMPARE(pictura::channels_alpha_names(*reopened).at(0), QStringLiteral("Hair"));
    const auto roundTrip = pictura::channel_coverage(*reopened, QStringLiteral("alpha:0"));
    QCOMPARE(int(roundTrip.size()), int(coverage.size()));
    for (int i = 0; i < int(roundTrip.size()); ++i) {
        QCOMPARE(int(roundTrip[i]), int(coverage[i]));
    }
}

void SelectionSaveLoadTest::loadDialogDefaults()
{
    pictura::PictureView* view = newDoc(16, 16);
    QVERIFY(view);
    view->select_all();
    QVERIFY(pictura::save_selection_to(*view, QStringLiteral("new"), QStringLiteral("Hair"),
                                    QStringLiteral("replace")));
    const QList<pictura::SelectionDocument> documents {{view, QStringLiteral("Doc"), true}};
    pictura::SaveLoadSelectionDialog dialog(pictura::SaveLoadSelectionDialog::Mode::Load, documents,
                                            window_.get());
    QCOMPARE(dialog.documentCountForTest(), 1);
    QCOMPARE(dialog.channelCountForTest(), 5);
    QCOMPARE(dialog.channelTextForTest(0), QStringLiteral("RGB"));
    QCOMPARE(dialog.channelTextForTest(4), QStringLiteral("Hair"));
    QCOMPARE(dialog.operationTextForTest(0), QStringLiteral("New Selection"));
    QVERIFY(dialog.okEnabledForTest());
    const pictura::SelectionLoadRequest request = dialog.loadRequest();
    QCOMPARE(request.document, view);
    QCOMPARE(request.target, QStringLiteral("alpha:0"));
    QCOMPARE(request.mode, QStringLiteral("new"));
    QVERIFY(!request.invert);
}

void SelectionSaveLoadTest::loadDialogDefaultsToExtraChannel()
{
    pictura::PictureView* view = newDoc(16, 16);
    QVERIFY(view);
    QVERIFY(channels_new_alpha(*view) >= 0);
    const QString layer = view->add_layer_in(QString());
    QVERIFY(!layer.isEmpty());
    QVERIFY(pictura::layer_mask_add_path(*view, layer, QStringLiteral("reveal-all")));
    const QList<pictura::SelectionDocument> documents {{view, QStringLiteral("Doc"), true}};
    pictura::SaveLoadSelectionDialog dialog(pictura::SaveLoadSelectionDialog::Mode::Load, documents,
                                            window_.get());
    QCOMPARE(dialog.loadRequest().target, QStringLiteral("alpha:0"));
}

void SelectionSaveLoadTest::loadDialogListsColourRows()
{
    pictura::PictureView* view = newDoc(16, 16);
    QVERIFY(view);
    QVERIFY(channels_new_alpha(*view) >= 0);
    const QString layer = view->add_layer_in(QString());
    QVERIFY(!layer.isEmpty());
    QVERIFY(pictura::layer_mask_add_path(*view, layer, QStringLiteral("reveal-all")));
    const QList<pictura::SelectionDocument> documents {{view, QStringLiteral("Doc"), true}};
    pictura::SaveLoadSelectionDialog dialog(pictura::SaveLoadSelectionDialog::Mode::Load, documents,
                                            window_.get());
    QCOMPARE(dialog.channelCountForTest(), 6);
    QCOMPARE(dialog.channelTextForTest(0), QStringLiteral("RGB"));
    QCOMPARE(dialog.channelTextForTest(1), QStringLiteral("Red"));
    QCOMPARE(dialog.channelTextForTest(2), QStringLiteral("Green"));
    QCOMPARE(dialog.channelTextForTest(3), QStringLiteral("Blue"));
    QVERIFY(dialog.channelTextForTest(4).contains(QStringLiteral("Mask")));
    QCOMPARE(dialog.channelTextForTest(5), QStringLiteral("Alpha 1"));
}

void SelectionSaveLoadTest::loadDialogWithNoSavedChannels()
{
    pictura::PictureView* view = newDoc(16, 16);
    QVERIFY(view);
    const QList<pictura::SelectionDocument> documents {{view, QStringLiteral("Doc"), true}};
    pictura::SaveLoadSelectionDialog dialog(pictura::SaveLoadSelectionDialog::Mode::Load, documents,
                                            window_.get());
    QCOMPARE(dialog.channelCountForTest(), 4);
    QVERIFY(dialog.okEnabledForTest());
    QCOMPARE(dialog.loadRequest().target, QStringLiteral("color:0"));
}

void SelectionSaveLoadTest::loadDialogGrayscaleCompositeRow()
{
    QVERIFY(window_->newDocument(QStringLiteral("Gray"), 16, 16, QStringLiteral("grayscale"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    QVERIFY(view);
    const QList<pictura::SelectionDocument> documents {{view, QStringLiteral("Doc"), true}};
    pictura::SaveLoadSelectionDialog dialog(pictura::SaveLoadSelectionDialog::Mode::Load, documents,
                                            window_.get());
    QCOMPARE(dialog.channelCountForTest(), 1);
    QCOMPARE(dialog.channelTextForTest(0), QStringLiteral("Gray"));
    QCOMPARE(dialog.loadRequest().target, QStringLiteral("color:0"));

    const auto coverage = pictura::color_row_coverage(*view, 0, 0x7);
    QCOMPARE(int(coverage.size()), 16 * 16);
    QVERIFY(pictura::load_selection_coverage(*view,
        ::rust::Slice<const std::uint8_t>(coverage.data(), coverage.size()), 16, 16, false,
        QStringLiteral("new")));
    QCOMPARE(view->selection_coverage(8, 8), 255);
}

void SelectionSaveLoadTest::loadDialogModeRows()
{
    const auto checkMode = [this](pictura::PictureView* view, const QStringList& names) {
        QVERIFY(view);
        const QList<pictura::SelectionDocument> documents {{view, QStringLiteral("Doc"), true}};
        pictura::SaveLoadSelectionDialog dialog(pictura::SaveLoadSelectionDialog::Mode::Load,
                                                documents, window_.get());
        QCOMPARE(dialog.channelCountForTest(), int(names.size()));
        for (int i = 0; i < int(names.size()); ++i) {
            QCOMPARE(dialog.channelTextForTest(i), names.at(i));
            const auto coverage = pictura::color_row_coverage(*view, i, 0x7);
            QCOMPARE(int(coverage.size()), 16 * 16);
        }
        QCOMPARE(dialog.loadRequest().target, QStringLiteral("color:0"));
    };

    pictura::PictureView* rgb = newDoc(16, 16);
    QVERIFY(rgb);
    checkMode(rgb, {QStringLiteral("RGB"), QStringLiteral("Red"), QStringLiteral("Green"),
                    QStringLiteral("Blue")});

    QVERIFY(window_->newDocument(QStringLiteral("Gray"), 16, 16, QStringLiteral("grayscale"), 8,
                                 QStringLiteral("white")));
    checkMode(window_->activeView(), {QStringLiteral("Gray")});

    pictura::PictureView* cmyk = newDoc(16, 16);
    QVERIFY(cmyk && pictura::convert_image_mode(*cmyk, QStringLiteral("cmyk")));
    checkMode(cmyk, {QStringLiteral("CMYK"), QStringLiteral("Cyan"), QStringLiteral("Magenta"),
                     QStringLiteral("Yellow"), QStringLiteral("Black")});

    pictura::PictureView* lab = newDoc(16, 16);
    QVERIFY(lab && pictura::convert_image_mode(*lab, QStringLiteral("lab")));
    checkMode(lab, {QStringLiteral("Lab"), QStringLiteral("Lightness"), QStringLiteral("a"),
                    QStringLiteral("b")});

    pictura::PictureView* indexed = newDoc(16, 16);
    QVERIFY(indexed && pictura::convert_to_indexed(*indexed, 1, 256, 0, 0));
    checkMode(indexed, {QStringLiteral("Index")});

    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    QFile legacy(dir.filePath(QStringLiteral("legacy.psd")));
    QVERIFY(legacy.open(QIODevice::WriteOnly));
    QByteArray psd = QByteArray::fromHex(
        "3842505300010000000000000003000000100000001000080007"
        "0000000000000000000000000000");
    psd.append(QByteArray(3 * 16 * 16, char(100)));
    QCOMPARE(legacy.write(psd), qint64(psd.size()));
    legacy.close();
    QVERIFY(window_->openDocumentAtPath(legacy.fileName()));
    checkMode(window_->activeView(), {QStringLiteral("Cyan"), QStringLiteral("Magenta"),
                                     QStringLiteral("Yellow")});
}

void SelectionSaveLoadTest::multichannelLoadsStoredPlates()
{
    const QStringList grayNames{QStringLiteral("Black")};
    const QStringList cmykNames{QStringLiteral("Cyan"), QStringLiteral("Magenta"),
                                QStringLiteral("Yellow"), QStringLiteral("Black")};
    for (const QString& sourceMode : {QStringLiteral("grayscale"), QStringLiteral("cmyk")}) {
        auto* view = openFlatImage(*window_, QColor(100, 150, 200));
        QVERIFY(view && pictura::convert_image_mode(*view, sourceMode));
        const QByteArray saved(16 * 16, char(87));
        QVERIFY(pictura::load_selection_coverage(*view, coverageSlice(saved), 16, 16, false,
                                                 QStringLiteral("new")));
        QVERIFY(pictura::save_selection_to(*view, QStringLiteral("new"), QStringLiteral("Saved"),
                                           QStringLiteral("new")));
        QVERIFY(pictura::convert_image_mode(*view, QStringLiteral("multichannel")));

        const QList<pictura::SelectionDocument> documents{{view, QStringLiteral("Doc"), true}};
        pictura::SaveLoadSelectionDialog dialog(pictura::SaveLoadSelectionDialog::Mode::Load,
                                                documents, window_.get());
        const auto plates = sourceMode == QLatin1String("cmyk") ? cmykNames : grayNames;
        QCOMPARE(dialog.channelCountForTest(), int(plates.size() + 1));
        for (int i = 0; i < dialog.channelCountForTest(); ++i) {
            const bool isPlate = i < plates.size();
            QCOMPARE(dialog.channelTextForTest(i), isPlate ? plates.at(i) : QStringLiteral("Saved"));
            QCOMPARE(pictura::channels_alpha_is_spot(*view, i), isPlate);
            dialog.setChannelIndexForTest(i);
            const auto request = dialog.loadRequest();
            QCOMPARE(request.target, QStringLiteral("alpha:%1").arg(i));

            int width = 0;
            int height = 0;
            const auto samples = pictura::channels_alpha_thumbnail(*view, i, 16, width, height);
            QCOMPARE(width, 16);
            QCOMPARE(height, 16);
            QCOMPARE(samples.size(), std::size_t(16 * 16));
            QVERIFY(pictura::load_selection_target(*view, request.target, request.invert, request.mode));
            QCOMPARE(view->selection_coverage(8, 8), isPlate ? int(samples[8 * 16 + 8]) : 87);
        }
    }
}

void SelectionSaveLoadTest::multichannelSpotSelectionRefreshesAndUndoes()
{
    auto* view = openFlatImage(*window_, QColor(0, 0, 0));
    QVERIFY(view);
    QVERIFY(pictura::convert_image_mode(*view, QStringLiteral("multichannel")));
    QCOMPARE(view->composite_argb(0, 0), QColor(0, 0, 0).rgba());
    view->select_all();
    QVERIFY(pictura::save_selection_to(*view, QStringLiteral("alpha:0"), QString(),
                                       QStringLiteral("replace")));
    QCOMPARE(view->composite_argb(0, 0), QColor(255, 0, 0).rgba());
    QVERIFY(view->undo());
    QCOMPARE(view->composite_argb(0, 0), QColor(0, 0, 0).rgba());
    QVERIFY(view->redo());
    QCOMPARE(view->composite_argb(0, 0), QColor(255, 0, 0).rgba());
}

void SelectionSaveLoadTest::loadDialogColourRowTarget()
{
    pictura::PictureView* view = openFlatImage(*window_, QColor(100, 150, 200));
    QVERIFY(view);
    const QList<pictura::SelectionDocument> documents {{view, QStringLiteral("Doc"), true}};
    pictura::SaveLoadSelectionDialog dialog(pictura::SaveLoadSelectionDialog::Mode::Load, documents,
                                            window_.get());
    const QStringList expected {QStringLiteral("color:0"), QStringLiteral("color:1"),
                                QStringLiteral("color:2"), QStringLiteral("color:3")};
    for (int i = 0; i < int(expected.size()); ++i) {
        dialog.setChannelIndexForTest(i);
        QCOMPARE(dialog.loadRequest().target, expected.at(i));
    }
}

void SelectionSaveLoadTest::loadColourRowCoverage()
{
    pictura::PictureView* view = openFlatImage(*window_, QColor(100, 150, 200));
    QVERIFY(view);
    const int expected[4] = {140, 100, 150, 200};
    for (int row = 0; row < 4; ++row) {
        const auto coverage = pictura::color_row_coverage(*view, row, 0x7);
        QCOMPARE(int(coverage.size()), 16 * 16);
        QVERIFY(pictura::load_selection_coverage(*view,
            ::rust::Slice<const std::uint8_t>(coverage.data(), coverage.size()), 16, 16, false,
            QStringLiteral("new")));
        QCOMPARE(view->selection_coverage(8, 8), expected[row]);
    }
}

void SelectionSaveLoadTest::loadColourRowCombines()
{
    pictura::PictureView* view = openFlatImage(*window_, QColor(100, 150, 200));
    QVERIFY(view);
    const QByteArray base(16 * 16, char(200));
    QVERIFY(pictura::load_selection_coverage(*view, coverageSlice(base), 16, 16, false,
                                             QStringLiteral("new")));
    const int before = view->history_count();
    const auto coverage = pictura::color_row_coverage(*view, 1, 0x7);
    QVERIFY(pictura::load_selection_coverage(*view,
        ::rust::Slice<const std::uint8_t>(coverage.data(), coverage.size()), 16, 16, false,
        QStringLiteral("subtract")));
    QCOMPARE(view->selection_coverage(8, 8), 100);
    QCOMPARE(view->history_count(), before + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Load Selection"));
}

void SelectionSaveLoadTest::loadColourRowCrossDocument()
{
    pictura::PictureView* source = openFlatImage(*window_, QColor(100, 150, 200));
    QVERIFY(source);
    pictura::PictureView* target = newDoc(16, 16);
    QVERIFY(target);
    const int before = target->history_count();
    const auto coverage = pictura::color_row_coverage(*source, 1, 0x7);
    QCOMPARE(int(coverage.size()), 16 * 16);
    QVERIFY(pictura::load_selection_coverage(*target,
        ::rust::Slice<const std::uint8_t>(coverage.data(), coverage.size()), 16, 16, false,
        QStringLiteral("new")));
    QCOMPARE(target->selection_coverage(8, 8), 100);
    QCOMPARE(target->history_count(), before + 1);
    QCOMPARE(target->history_label(target->history_index()), QStringLiteral("Load Selection"));
}

void SelectionSaveLoadTest::loadColourRowThroughFrame()
{
    pictura::PictureView* source = openFlatImage(*window_, QColor(100, 150, 200));
    QVERIFY(source);
    pictura::PictureView* target = newDoc(16, 16);
    QVERIFY(target && target != source);

    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(registry);

    struct Picked {
        bool found = false;
        QString target;
    };
    const auto runLoadDialog = [this, registry](int documentIndex, Picked& picked) {
        // A local timer drives the modal dialog and is stopped before this call
        // returns, so no callback outlives the stack frame it captures.
        QTimer driver;
        driver.setInterval(0);
        QObject::connect(&driver, &QTimer::timeout,
                         [this, documentIndex, &picked, &driver]() {
            auto* dialog = window_->findChild<pictura::SaveLoadSelectionDialog*>();
            if (!dialog) {
                return;
            }
            driver.stop();
            if (dialog->channelCountForTest() < 2) {
                dialog->reject();
                return;
            }
            dialog->setDocumentIndexForTest(documentIndex);
            dialog->setChannelIndexForTest(1);
            picked.target = dialog->loadRequest().target;
            picked.found = true;
            dialog->accept();
        });
        driver.start();
        registry->refresh();
        const bool ran =
            registry->dispatch(QString::fromLatin1(pictura::command_ids::SelectLoad));
        driver.stop();
        return ran && picked.found;
    };

    // Same document: the active source derives its own Red row.
    window_->setActiveDocumentIndex(0);
    const int sourceBefore = source->history_count();
    Picked same;
    QVERIFY(runLoadDialog(0, same));
    QVERIFY(same.found);
    QCOMPARE(same.target, QStringLiteral("color:1"));
    QCOMPARE(source->selection_coverage(8, 8), 100);
    QCOMPARE(source->history_count(), sourceBefore + 1);
    QCOMPARE(source->history_label(source->history_index()), QStringLiteral("Load Selection"));

    // Cross document: the active target loads the source's Red row.
    window_->setActiveDocumentIndex(1);
    const int targetBefore = target->history_count();
    Picked cross;
    QVERIFY(runLoadDialog(0, cross));
    QVERIFY(cross.found);
    QCOMPARE(cross.target, QStringLiteral("color:1"));
    QCOMPARE(target->selection_coverage(8, 8), 100);
    QCOMPARE(target->history_count(), targetBefore + 1);
    QCOMPARE(target->history_label(target->history_index()), QStringLiteral("Load Selection"));
}

void SelectionSaveLoadTest::saveDialogThroughFrame()
{
    pictura::PictureView* view = newDoc(16, 16);
    QVERIFY(view);
    view->select_all();
    const int before = view->history_count();
    window_->show();
    QTest::qWait(0);
    bool seen = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = qobject_cast<pictura::SaveLoadSelectionDialog*>(
            QApplication::activeModalWidget());
        if (!dialog) {
            return;
        }
        seen = true;
        dialog->setNameForTest(QStringLiteral("Frame"));
        dialog->accept();
    });
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(registry);
    registry->refresh();
    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::SelectSave)));
    QVERIFY(seen);
    QCOMPARE(view->history_count(), before + 1);
    QCOMPARE(view->history_label(view->history_index()), QStringLiteral("Save Selection"));
    QVERIFY(pictura::channels_alpha_names(*view).contains(QStringLiteral("Frame")));
}

void SelectionSaveLoadTest::saveDialogCrossDocument()
{
    pictura::PictureView* source = newDoc(16, 16);
    QVERIFY(source);
    source->select_all();
    pictura::PictureView* target = newDoc(16, 16);
    QVERIFY(target && target != source);
    window_->setActiveDocumentIndex(0);
    const int sourceHistory = source->history_count();
    const int targetHistory = target->history_count();
    window_->show();
    QTest::qWait(0);
    bool seen = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = qobject_cast<pictura::SaveLoadSelectionDialog*>(
            QApplication::activeModalWidget());
        if (!dialog) {
            return;
        }
        seen = true;
        dialog->setDocumentIndexForTest(1);
        dialog->setNameForTest(QStringLiteral("Cross"));
        dialog->accept();
    });
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(registry);
    registry->refresh();
    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::SelectSave)));
    QVERIFY(seen);
    QCOMPARE(target->selection_channel_count(), 1);
    const QStringList names = pictura::channels_alpha_names(*target);
    QCOMPARE(names.size(), 1);
    QCOMPARE(names.at(0), QStringLiteral("Cross"));
    QCOMPARE(target->history_count(), targetHistory + 1);
    QCOMPARE(target->history_label(target->history_index()), QStringLiteral("Save Selection"));
    QCOMPARE(source->selection_channel_count(), 0);
    QCOMPARE(source->history_count(), sourceHistory);
}

void SelectionSaveLoadTest::loadDialogThroughFrame()
{
    pictura::PictureView* source = newDoc(16, 16);
    QVERIFY(source);
    source->select_all();
    QVERIFY(pictura::save_selection_to(*source, QStringLiteral("new"), QStringLiteral("Hair"),
                                      QStringLiteral("replace")));
    pictura::PictureView* target = newDoc(16, 16);
    QVERIFY(target && target != source);
    window_->setActiveDocumentIndex(1);
    const int before = target->history_count();
    window_->show();
    QTest::qWait(0);
    bool seen = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = qobject_cast<pictura::SaveLoadSelectionDialog*>(
            QApplication::activeModalWidget());
        if (!dialog) {
            return;
        }
        seen = true;
        dialog->setDocumentIndexForTest(0);
        dialog->setChannelIndexForTest(4);
        dialog->accept();
    });
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(registry);
    registry->refresh();
    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::SelectLoad)));
    QVERIFY(seen);
    QCOMPARE(target->selection_coverage(8, 8), 255);
    QCOMPARE(target->history_count(), before + 1);
    QCOMPARE(target->history_label(target->history_index()), QStringLiteral("Load Selection"));
}

void SelectionSaveLoadTest::selectLoadEnabledWithoutChannels()
{
    pictura::PictureView* view = newDoc(16, 16);
    QVERIFY(view);
    QCOMPARE(view->selection_channel_count(), 0);
    QVERIFY(pictura::selection_mask_targets(*view).isEmpty());
    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(registry);
    registry->refresh();
    QAction* load = registry->action(QString::fromLatin1(pictura::command_ids::SelectLoad));
    QVERIFY(load);
    QVERIFY(load->isEnabled());
}

void SelectionSaveLoadTest::loadDialogInvertAndOps()
{
    pictura::PictureView* view = newDoc(2, 2);
    QVERIFY(view);
    const QByteArray base(4, 128);
    const QByteArray source(4, 64);
    QVERIFY(pictura::load_selection_coverage(*view, 
        ::rust::Slice<const std::uint8_t>(reinterpret_cast<const std::uint8_t*>(base.constData()),
                                          base.size()),
        2, 2, false, QStringLiteral("new")));
    QVERIFY(pictura::load_selection_coverage(*view, 
        ::rust::Slice<const std::uint8_t>(reinterpret_cast<const std::uint8_t*>(source.constData()),
                                          source.size()),
        2, 2, false, QStringLiteral("new")));
    QVERIFY(pictura::save_selection_to(*view, QStringLiteral("new"), QStringLiteral("Soft"),
                                    QStringLiteral("replace")));
    QVERIFY(pictura::load_selection_coverage(*view, 
        ::rust::Slice<const std::uint8_t>(reinterpret_cast<const std::uint8_t*>(base.constData()),
                                          base.size()),
        2, 2, false, QStringLiteral("new")));
    QVERIFY(pictura::load_selection_target(*view, QStringLiteral("alpha:0"), true, QStringLiteral("add")));
    QCOMPARE(view->selection_coverage(0, 0), 191);

    const QList<pictura::SelectionDocument> documents {{view, QStringLiteral("Doc"), true}};
    pictura::SaveLoadSelectionDialog dialog(pictura::SaveLoadSelectionDialog::Mode::Load, documents,
                                            window_.get());
    dialog.setInvertForTest(true);
    dialog.setOperationIndexForTest(1);
    const pictura::SelectionLoadRequest request = dialog.loadRequest();
    QVERIFY(request.invert);
    QCOMPARE(request.mode, QStringLiteral("add"));
}

void SelectionSaveLoadTest::loadDialogCrossDocument()
{
    pictura::PictureView* source = newDoc(16, 16);
    QVERIFY(source);
    source->select_all();
    QVERIFY(pictura::save_selection_to(*source, QStringLiteral("new"), QStringLiteral("Hair"),
                                      QStringLiteral("replace")));
    pictura::PictureView* target = newDoc(16, 16);
    QVERIFY(target);
    const auto coverage = pictura::channel_coverage(*source, QStringLiteral("alpha:0"));
    QVERIFY(pictura::load_selection_coverage(*target, 
        ::rust::Slice<const std::uint8_t>(coverage.data(), coverage.size()), 16, 16, false,
        QStringLiteral("new")));
    QCOMPARE(target->selection_coverage(8, 8), 255);

    const QList<pictura::SelectionDocument> documents {
        {source, QStringLiteral("Source"), false},
        {target, QStringLiteral("Target"), true},
    };
    pictura::SaveLoadSelectionDialog dialog(pictura::SaveLoadSelectionDialog::Mode::Load, documents,
                                            window_.get());
    QCOMPARE(dialog.documentCountForTest(), 2);
    dialog.setDocumentIndexForTest(0);
    QCOMPARE(dialog.channelCountForTest(), 5);
    QCOMPARE(dialog.channelTextForTest(0), QStringLiteral("RGB"));
    QCOMPARE(dialog.channelTextForTest(4), QStringLiteral("Hair"));
}

void SelectionSaveLoadTest::channelLimitRefusal()
{
    pictura::PictureView* view = newDoc(16, 16);
    QVERIFY(view);
    view->select_all();
    for (int i = 0; i < 53; ++i) {
        QVERIFY(channels_new_alpha(*view) >= 0);
    }
    QCOMPARE(view->selection_channel_count(), 53);
    const int before = view->history_index();
    QVERIFY(!pictura::save_selection_to(*view, QStringLiteral("new"), QStringLiteral("Overflow"),
                                     QStringLiteral("replace")));
    QCOMPARE(view->selection_channel_count(), 53);
    QCOMPARE(view->history_index(), before);
}

void SelectionSaveLoadTest::wrongLengthRefused()
{
    pictura::PictureView* view = newDoc(16, 16);
    QVERIFY(view);
    const QByteArray shortCoverage(4, 0);
    const int before = view->history_index();
    QVERIFY(!pictura::load_selection_coverage(*view, 
        ::rust::Slice<const std::uint8_t>(
            reinterpret_cast<const std::uint8_t*>(shortCoverage.constData()),
            shortCoverage.size()),
        16, 16, false, QStringLiteral("new")));
    QCOMPARE(view->history_index(), before);
}


void SelectionSaveLoadTest::selectionDragSameSize()
{
    pictura::PictureView* source = newDoc(16, 16);
    QVERIFY(source);
    QVERIFY(source->select_rect(4, 4, 8, 8, QStringLiteral("new"), 0.0));
    pictura::PictureView* target = newDoc(16, 16);
    QVERIFY(target);
    window_->setActiveDocumentIndex(0);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    auto* bar = window_->findChild<QTabBar*>(QStringLiteral("documentTabBar"));
    QVERIFY(bar && bar->count() == 2);
    const int sourceHistory = source->history_count();
    const int targetHistory = target->history_count();
    QVERIFY2(dragSelectionOver(bar, source, bar->tabRect(1).center(), true), "tab drop");
    QCOMPARE(target->selection_coverage(8, 8), 255);
    QCOMPARE(target->selection_coverage(0, 0), 0);
    QCOMPARE(target->history_count(), targetHistory + 1);
    QCOMPARE(target->history_label(target->history_index()), QStringLiteral("Load Selection"));
    QCOMPARE(source->history_count(), sourceHistory);
    QCOMPARE(source->selection_coverage(8, 8), 255);
}

void SelectionSaveLoadTest::selectionDragDifferentSizeFits()
{
    pictura::PictureView* source = newDoc(16, 16);
    QVERIFY(source);
    QVERIFY(source->select_rect(4, 4, 8, 8, QStringLiteral("new"), 0.0));
    pictura::PictureView* target = newDoc(32, 32);
    QVERIFY(target);
    window_->setActiveDocumentIndex(0);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    auto* bar = window_->findChild<QTabBar*>(QStringLiteral("documentTabBar"));
    QVERIFY(bar && bar->count() == 2);
    const int targetHistory = target->history_count();
    QVERIFY2(dragSelectionOver(bar, source, bar->tabRect(1).center(), true), "tab drop");
    QCOMPARE(target->selection_coverage(16, 16), 255);
    QCOMPARE(target->selection_coverage(0, 0), 0);
    QCOMPARE(target->selection_coverage(31, 31), 0);
    QCOMPARE(target->history_count(), targetHistory + 1);
    QCOMPARE(target->history_label(target->history_index()), QStringLiteral("Load Selection"));
}

void SelectionSaveLoadTest::selectionDragSourceUnchanged()
{
    pictura::PictureView* source = newDoc(16, 16);
    QVERIFY(source);
    QVERIFY(source->select_rect(2, 2, 6, 6, QStringLiteral("new"), 0.0));
    pictura::PictureView* target = newDoc(16, 16);
    QVERIFY(target);
    window_->setActiveDocumentIndex(0);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    auto* bar = window_->findChild<QTabBar*>(QStringLiteral("documentTabBar"));
    QVERIFY(bar && bar->count() == 2);
    const int sourceHistory = source->history_count();
    QVERIFY2(dragSelectionOver(bar, source, bar->tabRect(1).center(), true), "tab drop");
    QCOMPARE(source->selection_coverage(4, 4), 255);
    QCOMPARE(source->selection_coverage(0, 0), 0);
    QCOMPARE(source->history_count(), sourceHistory);
}

void SelectionSaveLoadTest::selectionDragOntoSourceIgnored()
{
    pictura::PictureView* source = newDoc(16, 16);
    QVERIFY(source);
    QVERIFY(source->select_rect(4, 4, 8, 8, QStringLiteral("new"), 0.0));
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    auto* bar = window_->findChild<QTabBar*>(QStringLiteral("documentTabBar"));
    QVERIFY(bar && bar->count() == 1);
    const int history = source->history_count();
    QVERIFY2(!dragSelectionOver(bar, source, bar->tabRect(0).center(), true), "own tab drop");
    QCOMPARE(source->selection_coverage(8, 8), 255);
    QCOMPARE(source->history_count(), history);
}

void SelectionSaveLoadTest::selectionDragInvalidSourceIgnored()
{
    pictura::PictureView* source = newDoc(16, 16);
    QVERIFY(source);
    QVERIFY(source->select_rect(4, 4, 8, 8, QStringLiteral("new"), 0.0));
    pictura::PictureView* target = newDoc(16, 16);
    QVERIFY(target);
    window_->setActiveDocumentIndex(0);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    auto* bar = window_->findChild<QTabBar*>(QStringLiteral("documentTabBar"));
    QVERIFY(bar && bar->count() == 2);
    const int targetHistory = target->history_count();
    QVERIFY2(!dragSelectionOver(bar, reinterpret_cast<const void*>(quintptr(1)),
                                bar->tabRect(1).center(), true),
             "an unknown source is ignored");
    QCOMPARE(window_->activeView(), source);
    QCOMPARE(target->selection_coverage(8, 8), 0);
    QCOMPARE(target->history_count(), targetHistory);
}

void SelectionSaveLoadTest::selectionDragPreparationRestoresSource()
{
    pictura::PictureView* source = newDoc(16, 16);
    QVERIFY(source);
    QVERIFY(source->select_rect(4, 4, 8, 8, QStringLiteral("new"), 0.0));
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    auto* tools = window_->findChild<pictura::ToolController*>();
    QVERIFY(tools);
    tools->setActiveTool(pictura::ToolId::Marquee);
    pictura::ImageView* canvas = window_->imageView();
    QVERIFY(canvas);
    const int history = source->history_count();
    QTest::mousePress(canvas, Qt::LeftButton, Qt::NoModifier,
                      canvas->imageToWidget(QPointF(8, 8)).toPoint());
    QVERIFY(tools->prepareSelectionDocumentDragForTest());
    QCOMPARE(source->selection_coverage(8, 8), 255);
    QCOMPARE(source->selection_coverage(0, 0), 0);
    QCOMPARE(source->history_count(), history);
}

PICTURA_QT_TEST_MAIN(SelectionSaveLoadTest)
#include "tst_selection_save_load.moc"
