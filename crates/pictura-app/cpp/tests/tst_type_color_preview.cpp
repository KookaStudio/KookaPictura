// The Text Color picker previews live (#957): each colour the user picks shows
// on the selected type layer, or the text being typed, before the picker closes.
// OK records one state; Cancel restores the original colour with none.

#include <QtTest/QtTest>

#include <QtCore/QTimer>
#include <QtWidgets/QApplication>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QToolButton>

#include "api/transform_api.h"
#include "api/type_api.h"
#include "color_picker_dialog.h"
#include "panels/character_panel.h"
#include "selftest_paint_fixture.h"

#include "qt_test_support.h"

namespace {

using paint_fixture::Fixture;

QImage seed()
{
    QImage image(200, 160, QImage::Format_RGB32);
    image.fill(Qt::white);
    return image;
}

QString typeLayer(const Fixture& f)
{
    for (int i = 0; i < f.view->layer_count(); ++i) {
        if (pictura::type_api::readLayer(*f.view, QString::number(i)).isType) {
            return QString::number(i);
        }
    }
    return QString();
}

QColor layerColor(const Fixture& f, const QString& path)
{
    return QColor::fromRgba(pictura::type_api::readLayer(*f.view, path).spec.character.color);
}

bool inked(const Fixture& f, const QString& path, const QColor& want)
{
    const QStringList parts = pictura::transform_api::layerRect(*f.view, path).toString().split(QLatin1Char(' '));
    if (parts.size() != 4) {
        return false;
    }
    for (int y = parts[1].toInt(); y < parts[3].toInt(); ++y) {
        for (int x = parts[0].toInt(); x < parts[2].toInt(); ++x) {
            if (QColor::fromRgba(f.view->sample_argb(x, y)) == want) {
                return true;
            }
        }
    }
    return false;
}

// Runs `open`, which blocks in the modal picker, while a timer drives the
// dialog: it types `hex`, runs `whileOpen`, then accepts or rejects.
bool drivePicker(const std::function<void()>& open, const QString& hex,
                 const std::function<void()>& whileOpen, bool accept)
{
    bool driven = false;
    QTimer timer;
    timer.setInterval(10);
    QObject::connect(&timer, &QTimer::timeout, [&]() {
        pictura::ColorPickerDialog* dialog = nullptr;
        for (QWidget* widget : QApplication::topLevelWidgets()) {
            if (auto* candidate = qobject_cast<pictura::ColorPickerDialog*>(widget)) {
                dialog = candidate;
                break;
            }
        }
        if (!dialog) {
            return;
        }
        timer.stop();
        auto* field = dialog->findChild<QLineEdit*>(QStringLiteral("colorPickerHex"));
        if (field) {
            field->setText(hex);
            QMetaObject::invokeMethod(field, "textEdited", Q_ARG(QString, hex));
            driven = true;
            whileOpen();
        }
        accept ? dialog->accept() : dialog->reject();
    });
    pictura::ColorPickerDialog::setPickerPreference(0);
    timer.start();
    open();
    timer.stop();
    return driven;
}

} // namespace

class TypeColorPreviewTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void pickerReportsLiveChanges();
    void barPreviewsThenOkCommitsOnce();
    void barCancelRestores();
    void sessionPreviewsAndRestores();
    void characterPanelPreviewsAndCommits();
    void characterPanelCancelRestores();

private:
    void showFrame();
    QToolButton* barSwatch();
    // A red type layer, selected with the Type tool active, as after the
    // issue's repro steps.
    QString redLayerSelected(Fixture& f);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void TypeColorPreviewTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void TypeColorPreviewTest::showFrame()
{
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    window_->activateWindow();
    QCoreApplication::processEvents();
}

QToolButton* TypeColorPreviewTest::barSwatch()
{
    for (auto* button : window_->findChildren<QToolButton*>(QStringLiteral("optionsTypeColor"))) {
        if (button->isVisible()) {
            return button;
        }
    }
    return nullptr;
}

