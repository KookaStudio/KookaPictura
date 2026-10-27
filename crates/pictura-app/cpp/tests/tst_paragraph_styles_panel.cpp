// The Paragraph Styles panel (#76): lists the document's named paragraph
// styles, applies one to the active type layer, creates/edits/deletes through
// the style sheet, and opens from the Window / Type menu paths.

#include <QtTest/QtTest>

#include "commands.h"
#include "font_combo.h"
#include "frame.h"
#include "panels/paragraph_styles_panel.h"
#include "paragraph_style_dialog.h"
#include "selftest_paint_fixture.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "api/type_api.h"

#include <algorithm>
#include <cstdlib>

#include <QtCore/QElapsedTimer>
#include <QtGui/QAction>
#include <QtGui/QFontDatabase>
#include <QtGui/QRawFont>
#include <QtWidgets/QAbstractItemView>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFontComboBox>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>

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

} // namespace

class ParagraphStylesPanelTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void listsAndApplies();
    void createAndDelete();
    void menusTogglePanel();
    void dialogShowsEveryPage();
    void dialogRoundTripsAttributes();
    void dialogPreviewAppliesAndCancelRestores();
    void dialogFontFamilyPopup();
    void fontRowsNameFamiliesInTheUiFont();
    void fontPopupPaintsPromptlyAcrossTheList();
    void doubleClickOpensDialog();
    void fontHoverDoesNotPreview();
    void styleLibraryReadsInOneCall();

private:
    void showFrame();
    QString makeTypeLayer(Fixture& f);

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void ParagraphStylesPanelTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
}

void ParagraphStylesPanelTest::showFrame()
{
    window_->show();
    QVERIFY(QTest::qWaitForWindowExposed(window_.get()));
    window_->activateWindow();
    QCoreApplication::processEvents();
}

QString ParagraphStylesPanelTest::makeTypeLayer(Fixture& f)
{
    window_->setActiveTool(pictura::ToolId::HorizontalType);
    pictura::TypeOptions o = f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->get();
    o.size = 30.0;
    f.tools->optionsFor<pictura::TypeGroup>(pictura::ToolId::HorizontalType)->set(o);
    f.drag({QPointF(10, 60)});
    QTest::keyClicks(f.canvas, QStringLiteral("Hello"));
    QTest::keyClick(f.canvas, Qt::Key_Return, Qt::ControlModifier);
    return typeLayer(f);
}

void ParagraphStylesPanelTest::listsAndApplies()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_para_styles_list"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();

    const QString path = makeTypeLayer(f);
    QVERIFY2(!path.isEmpty(), "a type layer");

    auto* panel = frame.findChild<pictura::ParagraphStylesPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);

    // A fresh document carries exactly the default Basic Paragraph style.
    QCOMPARE(panel->listForTest()->count(), 1);
    QCOMPARE(panel->listForTest()->item(0)->text(), QStringLiteral("Basic Paragraph"));

    // Add a second style through the sheet, then the panel lists both.
    pictura::CharacterSetting c = pictura::type_api::defaultCharacterSetting();
    c.size = 48.0;
    const pictura::ParagraphSetting p = pictura::type_api::defaultParagraphSetting();
    QVERIFY(pictura::type_api::createParagraphStyle(*f.view, QStringLiteral("Heading"), QString(),
                                                 c, p));
    panel->refresh();
    QCOMPARE(panel->listForTest()->count(), 2);
    QCOMPARE(panel->listForTest()->item(1)->text(), QStringLiteral("Heading"));
    QCOMPARE(pictura::type_api::findStyle(pictura::type_api::paragraphStyles(*f.view), QStringLiteral("Heading")).character.size,
             48.0);

    // Applying the style sets the layer and records one state.
    const int base = f.view->history_index();
    panel->applyRowForTest(1);
    QVERIFY2(f.committedOnce(base, "Apply Type Style"), "one apply state");
    QCOMPARE(pictura::type_api::readLayer(*f.view, path).spec.character.size, 48.0);
}

