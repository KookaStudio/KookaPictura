// The shape tools: Rectangle (#43), Rounded Rectangle (#44), Ellipse (#45),
// Polygon (#46), Line (#47), and Custom Shape (#48), in each of the Shape / Path / Pixels modes; live shapes
// and their conversion prompt; the Create dialogs; the Layers shape badge.

#include <QtTest/QtTest>

#include "panels/layers_panel.h"
#include "panels/numeric_field.h"
#include "selftest_paint_fixture.h"
#include "session.h"

#include "pictura_app/src/cxxqt_object/paths.cxxqt.h"
#include "pictura_app/src/cxxqt_object/shapes.cxxqt.h"

#include <QtCore/QTimer>
#include <QtGui/QKeyEvent>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QMenu>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>

#include <functional>
#include <memory>

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

void sendKey(pictura::PicturaMainWindow& frame, int key, Qt::KeyboardModifiers mods,
             const QString& text)
{
    QKeyEvent event(QEvent::KeyPress, key, mods, text);
    QApplication::sendEvent(&frame, &event);
}

// The active tool's own options-bar control: each tool has its own page.
template <typename T>
T* visibleChild(pictura::PicturaMainWindow& frame, const QString& name)
{
    for (T* child : frame.findChildren<T*>(name)) {
        if (child->isVisible()) {
            return child;
        }
    }
    return nullptr;
}

// The foreground colour a check changes, restored on scope exit; the shape
// options start at their defaults and are restored likewise.
struct State {
    pictura::ToolController* tools;
    QColor foreground;
    pictura::ShapeOptions shape;

    explicit State(pictura::ToolController* t)
        : tools(t)
        , foreground(t->foreground())
        , shape(t->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->get())
    {
        tools->setForeground(Qt::red);
        // Defaults: the fill follows the foreground until a shape is mirrored.
        tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->set({});
    }
    ~State()
    {
        tools->setForeground(foreground);
        tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->set(shape);
    }
};

// Answers the next modal dialog (polled from the dialog's own event loop)
// with `answer` and counts it in `*seen`. Destroy the timer once the action
// that may open the dialog has returned.
std::unique_ptr<QTimer> answerNextModal(std::function<void(QWidget*)> answer, int* seen)
{
    auto timer = std::make_unique<QTimer>();
    timer->setInterval(5);
    QTimer* raw = timer.get();
    QObject::connect(raw, &QTimer::timeout, [raw, answer, seen]() {
        if (QWidget* modal = QApplication::activeModalWidget()) {
            raw->stop();
            ++*seen;
            answer(modal);
        }
    });
    timer->start();
    return timer;
}

void pressButton(QWidget* modal, QMessageBox::StandardButton which, bool dontShowAgain = false)
{
    auto* box = qobject_cast<QMessageBox*>(modal);
    QVERIFY(box && box->objectName() == QStringLiteral("liveShapeToPathPrompt"));
    QVERIFY(box->text().contains(QStringLiteral("live shape into a regular path")));
    if (dontShowAgain) {
        box->checkBox()->setChecked(true);
    }
    box->button(which)->click();
}

// Fill in a Create dialog's numeric fields by object name, then OK it.
void fillCreateDialog(QWidget* modal, const QList<QPair<QString, double>>& fields,
                      const QStringList& checks = {})
{
    auto* dialog = qobject_cast<QDialog*>(modal);
    QVERIFY(dialog && dialog->objectName() == QStringLiteral("createShapeDialog"));
    for (const auto& [name, value] : fields) {
        if (auto* d = dialog->findChild<QDoubleSpinBox*>(name)) {
            d->setValue(value);
        } else if (auto* i = dialog->findChild<QSpinBox*>(name)) {
            i->setValue(int(value));
        } else {
            QFAIL(qPrintable(QStringLiteral("no field ") + name));
        }
    }
    for (const QString& name : checks) {
        auto* box = dialog->findChild<QCheckBox*>(name);
        QVERIFY(box);
        box->setChecked(true);
    }
    dialog->accept();
}

QRectF subpathBounds(const Fixture& f, int sp)
{
    const ::rust::Vec<double> b = pictura::path_subpath_bounds(*f.view, sp);
    return b.size() == 4 ? QRectF(QPointF(b[0], b[1]), QPointF(b[2], b[3])) : QRectF();
}

