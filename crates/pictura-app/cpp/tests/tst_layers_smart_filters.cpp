#include <QtTest/QtTest>

#include <QtCore/QCoreApplication>
#include <QtCore/QStringList>
#include <QtCore/QTimer>
#include <QtGui/QAction>
#include <QtWidgets/QApplication>
#include <QtWidgets/QTreeView>

#include <functional>

#include "frame.h"
#include "commands.h"
#include "pictura_raw_dialog.h"
#include "panels/layers_panel.h"
#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layers_smart_filters.cxxqt.h"

#include "qt_test_support.h"

class LayersSmartFiltersTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void treeShowsGroupAndToggles();
    void clearMenuCommandRemovesFilters();
    void smartFiltersGroupMenu();
    void smartFilterRowMenu();
    void deleteFilterMaskFollowsTheMask();
    void editSmartFilterIsGatedOnKind();
    void editSmartFilterRequestsTheDialog();
    void menuTogglesAndClearsFilters();
    void filterRowDisablesAndDeletes();

private:
    // A smart object with one camera-raw smart filter; returns its layer path.
    QString filteredSmartObject();
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void LayersSmartFiltersTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->show();
    QCoreApplication::processEvents();
}

void LayersSmartFiltersTest::cleanup()
{
    if (window_) {
        while (window_->activeDocumentIndex() >= 0) {
            window_->closeDocument(window_->activeDocumentIndex(), false);
        }
    }
}

