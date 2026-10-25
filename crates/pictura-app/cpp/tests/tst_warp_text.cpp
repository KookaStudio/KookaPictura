// Type > Warp Text (#684): the dialog and its options-bar button, live preview,
// Cancel and the one "Warp Text" state, the warp staying live through edits,
// the refusals, and the Layers panel's warped row.

#include <QtTest/QtTest>

#include "api/transform_api.h"
#include "api/type_api.h"
#include "commands.h"
#include "icons.h"
#include "options_bar.h"
#include "panels/layers_panel.h"
#include "selftest_paint_fixture.h"
#include "warp_text_dialog.h"

#include <QtWidgets/QComboBox>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>

#include <memory>

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;
using pictura::type_api::WarpSetting;

const QString kFont = QStringLiteral("Liberation Sans");

QImage seed()
{
    QImage image(360, 240, QImage::Format_RGB32);
    image.fill(Qt::white);
    return image;
}

QString warpCommand()
{
    return pictura::commandIdForPath({QStringLiteral("Type"), QStringLiteral("Warp Text…")});
}

// Commit `text` as point type at (30, 120) and make it the current layer.
QString addType(Fixture& f, const QString& text, bool fauxBold = false,
                const QString& pathPoints = QString())
{
    pictura::TypeSetting setting{};
    setting.x = 30.0;
    setting.y = 120.0;
    setting.xx = 1.0;
    setting.yy = 1.0;
    if (!pathPoints.isEmpty()) {
        setting.path_kind = 1;
        setting.path_points = pathPoints;
    }
    pictura::CharacterSetting character = pictura::type_api::defaultCharacterSetting();
    character.size = 40.0;
    character.faux_bold = fauxBold;
    const QString path = pictura::type_api::commitLayer(*f.view, {text, kFont, setting, character, pictura::type_api::defaultParagraphSetting()});
    f.frame.selectLayerPath(path);
    f.frame.refresh();
    return path;
}

QImage pixels(const Fixture& f, const QString& path)
{
    return f.view->layer_thumbnail(path.toInt(), 256);
}

pictura::WarpTextDialog* openDialog(Fixture& f)
{
    f.frame.registry()->refresh();
    QAction* action = f.frame.registry()->action(warpCommand());
    if (!action || !action->isEnabled() || !f.frame.registry()->dispatch(warpCommand())) {
        return nullptr;
    }
    return f.frame.findChild<pictura::WarpTextDialog*>(QStringLiteral("warpTextDialog"));
}

QComboBox* styleBox(pictura::WarpTextDialog* dialog)
{
    return dialog->findChild<QComboBox*>(QStringLiteral("warpTextStyle"));
}

QSpinBox* value(pictura::WarpTextDialog* dialog, const char* name)
{
    return dialog->findChild<QSpinBox*>(QString::fromLatin1(name));
}

void chooseStyle(pictura::WarpTextDialog* dialog, const QString& id)
{
    styleBox(dialog)->setCurrentIndex(styleBox(dialog)->findData(id));
}

// Close `dialog` with `accept` and let its deferred delete run.
void finish(pictura::WarpTextDialog* dialog, bool accept)
{
    if (accept) {
        dialog->accept();
    } else {
        dialog->reject();
    }
    QCoreApplication::sendPostedEvents(nullptr, QEvent::DeferredDelete);
    QCoreApplication::processEvents();
}

// Warp `path` with Arc +50 through the dialog.
bool commitArc(Fixture& f)
{
    pictura::WarpTextDialog* dialog = openDialog(f);
    if (!dialog) {
        return false;
    }
    chooseStyle(dialog, QStringLiteral("arc"));
    finish(dialog, true);
    return true;
}

} // namespace

class WarpTextTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();
    void menuOpensDialog();
    void menuDisabledForPixelLayer();
    void optionsButtonOpensDialog();
    void optionsButtonHasWarpTextIcon();
    void dialogControls();
    void presetDefaultsBend();
    void reopenShowsCurrentWarp();
    void changesPreviewLive();
    void cancelRestoresLayer();
    void okRecordsOneState();
    void noneRemovesWarp();
    void editKeepsWarp();
    void attributesAndTransformKeepWarp();
    void fauxBoldIsRefused();
    void typeOnPathIsRefused();
    void layersRowMarksWarp();
    void warpSessionCancelsOrCommitsOnce();

