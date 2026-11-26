// The Paths panel (#72): saved paths and the Work Path, the active row's
// visibility, Fill Path / Stroke Path / Load Path as a Selection, and a pixel
// layer's vector-mask row (#261).

#include <QtTest/QtTest>

#include "panels/paths_panel.h"
#include "selftest_paint_fixture.h"

#include "api/layers_bridge.h"

#include <QtWidgets/QListWidget>

#include <algorithm>

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

// A closed square from (l, t) to (r, b), clockwise, drawn on the active path
// as one subpath; `reverse` winds it the other way.
void square(pictura::PictureView& v, double l, double t, double r, double b, bool reverse = false)
{
    QList<QPointF> corners {QPointF(l, t), QPointF(r, t), QPointF(r, b), QPointF(l, b)};
    if (reverse) {
        std::reverse(corners.begin(), corners.end());
    }
    for (const QPointF& p : corners) {
        pictura::path_append_corner(v, p.x(), p.y(), false);
        pictura::path_commit_anchor(v);
    }
    pictura::path_close(v);
}

// A rasterized fill layer above the seed, selected, with a Reveal All vector
// mask over the canvas; its path, empty on failure.
QString maskedLayer(pictura::PicturaMainWindow& frame, pictura::PictureView& v)
{
    const QString layer = v.add_solid_fill(0xff2244aau);
    if (layer.isEmpty()) {
        return {};
    }
    pictura::layer_mask_delete_path(v, layer);
    if (!v.rasterize_fill_content(layer)) {
        return {};
    }
    frame.selectLayerPath(layer);
    QCoreApplication::processEvents();
    return pictura::vector_mask_add(v, QStringLiteral("reveal-all")) ? layer : QString();
}

bool sameOutline(const ::rust::Vec<double>& a, const ::rust::Vec<double>& b)
{
    return std::equal(a.begin(), a.end(), b.begin(), b.end());
}

QStringList rowNames(QListWidget* list)
{
    QStringList names;
    for (int i = 0; i < list->count(); ++i) {
        names << list->item(i)->text();
    }
    return names;
}

} // namespace

class PathsPanelTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void savedPathsAndWorkPath();
    void visibilityFollowsTheSelectedRow();
    void fillStrokeAndLoadSelection();
    void vectorMaskRowFollowsTheLayer();
    void vectorMaskRowEditsTheMask();
    void penDrawsIntoTheVectorMask();
    void savedPathKeepsPriorityOverTheMask();
    void deletingTheVectorMaskRow();

private:
    pictura::PathsPanel* panel() const
    {
        return window_->findChild<pictura::PathsPanel*>(QStringLiteral("pathsPanel"));
    }

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void PathsPanelTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    QVERIFY(panel());
}