void ParagraphStylesPanelTest::createAndDelete()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_para_styles_admin"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();

    auto* panel = frame.findChild<pictura::ParagraphStylesPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    QCOMPARE(panel->listForTest()->count(), 1);

    // The default style is protected: deleting the first row does nothing.
    panel->listForTest()->setCurrentRow(0);
    QVERIFY(!panel->deleteSelectedStyle());
    QCOMPARE(panel->listForTest()->count(), 1);

    // A second style deletes, recording one state, and drops back to the
    // default.
    pictura::CharacterSetting c = pictura::type_api::defaultCharacterSetting();
    const pictura::ParagraphSetting p = pictura::type_api::defaultParagraphSetting();
    QVERIFY(pictura::type_api::createParagraphStyle(*f.view, QStringLiteral("Body"), QString(), c,
                                                 p));
    panel->refresh();
    QCOMPARE(panel->listForTest()->count(), 2);

    panel->listForTest()->setCurrentRow(1);
    const int base = f.view->history_index();
    QVERIFY(panel->deleteSelectedStyle());
    QVERIFY2(f.committedOnce(base, "Delete Paragraph Style"), "one delete state");
    QCOMPARE(panel->listForTest()->count(), 1);
}

void ParagraphStylesPanelTest::dialogShowsEveryPage()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_para_styles_pages"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();

    auto* panel = frame.findChild<pictura::ParagraphStylesPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);

    const pictura::CharacterSetting c = pictura::type_api::defaultCharacterSetting();
    const pictura::ParagraphSetting p = pictura::type_api::defaultParagraphSetting();
    QVERIFY(pictura::type_api::createParagraphStyle(*f.view, QStringLiteral("Heading"), QString(), c,
                                                 p));
    panel->refresh();

    bool inspected = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = frame.findChild<pictura::ParagraphStyleDialog*>();
        QVERIFY(dialog);
        if (!dialog) {
            return;
        }
        QCOMPARE(dialog->pageListForTest()->count(), 7);
        QCOMPARE(dialog->pageListForTest()->item(0)->text(),
                 QStringLiteral("Basic Character Formats"));
        QCOMPARE(dialog->pageListForTest()->item(2)->text(),
                 QStringLiteral("OpenType Features"));
        QCOMPARE(dialog->pageListForTest()->item(3)->text(),
                 QStringLiteral("Indents and Spacing"));
        QCOMPARE(dialog->pageListForTest()->item(4)->text(), QStringLiteral("Composition"));
        QCOMPARE(dialog->pageListForTest()->item(5)->text(), QStringLiteral("Justification"));
        QCOMPARE(dialog->pageListForTest()->item(6)->text(), QStringLiteral("Hyphenation"));
        QVERIFY(dialog->previewAvailableForTest());
        QVERIFY(dialog->previewForTest()->isChecked());
        inspected = true;
        dialog->reject();
    });
    QVERIFY(!panel->editStyle(1));
    QVERIFY(inspected);
}

void ParagraphStylesPanelTest::dialogRoundTripsAttributes()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_para_styles_roundtrip"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();

    auto* panel = frame.findChild<pictura::ParagraphStylesPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);

    const pictura::CharacterSetting c = pictura::type_api::defaultCharacterSetting();
    const pictura::ParagraphSetting p = pictura::type_api::defaultParagraphSetting();
    QVERIFY(pictura::type_api::createParagraphStyle(*f.view, QStringLiteral("Heading"),
                                                 QStringLiteral("Liberation Sans"), c, p));
    panel->refresh();

    const int base = f.view->history_index();
    bool edited = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = frame.findChild<pictura::ParagraphStyleDialog*>();
        QVERIFY(dialog);
        if (!dialog) {
            return;
        }
        dialog->findChild<QCheckBox*>(QStringLiteral("paragraphStyleFauxBold"))->setChecked(true);
        dialog->findChild<QCheckBox*>(QStringLiteral("paragraphStyleStandardLigatures"))
            ->setChecked(false);
        dialog->findChild<QCheckBox*>(QStringLiteral("paragraphStyleFractions"))->setChecked(true);
        dialog->findChild<QComboBox*>(QStringLiteral("paragraphStyleLanguage"))
            ->setCurrentText(QStringLiteral("French"));
        dialog->findChild<QDoubleSpinBox*>(QStringLiteral("paragraphStyleAutoLeading"))
            ->setValue(150.0);
        dialog->findChild<QSpinBox*>(QStringLiteral("paragraphStyleHyphenLimit"))->setValue(3);
        dialog->findChild<QCheckBox*>(QStringLiteral("paragraphStyleHyphenate"))->setChecked(true);
        dialog->findChild<QCheckBox*>(QStringLiteral("paragraphStyleHyphenateCaps"))
            ->setChecked(false);
        auto* alignment = dialog->findChild<QComboBox*>(QStringLiteral("paragraphStyleAlignment"));
        alignment->setCurrentIndex(alignment->findData(2));
        edited = true;
        dialog->accept();
    });
    QVERIFY(panel->editStyle(1));
    QVERIFY(edited);
    QVERIFY2(f.committedOnce(base, "Edit Paragraph Style"), "one edit state");

    const pictura::CharacterSetting character =
        pictura::type_api::findStyle(pictura::type_api::paragraphStyles(*f.view), QStringLiteral("Heading")).character;
    QVERIFY(character.faux_bold);
    QVERIFY(!character.standard_ligatures);
    QVERIFY(character.fractions);
    QCOMPARE(character.language, QStringLiteral("French"));
    const pictura::ParagraphSetting paragraph =
        pictura::type_api::findStyle(pictura::type_api::paragraphStyles(*f.view), QStringLiteral("Heading")).paragraph;
    QCOMPARE(paragraph.auto_leading, 150.0);
    QCOMPARE(paragraph.hyphen_limit, 3);
    QVERIFY(paragraph.hyphenate);
    QVERIFY(!paragraph.hyphenate_caps);
    QCOMPARE(paragraph.justify, 2);
}

