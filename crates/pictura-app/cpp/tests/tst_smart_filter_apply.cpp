#include <QtTest/QtTest>

#include <QtCore/QCoreApplication>
#include <QtCore/QJsonArray>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtWidgets/QApplication>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QTreeView>

#include <functional>

#include "commands.h"
#include "filter_commands.h"
#include "filter_preview_dialog.h"
#include "frame.h"
#include "panels/layers_panel.h"
#include "session.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/filter_tools.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layers_smart_filters.cxxqt.h"

#include "qt_test_support.h"

// Filters on a Smart Object become Smart Filters (#984): the Filter menu, the
// Filter Gallery, and the dialog preview append to the object's chain instead
// of filtering its proxy.
class SmartFilterApplyTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void filterMenuAppendsSmartFilter();
    void selectionBecomesFilterMask();
    void filterGalleryAddsOneEntry();
    void excludedFiltersDisabledOnSmartObject();
    void previewCancelRestoresSmartObject();
    void undoRemovesSmartFilter();
    void adjustmentsDisabledOnSmartObject();
    void editSmartFilterEnabledForGenericEntry();
    void filterRowsShowNewestFirst();
    void doubleClickReopensTheFilterDialog();
    void dragReordersSmartFilters();

private:
    // A new document whose active layer is an embedded Smart Object; returns
    // its layer path.
    QString smartObject(const QString& name);
    bool applyGaussianBlur(double radius);
    bool applyEmboss();
    pictura::LayersPanel* panelOn(pictura::PictureView* view, const QString& path);
    QAction* action(const QStringList& path);
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void SmartFilterApplyTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->show();
    QCoreApplication::processEvents();
}

void SmartFilterApplyTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
}

QString SmartFilterApplyTest::smartObject(const QString& name)
{
    if (!window_->newDocument(name, 16, 16, QStringLiteral("rgb"), 8, QStringLiteral("white"))) {
        return {};
    }
    pictura::PictureView* view = window_->activeView();
    const QString path = view->add_solid_fill(0xff808080u);
    if (!view->rasterize_fill_content(path) || !view->convert_to_smart_object(path)) {
        return {};
    }
    view->set_active_layer(path);
    return path;
}

bool SmartFilterApplyTest::applyGaussianBlur(double radius)
{
    pictura::CommandParams params;
    params.values.insert(QStringLiteral("kind"), QStringLiteral("gaussian-blur"));
    params.values.insert(QStringLiteral("params"), QJsonArray{radius});
    return window_->registry()->executeRecorded(
        pictura::commandIdForPath(
            {QStringLiteral("Filter"), QStringLiteral("Blur"), QStringLiteral("Gaussian Blur")}),
        params);
}

bool SmartFilterApplyTest::applyEmboss()
{
    pictura::CommandParams params;
    params.values.insert(QStringLiteral("kind"), QStringLiteral("emboss"));
    params.values.insert(QStringLiteral("params"), QJsonArray{});
    return window_->registry()->executeRecorded(
        pictura::commandIdForPath(
            {QStringLiteral("Filter"), QStringLiteral("Stylize"), QStringLiteral("Emboss")}),
        params);
}

pictura::LayersPanel* SmartFilterApplyTest::panelOn(pictura::PictureView* view, const QString& path)
{
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    if (!panel) {
        return nullptr;
    }
    panel->setView(view);
    panel->refresh();
    QCoreApplication::processEvents();
    panel->expandForTest(path);
    panel->expandForTest(path + QStringLiteral("/@sf"));
    QCoreApplication::processEvents();
    return panel;
}

QAction* SmartFilterApplyTest::action(const QStringList& path)
{
    return window_->registry()->action(pictura::commandIdForPath(path));
}

void SmartFilterApplyTest::filterMenuAppendsSmartFilter()
{
    const QString path = smartObject(QStringLiteral("Append"));
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    QVERIFY(applyGaussianBlur(2.0));
    QVERIFY(applyGaussianBlur(3.0));
    QCOMPARE(pictura::layer_smart_filter_count(*view, path), 2);
    QCOMPARE(pictura::layer_smart_filter_name(*view, path, 0), QStringLiteral("Gaussian Blur"));
    QCOMPARE(pictura::layer_smart_filter_name(*view, path, 1), QStringLiteral("Gaussian Blur"));
    QVERIFY(!pictura::layer_has_smart_filter_mask(*view, path));
}

