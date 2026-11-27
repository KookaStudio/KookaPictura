#include <QtTest/QtTest>

#include "api/layers_bridge.h"
#include "qt_test_support.h"
#include "selftest_paint_fixture.h"

#include <QtWidgets/QMenu>
#include <QtWidgets/QToolButton>

#include <algorithm>
#include <initializer_list>
#include <limits>
#include <vector>

namespace {

using paint_fixture::Fixture;
using pictura::PictureView;

QImage seed()
{
    QImage image(96, 72, QImage::Format_RGB32);
    image.fill(Qt::white);
    return image;
}

void appendContour(rust::Vec<double>& out, std::initializer_list<QPointF> anchors,
                   bool curved = false)
{
    out.push_back(std::numeric_limits<double>::quiet_NaN());
    for (double n : {1.0, 0.0, 0.0, 0.0, 0.0}) {
        out.push_back(n);
    }
    for (const QPointF& point : anchors) {
        const QPointF handle = curved && point == *anchors.begin() ? point + QPointF(3, -4)
                                                                   : point;
        for (double n : {point.x(), point.y(), point.x(), point.y(), handle.x(), handle.y()}) {
            out.push_back(n);
        }
    }
}

rust::Vec<double> overlapping()
{
    rust::Vec<double> paths;
    appendContour(paths, {{4, 4}, {36, 4}, {36, 36}, {4, 36}});
    appendContour(paths, {{22, 22}, {48, 22}, {48, 50}, {22, 50}});
    return paths;
}

bool replacePath(PictureView& view, const rust::Vec<double>& coords)
{
    return pictura::path_replace_target_outline(view, {coords.data(), coords.size()});
}

std::vector<double> outline(const PictureView& view)
{
    const auto values = pictura::path_overlay_points(view);
    return {values.begin(), values.end()};
}

QAction* mergeAction(pictura::PicturaMainWindow& frame)
{
    auto* button = frame.findChild<QToolButton*>(QStringLiteral("optionsPathOperations"));
    if (!button || !button->menu()) {
        return nullptr;
    }
    QMetaObject::invokeMethod(button->menu(), "aboutToShow");
    return button->menu()->findChild<QAction*>(QStringLiteral("optionsMergeShapeComponents"));
}

} // namespace

class MergeShapeComponentsTest : public QObject {
    Q_OBJECT

private slots:
    void mergeWorkPath();
    void mergeSavedPathAndLayer();
    void noMergeWithoutOverlap();
    void mergeWholePathWithFocusedContour();

private:
    pictura::test::ScopedStateHome stateHome_;
};

void MergeShapeComponentsTest::mergeWorkPath()
{
    QVERIFY(stateHome_.isValid());
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    Fixture f(*window, seed(), QStringLiteral("merge_work_path"));
    QVERIFY(f.ok());
    auto paths = overlapping();
    appendContour(paths, {{8, 8}, {8, 15}, {15, 15}, {15, 8}});
    appendContour(paths, {{65, 10}, {75, 10}, {75, 24}, {65, 24}}, true);
    QVERIFY(replacePath(*f.view, paths));
    window->setActiveTool(pictura::ToolId::PathSelection);
    f.drag({QPointF(8, 30)});
    QVERIFY(f.canvas->pathOverlayAnchorsSolidForTest());
    const auto original = outline(*f.view);
    const auto independent = pictura::path_point(*f.view, 3, 0);
    const int base = f.view->history_index();
    auto* merge = mergeAction(*window);
    QVERIFY(merge && merge->isEnabled());
    merge->trigger();
    QVERIFY(f.committedOnce(base, "Merge Shape Components"));
    QCOMPARE(pictura::path_subpath_count(*f.view), 3);
    const auto after = pictura::path_point(*f.view, 2, 0);
    QVERIFY(independent.size() == 9 && after.size() == independent.size());
    QVERIFY(std::equal(independent.begin(), independent.end(), after.begin()));
    QCOMPARE(f.canvas->pathOverlayAnchorCountForTest(), 0);
    QVERIFY(f.view->undo());
    QVERIFY(outline(*f.view) == original);
}