void ParagraphStylesPanelTest::dialogPreviewAppliesAndCancelRestores()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_para_styles_preview"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();

    auto* panel = frame.findChild<pictura::ParagraphStylesPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);

    pictura::CharacterSetting c = pictura::type_api::defaultCharacterSetting();
    c.size = 48.0;
    const pictura::ParagraphSetting p = pictura::type_api::defaultParagraphSetting();
    QVERIFY(pictura::type_api::createParagraphStyle(*f.view, QStringLiteral("Body"),
                                                 QStringLiteral("Liberation Sans"), c, p));
    panel->refresh();

    const int base = f.view->history_index();
    bool previewed = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = frame.findChild<pictura::ParagraphStyleDialog*>();
        QVERIFY(dialog);
        if (!dialog) {
            return;
        }
        dialog->findChild<QDoubleSpinBox*>(QStringLiteral("paragraphStyleSize"))
            ->setValue(72.0);
        // Preview is checked by default, so the change re-applies at once and
        // records nothing.
        previewed = pictura::type_api::findStyle(pictura::type_api::paragraphStyles(*f.view), QStringLiteral("Body")).character.size
            == 72.0;
        dialog->reject();
    });
    QVERIFY(!panel->editStyle(1));
    QVERIFY(previewed);
    QCOMPARE(pictura::type_api::findStyle(pictura::type_api::paragraphStyles(*f.view), QStringLiteral("Body")).character.size, 48.0);
    QCOMPARE(f.view->history_index(), base);
}