void PathsPanelTest::savedPathsAndWorkPath()
{
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_paths_saved_seed"));
    QVERIFY2(f.ok(), "fixture");
    pictura::PathsPanel* p = panel();
    QListWidget* list = p->listForTest();
    pictura::PictureView& v = *f.view;

    square(v, 10, 10, 50, 50);
    p->refresh();
    QCOMPARE(rowNames(list), QStringList {QStringLiteral("Work Path")});

    // Saving moves the Work Path into a named, active saved path.
    QVERIFY(p->saveWorkPath(QStringLiteral("Outline")));
    QCOMPARE(v.history_label(v.history_index()), QStringLiteral("Save Path"));
    QCOMPARE(rowNames(list), QStringList {QStringLiteral("Outline")});
    QVERIFY(!list->item(0)->font().italic());
    QVERIFY(list->item(0)->isSelected());
    QVERIFY(!pictura::paths_has_work_path(v));
    QCOMPARE(pictura::path_subpath_count(v), 1);

    // New Path takes the next default name; Delete removes the selected row.
    QVERIFY(p->newPath());
    QCOMPARE(rowNames(list), (QStringList {QStringLiteral("Outline"), QStringLiteral("Path 1")}));
    QCOMPARE(pictura::paths_active(v), 1);
    QCOMPARE(pictura::path_subpath_count(v), 0);
    QVERIFY(p->deletePath());
    QCOMPARE(v.history_label(v.history_index()), QStringLiteral("Delete Path"));
    QCOMPARE(rowNames(list), QStringList {QStringLiteral("Outline")});

    // Duplicate and rename; undo walks them back.
    list->setCurrentRow(0);
    QVERIFY(p->duplicatePath());
    QCOMPARE(rowNames(list),
             (QStringList {QStringLiteral("Outline"), QStringLiteral("Outline copy")}));
    QCOMPARE(pictura::path_subpath_count(v), 1);
    QVERIFY(pictura::paths_rename(v, 1, QStringLiteral("Inner")));
    QVERIFY(!pictura::paths_rename(v, 1, QStringLiteral("Inner")));
    p->refresh();
    QCOMPARE(list->item(1)->text(), QStringLiteral("Inner"));
    QVERIFY(v.undo() && v.undo());
    p->refresh();
    QCOMPARE(rowNames(list), QStringList {QStringLiteral("Outline")});

    // With no row selected, the Pen starts a fresh Work Path below the saved one.
    QVERIFY(pictura::paths_set_active(v, -2));
    square(v, 60, 60, 90, 90);
    p->refresh();
    QCOMPARE(rowNames(list), (QStringList {QStringLiteral("Outline"), QStringLiteral("Work Path")}));
    QVERIFY(list->item(1)->isSelected());
    const ::rust::Vec<double> outline = pictura::paths_outline(v, 0);
    QVERIFY(!outline.empty());
    QCOMPARE(outline.size(), std::size_t(1 + 2 * outline[0]));
}

void PathsPanelTest::visibilityFollowsTheSelectedRow()
{
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_paths_visible_seed"));
    QVERIFY2(f.ok(), "fixture");
    // A standalone panel, so the clicks land on an exposed list.
    pictura::PathsPanel standalone;
    pictura::PathsPanel* p = &standalone;
    QListWidget* list = p->listForTest();
    pictura::PictureView& v = *f.view;
    p->setView(&v);
    square(v, 10, 10, 50, 50);
    p->refresh();
    standalone.resize(240, 300);
    standalone.show();
    QVERIFY(QTest::qWaitForWindowExposed(&standalone));
    QCOMPARE(pictura::path_subpath_count(v), 1);

    // A click below the rows hides the path from the tools and the canvas.
    const QPoint below(5, list->viewport()->height() - 2);
    QVERIFY(!list->itemAt(below));
    QTest::mouseClick(list->viewport(), Qt::LeftButton, {}, below);
    QCOMPARE(pictura::paths_active(v), -2);
    QCOMPARE(pictura::path_subpath_count(v), 0);
    QVERIFY(!p->fillPath());
    QVERIFY(pictura::paths_has_work_path(v));

    // Selecting the row shows it again.
    QTest::mouseClick(list->viewport(), Qt::LeftButton, {},
                      list->visualItemRect(list->item(0)).center());
    QCOMPARE(pictura::paths_active(v), -1);
    QCOMPARE(pictura::path_subpath_count(v), 1);
}