QString TypeColorPreviewTest::redLayerSelected(Fixture& f)
{
    window_->setActiveTool(pictura::ToolId::HorizontalType);
    f.tools->setForeground(QColor(200, 0, 0));
    pictura::TypeOptions o = f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get();
    o.size = 40.0;
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(o);
    f.drag({QPointF(10, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("HH"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    window_->setActiveTool(pictura::ToolId::Move);
    window_->setActiveTool(pictura::ToolId::HorizontalType);
    return typeLayer(f);
}

void TypeColorPreviewTest::pickerReportsLiveChanges()
{
    QList<QColor> heard;
    QColor chosen;
    QVERIFY2(drivePicker(
                 [&]() {
                     chosen = pictura::ColorPickerDialog::getColor(
                         QColor(200, 0, 0), nullptr, QStringLiteral("Text Color"), nullptr, {},
                         [&heard](const QColor& c) { heard.append(c); });
                 },
                 QStringLiteral("0000ff"), [&]() { QVERIFY(heard.contains(QColor(0, 0, 255))); }, true),
             "the picker opened");
    QCOMPARE(chosen, QColor(0, 0, 255));
    QCOMPARE(heard.last(), QColor(0, 0, 255));

    heard.clear();
    QVERIFY(drivePicker(
        [&]() {
            chosen = pictura::ColorPickerDialog::getColor(
                QColor(200, 0, 0), nullptr, QStringLiteral("Text Color"), nullptr, {},
                [&heard](const QColor& c) { heard.append(c); });
        },
        QStringLiteral("0000ff"), []() {}, false));
    QVERIFY(!chosen.isValid());
    QVERIFY2(heard.contains(QColor(0, 0, 255)), "the pick was heard");
    QCOMPARE(heard.last(), QColor(200, 0, 0));
}

void TypeColorPreviewTest::barPreviewsThenOkCommitsOnce()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_color_ok"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const QString path = redLayerSelected(f);
    QVERIFY(!path.isEmpty());
    QToolButton* swatch = barSwatch();
    QVERIFY2(swatch, "colour swatch");
    const int base = f.view->history_index();
    bool shown = false;
    bool quiet = false;

    QVERIFY(drivePicker(
        [&]() { swatch->click(); }, QStringLiteral("0000ff"),
        [&]() {
            shown = layerColor(f, path) == QColor(0, 0, 255) && inked(f, path, QColor(0, 0, 255));
            quiet = f.view->history_index() == base;
        },
        true));
    QVERIFY2(shown, "the layer shows the colour while the picker is open");
    QVERIFY2(quiet, "the preview records no state");
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "OK records one state");
    QCOMPARE(layerColor(f, path), QColor(0, 0, 255));
    QVERIFY(f.view->undo());
    QCOMPARE(layerColor(f, path), QColor(200, 0, 0));
}

void TypeColorPreviewTest::barCancelRestores()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_color_cancel"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const QString path = redLayerSelected(f);
    QVERIFY(!path.isEmpty());
    QToolButton* swatch = barSwatch();
    QVERIFY2(swatch, "colour swatch");
    const int base = f.view->history_index();
    bool shown = false;

    QVERIFY(drivePicker([&]() { swatch->click(); }, QStringLiteral("0000ff"),
                        [&]() { shown = layerColor(f, path) == QColor(0, 0, 255); }, false));
    QVERIFY2(shown, "the layer showed the colour while the picker was open");
    QCOMPARE(layerColor(f, path), QColor(200, 0, 0));
    QVERIFY(inked(f, path, QColor(200, 0, 0)));
    QCOMPARE(f.view->history_index(), base);
}

void TypeColorPreviewTest::sessionPreviewsAndRestores()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_color_session"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    window_->setActiveTool(pictura::ToolId::HorizontalType);
    f.tools->setForeground(QColor(200, 0, 0));
    f.drag({QPointF(10, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("HH"));
    QToolButton* swatch = barSwatch();
    QVERIFY2(swatch, "colour swatch");
    auto typeColor = [&f]() {
        return f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get().color;
    };
    QCOMPARE(typeColor(), QColor(200, 0, 0));
    const int base = f.view->history_index();
    bool followed = false;

    QVERIFY(drivePicker([&]() { swatch->click(); }, QStringLiteral("0000ff"),
                        [&]() { followed = typeColor() == QColor(0, 0, 255); }, false));
    QVERIFY2(followed, "the session's text colour followed the pick");
    QCOMPARE(typeColor(), QColor(200, 0, 0));
    QCOMPARE(f.view->history_index(), base);
    QTest::keyClick(f.canvas, Qt::Key_Escape);
}

void TypeColorPreviewTest::characterPanelPreviewsAndCommits()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_color_panel"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const QString path = redLayerSelected(f);
    QVERIFY(!path.isEmpty());
    window_->setActiveTool(pictura::ToolId::Move);
    auto* panel = window_->findChild<pictura::CharacterPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    panel->refresh();
    auto* swatch = panel->findChild<QToolButton*>(QStringLiteral("characterColor"));
    QVERIFY2(swatch && swatch->isEnabled(), "panel colour swatch");
    const int base = f.view->history_index();
    bool shown = false;
    bool quiet = false;

    QVERIFY(drivePicker(
        [&]() { swatch->click(); }, QStringLiteral("0000ff"),
        [&]() {
            shown = layerColor(f, path) == QColor(0, 0, 255);
            quiet = f.view->history_index() == base;
        },
        true));
    QVERIFY2(shown, "the layer shows the colour while the picker is open");
    QVERIFY2(quiet, "the preview records no state");
    QVERIFY2(f.committedOnce(base, "Edit Type Layer"), "OK records one state");
    QCOMPARE(layerColor(f, path), QColor(0, 0, 255));
}

void TypeColorPreviewTest::characterPanelCancelRestores()
{
    Fixture f(*window_, seed(), QStringLiteral("pictura_type_color_panel_cancel"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();
    const QString path = redLayerSelected(f);
    QVERIFY(!path.isEmpty());
    window_->setActiveTool(pictura::ToolId::Move);
    auto* panel = window_->findChild<pictura::CharacterPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    panel->refresh();
    auto* swatch = panel->findChild<QToolButton*>(QStringLiteral("characterColor"));
    QVERIFY2(swatch && swatch->isEnabled(), "panel colour swatch");
    const int base = f.view->history_index();
    bool shown = false;

    QVERIFY(drivePicker([&]() { swatch->click(); }, QStringLiteral("0000ff"),
                        [&]() { shown = layerColor(f, path) == QColor(0, 0, 255); }, false));
    QVERIFY2(shown, "the layer showed the colour while the picker was open");
    QCOMPARE(layerColor(f, path), QColor(200, 0, 0));
    QCOMPARE(f.view->history_index(), base);
}

QTEST_MAIN(TypeColorPreviewTest)
#include "tst_type_color_preview.moc"
