// Type menu wiring: the structural outline commands, the option rows, Paste
// Lorem Ipsum, the Language/Preview preferences, the OpenType rows, and the
// default type styles.

#include <QtTest/QtTest>

#include "api/layers_api.h"
#include "api/layers_bridge.h"
#include "api/type_api.h"
#include "command_ids.h"
#include "commands.h"
#include "font_combo.h"
#include "import_save_coordinator.h"
#include "panels/character_panel.h"
#include "panels/layers_panel.h"
#include "panels/panel_column.h"
#include "selftest_paint_fixture.h"
#include "tool_sessions.h"
#include "tools.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtGui/QAction>
#include <QtWidgets/QAbstractItemDelegate>
#include <QtWidgets/QFontComboBox>
#include <QtWidgets/QStyleOptionViewItem>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QWidget>

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

QImage seed()
{
    QImage image(360, 240, QImage::Format_RGB32);
    image.fill(Qt::white);
    return image;
}

QString addType(Fixture& f, const QString& text, double size = 40.0)
{
    pictura::TypeSetting setting{};
    setting.x = 30.0;
    setting.y = 120.0;
    setting.xx = 1.0;
    setting.yy = 1.0;
    pictura::CharacterSetting character = pictura::type_api::defaultCharacterSetting();
    character.size = size;
    const QString path = pictura::type_api::commitLayer(
        *f.view, {text, QStringLiteral("Liberation Sans"), setting, character,
                  pictura::type_api::defaultParagraphSetting()});
    f.frame.selectLayerPath(path);
    f.frame.refresh();
    return path;
}

QAction* action(Fixture& f, const QString& id)
{
    f.frame.registry()->refresh();
    return f.frame.registry()->action(id);
}

bool dispatch(Fixture& f, const QString& id)
{
    f.frame.registry()->refresh();
    return f.frame.registry()->dispatch(id);
}

QString typeId(const QString& leaf)
{
    return pictura::commandIdForPath({QStringLiteral("Type"), leaf});
}

QString typeSubId(const QString& group, const QString& leaf)
{
    return pictura::commandIdForPath(
        {QStringLiteral("Type"), group, leaf});
}

int previewRowHeight()
{
    QFontComboBox combo;
    pictura::useFastFontPreviews(&combo);
    QStyleOptionViewItem option;
    option.font = combo.font();
    return combo.itemDelegate()->sizeHint(option, combo.model()->index(0, 0)).height();
}

QString pixelLayer(Fixture& f)
{
    f.view->add_layer(-1);
    f.frame.refresh();
    const QString path = QString::number(f.view->layer_count() - 1);
    f.frame.selectLayerPath(path);
    f.frame.refresh();
    return path;
}

} // namespace

class TypeMenuTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void createWorkPath();
    void createWorkPathReplaces();
    void outlineRefusals();
    void convertToShape();
    void convertToShapeKeepsAppearance();
    void convertToShapeUndo();
    void convertToShapeRoundTrip();
    void warpedOutlines();
    void onPathOutlines();
    void areaOutlines();
    void layersRowParity();
    void sessionGate();
    void antiAlias();
    void orientation();
    void rowsDisabled();
    void rasterizeTypeLayer();
    void updateAllTextLayers();
    void languageOptions();
    void languageOptionsPersist();
    void fontPreviewSize();
    void fontPreviewNone();
    void pasteLoremSession();
    void pasteLoremLayer();
    void pasteLoremDisabled();
    void glyphsRow();
    void openTypeRow();
    void openTypeCrossSurface();
    void defaultStyleRows();
    void defaultsDisabled();
    void saveLoadDefaults();
    void defaultsPersist();
    void removedRows();

private:
    bool startTypeSession(Fixture& f);
    void endTypeSession(Fixture& f);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void TypeMenuTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
}

void TypeMenuTest::cleanup()
{
    QCoreApplication::sendPostedEvents(nullptr, QEvent::DeferredDelete);
}

bool TypeMenuTest::startTypeSession(Fixture& f)
{
    f.frame.setActiveTool(pictura::ToolId::HorizontalType);
    f.frame.refresh();
    if (!f.canvas) {
        return false;
    }
    QTest::mouseClick(f.canvas, Qt::LeftButton, Qt::NoModifier, QPoint(40, 120));
    QCoreApplication::processEvents();
    return f.tools && f.tools->textSession() && f.tools->textSession()->textActive();
}