void PathsPanelTest::fillStrokeAndLoadSelection()
{
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_paths_fill_seed"));
    QVERIFY2(f.ok(), "fixture");
    pictura::PathsPanel* p = panel();
    QListWidget* list = p->listForTest();
    pictura::PictureView& v = *f.view;
    v.set_active_layer(QStringLiteral("0"));
    const QColor foreground = f.tools->foreground();
    const int brush = f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->size();

    // Two same-direction squares: nonzero keeps the inner one filled.
    square(v, 10, 10, 60, 60);
    square(v, 25, 25, 45, 45);
    p->refresh();
    list->setCurrentRow(0);
    QVERIFY(p->loadSelection());
    QCOMPARE(v.history_label(v.history_index()), QStringLiteral("Make Selection"));
    QCOMPARE(v.selection_coverage(35, 35), 255);
    QCOMPARE(v.selection_coverage(15, 35), 255);
    QCOMPARE(v.selection_coverage(5, 5), 0);
    v.deselect();

    const QColor red(255, 0, 0);
    f.tools->setForeground(red);
    QVERIFY(p->fillPath());
    QCOMPARE(v.history_label(v.history_index()), QStringLiteral("Fill Path"));
    QCOMPARE(v.sample_argb(35, 35), red.rgba());
    QCOMPARE(v.sample_argb(80, 80), QColor(Qt::white).rgba());

    const QColor blue(0, 0, 255);
    f.tools->setForeground(blue);
    f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setSize(4);
    QVERIFY(p->strokePath());
    QCOMPARE(v.history_label(v.history_index()), QStringLiteral("Stroke Path"));
    QCOMPARE(v.sample_argb(10, 35), blue.rgba());
    QCOMPARE(v.sample_argb(25, 35), blue.rgba());
    QCOMPARE(v.sample_argb(35, 35), red.rgba());

    // An opposite inner square cuts a hole.
    QVERIFY(p->deletePath());
    QVERIFY(pictura::paths_set_active(v, -2));
    square(v, 10, 10, 60, 60);
    square(v, 25, 25, 45, 45, true);
    p->refresh();
    list->setCurrentRow(0);
    QVERIFY(p->loadSelection());
    QCOMPARE(v.selection_coverage(35, 35), 0);
    QCOMPARE(v.selection_coverage(15, 35), 255);

    f.tools->setForeground(foreground);
    f.tools->optionsFor<pictura::PaintOptions>(pictura::ToolId::Brush)->setSize(brush);
}

// vmask_row_listed
void PathsPanelTest::vectorMaskRowFollowsTheLayer()
{
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_paths_mask_row_seed"));
    QVERIFY2(f.ok(), "fixture");
    pictura::PathsPanel* p = panel();
    QListWidget* list = p->listForTest();
    pictura::PictureView& v = *f.view;
    const QString layer = maskedLayer(f.frame, v);
    QVERIFY(!layer.isEmpty());

    p->refresh();
    const QString name = pictura::paths_vector_mask_name(v);
    QVERIFY(!name.isEmpty());
    QCOMPARE(rowNames(list), QStringList {name + QStringLiteral(" Vector Mask")});
    QVERIFY(!pictura::paths_outline(v, -4).empty());
    // Listing the row does not activate it.
    QCOMPARE(pictura::path_subpath_count(v), 0);

    f.frame.selectLayerPath(QStringLiteral("0"));
    QCoreApplication::processEvents();
    p->refresh();
    QVERIFY(pictura::paths_vector_mask_name(v).isEmpty());
    QVERIFY(rowNames(list).isEmpty());
    QVERIFY(!pictura::paths_set_active(v, -4));
}

// vmask_row_edit
void PathsPanelTest::vectorMaskRowEditsTheMask()
{
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_paths_mask_edit_seed"));
    QVERIFY2(f.ok(), "fixture");
    pictura::PathsPanel* p = panel();
    QListWidget* list = p->listForTest();
    pictura::PictureView& v = *f.view;
    const QString layer = maskedLayer(f.frame, v);
    QVERIFY(!layer.isEmpty());
    p->refresh();
    list->setCurrentRow(0);
    QCOMPARE(pictura::paths_active(v), -4);
    QCOMPARE(pictura::path_subpath_count(v), 1);
    QCOMPARE(pictura::path_point_count(v, 0), 4);
    const QRgb white = QColor(Qt::white).rgba();
    const QRgb fill = v.sample_argb(10, 10);
    QVERIFY(fill != white);

    // Pull the top-left corner to the centre: the corner it left is uncovered.
    const int before = v.history_index();
    pictura::path_gesture_begin(v);
    QVERIFY(pictura::path_move_anchor(v, 0, 0, 50, 50));
    pictura::path_commit_drag(v, "Drag Anchor Point");
    pictura::path_gesture_end(v);
    QCOMPARE(v.history_index(), before + 1);
    QCOMPARE(pictura::path_point(v, 0, 0)[0], 50.0);
    QCOMPARE(v.sample_argb(10, 10), white);
    QCOMPARE(v.sample_argb(90, 90), fill);
    QVERIFY(pictura::vector_mask_present_path(v, layer));
    QVERIFY(!pictura::paths_has_work_path(v));

    QVERIFY(v.undo());
    QCOMPARE(pictura::path_point(v, 0, 0)[0], 0.0);
    QCOMPARE(v.sample_argb(10, 10), fill);
}