const QRgb kRed = QColor(Qt::red).rgba();
const QRgb kWhite = QColor(Qt::white).rgba();

} // namespace

class ShapeToolsTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void shapeTools();
    void liveShapes();
    void createDialogs();
    void lineAndCustomShape();
    void optionsBar();
    void shapeLayerActions();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void ShapeToolsTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void ShapeToolsTest::shapeTools()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_shape_seed"));
    QVERIFY2(f.ok(), "shape fixture");
    const State state(f.tools);

    // U and Shift+U cycle the six tools of the group.
    frame.show();
    QVERIFY(QTest::qWaitForWindowExposed(&frame));
    frame.activateWindow();
    QCoreApplication::processEvents();
    frame.setActiveTool(pictura::ToolId::Move);
    sendKey(frame, Qt::Key_U, Qt::NoModifier, QStringLiteral("u"));
    QCOMPARE(frame.activeTool(), pictura::ToolId::Rectangle);
    const QList<pictura::ToolId> cycle = {pictura::ToolId::RoundedRectangle,
                                          pictura::ToolId::Ellipse, pictura::ToolId::Polygon,
                                          pictura::ToolId::Line, pictura::ToolId::CustomShape,
                                          pictura::ToolId::Rectangle};
    for (pictura::ToolId next : cycle) {
        sendKey(frame, Qt::Key_U, Qt::ShiftModifier, QStringLiteral("U"));
        QCOMPARE(frame.activeTool(), next);
    }
    // The tools read Shift and Alt live; a plain key clears the tracked Shift.
    sendKey(frame, Qt::Key_U, Qt::NoModifier, QStringLiteral("u"));
    QCOMPARE(frame.activeTool(), pictura::ToolId::Rectangle);

    // Pixels: a hexagon from the centre out paints the foreground onto the
    // background, adding no layer.
    frame.setActiveTool(pictura::ToolId::Polygon);
    auto* sides = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsShapeSides"));
    QVERIFY(sides != nullptr);
    emit sides->valueChanged(6);
    QCOMPARE(f.tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->get().sides, 6);
    auto* mode = visibleChild<QComboBox>(frame, QStringLiteral("optionsShapeMode"));
    QVERIFY(mode != nullptr);
    mode->setCurrentIndex(2);
    QCOMPARE(f.tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->get().mode, 2);
    const int layers = f.view->layer_count();
    int base = f.view->history_index();
    f.drag({QPointF(50, 50), QPointF(50, 30), QPointF(50, 20)});
    QVERIFY2(f.committedOnce(base, "Polygon Tool"), "polygon pixels");
    QCOMPARE(f.view->layer_count(), layers);
    QCOMPARE(f.view->sample_argb(50, 50), kRed);
    QCOMPARE(f.view->sample_argb(50, 22), kRed);
    QCOMPARE(f.view->sample_argb(50, 85), kWhite);

    // Path: an ellipse joins the Work Path as four anchors; no layer, no pixels.
    frame.setActiveTool(pictura::ToolId::Ellipse);
    mode->setCurrentIndex(1);
    for (auto* page : frame.findChildren<QComboBox*>(QStringLiteral("optionsShapeMode"))) {
        QCOMPARE(page->currentIndex(), 1);
    }
    base = f.view->history_index();
    f.drag({QPointF(60, 60), QPointF(80, 70), QPointF(90, 80)});
    QVERIFY2(f.committedOnce(base, "Ellipse Tool"), "ellipse path");
    QCOMPARE(pictura::path_subpath_count(*f.view), 1);
    QCOMPARE(pictura::path_point_count(*f.view, 0), 4);
    QVERIFY(pictura::path_subpath_closed(*f.view, 0));
    const ::rust::Vec<double> box = pictura::path_subpath_bounds(*f.view, 0);
    QCOMPARE(box.size(), std::size_t(4));
    QCOMPARE(QRectF(QPointF(box[0], box[1]), QPointF(box[2], box[3])),
             QRectF(60, 60, 30, 20));
    QCOMPARE(f.view->layer_count(), layers);
    QCOMPARE(f.view->sample_argb(75, 70), kWhite);

    // The Rounded Rectangle's Radius rounds its corners into eight anchors.
    frame.setActiveTool(pictura::ToolId::RoundedRectangle);
    auto* radius = frame.findChild<pictura::NumericField*>(QStringLiteral("optionsShapeRadius"));
    QVERIFY(radius != nullptr);
    QCOMPARE(radius->value(), 10.0);
    base = f.view->history_index();
    f.drag({QPointF(5, 60), QPointF(30, 80), QPointF(45, 95)});
    QVERIFY2(f.committedOnce(base, "Rounded Rectangle Tool"), "rounded rectangle path");
    QCOMPARE(pictura::path_point_count(*f.view, 1), 8);

    // Shape: a rectangle becomes a "Rectangle 1" fill layer cut to it. The
    // outline is previewed while dragging and cleared on release.
    frame.setActiveTool(pictura::ToolId::Rectangle);
    mode->setCurrentIndex(0);
    base = f.view->history_index();
    f.canvas->mousePressed(QPointF(10, 10), Qt::LeftButton, 0);
    f.canvas->mouseMoved(QPointF(30, 30));
    QVERIFY(f.canvas->pathOverlayHasPreviewForTest());
    f.canvas->mouseMoved(QPointF(40, 30));
    f.canvas->mouseReleased(QPointF(40, 30));
    QVERIFY(!f.canvas->pathOverlayHasPreviewForTest());
    QVERIFY2(f.committedOnce(base, "Rectangle Tool"), "rectangle shape");
    QCOMPARE(f.view->layer_count(), layers + 1);
    QCOMPARE(f.view->layer_name(layers), QStringLiteral("Rectangle 1"));
    QCOMPARE(f.view->sample_argb(25, 20), kRed);
    QCOMPARE(f.view->sample_argb(45, 20), kWhite);
    QCOMPARE(f.view->sample_argb(25, 32), kWhite);

    // The new layer's outline is drawn with its four corners, and the path
    // calls now read it; the Work Path keeps its two components. The Layers
    // panel marks the row as a shape layer.
    QCOMPARE(f.canvas->pathOverlayAnchorCountForTest(), 4);
    QCOMPARE(pictura::path_subpath_count(*f.view), 1);
    QVERIFY(pictura::path_target_is_live_shape(*f.view));
    pictura::path_set_layer_target(*f.view, false);
    QCOMPARE(pictura::path_subpath_count(*f.view), 2);
    pictura::path_set_layer_target(*f.view, true);
    auto* panel = frame.findChild<pictura::LayersPanel*>();
    QVERIFY(panel != nullptr);
    QVERIFY(panel->rowShapeForTest(f.view->active_layer_path()));
    QVERIFY(!panel->rowThumbnailForTest(f.view->active_layer_path()).isNull());

    // A click opens the Create dialog; cancelling it draws nothing. Undo
    // removes the shape layer.
    base = f.view->history_index();
    int seen = 0;
    {
        const auto cancel = answerNextModal(
            [](QWidget* modal) { qobject_cast<QDialog*>(modal)->reject(); }, &seen);
        f.drag({QPointF(70, 10)});
    }
    QCOMPARE(seen, 1);
    QCOMPARE(f.view->history_index(), base);
    QVERIFY(f.view->undo());
    QCOMPARE(f.view->layer_count(), layers);
    QCOMPARE(f.view->sample_argb(25, 20), kWhite);
}