void ParagraphStylesPanelTest::dialogFontFamilyPopup()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_para_styles_fontpopup"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();

    auto* panel = frame.findChild<pictura::ParagraphStylesPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);

    const pictura::CharacterSetting c = pictura::type_api::defaultCharacterSetting();
    const pictura::ParagraphSetting p = pictura::type_api::defaultParagraphSetting();
    QVERIFY(pictura::type_api::createParagraphStyle(*f.view, QStringLiteral("Heading"), QString(), c,
                                                 p));
    panel->refresh();

    bool stayedOpen = false;
    qint64 colorRenderMs = -1;
    QString colorFace;
    QTimer::singleShot(0, [&] {
        auto* dialog = frame.findChild<pictura::ParagraphStyleDialog*>();
        QVERIFY(dialog);
        if (!dialog) {
            return;
        }
        auto* family =
            dialog->findChild<QFontComboBox*>(QStringLiteral("paragraphStyleFamily"));
        auto* barFamily = frame.findChild<QFontComboBox*>(QStringLiteral("optionsTypeFamily"));
        QVERIFY(family);
        QVERIFY(barFamily);
        if (!family || !barFamily) {
            return;
        }
        // The dropdown is the Type options bar's font combo: the same family
        // list and previews, and its popup must stay open after a click.
        QCOMPARE(family->count(), barFamily->count());
        QVERIFY(family->isEditable());
        // An editable combo opens from its arrow, not its text field.
        QTest::mouseClick(family, Qt::LeftButton, Qt::NoModifier,
                          QPoint(family->width() - 6, family->height() / 2));
        QTest::qWait(200);
        stayedOpen = family->view()->isVisible();
        family->hidePopup();

        // A list scrolled onto a color face must render at the speed of any
        // other: previewing one took ~400 ms per repaint and left the popup
        // blank for seconds. A color face with Latin letters (a COLR display
        // face) is preferred: the emoji-only rule missed those.
        QString color;
        for (int i = 0; i < family->count(); ++i) {
            const QString name = family->itemText(i);
            if (!pictura::isColorFont(name)) {
                continue;
            }
            const bool latin = QRawFont::fromFont(QFont(name)).supportsCharacter(QLatin1Char('A'));
            if (color.isEmpty() || latin) {
                color = name;
                colorFace = name;
            }
            if (latin) {
                break;
            }
        }
        if (!color.isEmpty()) {
            family->setCurrentFont(QFont(color));
            family->showPopup();
            QElapsedTimer timer;
            timer.start();
            family->view()->window()->grab();
            colorRenderMs = timer.elapsed();
            family->hidePopup();
        }
        dialog->reject();
    });
    QVERIFY(!panel->editStyle(1));
    QVERIFY2(stayedOpen, "the font family popup closed itself");
    if (colorRenderMs < 0) {
        QSKIP("no color font installed");
    }
    QVERIFY2(colorRenderMs < 150,
             qPrintable(QStringLiteral("%1 row render took %2 ms").arg(colorFace).arg(colorRenderMs)));
}

// A family's name is drawn in the UI font, as in CS6's font menu: in the open
// popup, the lightest installed face (a Thin or ExtraLight weight) still reads
// at full contrast.
void ParagraphStylesPanelTest::fontRowsNameFamiliesInTheUiFont()
{
    showFrame();
    auto* family = window_->findChild<QFontComboBox*>(QStringLiteral("optionsTypeFamily"));
    QVERIFY(family);
    QString lightest;
    int lightestWeight = QFont::Normal;
    for (int i = 0; i < family->count(); ++i) {
        const QString name = family->itemText(i);
        const QStringList styles = QFontDatabase::styles(name);
        const int weight = styles.isEmpty() ? QFont::Normal
                                            : QFontDatabase::weight(name, styles.first());
        if (weight < lightestWeight) {
            lightest = name;
            lightestWeight = weight;
        }
    }
    if (lightest.isEmpty()) {
        QSKIP("no face lighter than Normal installed");
    }
    const int row = family->findText(lightest);
    family->setCurrentIndex(row);
    family->showPopup();
    QAbstractItemView* view = family->view();
    const QRect rect = view->visualRect(family->model()->index(row, 0));
    const QImage shot = view->viewport()->grab().toImage();
    family->hidePopup();
    QVERIFY(!rect.isEmpty());

    // The current row is selected: the name is highlighted text on highlight.
    const int background = qGray(view->palette().highlight().color().rgb());
    const int nameEnd = rect.left() + 22 + QFontMetrics(view->font()).horizontalAdvance(lightest);
    int contrast = 0;
    for (int y = rect.top(); y <= rect.bottom(); ++y) {
        for (int x = rect.left() + 20; x < std::min(nameEnd, shot.width()); ++x) {
            contrast = std::max(contrast, std::abs(qGray(shot.pixel(x, y)) - background));
        }
    }
    QVERIFY2(contrast > 120, qPrintable(QStringLiteral("%1 (weight %2) name is faint: %3")
                                            .arg(lightest)
                                            .arg(lightestWeight)
                                            .arg(contrast)));
}

// The popup paints its rows promptly wherever the list opens: a slow first
// paint shows the popup's frame with no rows.
void ParagraphStylesPanelTest::fontPopupPaintsPromptlyAcrossTheList()
{
    showFrame();
    auto* family = window_->findChild<QFontComboBox*>(QStringLiteral("optionsTypeFamily"));
    QVERIFY(family);
    const int saved = family->currentIndex();
    qint64 worst = 0;
    QString worstAt;
    const int step = std::max(1, family->maxVisibleItems());
    for (int i = 0; i < family->count(); i += step) {
        family->setCurrentIndex(i);
        family->showPopup();
        QElapsedTimer timer;
        timer.start();
        family->view()->window()->grab();
        if (timer.elapsed() > worst) {
            worst = timer.elapsed();
            worstAt = family->itemText(i);
        }
        family->hidePopup();
    }
    family->setCurrentIndex(saved);
    qInfo("worst popup paint %lld ms at %s (%d families)", worst, qPrintable(worstAt),
          family->count());
    QVERIFY2(worst < 250, qPrintable(QStringLiteral("popup at %1 took %2 ms")
                                         .arg(worstAt)
                                         .arg(worst)));
}