void LayersSmartFiltersTest::treeShowsGroupAndToggles()
{
    QVERIFY(window_->newDocument(QStringLiteral("SmartFilters"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view && panel);

    const QString path = view->add_solid_fill(0xff808080u);
    QVERIFY(view->rasterize_fill_content(path));
    QVERIFY(view->convert_to_smart_object(path));
    QVERIFY(view->apply_pictura_raw_filter(path, QStringLiteral("Temp=20;Ex12=1.5")));

    const QString group = path + QStringLiteral("/@sf");
    const QString child = group + QStringLiteral("/0");

    panel->setView(view);
    panel->refresh();
    QCoreApplication::processEvents();
    panel->expandForTest(path);
    panel->expandForTest(group);
    QCoreApplication::processEvents();

    auto* tree = panel->findChild<QTreeView*>();
    QVERIFY(tree);
    QAbstractItemModel* model = tree->model();
    QVERIFY(model);

    std::function<QModelIndex(const QModelIndex&)> findText =
        [&](const QModelIndex& parent) -> QModelIndex {
        for (int r = 0; r < model->rowCount(parent); ++r) {
            const QModelIndex index = model->index(r, 0, parent);
            if (index.data(Qt::DisplayRole).toString() == QStringLiteral("Smart Filters")) {
                return index;
            }
            const QModelIndex found = findText(index);
            if (found.isValid()) {
                return found;
            }
        }
        return {};
    };
    const QModelIndex groupIndex = findText(QModelIndex());
    QVERIFY(groupIndex.isValid());
    QCOMPARE(model->rowCount(groupIndex), 1);
    QCOMPARE(model->index(0, 0, groupIndex).data(Qt::DisplayRole).toString(),
             QStringLiteral("Camera Raw Filter"));

    QVERIFY(pictura::layer_smart_filter_visible(*view, path, 0));
    QVERIFY(panel->clickSmartFilterEyeForTest(child));
    QCoreApplication::processEvents();
    QVERIFY(!pictura::layer_smart_filter_visible(*view, path, 0));

    QVERIFY(pictura::layer_smart_filters_enabled(*view, path));
    QVERIFY(panel->clickSmartFilterEyeForTest(group));
    QCoreApplication::processEvents();
    QVERIFY(!pictura::layer_smart_filters_enabled(*view, path));
}

void LayersSmartFiltersTest::clearMenuCommandRemovesFilters()
{
    QVERIFY(window_->newDocument(QStringLiteral("ClearFilters"), 16, 16, QStringLiteral("rgb"), 8,
                                 QStringLiteral("white")));
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(view && panel);

    const QString path = view->add_solid_fill(0xff808080u);
    QVERIFY(view->rasterize_fill_content(path));
    QVERIFY(view->convert_to_smart_object(path));
    QVERIFY(view->apply_pictura_raw_filter(path, QStringLiteral("Temp=20;Ex12=1.5")));
    QCOMPARE(pictura::layer_smart_filter_count(*view, path), 1);

    panel->setView(view);
    panel->refresh();
    QVERIFY(panel->selectRowForTest(path));
    QCoreApplication::processEvents();

    pictura::CommandRegistry* registry = window_->registry();
    QVERIFY(registry != nullptr);
    QAction* clear = registry->action(QString::fromLatin1(pictura::command_ids::LayerSmartFiltersClear));
    QVERIFY(clear != nullptr);
    registry->refresh();
    QVERIFY2(clear->isEnabled(), "Clear Smart Filters is enabled when the layer has filters");

    QVERIFY(registry->dispatch(
        QString::fromLatin1(pictura::command_ids::LayerSmartFiltersClear)));
    QCOMPARE(pictura::layer_smart_filter_count(*view, path), 0);
    registry->refresh();
    QVERIFY2(!clear->isEnabled(), "Clear Smart Filters disables once the stack is empty");
}

QString LayersSmartFiltersTest::filteredSmartObject()
{
    if (!window_->newDocument(QStringLiteral("SmartMenus"), 64, 64, QStringLiteral("rgb"), 8,
                              QStringLiteral("white"))) {
        return QString();
    }
    pictura::PictureView* view = window_->activeView();
    const QString path = view->add_solid_fill(0xff808080u);
    view->rasterize_fill_content(path);
    view->convert_to_smart_object(path);
    view->apply_pictura_raw_filter(path, QStringLiteral("Temp=20;Ex12=1.5"));
    return path;
}

void LayersSmartFiltersTest::smartFiltersGroupMenu()
{
    const QString path = filteredSmartObject();
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel);
    panel->setView(view);
    panel->refresh();

    QCOMPARE(panel->smartFiltersGroupMenuTextsForTest(path + QStringLiteral("/@sf")),
             QStringList({QStringLiteral("Disable Smart Filters"),
                          QStringLiteral("Delete Filter Mask"),
                          QStringLiteral("Clear Smart Filters")}));
    QVERIFY(window_->closeActiveDocument(false));
}

void LayersSmartFiltersTest::smartFilterRowMenu()
{
    const QString path = filteredSmartObject();
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel);
    panel->setView(view);
    panel->refresh();

    QCOMPARE(panel->smartFilterRowMenuTextsForTest(path + QStringLiteral("/@sf/0")),
             QStringList({QStringLiteral("Edit Smart Filter Blending Options…"),
                          QStringLiteral("Edit Smart Filter…"),
                          QStringLiteral("Disable Smart Filter"),
                          QStringLiteral("Delete Smart Filter")}));
    // Blending options is a follow-up: shown, disabled.
    QVERIFY(!panel->smartFilterMenuEnabledForTest(path + QStringLiteral("/@sf/0"),
                                                  QStringLiteral("Edit Smart Filter Blending Options…")));
    QVERIFY(window_->closeActiveDocument(false));
}

void LayersSmartFiltersTest::deleteFilterMaskFollowsTheMask()
{
    const QString path = filteredSmartObject();
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel);
    panel->setView(view);
    panel->refresh();

    // The camera-raw filter carries no filter mask, so the item is disabled.
    QVERIFY(!pictura::layer_has_smart_filter_mask(*view, path));
    QVERIFY(!panel->smartFilterMenuEnabledForTest(path + QStringLiteral("/@sf"),
                                                  QStringLiteral("Delete Filter Mask")));
    QVERIFY(window_->closeActiveDocument(false));
}