void ShapeToolsTest::liveShapes()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_live_shape_seed"));
    QVERIFY2(f.ok(), "live shape fixture");
    const State state(f.tools);
    f.tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->set({0, 10.0, 5});

    // A Shape-mode rectangle is a live shape.
    frame.setActiveTool(pictura::ToolId::Rectangle);
    f.drag({QPointF(20, 20), QPointF(40, 40), QPointF(60, 50)});
    QVERIFY(pictura::path_target_is_live_shape(*f.view));

    // Path Selection moves it whole and it stays live.
    frame.setActiveTool(pictura::ToolId::PathSelection);
    int base = f.view->history_index();
    f.drag({QPointF(40, 35), QPointF(45, 35), QPointF(50, 40)});
    QVERIFY2(f.committedOnce(base, "Drag Path"), "move the live shape");
    QVERIFY(pictura::path_target_is_live_shape(*f.view));
    QCOMPARE(f.view->sample_argb(65, 52), kRed);
    QCOMPARE(f.view->sample_argb(80, 70), kWhite);

    // Direct Selection on a corner asks first; No leaves the shape alone.
    frame.setActiveTool(pictura::ToolId::DirectSelection);
    base = f.view->history_index();
    int seen = 0;
    {
        const auto no = answerNextModal(
            [](QWidget* m) { pressButton(m, QMessageBox::No); }, &seen);
        f.drag({QPointF(70, 55), QPointF(75, 60), QPointF(90, 80)});
    }
    QCOMPARE(seen, 1);
    QCOMPARE(f.view->history_index(), base);
    QVERIFY(pictura::path_target_is_live_shape(*f.view));
    QCOMPARE(f.view->sample_argb(80, 70), kWhite);

    // Yes turns it into a regular path. The prompt took the button release,
    // so that drag ends there; the next one reshapes it without asking.
    {
        const auto yes = answerNextModal(
            [](QWidget* m) { pressButton(m, QMessageBox::Yes); }, &seen);
        f.drag({QPointF(70, 55), QPointF(75, 60), QPointF(90, 80)});
    }
    QCOMPARE(seen, 2);
    QVERIFY(!pictura::path_target_is_live_shape(*f.view));
    {
        const auto unexpected = answerNextModal(
            [](QWidget* m) { pressButton(m, QMessageBox::No); }, &seen);
        f.drag({QPointF(70, 55), QPointF(75, 60), QPointF(90, 80)});
    }
    QCOMPARE(seen, 2);
    QVERIFY2(f.committedOnce(base, "Drag Anchor Point"), "reshape the converted shape");
    QCOMPARE(f.view->sample_argb(80, 70), kRed);

    // Undoing the reshape brings back the live shape.
    QVERIFY(f.view->undo());
    QVERIFY(pictura::path_target_is_live_shape(*f.view));
    QCOMPARE(f.view->sample_argb(80, 70), kWhite);

    // "Don't show again" is remembered: the next live shape converts and
    // reshapes in one drag, without asking.
    {
        const auto yes = answerNextModal(
            [](QWidget* m) { pressButton(m, QMessageBox::Yes, true); }, &seen);
        f.drag({QPointF(70, 55), QPointF(75, 60), QPointF(90, 80)});
    }
    QCOMPARE(seen, 3);
    QVERIFY(!pictura::loadSession().confirmLiveShapeToPath);
    frame.setActiveTool(pictura::ToolId::Ellipse);
    f.drag({QPointF(10, 60), QPointF(30, 80), QPointF(50, 90)});
    QVERIFY(pictura::path_target_is_live_shape(*f.view));
    frame.setActiveTool(pictura::ToolId::DirectSelection);
    base = f.view->history_index();
    {
        const auto unexpected = answerNextModal(
            [](QWidget* m) { pressButton(m, QMessageBox::No); }, &seen);
        f.drag({QPointF(50, 75), QPointF(55, 75), QPointF(60, 75)});
    }
    QCOMPARE(seen, 3);
    QVERIFY2(f.committedOnce(base, "Drag Anchor Point"), "reshape without asking");
    QVERIFY(!pictura::path_target_is_live_shape(*f.view));
    QCOMPARE(f.view->sample_argb(55, 75), kRed);

    pictura::SessionState session = pictura::loadSession();
    session.confirmLiveShapeToPath = true;
    QVERIFY(pictura::saveSession(session));
}