void SmartFilterApplyTest::selectionBecomesFilterMask()
{
    const QString path = smartObject(QStringLiteral("Mask"));
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    view->select_rect(0, 0, 8, 16, QStringLiteral("new"), 0.0);
    QVERIFY(applyGaussianBlur(2.0));
    QCOMPARE(pictura::layer_smart_filter_count(*view, path), 1);
    QVERIFY(pictura::layer_has_smart_filter_mask(*view, path));
}

void SmartFilterApplyTest::filterGalleryAddsOneEntry()
{
    const QString path = smartObject(QStringLiteral("Gallery"));
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    const QStringList kinds{QStringLiteral("gaussian-blur"), QStringLiteral("gaussian-blur")};
    QVERIFY(pictura::apply_filter_stack(*view, kinds, QList<double>{1.0, 2.0}));
    QCOMPARE(pictura::layer_smart_filter_count(*view, path), 1);
    QCOMPARE(pictura::layer_smart_filter_name(*view, path, 0), QStringLiteral("Filter Gallery"));
}

void SmartFilterApplyTest::excludedFiltersDisabledOnSmartObject()
{
    QVERIFY(!smartObject(QStringLiteral("Excluded")).isEmpty());
    window_->registry()->refresh();
    for (const char* kind :
         {"liquify", "vanishing-point", "lens-blur", "displace", "adaptive-wide-angle"}) {
        const pictura::FilterCommandSpec* spec =
            pictura::filterCommandForKind(QString::fromLatin1(kind));
        QVERIFY2(spec, kind);
        QAction* entry = action(spec->path);
        QVERIFY2(entry && !entry->isEnabled(), kind);
    }
    QAction* blur =
        action({QStringLiteral("Filter"), QStringLiteral("Blur"), QStringLiteral("Gaussian Blur")});
    QVERIFY(blur && blur->isEnabled());
}

void SmartFilterApplyTest::previewCancelRestoresSmartObject()
{
    const QString path = smartObject(QStringLiteral("Preview"));
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    QVERIFY(pictura::filter_preview(*view, QStringLiteral("gaussian-blur"), {4.0}));
    QCOMPARE(pictura::layer_smart_filter_count(*view, path), 1);
    QVERIFY(pictura::filter_preview_cancel(*view));
    QCOMPARE(pictura::layer_smart_filter_count(*view, path), 0);
}

void SmartFilterApplyTest::undoRemovesSmartFilter()
{
    const QString path = smartObject(QStringLiteral("Undo"));
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    QVERIFY(applyGaussianBlur(2.0));
    QCOMPARE(pictura::layer_smart_filter_count(*view, path), 1);
    QVERIFY(view->undo());
    QCOMPARE(pictura::layer_smart_filter_count(*view, path), 0);
}

void SmartFilterApplyTest::adjustmentsDisabledOnSmartObject()
{
    QVERIFY(!smartObject(QStringLiteral("Adjust")).isEmpty());
    pictura::PictureView* view = window_->activeView();
    const QStringList invert{QStringLiteral("Image"), QStringLiteral("Adjustments"),
                             QStringLiteral("Invert")};
    QVERIFY(pictura::filter_target_smart(*view));
    window_->registry()->refresh();
    QVERIFY(action(invert) && !action(invert)->isEnabled());

    // The Background is a raster layer, where the adjustments apply again.
    view->set_active_layer(QStringLiteral("0"));
    QVERIFY(!pictura::filter_target_smart(*view));
    window_->registry()->refresh();
    QVERIFY(action(invert)->isEnabled());
}

void SmartFilterApplyTest::editSmartFilterEnabledForGenericEntry()
{
    const QString path = smartObject(QStringLiteral("Edit"));
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    QVERIFY(applyGaussianBlur(2.0));
    QVERIFY(pictura::apply_filter_stack(*view, {QStringLiteral("gaussian-blur")}, {1.0}));
    QCOMPARE(pictura::layer_smart_filter_edit_kind(*view, path, 0), 3);
    QCOMPARE(pictura::layer_smart_filter_edit_kind(*view, path, 1), 4);
    QCOMPARE(pictura::layer_smart_filter_kinds(*view, path, 0),
             QStringList{QStringLiteral("gaussian-blur")});
    QCOMPARE(pictura::layer_smart_filter_params(*view, path, 0), QList<double>{2.0});
}