void LayersSmartFiltersTest::editSmartFilterIsGatedOnKind()
{
    const QString path = filteredSmartObject();
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel);
    panel->setView(view);
    panel->refresh();

    // Camera Raw is editable; an out-of-range index (no dialog) is not.
    QCOMPARE(pictura::layer_smart_filter_edit_kind(*view, path, 0), 1);
    QVERIFY(panel->smartFilterMenuEnabledForTest(path + QStringLiteral("/@sf/0"),
                                                 QStringLiteral("Edit Smart Filter…")));
    QCOMPARE(pictura::layer_smart_filter_edit_kind(*view, path, 9), 0);
    QVERIFY(!panel->smartFilterMenuEnabledForTest(path + QStringLiteral("/@sf/9"),
                                                  QStringLiteral("Edit Smart Filter…")));
    QVERIFY(window_->closeActiveDocument(false));
}

void LayersSmartFiltersTest::editSmartFilterRequestsTheDialog()
{
    const QString path = filteredSmartObject();
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel);
    panel->setView(view);
    panel->refresh();

    QString gotPath;
    int gotIndex = -1;
    connect(panel, &pictura::LayersPanel::smartFilterEditRequested, panel,
            [&](const QString& p, int i) {
                gotPath = p;
                gotIndex = i;
            });
    // The window re-dispatches the filter's own command, which opens the modal
    // dialog; close it from the event loop and record that it appeared.
    bool opened = false;
    QTimer::singleShot(0, [&opened] {
        for (QWidget* widget : QApplication::topLevelWidgets()) {
            if (auto* dialog = qobject_cast<pictura::PicturaRawDialog*>(widget);
                dialog && dialog->isVisible()) {
                opened = true;
                dialog->reject();
            }
        }
    });
    QVERIFY(panel->triggerSmartFilterMenuActionForTest(path + QStringLiteral("/@sf/0"),
                                                       QStringLiteral("Edit Smart Filter…")));
    QCOMPARE(gotPath, path);
    QCOMPARE(gotIndex, 0);
    QVERIFY2(opened, "Edit Smart Filter… reopened the filter dialog");
    QVERIFY(window_->closeActiveDocument(false));
}

void LayersSmartFiltersTest::menuTogglesAndClearsFilters()
{
    const QString path = filteredSmartObject();
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel);
    panel->setView(view);
    panel->refresh();

    QVERIFY(panel->triggerSmartFilterMenuActionForTest(path + QStringLiteral("/@sf"),
                                                       QStringLiteral("Disable Smart Filters")));
    QVERIFY(!pictura::layer_smart_filters_enabled(*view, path));
    QVERIFY(panel->triggerSmartFilterMenuActionForTest(path + QStringLiteral("/@sf"),
                                                       QStringLiteral("Enable Smart Filters")));
    QVERIFY(pictura::layer_smart_filters_enabled(*view, path));
    QVERIFY(panel->triggerSmartFilterMenuActionForTest(path + QStringLiteral("/@sf"),
                                                       QStringLiteral("Clear Smart Filters")));
    QCOMPARE(pictura::layer_smart_filter_count(*view, path), 0);
    QVERIFY(window_->closeActiveDocument(false));
}

void LayersSmartFiltersTest::filterRowDisablesAndDeletes()
{
    const QString path = filteredSmartObject();
    QVERIFY(!path.isEmpty());
    pictura::PictureView* view = window_->activeView();
    pictura::LayersPanel* panel =
        window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel);
    panel->setView(view);
    panel->refresh();

    QVERIFY(panel->triggerSmartFilterMenuActionForTest(path + QStringLiteral("/@sf/0"),
                                                       QStringLiteral("Disable Smart Filter")));
    QVERIFY(!pictura::layer_smart_filter_visible(*view, path, 0));
    QVERIFY(panel->triggerSmartFilterMenuActionForTest(path + QStringLiteral("/@sf/0"),
                                                       QStringLiteral("Delete Smart Filter")));
    QCOMPARE(pictura::layer_smart_filter_count(*view, path), 0);
    QVERIFY(window_->closeActiveDocument(false));
}

QTEST_MAIN(LayersSmartFiltersTest)
#include "tst_layers_smart_filters.moc"