void ShapeToolsTest::createDialogs()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_create_shape_seed"));
    QVERIFY2(f.ok(), "create dialog fixture");
    const State state(f.tools);
    int seen = 0;

    // Shape mode: a 30 x 20 rectangle with its top-left corner at the click.
    f.tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->set({0, 10.0, 5});
    frame.setActiveTool(pictura::ToolId::Rectangle);
    int base = f.view->history_index();
    {
        const auto ok = answerNextModal(
            [](QWidget* m) {
                QCOMPARE(m->windowTitle(), QStringLiteral("Create Rectangle"));
                fillCreateDialog(m, {{QStringLiteral("createShapeWidth"), 30},
                                     {QStringLiteral("createShapeHeight"), 20}});
            },
            &seen);
        f.drag({QPointF(10, 10)});
    }
    QCOMPARE(seen, 1);
    QVERIFY2(f.committedOnce(base, "Rectangle Tool"), "create rectangle");
    QCOMPARE(f.view->sample_argb(35, 25), kRed);
    QCOMPARE(f.view->sample_argb(45, 25), kWhite);
    QCOMPARE(f.view->sample_argb(35, 35), kWhite);

    // Path mode from here on, so the Work Path shows what each dialog drew.
    f.tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->set({1, 10.0, 5});

    // From Center puts the ellipse's centre at the click.
    frame.setActiveTool(pictura::ToolId::Ellipse);
    {
        const auto ok = answerNextModal(
            [](QWidget* m) {
                QCOMPARE(m->windowTitle(), QStringLiteral("Create Ellipse"));
                fillCreateDialog(m,
                                 {{QStringLiteral("createShapeWidth"), 40},
                                  {QStringLiteral("createShapeHeight"), 20}},
                                 {QStringLiteral("createShapeFromCenter")});
            },
            &seen);
        f.drag({QPointF(50, 50)});
    }
    QCOMPARE(pictura::path_subpath_count(*f.view), 1);
    QCOMPARE(subpathBounds(f, 0), QRectF(30, 40, 40, 20));

    // Each corner takes its own radius: a square top-left corner is one anchor.
    frame.setActiveTool(pictura::ToolId::RoundedRectangle);
    {
        const auto ok = answerNextModal(
            [](QWidget* m) {
                QCOMPARE(m->windowTitle(), QStringLiteral("Create Rounded Rectangle"));
                fillCreateDialog(m, {{QStringLiteral("createShapeWidth"), 40},
                                     {QStringLiteral("createShapeHeight"), 40},
                                     {QStringLiteral("createShapeRadiusTopLeft"), 0},
                                     {QStringLiteral("createShapeRadiusBottomRight"), 15}});
            },
            &seen);
        f.drag({QPointF(5, 55)});
    }
    QCOMPARE(pictura::path_point_count(*f.view, 1), 7);
    QCOMPARE(subpathBounds(f, 1), QRectF(5, 55, 40, 40));

    // A five-pointed star fills its box with ten anchors.
    frame.setActiveTool(pictura::ToolId::Polygon);
    {
        const auto ok = answerNextModal(
            [](QWidget* m) {
                QCOMPARE(m->windowTitle(), QStringLiteral("Create Polygon"));
                auto* indent = m->findChild<QDoubleSpinBox*>(QStringLiteral("createShapeIndent"));
                QVERIFY(indent && !indent->isEnabled());
                fillCreateDialog(m,
                                 {{QStringLiteral("createShapeWidth"), 30},
                                  {QStringLiteral("createShapeHeight"), 30},
                                  {QStringLiteral("createShapeSides"), 5}},
                                 {QStringLiteral("createShapeStar")});
            },
            &seen);
        f.drag({QPointF(60, 10)});
    }
    QCOMPARE(seen, 4);
    QCOMPARE(pictura::path_point_count(*f.view, 2), 10);
    const QRectF star = subpathBounds(f, 2);
    QVERIFY2(qAbs(star.left() - 60) < 0.01 && qAbs(star.top() - 10) < 0.01
                 && qAbs(star.width() - 30) < 0.01 && qAbs(star.height() - 30) < 0.01,
             "the star fills its box");
}