void TypeMenuTest::endTypeSession(Fixture& f)
{
    if (f.tools && f.tools->textSession() && f.tools->textSession()->textActive()) {
        f.tools->textSession()->cancelText();
    }
    f.frame.setActiveTool(pictura::ToolId::Move);
}

void TypeMenuTest::createWorkPath()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_path"));
    QVERIFY2(f.ok(), "fixture");
    const QString path = addType(f, QStringLiteral("Path"));
    QAction* create = action(f, typeId(QStringLiteral("Create Work Path")));
    QVERIFY(create && create->isEnabled());
    const int base = f.view->history_index();
    QVERIFY(dispatch(f, typeId(QStringLiteral("Create Work Path"))));
    QCOMPARE(f.view->history_index(), base + 1);
    QVERIFY(pictura::paths_has_work_path(*f.view));
    QCOMPARE(pictura::paths_active(*f.view), -1);
    QVERIFY(pictura::type_api::readLayer(*f.view, path).isType);
}

void TypeMenuTest::createWorkPathReplaces()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_path_replace"));
    QVERIFY2(f.ok(), "fixture");
    addType(f, QStringLiteral("Path"));
    QVERIFY(dispatch(f, typeId(QStringLiteral("Create Work Path"))));
    const int base = f.view->history_index();
    QVERIFY(dispatch(f, typeId(QStringLiteral("Create Work Path"))));
    QCOMPARE(f.view->history_index(), base + 1);
    QVERIFY(dispatch(f, QString::fromLatin1(pictura::command_ids::EditUndo)));
    QCOMPARE(f.view->history_index(), base);
}

void TypeMenuTest::outlineRefusals()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_refusal"));
    QVERIFY2(f.ok(), "fixture");
    addType(f, QStringLiteral("Path"));
    pixelLayer(f);
    QAction* create = action(f, typeId(QStringLiteral("Create Work Path")));
    QAction* shape = action(f, typeId(QStringLiteral("Convert to Shape")));
    QVERIFY(create && !create->isEnabled());
    QVERIFY(shape && !shape->isEnabled());
}

void TypeMenuTest::convertToShape()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_shape"));
    QVERIFY2(f.ok(), "fixture");
    const QString path = addType(f, QStringLiteral("Shape"));
    const int base = f.view->history_index();
    QVERIFY(dispatch(f, typeId(QStringLiteral("Convert to Shape"))));
    QCOMPARE(f.view->history_index(), base + 1);
    QVERIFY(!pictura::type_api::readLayer(*f.view, path).isType);
    QVERIFY(pictura::layers_api::rowForPath(*f.view, path).shape);
}

void TypeMenuTest::convertToShapeKeepsAppearance()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_shape_keep"));
    QVERIFY2(f.ok(), "fixture");
    const QString path = addType(f, QStringLiteral("Shape"));
    const QString name = pictura::layers_api::rowForPath(*f.view, path).name;
    QVERIFY(dispatch(f, typeId(QStringLiteral("Convert to Shape"))));
    const auto row = pictura::layers_api::rowForPath(*f.view, path);
    QVERIFY(row.shape);
    QCOMPARE(row.name, name);
}

void TypeMenuTest::convertToShapeUndo()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_shape_undo"));
    QVERIFY2(f.ok(), "fixture");
    const QString path = addType(f, QStringLiteral("Shape"));
    QVERIFY(dispatch(f, typeId(QStringLiteral("Convert to Shape"))));
    QVERIFY(dispatch(f, QString::fromLatin1(pictura::command_ids::EditUndo)));
    QVERIFY(pictura::type_api::readLayer(*f.view, path).isType);
}