void ParagraphStylesPanelTest::doubleClickOpensDialog()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_para_styles_dblclick"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();

    auto* panel = frame.findChild<pictura::ParagraphStylesPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    frame.columnForPanel(QStringLiteral("paragraphStylesPanel"))
        ->showPanel(QStringLiteral("paragraphStylesPanel"), true);
    QCoreApplication::processEvents();

    // A type layer makes the first click apply the style, which records
    // history and schedules the frame's debounced panel refresh; the refresh
    // must not rebuild the list under the second click.
    QVERIFY2(!makeTypeLayer(f).isEmpty(), "a type layer");

    const pictura::CharacterSetting c = pictura::type_api::defaultCharacterSetting();
    const pictura::ParagraphSetting p = pictura::type_api::defaultParagraphSetting();
    QVERIFY(pictura::type_api::createParagraphStyle(*f.view, QStringLiteral("Heading"), QString(), c,
                                                 p));
    panel->refresh();
    QCoreApplication::processEvents();

    bool opened = false;
    QTimer::singleShot(700, [&] {
        auto* dialog = frame.findChild<pictura::ParagraphStyleDialog*>();
        if (dialog) {
            opened = true;
            dialog->reject();
        }
    });
    auto* list = panel->listForTest();
    bool doubleClicked = false;
    QObject::connect(list, &QListWidget::itemDoubleClicked,
                     [&doubleClicked](QListWidgetItem*) { doubleClicked = true; });
    const QRect rect = list->visualItemRect(list->item(1));
    QVERIFY2(rect.isValid() && !rect.isEmpty(), "the style row has no rect");
    // A real double-click is a press/release then a double-click event;
    // `mouseDClick` alone synthesizes only the latter. The wait lets the
    // debounced panel refresh land between the two clicks.
    QTest::mouseClick(list->viewport(), Qt::LeftButton, Qt::NoModifier, rect.center());
    QTest::qWait(250);
    QTest::mouseDClick(list->viewport(), Qt::LeftButton, Qt::NoModifier, rect.center());
    QTest::qWait(900);
    QVERIFY2(doubleClicked, "the item double-click signal did not fire");
    QVERIFY2(opened, "double-click did not open the options dialog");
}

void ParagraphStylesPanelTest::fontHoverDoesNotPreview()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_para_styles_hover"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();

    auto* panel = frame.findChild<pictura::ParagraphStylesPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);

    const pictura::CharacterSetting c = pictura::type_api::defaultCharacterSetting();
    const pictura::ParagraphSetting p = pictura::type_api::defaultParagraphSetting();
    QVERIFY(pictura::type_api::createParagraphStyle(*f.view, QStringLiteral("Body"),
                                                 QStringLiteral("Liberation Sans"), c, p));
    panel->refresh();

    bool checked = false;
    QTimer::singleShot(0, [&] {
        auto* dialog = frame.findChild<pictura::ParagraphStyleDialog*>();
        QVERIFY(dialog);
        if (!dialog) {
            return;
        }
        auto* family = dialog->findChild<QComboBox*>(QStringLiteral("paragraphStyleFamily"));
        QVERIFY(family);
        if (!family) {
            return;
        }
        int other = -1;
        for (int i = 0; i < family->count(); ++i) {
            if (family->itemText(i) != QStringLiteral("Liberation Sans")) {
                other = i;
                break;
            }
        }
        QVERIFY(other >= 0);
        const int base = f.view->history_index();
        // Hovering the popup moves the combo's current index; it must not
        // recomposite the document (the list crawls and closes if it does).
        family->setCurrentIndex(other);
        checked = pictura::type_api::findStyle(pictura::type_api::paragraphStyles(*f.view), QStringLiteral("Body")).font
            == QStringLiteral("Liberation Sans");
        // Picking an entry is an `activated` and does preview.
        QMetaObject::invokeMethod(family, "activated", Q_ARG(int, other));
        checked = checked
            && pictura::type_api::findStyle(pictura::type_api::paragraphStyles(*f.view), QStringLiteral("Body")).font
                == family->itemText(other);
        checked = checked && f.view->history_index() == base;
        dialog->reject();
    });
    QVERIFY(!panel->editStyle(1));
    QVERIFY(checked);
    QCOMPARE(pictura::type_api::findStyle(pictura::type_api::paragraphStyles(*f.view), QStringLiteral("Body")).font,
             QStringLiteral("Liberation Sans"));
}