void ShapeToolsTest::lineAndCustomShape()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_line_custom_seed"));
    QVERIFY2(f.ok(), "line and custom shape fixture");
    const State state(f.tools);
    int seen = 0;

    // Pixels: a 4 px line paints a band 4 px tall.
    frame.setActiveTool(pictura::ToolId::Line);
    auto* weight = visibleChild<pictura::NumericField>(frame, QStringLiteral("optionsShapeWeight"));
    QVERIFY(weight != nullptr);
    QCOMPARE(weight->value(), 1.0);
    emit weight->valueChanged(4);
    pictura::ShapeOptions o = f.tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->get();
    QCOMPARE(o.weight, 4.0);
    o.mode = 2;
    f.tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->set(o);
    int base = f.view->history_index();
    f.drag({QPointF(10, 50), QPointF(50, 50), QPointF(90, 50)});
    QVERIFY2(f.committedOnce(base, "Line Tool"), "line pixels");
    QCOMPARE(f.view->sample_argb(50, 49), kRed);
    QCOMPARE(f.view->sample_argb(50, 51), kRed);
    QCOMPARE(f.view->sample_argb(50, 46), kWhite);

    // The Line has no Create dialog: a click draws nothing.
    base = f.view->history_index();
    {
        const auto unexpected = answerNextModal(
            [](QWidget* modal) { qobject_cast<QDialog*>(modal)->reject(); }, &seen);
        f.drag({QPointF(30, 80)});
    }
    QCOMPARE(seen, 0);
    QCOMPARE(f.view->history_index(), base);

    // An end arrowhead, switched on in the pop-up, adds a tip: seven anchors.
    auto* end = frame.findChild<QCheckBox*>(QStringLiteral("optionsShapeArrowEnd"));
    QVERIFY(end != nullptr);
    end->setChecked(true);
    QVERIFY(f.tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->get().arrowEnd);
    o = f.tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->get();
    o.mode = 1;
    f.tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->set(o);
    f.drag({QPointF(10, 20), QPointF(50, 20), QPointF(90, 20)});
    QCOMPARE(pictura::path_subpath_count(*f.view), 1);
    QCOMPARE(pictura::path_point_count(*f.view, 0), 7);
    const QRectF line = subpathBounds(f, 0);
    QVERIFY2(qAbs(line.right() - 90) < 1e-6 && qAbs(line.height() - 20) < 1e-6,
             "the tip reaches the release point; the head is 500 % of 4 px wide");

    // The Custom Shape picker's grid lists the built-in shapes as icons.
    frame.setActiveTool(pictura::ToolId::CustomShape);
    auto* picker = visibleChild<QToolButton>(frame, QStringLiteral("optionsShapeCustom"));
    QVERIFY(picker != nullptr && !picker->icon().isNull());
    auto* grid = picker->menu()->findChild<QListWidget*>(QStringLiteral("optionsShapeCustomGrid"));
    QVERIFY(grid != nullptr);
    QCOMPARE(grid->count(), 6);
    QCOMPARE(grid->item(1)->toolTip(), QStringLiteral("Heart"));
    QVERIFY(!grid->item(1)->icon().isNull());
    grid->setCurrentRow(1);
    QCOMPARE(f.tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->get().custom, 1);

    // Shape mode: the heart lands on a "Shape 1" layer, not a live shape.
    o = f.tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->get();
    o.mode = 0;
    f.tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->set(o);
    base = f.view->history_index();
    f.drag({QPointF(20, 60), QPointF(50, 80), QPointF(80, 95)});
    QVERIFY2(f.committedOnce(base, "Shape Tool"), "custom shape layer");
    QCOMPARE(f.view->layer_name(f.view->layer_count() - 1), QStringLiteral("Shape 1"));
    QCOMPARE(f.view->sample_argb(50, 80), kRed);
    QCOMPARE(f.view->sample_argb(22, 62), kWhite);
    QVERIFY(!pictura::path_target_is_live_shape(*f.view));

    // A click opens Create Custom Shape.
    {
        const auto ok = answerNextModal(
            [](QWidget* m) {
                QCOMPARE(m->windowTitle(), QStringLiteral("Create Custom Shape"));
                fillCreateDialog(m, {{QStringLiteral("createShapeWidth"), 20},
                                     {QStringLiteral("createShapeHeight"), 20}});
            },
            &seen);
        f.drag({QPointF(5, 5)});
    }
    QCOMPARE(seen, 1);
    QCOMPARE(f.view->layer_name(f.view->layer_count() - 1), QStringLiteral("Shape 2"));
    QCOMPARE(f.view->sample_argb(15, 16), kRed);
}