void TypeMenuTest::convertToShapeRoundTrip()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_shape_round"));
    QVERIFY2(f.ok(), "fixture");
    const QString path = addType(f, QStringLiteral("Shape"));
    QVERIFY(dispatch(f, typeId(QStringLiteral("Convert to Shape"))));
    const QString psd
        = QDir::temp().filePath(QStringLiteral("pictura_type_menu_shape_round.psd"));
    QFile::remove(psd);
    auto* coordinator = window_->findChild<pictura::ImportSaveCoordinator*>();
    QVERIFY(coordinator);
    QVERIFY(coordinator->saveActiveAtPath(psd));
    QVERIFY(coordinator->openDocumentAtPath(psd, false));
    coordinator->waitForOpensForTest();
    pictura::PictureView* back = window_->activeView();
    QVERIFY(back);
    const auto row = pictura::layers_api::rowForPath(*back, path);
    QVERIFY(row.shape);
    QVERIFY(!pictura::type_api::readLayer(*back, path).isType);
    window_->closeDocument(window_->activeDocumentIndex(), false);
    QFile::remove(psd);
}

void TypeMenuTest::warpedOutlines()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_warp"));
    QVERIFY2(f.ok(), "fixture");
    const QString path = addType(f, QStringLiteral("Warped"));
    auto layer = pictura::type_api::readLayer(*f.view, path);
    layer.spec.setting.warp_style = QStringLiteral("arc");
    layer.spec.setting.warp_bend = 50.0;
    QVERIFY(pictura::type_api::updateLayer(*f.view, path, layer.spec));
    QVERIFY(dispatch(f, typeId(QStringLiteral("Create Work Path"))));
}

void TypeMenuTest::onPathOutlines()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_onpath"));
    QVERIFY2(f.ok(), "fixture");
    const QString path = addType(f, QStringLiteral("On path"));
    auto layer = pictura::type_api::readLayer(*f.view, path);
    layer.spec.setting.path_kind = 1;
    layer.spec.setting.path_points
        = QStringLiteral("50 100 50 100 300 100 300 100");
    QVERIFY(pictura::type_api::updateLayer(*f.view, path, layer.spec));
    QVERIFY(dispatch(f, typeId(QStringLiteral("Create Work Path"))));
}

void TypeMenuTest::areaOutlines()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_area"));
    QVERIFY2(f.ok(), "fixture");
    const QString path = addType(f, QStringLiteral("Area"));
    auto layer = pictura::type_api::readLayer(*f.view, path);
    layer.spec.setting.path_kind = 2;
    layer.spec.setting.path_closed = true;
    layer.spec.setting.path_points = QStringLiteral(
        "50 50 50 50 300 50 300 50;300 50 300 50 300 200 300 200;"
        "300 200 300 200 50 200 50 200;50 200 50 200 50 50 50 50");
    layer.spec.setting.boxed = true;
    layer.spec.setting.box_left = 0.0;
    layer.spec.setting.box_top = 0.0;
    layer.spec.setting.box_right = 250.0;
    layer.spec.setting.box_bottom = 150.0;
    QVERIFY(pictura::type_api::updateLayer(*f.view, path, layer.spec));
    QVERIFY(dispatch(f, typeId(QStringLiteral("Create Work Path"))));
}

void TypeMenuTest::layersRowParity()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_rows"));
    QVERIFY2(f.ok(), "fixture");
    const QString path = addType(f, QStringLiteral("Rows"));
    auto* panel = window_->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel"));
    QVERIFY(panel);
    panel->refresh();
    QVERIFY(panel->rowMenuEnabledForPathForTest(path, QStringLiteral("Create Work Path")));
    QVERIFY(panel->rowMenuEnabledForPathForTest(path, QStringLiteral("Convert to Shape")));
    const QString pixel = pixelLayer(f);
    panel->refresh();
    QVERIFY(!panel->rowMenuEnabledForPathForTest(pixel, QStringLiteral("Create Work Path")));
    QVERIFY(!panel->rowMenuEnabledForPathForTest(pixel, QStringLiteral("Convert to Shape")));
}

void TypeMenuTest::sessionGate()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_session"));
    QVERIFY2(f.ok(), "fixture");
    addType(f, QStringLiteral("Session"));
    QVERIFY2(startTypeSession(f), "a type session opens");
    QVERIFY(!action(f, typeId(QStringLiteral("Create Work Path")))->isEnabled());
    QVERIFY(!action(f, typeSubId(QStringLiteral("Anti-Alias"), QStringLiteral("Crisp")))->isEnabled());
    QVERIFY(action(f, typeId(QStringLiteral("Paste Lorem Ipsum")))->isEnabled());
    endTypeSession(f);
}