void ParagraphStylesPanelTest::menusTogglePanel()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_para_styles_menu"));
    QVERIFY2(f.ok(), "type fixture");
    showFrame();

    auto* column = frame.columnForPanel(QStringLiteral("paragraphStylesPanel"));
    QVERIFY(column);
    column->showPanel(QStringLiteral("paragraphStylesPanel"), false);
    frame.registry()->refresh();

    // Both the Window and Type menu entries drive the panel and reflect its
    // visibility through their own action.
    const char* ids[] = {pictura::command_ids::WindowPanelsParagraphStyles,
                         pictura::command_ids::TypePanelsParagraphStyles};
    for (const char* id : ids) {
        const QString command = QString::fromLatin1(id);
        QAction* action = frame.registry()->action(command);
        QVERIFY2(action, qPrintable(command));
        QVERIFY2(action->isEnabled(), qPrintable(command));
        QVERIFY(action->isCheckable());
        QVERIFY(!action->isChecked());

        action->setChecked(true);
        QVERIFY2(frame.registry()->dispatch(command), qPrintable(command));
        frame.registry()->refresh();
        QVERIFY2(column->isPanelVisible(QStringLiteral("paragraphStylesPanel")),
                 qPrintable(command));

        action->setChecked(false);
        QVERIFY2(frame.registry()->dispatch(command), qPrintable(command));
        frame.registry()->refresh();
        QVERIFY2(!column->isPanelVisible(QStringLiteral("paragraphStylesPanel")),
                 qPrintable(command));
    }
}

// The library reads in one adapter call (#403): Basic Paragraph first, then
// every style in panel order with its font and resolved attributes.
void ParagraphStylesPanelTest::styleLibraryReadsInOneCall()
{
    pictura::PicturaMainWindow& frame = *window_;
    Fixture f(frame, seed(), QStringLiteral("pictura_para_styles_library"));
    QVERIFY2(f.ok(), "type fixture");
    const QStringList names = {QStringLiteral("Heading"), QStringLiteral("Body")};
    for (int i = 0; i < names.size(); ++i) {
        pictura::CharacterSetting c = pictura::type_api::defaultCharacterSetting();
        c.size = 40.0 - 20.0 * i;
        pictura::ParagraphSetting p = pictura::type_api::defaultParagraphSetting();
        p.justify = i;
        QVERIFY(pictura::type_api::createParagraphStyle(*f.view, names.at(i),
                                                         QStringLiteral("Liberation Sans"), c, p));
    }
    const QVector<pictura::type_api::ParagraphStyle> styles =
        pictura::type_api::paragraphStyles(*f.view);
    QCOMPARE(styles.size(), names.size() + 1);
    QCOMPARE(styles.first().name, QStringLiteral("Basic Paragraph"));
    for (int i = 0; i < names.size(); ++i) {
        const pictura::type_api::ParagraphStyle& style = styles.at(i + 1);
        QCOMPARE(style.name, names.at(i));
        QCOMPARE(style.character.size, 40.0 - 20.0 * i);
        QCOMPARE(style.paragraph.justify, i);
        QVERIFY(!style.font.isEmpty());
    }

    auto* panel = frame.findChild<pictura::ParagraphStylesPanel*>();
    QVERIFY(panel);
    panel->setView(f.view);
    panel->refresh();
    QCOMPARE(panel->listForTest()->count(), styles.size());
    for (int i = 0; i < styles.size(); ++i) {
        QCOMPARE(panel->listForTest()->item(i)->text(), styles.at(i).name);
    }
}

QTEST_MAIN(ParagraphStylesPanelTest)
#include "tst_paragraph_styles_panel.moc"