void ShapeToolsTest::optionsBar()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(100, 100, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_shape_bar_seed"));
    QVERIFY2(f.ok(), "options bar fixture");
    const State state(f.tools);
    const QRgb kGreen = QColor(Qt::green).rgba();

    // The bar carries CS6's controls; the unimplemented ones are disabled.
    frame.setActiveTool(pictura::ToolId::Rectangle);
    for (const char* name : {"optionsShapeFill", "optionsShapeStroke", "optionsShapeStrokeWidth",
                             "optionsShapeStrokeType", "optionsShapeWidth", "optionsShapeHeight",
                             "optionsShapeLinkSize", "optionsShapeGeometry",
                             "optionsShapeAlignEdges"}) {
        QVERIFY2(visibleChild<QWidget>(frame, QString::fromLatin1(name)), name);
    }
    for (const char* name : {"optionsShapePathOperations", "optionsShapePathAlignment",
                             "optionsShapePathArrangement"}) {
        auto* stub = visibleChild<QToolButton>(frame, QString::fromLatin1(name));
        QVERIFY2(stub && !stub->isEnabled(), name);
    }
    auto* width = visibleChild<pictura::NumericField>(frame, QStringLiteral("optionsShapeWidth"));
    QVERIFY(!width->isEnabled());

    // A new shape takes the bar's appearance: no fill, a 2 px green inside
    // stroke, and Align Edges snapping its edges to whole pixels.
    pictura::ShapeOptions o = f.tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->get();
    o.fillEnabled = false;
    o.strokeEnabled = true;
    o.strokeColor = Qt::green;
    o.strokeWidth = 2;
    o.strokeAlign = 0;
    f.tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->set(o);
    f.drag({QPointF(10.4, 10.4), QPointF(30, 30), QPointF(40.4, 30.6)});
    QCOMPARE(f.view->sample_argb(25, 20), kWhite);
    QCOMPARE(f.view->sample_argb(10, 20), kGreen);
    QCOMPARE(f.view->sample_argb(39, 20), kGreen);
    QCOMPARE(f.view->sample_argb(8, 20), kWhite);

    // The bar mirrors the selected shape: its size, and its fill and stroke.
    QVERIFY(width->isEnabled());
    QCOMPARE(width->value(), 30.0);
    auto* height = visibleChild<pictura::NumericField>(frame, QStringLiteral("optionsShapeHeight"));
    QCOMPARE(height->value(), 21.0);
    QVERIFY(!f.tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->get().fillEnabled);

    // Restyling it: a red fill, then No Color for the stroke, one state each.
    int base = f.view->history_index();
    o = f.tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->get();
    o.fillEnabled = true;
    o.fillColor = Qt::red;
    f.tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->set(o);
    QVERIFY2(f.committedOnce(base, "Change Shape Fill"), "restyle the fill");
    QCOMPARE(f.view->sample_argb(25, 20), kRed);
    auto* stroke = visibleChild<QToolButton>(frame, QStringLiteral("optionsShapeStroke"));
    QAction* none = stroke->menu()->findChild<QAction*>(QStringLiteral("optionsShapeStrokeNone"));
    QVERIFY(none != nullptr);
    base = f.view->history_index();
    none->trigger();
    QVERIFY2(f.committedOnce(base, "Change Shape Stroke"), "remove the stroke");
    QCOMPARE(f.view->sample_argb(10, 20), kRed);

    // W / H resize it about its top-left; the link keeps its proportions.
    auto* link = visibleChild<QToolButton>(frame, QStringLiteral("optionsShapeLinkSize"));
    link->setChecked(true);
    base = f.view->history_index();
    emit width->valueCommitted(60.0);
    QVERIFY2(f.committedOnce(base, "Resize Shape"), "resize the shape");
    QCOMPARE(width->value(), 60.0);
    QCOMPARE(height->value(), 42.0);
    QCOMPARE(f.view->sample_argb(65, 48), kRed);
    QCOMPARE(f.view->sample_argb(75, 48), kWhite);

    // The geometry gear: Fixed Size from the centre places a 20 x 10 box on
    // the pointer; Square constrains a drag.
    auto* gear = visibleChild<QToolButton>(frame, QStringLiteral("optionsShapeGeometry"));
    auto* fixed = gear->menu()->findChild<QRadioButton*>(QStringLiteral("optionsShapeGeometry2"));
    QVERIFY(fixed != nullptr && fixed->text() == QStringLiteral("Fixed Size"));
    fixed->click();
    QCOMPARE(f.tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->get().geometry, 2);
    o = f.tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->get();
    o.fixedWidth = 20;
    o.fixedHeight = 10;
    o.fromCenter = true;
    o.mode = 1;
    f.tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->set(o);
    f.drag({QPointF(50, 80), QPointF(70, 85)});
    QCOMPARE(pictura::path_subpath_count(*f.view), 1);
    QCOMPARE(subpathBounds(f, 0), QRectF(60, 80, 20, 10));
    gear->menu()->findChild<QRadioButton*>(QStringLiteral("optionsShapeGeometry1"))->click();
    gear->menu()->findChild<QCheckBox*>(QStringLiteral("optionsShapeFromCenter"))->setChecked(false);
    QVERIFY(!f.tools->optionsFor<pictura::ShapeGroup>(pictura::ToolId::Rectangle)->get().fromCenter);
    f.drag({QPointF(5, 60), QPointF(15, 62), QPointF(25, 64)});
    QCOMPARE(subpathBounds(f, 1), QRectF(5, 60, 20, 20));

    // Dashed and Dotted are listed but disabled.
    auto* type = visibleChild<QToolButton>(frame, QStringLiteral("optionsShapeStrokeType"));
    auto* line = type->menu()->findChild<QComboBox*>(QStringLiteral("optionsShapeStrokeLine"));
    QVERIFY(line != nullptr);
    QCOMPARE(line->itemData(1, Qt::UserRole - 1).toInt(), 0);
}