void TypeMenuTest::antiAlias()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_aa"));
    QVERIFY2(f.ok(), "fixture");
    const QString path = addType(f, QStringLiteral("AA"));
    const QString id = typeSubId(QStringLiteral("Anti-Alias"), QStringLiteral("Crisp"));
    QVERIFY(action(f, id)->isEnabled());
    QVERIFY(dispatch(f, id));
    QCOMPARE(pictura::type_api::readLayer(*f.view, path).spec.character.anti_alias, 2);
    QVERIFY(dispatch(f, QString::fromLatin1(pictura::command_ids::EditUndo)));
    QCOMPARE(pictura::type_api::readLayer(*f.view, path).spec.character.anti_alias, 1);
}

void TypeMenuTest::orientation()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_orientation"));
    QVERIFY2(f.ok(), "fixture");
    const QString path = addType(f, QStringLiteral("Orientation"));
    const QString id = typeSubId(QStringLiteral("Orientation"), QStringLiteral("Vertical"));
    QVERIFY(dispatch(f, id));
    QVERIFY(pictura::type_api::readLayer(*f.view, path).spec.setting.vertical);
    QVERIFY(dispatch(f, QString::fromLatin1(pictura::command_ids::EditUndo)));
    QVERIFY(!pictura::type_api::readLayer(*f.view, path).spec.setting.vertical);
}

void TypeMenuTest::rowsDisabled()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_disabled"));
    QVERIFY2(f.ok(), "fixture");
    addType(f, QStringLiteral("Type"));
    pixelLayer(f);
    QVERIFY(!action(f, typeSubId(QStringLiteral("Anti-Alias"), QStringLiteral("Crisp")))->isEnabled());
    QVERIFY(!action(f, typeSubId(QStringLiteral("Orientation"), QStringLiteral("Vertical")))->isEnabled());
}

void TypeMenuTest::rasterizeTypeLayer()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_rasterize"));
    QVERIFY2(f.ok(), "fixture");
    const QString path = addType(f, QStringLiteral("Rasterize"));
    const int base = f.view->history_index();
    QVERIFY(dispatch(f, typeId(QStringLiteral("Rasterize Type Layer"))));
    QCOMPARE(f.view->history_index(), base + 1);
    QVERIFY(!pictura::type_api::readLayer(*f.view, path).isType);
}

void TypeMenuTest::updateAllTextLayers()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_update_all"));
    QVERIFY2(f.ok(), "fixture");
    addType(f, QStringLiteral("Update"));
    const int base = f.view->history_index();
    QVERIFY(dispatch(f, typeId(QStringLiteral("Update All Text Layers"))));
    QCOMPARE(f.view->history_index(), base + 1);
}

void TypeMenuTest::languageOptions()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_language"));
    QVERIFY2(f.ok(), "fixture");
    const QString east
        = typeSubId(QStringLiteral("Language Options"), QStringLiteral("East Asian features"));
    const QString defaults
        = typeSubId(QStringLiteral("Language Options"), QStringLiteral("Default features"));
    QVERIFY(dispatch(f, east));
    QVERIFY(action(f, east)->isChecked());
    QVERIFY(!action(f, defaults)->isChecked());
    QVERIFY(dispatch(f, defaults));
    QVERIFY(action(f, defaults)->isChecked());
    QVERIFY(!action(f, east)->isChecked());
}

void TypeMenuTest::languageOptionsPersist()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_language_saved"));
    QVERIFY2(f.ok(), "fixture");
    const QString east
        = typeSubId(QStringLiteral("Language Options"), QStringLiteral("East Asian features"));
    QVERIFY(dispatch(f, east));
    window_->saveSessionNow();
    auto next = pictura::test::makeMainWindow();
    QVERIFY(next != nullptr);
    next->registry()->refresh();
    QAction* action = next->registry()->action(east);
    QVERIFY(action && action->isChecked());
}