private:
    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void WarpTextTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
}

void WarpTextTest::cleanup()
{
    for (auto* dialog : window_->findChildren<pictura::WarpTextDialog*>()) {
        finish(dialog, false);
    }
    for (auto* box : window_->findChildren<QMessageBox*>(QStringLiteral("warpTextRefusal"))) {
        box->close();
    }
    QCoreApplication::sendPostedEvents(nullptr, QEvent::DeferredDelete);
}

void WarpTextTest::menuOpensDialog()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_warp_menu"));
    QVERIFY2(f.ok(), "warp fixture");
    const QString path = addType(f, QStringLiteral("Warp me"));
    QVERIFY(!path.isEmpty());
    pictura::WarpTextDialog* dialog = openDialog(f);
    QVERIFY2(dialog, "the command is enabled and opens the dialog");
    QVERIFY(dialog->isVisible());
    finish(dialog, false);
}

void WarpTextTest::menuDisabledForPixelLayer()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_warp_disabled"));
    QVERIFY2(f.ok(), "warp fixture");
    addType(f, QStringLiteral("Warp me"));
    window_->setActiveTool(pictura::ToolId::HorizontalType);
    window_->selectLayerPath(QStringLiteral("0"));
    window_->refresh();
    window_->registry()->refresh();
    QAction* action = window_->registry()->action(warpCommand());
    QVERIFY(action && !action->isEnabled());
    auto* bar = window_->findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    QVERIFY(bar);
    auto* button = bar->pageForTest(pictura::ToolId::HorizontalType)
                       ->findChild<QToolButton*>(QStringLiteral("optionsWarpText"));
    QVERIFY(button && !button->isEnabled());
}

void WarpTextTest::optionsButtonOpensDialog()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_warp_button"));
    QVERIFY2(f.ok(), "warp fixture");
    window_->setActiveTool(pictura::ToolId::HorizontalType);
    addType(f, QStringLiteral("Warp me"));
    auto* bar = window_->findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    QVERIFY(bar);
    auto* button = bar->pageForTest(pictura::ToolId::HorizontalType)
                       ->findChild<QToolButton*>(QStringLiteral("optionsWarpText"));
    QVERIFY(button && button->isEnabled());
    button->click();
    auto* dialog = window_->findChild<pictura::WarpTextDialog*>(QStringLiteral("warpTextDialog"));
    QVERIFY2(dialog, "the button opens the dialog");
    finish(dialog, false);
}

void WarpTextTest::optionsButtonHasWarpTextIcon()
{
    auto* bar = window_->findChild<pictura::OptionsBar*>(QStringLiteral("optionsBar"));
    QVERIFY(bar);
    auto* button = bar->pageForTest(pictura::ToolId::HorizontalType)
                       ->findChild<QToolButton*>(QStringLiteral("optionsWarpText"));
    QVERIFY(button);
    const QImage own = button->icon().pixmap(24, 24).toImage();
    const QImage grid = pictura::icon(QStringLiteral("options.warp")).pixmap(24, 24).toImage();
    QVERIFY2(!own.isNull(), "the Warp Text icon loads");
    QVERIFY2(own != grid, "not Free Transform's warp grid");
}

void WarpTextTest::dialogControls()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_warp_controls"));
    QVERIFY2(f.ok(), "warp fixture");
    addType(f, QStringLiteral("Warp me"));
    pictura::WarpTextDialog* dialog = openDialog(f);
    QVERIFY(dialog);
    const QStringList expected = {
        QStringLiteral("None"),        QStringLiteral("Arc"),        QStringLiteral("Arc Lower"),
        QStringLiteral("Arc Upper"),   QStringLiteral("Arch"),       QStringLiteral("Bulge"),
        QStringLiteral("Shell Lower"), QStringLiteral("Shell Upper"), QStringLiteral("Flag"),
        QStringLiteral("Wave"),        QStringLiteral("Fish"),       QStringLiteral("Rise"),
        QStringLiteral("Fisheye"),     QStringLiteral("Inflate"),    QStringLiteral("Squeeze"),
        QStringLiteral("Twist")};
    QComboBox* style = styleBox(dialog);
    QStringList items;
    for (int i = 0; i < style->count(); ++i) {
        items << style->itemText(i);
    }
    QCOMPARE(items, expected);
    QCOMPARE(style->currentIndex(), 0);
    for (const char* name :
         {"warpTextBend", "warpTextHorizontalDistortion", "warpTextVerticalDistortion"}) {
        QSpinBox* spin = value(dialog, name);
        QVERIFY(spin);
        QCOMPARE(spin->minimum(), -100);
        QCOMPARE(spin->maximum(), 100);
        QVERIFY(!spin->isEnabled());
    }
    QVERIFY(!dialog->findChild<QRadioButton*>(QStringLiteral("warpTextVertical"))->isEnabled());
    finish(dialog, false);
}