void MergeShapeComponentsTest::mergeSavedPathAndLayer()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    Fixture f(*window, seed(), QStringLiteral("merge_saved_path_layer"));
    QVERIFY(f.ok());
    const auto paths = overlapping();
    QVERIFY(replacePath(*f.view, paths));
    const int saved = pictura::paths_save_work(*f.view, QStringLiteral("Saved outline"));
    QVERIFY(saved >= 0);
    pictura::ShapeSpec spec{};
    spec.kind = 0;
    spec.boxed = true;
    spec.x0 = 2;
    spec.y0 = 2;
    spec.x1 = 18;
    spec.y1 = 18;
    spec.stroke = true;
    spec.stroke_color = 0xff00ff00u;
    spec.stroke_width = 2;
    const auto shape = pictura::shape_add_layer(*f.view, spec, 0xffff0000u);
    QVERIFY(!shape.isEmpty());
    f.view->set_active_layer(shape);
    QVERIFY(pictura::shape_replace_active_outline(*f.view, {paths.data(), paths.size()}));
    QVERIFY(pictura::vector_mask_set_linked(*f.view, false));
    QVERIFY(pictura::vector_mask_set_enabled(*f.view, false));
    const auto style = pictura::shape_active_style(*f.view);

    window->setActiveTool(pictura::ToolId::PathSelection);
    QVERIFY(pictura::paths_set_active(*f.view, -2));
    const auto originalLayer = outline(*f.view);
    QVERIFY(pictura::paths_set_active(*f.view, saved));
    QVERIFY(!pictura::path_target_is_layer(*f.view));
    const int beforeSaved = f.view->history_index();
    auto* merge = mergeAction(*window);
    QVERIFY(merge && merge->isEnabled());
    merge->trigger();
    QVERIFY(f.committedOnce(beforeSaved, "Merge Shape Components"));
    QCOMPARE(pictura::path_subpath_count(*f.view), 1);
    const auto mergedSaved = outline(*f.view);

    QVERIFY(pictura::paths_set_active(*f.view, -2));
    QVERIFY(pictura::path_target_is_layer(*f.view));
    QVERIFY(outline(*f.view) == originalLayer);
    const int beforeLayer = f.view->history_index();
    merge = mergeAction(*window);
    QVERIFY(merge && merge->isEnabled());
    merge->trigger();
    QVERIFY(f.committedOnce(beforeLayer, "Merge Shape Components"));
    QVERIFY(outline(*f.view) != originalLayer);
    QCOMPARE(pictura::shape_active_style(*f.view), style);
    QVERIFY(pictura::vector_mask_disabled(*f.view));
    QVERIFY(!pictura::vector_mask_linked(*f.view));
    QVERIFY(pictura::paths_set_active(*f.view, saved));
    QVERIFY(outline(*f.view) == mergedSaved);
}

void MergeShapeComponentsTest::noMergeWithoutOverlap()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    Fixture f(*window, seed(), QStringLiteral("merge_disjoint_paths"));
    QVERIFY(f.ok());
    window->setActiveTool(pictura::ToolId::PathSelection);
    auto* merge = mergeAction(*window);
    QVERIFY(merge && !merge->isEnabled());

    rust::Vec<double> disjoint;
    appendContour(disjoint, {{4, 4}, {22, 4}, {22, 22}, {4, 22}});
    appendContour(disjoint, {{44, 4}, {62, 4}, {62, 22}, {44, 22}});
    QVERIFY(replacePath(*f.view, disjoint));
    const auto original = outline(*f.view);
    const auto paths = overlapping();
    QVERIFY(replacePath(*f.view, paths));
    QVERIFY(f.view->undo());
    const int before = f.view->history_index();
    merge = mergeAction(*window);
    QVERIFY(merge && merge->isEnabled());
    merge->trigger();
    QCOMPARE(f.view->history_index(), before);
    QVERIFY(outline(*f.view) == original);
    QVERIFY(f.view->redo());
    QVERIFY(outline(*f.view) != original);
}

void MergeShapeComponentsTest::mergeWholePathWithFocusedContour()
{
    auto window = pictura::test::makeMainWindow();
    QVERIFY(window);
    Fixture f(*window, seed(), QStringLiteral("merge_focused_contour"));
    QVERIFY(f.ok());
    auto paths = overlapping();
    QVERIFY(replacePath(*f.view, paths));
    const int saved = pictura::paths_save_work(*f.view, QStringLiteral("Untouched"));
    QVERIFY(saved >= 0);
    const auto originalSaved = outline(*f.view);
    QVERIFY(pictura::paths_set_active(*f.view, -2));
    appendContour(paths, {{58, 8}, {76, 8}, {76, 25}, {58, 25}});
    appendContour(paths, {{68, 16}, {85, 16}, {85, 33}, {68, 33}});
    QVERIFY(replacePath(*f.view, paths));
    window->setActiveTool(pictura::ToolId::PathSelection);
    f.drag({QPointF(8, 28)});
    QVERIFY(f.canvas->pathOverlayAnchorsSolidForTest());
    auto* merge = mergeAction(*window);
    QVERIFY(merge && merge->isEnabled());
    merge->trigger();
    QCOMPARE(pictura::path_subpath_count(*f.view), 2);
    QVERIFY(pictura::paths_set_active(*f.view, saved));
    QVERIFY(outline(*f.view) == originalSaved);
}

PICTURA_QT_TEST_MAIN(MergeShapeComponentsTest)

#include "tst_merge_shape_components.moc"