void TypeMenuTest::fontPreviewSize()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_preview"));
    QVERIFY2(f.ok(), "fixture");
    const QString huge = typeSubId(QStringLiteral("Font Preview Size"), QStringLiteral("Huge"));
    const QString small = typeSubId(QStringLiteral("Font Preview Size"), QStringLiteral("Small"));
    const QString none = typeSubId(QStringLiteral("Font Preview Size"), QStringLiteral("None"));
    QVERIFY(dispatch(f, huge));
    QCOMPARE(pictura::fontPreviewSize(), 5);
    QVERIFY(action(f, huge)->isChecked());
    const int hugeHeight = previewRowHeight();
    window_->saveSessionNow();
    auto next = pictura::test::makeMainWindow();
    QVERIFY(next != nullptr);
    next->registry()->refresh();
    QVERIFY(next->registry()->action(huge)->isChecked());
    QVERIFY(dispatch(f, small));
    QCOMPARE(pictura::fontPreviewSize(), 1);
    const int smallHeight = previewRowHeight();
    QVERIFY(dispatch(f, none));
    QCOMPARE(pictura::fontPreviewSize(), 0);
    QVERIFY(hugeHeight > smallHeight);
    QVERIFY(previewRowHeight() <= smallHeight);
}

void TypeMenuTest::fontPreviewNone()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_preview_none"));
    QVERIFY2(f.ok(), "fixture");
    const QString small = typeSubId(QStringLiteral("Font Preview Size"), QStringLiteral("Small"));
    const QString none = typeSubId(QStringLiteral("Font Preview Size"), QStringLiteral("None"));
    QVERIFY(dispatch(f, small));
    const int smallHeight = previewRowHeight();
    QVERIFY(dispatch(f, none));
    QCOMPARE(pictura::fontPreviewSize(), 0);
    QVERIFY(action(f, none)->isChecked());
    QVERIFY(previewRowHeight() <= smallHeight);
}

void TypeMenuTest::pasteLoremSession()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_lorem_session"));
    QVERIFY2(f.ok(), "fixture");
    addType(f, QStringLiteral("Lorem"));
    QVERIFY2(startTypeSession(f), "a type session opens");
    const int base = f.view->history_index();
    QVERIFY(dispatch(f, typeId(QStringLiteral("Paste Lorem Ipsum"))));
    QCOMPARE(f.view->history_index(), base);
    QVERIFY(f.tools->textSession()->textActive());
    endTypeSession(f);
}

void TypeMenuTest::pasteLoremLayer()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_lorem_layer"));
    QVERIFY2(f.ok(), "fixture");
    const QString path = addType(f, QStringLiteral("Hi"));
    const int base = f.view->history_index();
    QVERIFY(dispatch(f, typeId(QStringLiteral("Paste Lorem Ipsum"))));
    QCOMPARE(f.view->history_index(), base + 1);
    const QString text = pictura::type_api::readLayer(*f.view, path).spec.text;
    QVERIFY(text.startsWith(QStringLiteral("HiLorem ipsum")));
}

void TypeMenuTest::pasteLoremDisabled()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_lorem_disabled"));
    QVERIFY2(f.ok(), "fixture");
    addType(f, QStringLiteral("Type"));
    pixelLayer(f);
    QVERIFY(!action(f, typeId(QStringLiteral("Paste Lorem Ipsum")))->isEnabled());
}

void TypeMenuTest::glyphsRow()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_glyphs"));
    QVERIFY2(f.ok(), "fixture");
    pictura::PanelColumn* owner = window_->columnForPanel(QStringLiteral("glyphsPanel"));
    QVERIFY(owner);
    const QString id = typeSubId(QStringLiteral("Panels"), QStringLiteral("Glyphs"));
    QAction* glyphs = action(f, id);
    QVERIFY(glyphs && glyphs->isCheckable());
    glyphs->trigger();
    QTRY_VERIFY(owner->isPanelVisible(QStringLiteral("glyphsPanel")));
    glyphs->trigger();
    QTRY_VERIFY(!owner->isPanelVisible(QStringLiteral("glyphsPanel")));
}