void WarpTextTest::presetDefaultsBend()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_warp_bend"));
    QVERIFY2(f.ok(), "warp fixture");
    addType(f, QStringLiteral("Warp me"));
    pictura::WarpTextDialog* dialog = openDialog(f);
    QVERIFY(dialog);
    chooseStyle(dialog, QStringLiteral("arc"));
    QCOMPARE(value(dialog, "warpTextBend")->value(), 50);
    QVERIFY(value(dialog, "warpTextBend")->isEnabled());
    QVERIFY(dialog->findChild<QRadioButton*>(QStringLiteral("warpTextVertical"))->isEnabled());
    finish(dialog, false);
}

void WarpTextTest::reopenShowsCurrentWarp()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_warp_reopen"));
    QVERIFY2(f.ok(), "warp fixture");
    addType(f, QStringLiteral("Warp me"));
    pictura::WarpTextDialog* dialog = openDialog(f);
    QVERIFY(dialog);
    chooseStyle(dialog, QStringLiteral("wave"));
    dialog->findChild<QRadioButton*>(QStringLiteral("warpTextVertical"))->click();
    value(dialog, "warpTextBend")->setValue(-30);
    value(dialog, "warpTextHorizontalDistortion")->setValue(10);
    value(dialog, "warpTextVerticalDistortion")->setValue(-20);
    finish(dialog, true);

    dialog = openDialog(f);
    QVERIFY(dialog);
    QCOMPARE(styleBox(dialog)->currentData().toString(), QStringLiteral("wave"));
    QVERIFY(dialog->findChild<QRadioButton*>(QStringLiteral("warpTextVertical"))->isChecked());
    QCOMPARE(value(dialog, "warpTextBend")->value(), -30);
    QCOMPARE(value(dialog, "warpTextHorizontalDistortion")->value(), 10);
    QCOMPARE(value(dialog, "warpTextVerticalDistortion")->value(), -20);
    finish(dialog, false);
}

void WarpTextTest::changesPreviewLive()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_warp_preview"));
    QVERIFY2(f.ok(), "warp fixture");
    const QString path = addType(f, QStringLiteral("Warp me"));
    const QImage before = pixels(f, path);
    const int base = f.view->history_index();
    pictura::WarpTextDialog* dialog = openDialog(f);
    QVERIFY(dialog);
    chooseStyle(dialog, QStringLiteral("arc"));
    QVERIFY2(pixels(f, path) != before, "the canvas previews the warp");
    QCOMPARE(f.view->history_index(), base);
    finish(dialog, false);
}

void WarpTextTest::cancelRestoresLayer()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_warp_cancel"));
    QVERIFY2(f.ok(), "warp fixture");
    const QString path = addType(f, QStringLiteral("Warp me"));
    const QImage before = pixels(f, path);
    const int base = f.view->history_index();
    pictura::WarpTextDialog* dialog = openDialog(f);
    QVERIFY(dialog);
    chooseStyle(dialog, QStringLiteral("arc"));
    finish(dialog, false);
    QCOMPARE(pixels(f, path), before);
    QVERIFY(pictura::type_api::layerWarp(*f.view, path).style.isEmpty());
    QCOMPARE(f.view->history_index(), base);
}