// vmask_pen_adds
void PathsPanelTest::penDrawsIntoTheVectorMask()
{
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_paths_mask_pen_seed"));
    QVERIFY2(f.ok(), "fixture");
    pictura::PathsPanel* p = panel();
    pictura::PictureView& v = *f.view;
    QVERIFY(!maskedLayer(f.frame, v).isEmpty());
    p->refresh();
    p->listForTest()->setCurrentRow(0);
    QCOMPARE(pictura::paths_active(v), -4);

    // Every anchor extends the one new component, across the mask's re-decodes.
    for (const QPointF& corner : {QPointF(20, 20), QPointF(80, 20), QPointF(50, 80)}) {
        pictura::path_append_corner(v, corner.x(), corner.y(), false);
        pictura::path_commit_anchor(v);
    }
    QVERIFY(pictura::path_close(v));
    QCOMPARE(pictura::path_subpath_count(v), 2);
    QCOMPARE(pictura::path_point_count(v, 1), 3);
    QVERIFY(pictura::path_subpath_closed(v, 1));
    QVERIFY(!pictura::paths_has_work_path(v));
    QCOMPARE(v.history_label(v.history_index()), QStringLiteral("Close Path"));
}

// vmask_saved_unaffected
void PathsPanelTest::savedPathKeepsPriorityOverTheMask()
{
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_paths_mask_saved_seed"));
    QVERIFY2(f.ok(), "fixture");
    pictura::PathsPanel* p = panel();
    pictura::PictureView& v = *f.view;
    QVERIFY(!maskedLayer(f.frame, v).isEmpty());
    QVERIFY(pictura::paths_set_active(v, -2));
    square(v, 10, 10, 40, 40);
    QVERIFY(p->saveWorkPath(QStringLiteral("Outline")));
    QCOMPARE(pictura::paths_active(v), 0);
    const ::rust::Vec<double> mask = pictura::paths_outline(v, -4);
    QVERIFY(!mask.empty());

    // Path Selection turns the layer target on; a pixel layer's mask is still
    // not the target while a saved row is active.
    pictura::path_set_layer_target(v, true);
    QVERIFY(!pictura::path_target_is_layer(v));
    pictura::path_gesture_begin(v);
    QVERIFY(pictura::path_move_subpath(v, 0, 5, 5));
    pictura::path_commit_drag(v, "Drag Path");
    pictura::path_gesture_end(v);
    pictura::path_set_layer_target(v, false);
    QCOMPARE(pictura::path_point(v, 0, 0)[0], 15.0);
    QVERIFY(sameOutline(pictura::paths_outline(v, -4), mask));
}

// vmask_delete_row
void PathsPanelTest::deletingTheVectorMaskRow()
{
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(*window_, seed, QStringLiteral("pictura_paths_mask_delete_seed"));
    QVERIFY2(f.ok(), "fixture");
    pictura::PathsPanel* p = panel();
    QListWidget* list = p->listForTest();
    pictura::PictureView& v = *f.view;
    const QString layer = maskedLayer(f.frame, v);
    QVERIFY(!layer.isEmpty());
    p->refresh();
    list->setCurrentRow(0);
    QVERIFY(p->deletePath());
    QVERIFY(!pictura::vector_mask_present_path(v, layer));
    QCOMPARE(v.history_label(v.history_index()), QStringLiteral("Delete Vector Mask"));
    QVERIFY(rowNames(list).isEmpty());
    QCOMPARE(pictura::paths_active(v), -2);
}

PICTURA_QT_TEST_MAIN(PathsPanelTest)
#include "tst_paths_panel.moc"
