#include <QtTest/QtTest>

#include "frame.h"
#include "image_view.h"
#include "save_load_selection_dialog.h"
#include "selection_drag.h"
#include "tools.h"
#include "api/document_api.h"
#include "api/layers_bridge.h"
#include "api/selection_bridge.h"

#include "qt_test_support.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QMimeData>
#include <QtCore/QTemporaryDir>
#include <QtGui/QDragEnterEvent>
#include <QtGui/QDropEvent>
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
    void documentListSameSize();
    void saveDialogNewImage();
    void loadDialogDefaults();
    void loadDialogInvertAndOps();
    void loadDialogCrossDocument();
    void loadDialogDisabledWithoutSources();
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
    const QList<pictura::SelectionDocument> documents {{view, QStringLiteral("Doc"), true}};
    pictura::SaveLoadSelectionDialog dialog(pictura::SaveLoadSelectionDialog::Mode::Save, documents,
                                            window_.get());
    dialog.setDocumentIndexForTest(dialog.documentCountForTest() - 1);
    dialog.setNameForTest(QStringLiteral("Hair"));
    const pictura::SelectionSaveRequest request = dialog.saveRequest();
    QVERIFY(request.newDocument);
    QCOMPARE(request.target, QStringLiteral("new"));
    QCOMPARE(request.name, QStringLiteral("Hair"));

    const auto coverage = pictura::selection_coverage_bytes(*view);
    auto* created = new pictura::PictureView(window_.get());
    QVERIFY(pictura::new_selection_document(*created, coverageSlice(
        QByteArray(reinterpret_cast<const char*>(coverage.data()), int(coverage.size()))),
        16, 16, request.name, false));
    QCOMPARE(created->document_width(), 16);
    QCOMPARE(created->document_height(), 16);
    QCOMPARE(pictura::channels_mode(*created), QStringLiteral("grayscale"));
    QCOMPARE(created->selection_channel_count(), 1);
    const auto channel = pictura::channel_coverage(*created, QStringLiteral("alpha:0"));
    QCOMPARE(int(channel.size()), int(coverage.size()));
    for (int i = 0; i < int(channel.size()); ++i) {
        QCOMPARE(int(channel[i]), int(coverage[i]));
    }
    QCOMPARE(pictura::channels_alpha_names(*created).size(), 1);
    QCOMPARE(pictura::channels_alpha_names(*created).at(0), QStringLiteral("Hair"));

    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString path = dir.filePath(QStringLiteral("selection.psd"));
    QVERIFY(pictura::document_api::save(*created, path));
    delete created;

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
    QCOMPARE(dialog.channelCountForTest(), 1);
    QCOMPARE(dialog.channelTextForTest(0), QStringLiteral("Hair"));
    QCOMPARE(dialog.operationTextForTest(0), QStringLiteral("New Selection"));
    QVERIFY(dialog.okEnabledForTest());
    const pictura::SelectionLoadRequest request = dialog.loadRequest();
    QCOMPARE(request.document, view);
    QCOMPARE(request.target, QStringLiteral("alpha:0"));
    QCOMPARE(request.mode, QStringLiteral("new"));
    QVERIFY(!request.invert);
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
    QCOMPARE(dialog.channelCountForTest(), 1);
    QCOMPARE(dialog.channelTextForTest(0), QStringLiteral("Hair"));
}

void SelectionSaveLoadTest::loadDialogDisabledWithoutSources()
{
    pictura::PictureView* view = newDoc(16, 16);
    QVERIFY(view);
    const QList<pictura::SelectionDocument> documents {{view, QStringLiteral("Doc"), true}};
    pictura::SaveLoadSelectionDialog dialog(pictura::SaveLoadSelectionDialog::Mode::Load, documents,
                                            window_.get());
    QCOMPARE(dialog.channelCountForTest(), 0);
    QVERIFY(!dialog.okEnabledForTest());
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

QTEST_MAIN(SelectionSaveLoadTest)
#include "tst_selection_save_load.moc"