void WarpTextTest::okRecordsOneState()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_warp_ok"));
    QVERIFY2(f.ok(), "warp fixture");
    const QString path = addType(f, QStringLiteral("Warp me"));
    const QImage before = pixels(f, path);
    const int base = f.view->history_index();
    QVERIFY(commitArc(f));
    QVERIFY2(f.committedOnce(base, "Warp Text"), "one Warp Text state");
    const WarpSetting warp = pictura::type_api::layerWarp(*f.view, path);
    QCOMPARE(warp.style, QStringLiteral("arc"));
    QCOMPARE(warp.bend, 50.0);
    QVERIFY(f.view->undo());
    QCOMPARE(pixels(f, path), before);
}

void WarpTextTest::noneRemovesWarp()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_warp_none"));
    QVERIFY2(f.ok(), "warp fixture");
    const QString path = addType(f, QStringLiteral("Warp me"));
    const QImage plain = pixels(f, path);
    QVERIFY(commitArc(f));
    QVERIFY(pixels(f, path) != plain);
    pictura::WarpTextDialog* dialog = openDialog(f);
    QVERIFY(dialog);
    chooseStyle(dialog, QStringLiteral("none"));
    finish(dialog, true);
    QVERIFY(pictura::type_api::layerWarp(*f.view, path).style.isEmpty());
    QCOMPARE(pixels(f, path), plain);
}

void WarpTextTest::editKeepsWarp()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_warp_edit"));
    QVERIFY2(f.ok(), "warp fixture");
    const QString path = addType(f, QStringLiteral("Warp me"));
    QVERIFY(commitArc(f));

    // The setting the Type tool previews from carries the warp.
    const pictura::type_api::LayerType layer = pictura::type_api::readLayer(*f.view, path);
    pictura::TypeSetting flat = layer.spec.setting;
    flat.warp_style = QString();
    QVERIFY2(pictura::type_api::previewRgba({layer.spec.text, kFont, layer.spec.setting, layer.spec.character, layer.spec.paragraph})
                 != pictura::type_api::previewRgba({layer.spec.text, kFont, flat, layer.spec.character, layer.spec.paragraph}),
             "the edit preview is warped");

    // Retype it with the Type tool.
    window_->setActiveTool(pictura::ToolId::HorizontalType);
    const QStringList rect
        = pictura::transform_api::layerRect(*f.view, path).toString().split(QLatin1Char(' '));
    QCOMPARE(rect.size(), 4);
    f.drag({QPointF((rect[0].toDouble() + rect[2].toDouble()) / 2.0,
                    (rect[1].toDouble() + rect[3].toDouble()) / 2.0)});
    QVERIFY2(f.tools->textSession() && f.tools->textSession()->textActive(),
             "a click on the warped type reopens it");
    QTest::keyClick(f.canvas, Qt::Key_End);
    QTest::keyClicks(f.canvas, QStringLiteral("!"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    window_->setActiveTool(pictura::ToolId::Move);
    QCOMPARE(pictura::type_api::readLayer(*f.view, path).spec.text, QStringLiteral("Warp me!"));
    const WarpSetting warp = pictura::type_api::layerWarp(*f.view, path);
    QCOMPARE(warp.style, QStringLiteral("arc"));
    QCOMPARE(warp.bend, 50.0);
}

void WarpTextTest::attributesAndTransformKeepWarp()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_warp_attrs"));
    QVERIFY2(f.ok(), "warp fixture");
    const QString path = addType(f, QStringLiteral("Warp me"));
    QVERIFY(commitArc(f));
    const auto keeps = [&]() {
        const WarpSetting warp = pictura::type_api::layerWarp(*f.view, path);
        return warp.style == QStringLiteral("arc") && warp.bend == 50.0;
    };

    // The Character panel's write: the layer's own setting with a new size.
    pictura::type_api::LayerType layer = pictura::type_api::readLayer(*f.view, path);
    layer.spec.character.size = 52.0;
    QVERIFY(pictura::type_api::updateLayer(*f.view, path, {layer.spec.text, kFont, layer.spec.setting, layer.spec.character, layer.spec.paragraph}));
    QVERIFY2(keeps(), "a size change keeps the warp");

    // Free Transform: stretch the right edge out and commit.
    const QStringList parts
        = pictura::transform_api::layerRect(*f.view, path).toString().split(QLatin1Char(' '));
    QCOMPARE(parts.size(), 4);
    const double right = parts[2].toDouble();
    const double y = (parts[1].toDouble() + parts[3].toDouble()) / 2.0;
    QVERIFY(f.view->begin_free_transform(path));
    QVERIFY(f.view->transform_press(right, y, 1.0, false, false) >= 0);
    f.view->transform_move(right + 60.0, y, 1.0, false, false);
    f.view->transform_release();
    QVERIFY(f.view->commit_transform());
    QVERIFY(pictura::type_api::readLayer(*f.view, path).isType);
    QVERIFY2(keeps(), "Free Transform keeps the warp");
}