// Copy/Paste Shape Attributes transfers a shape's fill and stroke to another
// shape layer (and refuses a non-shape); Rasterize Shape bakes the shape and
// drops its vector definition.
void ShapeToolsTest::shapeLayerActions()
{
    pictura::PicturaMainWindow& frame = *window_;
    QImage seed(32, 32, QImage::Format_RGB32);
    seed.fill(Qt::white);
    Fixture f(frame, seed, QStringLiteral("pictura_shape_actions"));
    QVERIFY2(f.ok(), "shape actions fixture");
    pictura::PictureView* view = f.view;

    pictura::ShapeSpec a{};
    a.kind = 0;
    a.boxed = true;
    a.x0 = 2;
    a.y0 = 2;
    a.x1 = 12;
    a.y1 = 12;
    a.stroke = true;
    a.stroke_color = 0xff00ff00u;
    a.stroke_width = 2;
    a.stroke_align = 0;
    const QString shapeA = pictura::shape_add_layer(*view, a, 0xffff0000u);
    QVERIFY(!shapeA.isEmpty());
    view->set_active_layer(shapeA);

    pictura::ShapeSpec b{};
    b.kind = 0;
    b.boxed = true;
    b.x0 = 16;
    b.y0 = 16;
    b.x1 = 26;
    b.y1 = 26;
    const QString shapeB = pictura::shape_add_layer(*view, b, 0xff0000ffu);
    QVERIFY(!shapeB.isEmpty());

    QVERIFY(pictura::shape_copy_attributes(*view, shapeA));
    view->set_active_layer(shapeB);
    QVERIFY(pictura::shape_paste_attributes(*view, shapeB));

    const ::rust::Vec<double> info = pictura::shape_active(*view);
    QCOMPARE(info.size(), std::size_t(8));
    QCOMPARE(info[3], double(0xffff0000u));
    QCOMPARE(info[4], 1.0);
    QCOMPARE(info[5], double(0xff00ff00u));
    QCOMPARE(info[6], 2.0);
    QCOMPARE(info[7], 0.0);

    const int plain = view->add_layer(-1);
    QVERIFY(plain >= 0);
    QVERIFY(!pictura::shape_paste_attributes(*view, QString::number(plain)));

    QVERIFY(pictura::shape_is_shape(*view, shapeB));
    QVERIFY(pictura::shape_rasterize(*view, shapeB));
    QVERIFY(!pictura::shape_is_shape(*view, shapeB));
    QVERIFY(!view->layer_is_fill_content(shapeB));
    QCOMPARE(view->sample_argb(21, 21), 0xffff0000u);
}

QTEST_MAIN(ShapeToolsTest)
#include "tst_shape_tools.moc"