void SmartFilterApplyTest::filterRowsShowNewestFirst()
{
    const QString path = smartObject(QStringLiteral("Order"));
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    QVERIFY(applyGaussianBlur(2.0));
    QVERIFY(applyEmboss());
    pictura::LayersPanel* panel = panelOn(view, path);
    QVERIFY(panel);
    auto* tree = panel->findChild<QTreeView*>();
    QVERIFY(tree && tree->model());
    QAbstractItemModel* model = tree->model();
    std::function<QModelIndex(const QModelIndex&)> findGroup =
        [&](const QModelIndex& parent) -> QModelIndex {
        for (int r = 0; r < model->rowCount(parent); ++r) {
            const QModelIndex index = model->index(r, 0, parent);
            if (index.data(Qt::DisplayRole).toString() == QStringLiteral("Smart Filters")) {
                return index;
            }
            const QModelIndex found = findGroup(index);
            if (found.isValid()) {
                return found;
            }
        }
        return {};
    };
    const QModelIndex group = findGroup(QModelIndex());
    QVERIFY(group.isValid());
    QCOMPARE(model->rowCount(group), 2);
    QCOMPARE(model->index(0, 0, group).data(Qt::DisplayRole).toString(), QStringLiteral("Emboss"));
    QCOMPARE(model->index(1, 0, group).data(Qt::DisplayRole).toString(),
             QStringLiteral("Gaussian Blur"));
}

void SmartFilterApplyTest::doubleClickReopensTheFilterDialog()
{
    const QString path = smartObject(QStringLiteral("DoubleClick"));
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    QVERIFY(applyGaussianBlur(2.0));
    QVERIFY(applyEmboss());
    pictura::LayersPanel* panel = panelOn(view, path);
    QVERIFY(panel);

    // Gaussian Blur is under Emboss, so the stacked-filters notice comes
    // first; tick Don't show again, then OK the prefilled dialog.
    bool noticed = false;
    bool opened = false;
    QTimer poll;
    poll.setInterval(20);
    connect(&poll, &QTimer::timeout, [&] {
        for (QWidget* widget : QApplication::topLevelWidgets()) {
            if (!widget->isVisible()) {
                continue;
            }
            if (auto* box = qobject_cast<QMessageBox*>(widget);
                box && box->objectName() == QStringLiteral("smartFilterEditNotice")) {
                noticed = true;
                box->checkBox()->setChecked(true);
                box->accept();
            } else if (auto* dialog = qobject_cast<pictura::FilterPreviewDialog*>(widget)) {
                opened = true;
                dialog->accept();
            }
        }
    });
    poll.start();
    QVERIFY(panel->doubleClickRowCenterForTest(path + QStringLiteral("/@sf/0")));
    poll.stop();
    QVERIFY2(noticed, "the stacked-filters notice was shown");
    QVERIFY2(opened, "the filter's dialog reopened");
    QVERIFY(!pictura::loadSession().showSmartFilterEditMessage);
    QCOMPARE(pictura::layer_smart_filter_count(*view, path), 2);
    QCOMPARE(pictura::layer_smart_filter_name(*view, path, 0), QStringLiteral("Gaussian Blur"));
    QCOMPARE(pictura::layer_smart_filter_name(*view, path, 1), QStringLiteral("Emboss"));
}

void SmartFilterApplyTest::dragReordersSmartFilters()
{
    const QString path = smartObject(QStringLiteral("Drag"));
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    QVERIFY(applyGaussianBlur(2.0));
    QVERIFY(applyEmboss());
    pictura::LayersPanel* panel = panelOn(view, path);
    QVERIFY(panel);
    const QString bottom = path + QStringLiteral("/@sf/0");
    const QString top = path + QStringLiteral("/@sf/1");

    // Gaussian Blur (the bottom row) dragged above Emboss applies last.
    QVERIFY(panel->dropAtForTest(bottom, top, true));
    QCoreApplication::processEvents();
    QCOMPARE(pictura::layer_smart_filter_name(*view, path, 0), QStringLiteral("Emboss"));
    QCOMPARE(pictura::layer_smart_filter_name(*view, path, 1), QStringLiteral("Gaussian Blur"));

    QVERIFY(view->undo());
    QCOMPARE(pictura::layer_smart_filter_name(*view, path, 0), QStringLiteral("Gaussian Blur"));
    QCOMPARE(pictura::layer_smart_filter_name(*view, path, 1), QStringLiteral("Emboss"));

    // A filter row only drops inside its own list.
    panel->refresh();
    QCoreApplication::processEvents();
    QVERIFY(!panel->dropAtForTest(bottom, QStringLiteral("0"), true));
    QCOMPARE(pictura::layer_smart_filter_name(*view, path, 0), QStringLiteral("Gaussian Blur"));
}

QTEST_MAIN(SmartFilterApplyTest)
#include "tst_smart_filter_apply.moc"