void TypeMenuTest::openTypeRow()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_opentype"));
    QVERIFY2(f.ok(), "fixture");
    const QString path = addType(f, QStringLiteral("OpenType"));
    const QString id = QStringLiteral("type.openType.fractions");
    QVERIFY(dispatch(f, id));
    QVERIFY(pictura::type_api::readLayer(*f.view, path).spec.character.fractions);
    QVERIFY(dispatch(f, QString::fromLatin1(pictura::command_ids::EditUndo)));
    QVERIFY(!pictura::type_api::readLayer(*f.view, path).spec.character.fractions);
}

void TypeMenuTest::openTypeCrossSurface()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_opentype_cross"));
    QVERIFY2(f.ok(), "fixture");
    addType(f, QStringLiteral("OpenType"));
    QVERIFY(dispatch(f, QStringLiteral("type.openType.fractions")));
    auto* panel = window_->findChild<pictura::CharacterPanel*>(QStringLiteral("characterPanel"));
    QVERIFY(panel);
    panel->refresh();
    QVERIFY(panel->openTypeButtonForTest(7)->isChecked());
}

void TypeMenuTest::defaultStyleRows()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_defaults_rows"));
    QVERIFY2(f.ok(), "fixture");
    addType(f, QStringLiteral("Defaults"));
    QAction* save = action(f, typeId(QStringLiteral("Save Default Type Styles")));
    QAction* load = action(f, typeId(QStringLiteral("Load Default Type Styles")));
    QVERIFY(save && save->isEnabled());
    QVERIFY(load && !load->isEnabled());
    QVERIFY(dispatch(f, typeId(QStringLiteral("Save Default Type Styles"))));
    QVERIFY(action(f, typeId(QStringLiteral("Load Default Type Styles")))->isEnabled());
}

void TypeMenuTest::defaultsDisabled()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_defaults_disabled"));
    QVERIFY2(f.ok(), "fixture");
    addType(f, QStringLiteral("Type"));
    pixelLayer(f);
    QVERIFY(!action(f, typeId(QStringLiteral("Save Default Type Styles")))->isEnabled());
    QVERIFY(!action(f, typeId(QStringLiteral("Load Default Type Styles")))->isEnabled());
}

void TypeMenuTest::saveLoadDefaults()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_defaults_save"));
    QVERIFY2(f.ok(), "fixture");
    const QString first = addType(f, QStringLiteral("First"), 48.0);
    QVERIFY(dispatch(f, typeId(QStringLiteral("Save Default Type Styles"))));
    const QString second = addType(f, QStringLiteral("Second"), 24.0);
    QCOMPARE(pictura::type_api::readLayer(*f.view, second).spec.character.size, 24.0);
    QVERIFY(dispatch(f, typeId(QStringLiteral("Load Default Type Styles"))));
    QCOMPARE(pictura::type_api::readLayer(*f.view, second).spec.character.size, 48.0);
    QVERIFY(dispatch(f, QString::fromLatin1(pictura::command_ids::EditUndo)));
    QCOMPARE(pictura::type_api::readLayer(*f.view, second).spec.character.size, 24.0);
    QVERIFY(!first.isEmpty());
}

void TypeMenuTest::defaultsPersist()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_defaults_saved"));
    QVERIFY2(f.ok(), "fixture");
    addType(f, QStringLiteral("Defaults"));
    QVERIFY(dispatch(f, typeId(QStringLiteral("Save Default Type Styles"))));
    auto next = pictura::test::makeMainWindow();
    QVERIFY(next != nullptr);
    Fixture g(*next, seed(), QStringLiteral("pictura_type_menu_defaults_next"));
    QVERIFY(g.ok());
    addType(g, QStringLiteral("Next"));
    next->registry()->refresh();
    QAction* load = next->registry()->action(typeId(QStringLiteral("Load Default Type Styles")));
    QVERIFY(load && load->isEnabled());
}

void TypeMenuTest::removedRows()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_menu_removed"));
    QVERIFY2(f.ok(), "fixture");
    QVERIFY(window_->registry()->action(QStringLiteral("type.check.spelling")) == nullptr);
    QVERIFY(window_->registry()->action(QStringLiteral("type.find.and.replace.text")) == nullptr);
    QVERIFY(window_->registry()->action(QStringLiteral("type.extrude.to.3d")) == nullptr);
}

QTEST_MAIN(TypeMenuTest)
#include "tst_type_menu.moc"