void WarpTextTest::fauxBoldIsRefused()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_warp_faux_bold"));
    QVERIFY2(f.ok(), "warp fixture");
    addType(f, QStringLiteral("Bold"), true);
    const int base = f.view->history_index();
    QVERIFY(window_->registry()->dispatch(warpCommand()));
    auto* box = window_->findChild<QMessageBox*>(QStringLiteral("warpTextRefusal"));
    QVERIFY2(box, "a warning is shown");
    QVERIFY2(box->text().contains(QStringLiteral("faux bold")), qPrintable(box->text()));
    QVERIFY(!window_->findChild<pictura::WarpTextDialog*>(QStringLiteral("warpTextDialog")));
    QCOMPARE(f.view->history_index(), base);
}

void WarpTextTest::typeOnPathIsRefused()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_warp_on_path"));
    QVERIFY2(f.ok(), "warp fixture");
    const QString path
        = addType(f, QStringLiteral("On a path"), false,
                  QStringLiteral("20 150 110 150 220 150 330 150"));
    QVERIFY(!path.isEmpty());
    const int base = f.view->history_index();
    QVERIFY(window_->registry()->dispatch(warpCommand()));
    auto* box = window_->findChild<QMessageBox*>(QStringLiteral("warpTextRefusal"));
    QVERIFY2(box, "a warning is shown");
    QVERIFY2(box->text().contains(QStringLiteral("on a path")), qPrintable(box->text()));
    QVERIFY(!window_->findChild<pictura::WarpTextDialog*>(QStringLiteral("warpTextDialog")));
    QCOMPARE(f.view->history_index(), base);
}

void WarpTextTest::layersRowMarksWarp()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_warp_row"));
    QVERIFY2(f.ok(), "warp fixture");
    const QString path = addType(f, QStringLiteral("Warp me"));
    pictura::LayersPanel* panel = window_->layersPanelForTest();
    QVERIFY(panel);
    QVERIFY(!panel->rowWarpedForTest(path));
    QVERIFY(commitArc(f));
    QTRY_VERIFY(panel->rowWarpedForTest(path));
    pictura::WarpTextDialog* dialog = openDialog(f);
    QVERIFY(dialog);
    chooseStyle(dialog, QStringLiteral("none"));
    finish(dialog, true);
    QTRY_VERIFY(!panel->rowWarpedForTest(path));
}

// The adapter's warp session (#403): previews record nothing, a dropped session
// puts the layer back, and a commit records exactly one "Warp Text" state.
void WarpTextTest::warpSessionCancelsOrCommitsOnce()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_warp_session"));
    QVERIFY2(f.ok(), "warp fixture");
    const QString path = addType(f, QStringLiteral("Warp me"));
    const auto plain = pixels(f, path);
    const int base = f.view->history_index();
    WarpSetting arc;
    arc.style = QStringLiteral("arc");
    arc.bend = 50.0;
    {
        pictura::type_api::WarpSession session(*f.view, path);
        QVERIFY(session.preview(arc));
        QVERIFY(pixels(f, path) != plain);
        QCOMPARE(f.view->history_index(), base);
    }
    QCOMPARE(pixels(f, path), plain);
    QVERIFY(pictura::type_api::layerWarp(*f.view, path).style.isEmpty());
    QCOMPARE(f.view->history_index(), base);

    {
        pictura::type_api::WarpSession session(*f.view, path);
        QVERIFY(session.preview(arc));
        QVERIFY(session.commit(arc));
        QVERIFY(!session.isOpen());
        QVERIFY(!session.cancel());
    }
    QVERIFY2(f.committedOnce(base, "Warp Text"), "one Warp Text state");
    QCOMPARE(pictura::type_api::layerWarp(*f.view, path).style, QStringLiteral("arc"));
}

QTEST_MAIN(WarpTextTest)
#include "tst_warp_text.moc"
