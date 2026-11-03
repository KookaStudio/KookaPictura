// A type layer edit on a large document refreshes the text, not the document
// (#1135): commits, previews and sessions damage the text's reach, and the
// pixels match a document that never took the bounded path.

#include <QtTest/QtTest>
#include <QtTest/QSignalSpy>

#include <QtCore/QCoreApplication>
#include <QtCore/QList>
#include <QtCore/QRect>

#include <memory>
#include <optional>

#include "api/type_api.h"
#include "fonts.h"
#include "frame.h"
#include "type_fonts.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h"

#include "qt_test_support.h"

namespace {

namespace type_api = pictura::type_api;

const QRect kSampled(0, 80, 700, 240);

QRect rectOf(const type_api::TypeSpec& spec)
{
    const QVector<int> rect = type_api::previewRect(spec);
    return rect.size() == 4 ? QRect(rect[0], rect[1], rect[2], rect[3]) : QRect();
}

QList<quint32> pixels(const pictura::PictureView& view)
{
    QList<quint32> out;
    for (int y = kSampled.top(); y <= kSampled.bottom(); y += 2) {
        for (int x = kSampled.left(); x <= kSampled.right(); x += 2) {
            out.append(view.composite_argb(x, y));
        }
    }
    return out;
}

// Every damaged region, and that one arrived at all, against `bounds`.
bool damagedInside(const QSignalSpy& damaged, const QRect& bounds)
{
    if (damaged.isEmpty()) {
        return false;
    }
    for (const QList<QVariant>& region : damaged) {
        const QRect rect(region[0].toInt(), region[1].toInt(), region[2].toInt(),
                         region[3].toInt());
        if (!bounds.contains(rect)) {
            return false;
        }
    }
    return true;
}

} // namespace

class TypeEditDamageTest : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanup();

    void styleChangesStayInsideTheText_data();
    void styleChangesStayInsideTheText();
    void alignmentMatchesAFreshDocument();
    void undoAndRedoRestoreThePixels();
    void previewIsBoundedAndUnrecorded();
    void cancelledSessionRestoresThePixels();
    void committedSessionMatchesAFreshDocument();
    void soloedMaskKeepsTheFullPath();
    void undoAcrossAResizeStaysInsideTheText();
    void warpPreviewIsBoundedAndCancelRestores();
    void warpCommitUndoesPastEveryPreview();
    void stylePreviewStaysInsideTheText();
    void aHugeBoxRefreshesItsInk();

private:
    pictura::PictureView* newDoc();
    QString addType(pictura::PictureView& view, const type_api::TypeSpec& spec);
    type_api::TypeSpec baseSpec() const;

    pictura::test::ScopedStateHome stateHome_;
    std::unique_ptr<pictura::PicturaMainWindow> window_;
};

void TypeEditDamageTest::initTestCase()
{
    QVERIFY(stateHome_.isValid());
    window_ = pictura::test::makeMainWindow();
    QVERIFY(window_ != nullptr);
    window_->resize(800, 600);
    window_->show();
    QCoreApplication::processEvents();
    pictura::registerTypeFallbacks();
}

void TypeEditDamageTest::cleanup()
{
    while (window_->activeDocumentIndex() >= 0) {
        window_->closeDocument(window_->activeDocumentIndex(), false);
    }
}

pictura::PictureView* TypeEditDamageTest::newDoc()
{
    if (!window_->newDocument(QStringLiteral("TypeEditDamage"), 1600, 1200, QStringLiteral("rgb"),
                              8, QStringLiteral("white"))) {
        return nullptr;
    }
    QCoreApplication::processEvents();
    return window_->activeView();
}

type_api::TypeSpec TypeEditDamageTest::baseSpec() const
{
    pictura::TypeSetting setting{};
    setting.x = 300.0;
    setting.y = 200.0;
    setting.xx = 1.0;
    setting.yy = 1.0;
    pictura::CharacterSetting character = type_api::defaultCharacterSetting();
    character.size = 48.0;
    return {QStringLiteral("Bounded"), QString::fromLatin1(pictura::kBundledUiFamily), setting,
            character, type_api::defaultParagraphSetting()};
}

QString TypeEditDamageTest::addType(pictura::PictureView& view, const type_api::TypeSpec& spec)
{
    const QString path = type_api::commitLayer(view, spec);
    QCoreApplication::processEvents();
    return path;
}

void TypeEditDamageTest::styleChangesStayInsideTheText_data()
{
    QTest::addColumn<QString>("field");
    QTest::newRow("size") << QStringLiteral("size");
    QTest::newRow("family") << QStringLiteral("family");
    QTest::newRow("color") << QStringLiteral("color");
    QTest::newRow("paragraph") << QStringLiteral("paragraph");
}

void TypeEditDamageTest::styleChangesStayInsideTheText()
{
    QFETCH(QString, field);
    pictura::PictureView* view = newDoc();
    QVERIFY(view != nullptr);
    const QString path = addType(*view, baseSpec());
    QVERIFY(!path.isEmpty());

    type_api::TypeSpec spec = type_api::readLayer(*view, path).spec;
    const QRect before = rectOf(spec);
    if (field == QLatin1String("size")) {
        spec.character.size = 64.0;
    } else if (field == QLatin1String("family")) {
        spec.font = QStringLiteral("Liberation Serif");
        pictura::registerTypeFont(spec.font, spec.character.font_style);
    } else if (field == QLatin1String("color")) {
        spec.character.color = 0xff2040c0u;
    } else {
        spec.paragraph.justify = 2;
    }

    const int base = view->history_index();
    QSignalSpy damaged(view, &pictura::PictureView::regionDamaged);
    QSignalSpy changed(view, &pictura::PictureView::changed);
    QVERIFY(type_api::updateLayer(*view, path, spec));
    QVERIFY2(damagedInside(damaged, before.united(rectOf(spec))), "the text's region only");
    QCOMPARE(changed.count(), 1);
    QCOMPARE(view->history_index(), base + 1);
}

void TypeEditDamageTest::alignmentMatchesAFreshDocument()
{
    pictura::PictureView* view = newDoc();
    QVERIFY(view != nullptr);
    const QString path = addType(*view, baseSpec());
    QVERIFY(!path.isEmpty());

    type_api::TypeSpec spec = type_api::readLayer(*view, path).spec;
    const QRect before = rectOf(spec);
    spec.paragraph.justify = 1;
    QVERIFY(rectOf(spec) != before);
    QVERIFY(type_api::updateLayer(*view, path, spec));
    const QList<quint32> edited = pixels(*view);

    pictura::PictureView* fresh = newDoc();
    QVERIFY(fresh != nullptr);
    QVERIFY(!addType(*fresh, spec).isEmpty());
    QCOMPARE(edited, pixels(*fresh));
}

void TypeEditDamageTest::undoAndRedoRestoreThePixels()
{
    pictura::PictureView* view = newDoc();
    QVERIFY(view != nullptr);
    const QString path = addType(*view, baseSpec());
    QVERIFY(!path.isEmpty());
    const int base = view->history_index();
    const QList<quint32> original = pixels(*view);

    type_api::TypeSpec spec = type_api::readLayer(*view, path).spec;
    spec.character.size = 64.0;
    QVERIFY(type_api::updateLayer(*view, path, spec));
    const QList<quint32> resized = pixels(*view);
    QVERIFY(resized != original);

    spec.paragraph.justify = 1;
    QVERIFY(type_api::updateLayer(*view, path, spec));
    const QList<quint32> moved = pixels(*view);
    QVERIFY(moved != resized);

    QVERIFY(view->undo());
    QCOMPARE(pixels(*view), resized);
    QVERIFY(view->undo());
    QCOMPARE(pixels(*view), original);
    QVERIFY(view->history_jump(base + 1));
    QCOMPARE(pixels(*view), resized);
    QVERIFY(view->history_jump(base + 2));
    QCOMPARE(pixels(*view), moved);
}

void TypeEditDamageTest::previewIsBoundedAndUnrecorded()
{
    pictura::PictureView* view = newDoc();
    QVERIFY(view != nullptr);
    const QString path = addType(*view, baseSpec());
    QVERIFY(!path.isEmpty());

    type_api::TypeSpec spec = type_api::readLayer(*view, path).spec;
    spec.character.color = 0xff2040c0u;
    const int base = view->history_index();
    QSignalSpy damaged(view, &pictura::PictureView::regionDamaged);
    QSignalSpy changed(view, &pictura::PictureView::changed);
    QVERIFY(type_api::previewLayer(*view, path, spec));
    QVERIFY2(damagedInside(damaged, rectOf(spec)), "the text's region only");
    QCOMPARE(changed.count(), 1);
    QCOMPARE(view->history_index(), base);
}

void TypeEditDamageTest::cancelledSessionRestoresThePixels()
{
    pictura::PictureView* view = newDoc();
    QVERIFY(view != nullptr);
    const QString path = addType(*view, baseSpec());
    QVERIFY(!path.isEmpty());
    const QRect bounds = rectOf(type_api::readLayer(*view, path).spec);
    const QList<quint32> shown = pixels(*view);
    const int base = view->history_index();

    QSignalSpy damaged(view, &pictura::PictureView::regionDamaged);
    std::optional<type_api::EditSession> session = type_api::EditSession::begin(*view, path);
    QVERIFY(session.has_value());
    QVERIFY2(damagedInside(damaged, bounds), "hiding refreshes the text only");
    QVERIFY(pixels(*view) != shown);

    damaged.clear();
    session->cancel();
    QVERIFY2(damagedInside(damaged, bounds), "showing refreshes the text only");
    QCOMPARE(pixels(*view), shown);
    QCOMPARE(view->history_index(), base);
}

void TypeEditDamageTest::committedSessionMatchesAFreshDocument()
{
    pictura::PictureView* view = newDoc();
    QVERIFY(view != nullptr);
    const QString path = addType(*view, baseSpec());
    QVERIFY(!path.isEmpty());
    const int base = view->history_index();

    std::optional<type_api::EditSession> session = type_api::EditSession::begin(*view, path);
    QVERIFY(session.has_value());
    type_api::TypeSpec spec = type_api::readLayer(*view, path).spec;
    const QRect before = rectOf(spec);
    spec.text = QStringLiteral("Bound");
    spec.character.size = 40.0;

    QSignalSpy damaged(view, &pictura::PictureView::regionDamaged);
    QVERIFY(session->commit(spec));
    QVERIFY2(damagedInside(damaged, before.united(rectOf(spec))), "the text's region only");
    QCOMPARE(view->history_index(), base + 1);
    const QList<quint32> edited = pixels(*view);

    pictura::PictureView* fresh = newDoc();
    QVERIFY(fresh != nullptr);
    QVERIFY(!addType(*fresh, spec).isEmpty());
    QCOMPARE(edited, pixels(*fresh));
}

void TypeEditDamageTest::soloedMaskKeepsTheFullPath()
{
    pictura::PictureView* view = newDoc();
    QVERIFY(view != nullptr);
    const QString path = addType(*view, baseSpec());
    QVERIFY(!path.isEmpty());
    QVERIFY(pictura::layer_mask_add_path(*view, path, QStringLiteral("reveal-all")));
    QVERIFY(pictura::mask_solo_set(*view, path));

    type_api::TypeSpec spec = type_api::readLayer(*view, path).spec;
    spec.character.color = 0xff2040c0u;
    QSignalSpy damaged(view, &pictura::PictureView::regionDamaged);
    QSignalSpy changed(view, &pictura::PictureView::changed);
    QVERIFY(type_api::previewLayer(*view, path, spec));
    QCOMPARE(damaged.count(), 0);
    QCOMPARE(changed.count(), 1);
}

void TypeEditDamageTest::undoAcrossAResizeStaysInsideTheText()
{
    pictura::PictureView* view = newDoc();
    QVERIFY(view != nullptr);
    const QString path = addType(*view, baseSpec());
    QVERIFY(!path.isEmpty());
    const QList<quint32> original = pixels(*view);

    type_api::TypeSpec spec = type_api::readLayer(*view, path).spec;
    const QRect before = rectOf(spec);
    spec.character.size = 64.0;
    QVERIFY(type_api::updateLayer(*view, path, spec));
    const QRect bounds = before.united(rectOf(spec)).adjusted(-64, -64, 64, 64);

    QSignalSpy damaged(view, &pictura::PictureView::regionDamaged);
    QVERIFY(view->undo());
    QVERIFY2(damagedInside(damaged, bounds), "undo refreshes the text only");
    QCOMPARE(pixels(*view), original);

    damaged.clear();
    QVERIFY(view->history_jump(view->history_index() + 1));
    QVERIFY2(damagedInside(damaged, bounds), "redo refreshes the text only");
}

void TypeEditDamageTest::warpPreviewIsBoundedAndCancelRestores()
{
    pictura::PictureView* view = newDoc();
    QVERIFY(view != nullptr);
    const QString path = addType(*view, baseSpec());
    QVERIFY(!path.isEmpty());
    const QList<quint32> shown = pixels(*view);
    const int base = view->history_index();

    QSignalSpy damaged(view, &pictura::PictureView::regionDamaged);
    QSignalSpy changed(view, &pictura::PictureView::changed);
    type_api::WarpSetting warp;
    warp.style = QStringLiteral("arc");
    warp.bend = 50.0;
    {
        type_api::WarpSession session(*view, path);
        QVERIFY(session.preview(warp));
        QVERIFY2(damagedInside(damaged, QRect(0, 0, 1000, 700)), "the text's region only");
        QCOMPARE(changed.count(), 1);
        QCOMPARE(view->history_index(), base);
        QVERIFY(pixels(*view) != shown);
        QVERIFY(session.cancel());
    }
    QCOMPARE(pixels(*view), shown);
    QCOMPARE(view->history_index(), base);
}

// The layer's place before the dialog opened can lie outside every rectangle the
// commit sees, and history must still hold it.
void TypeEditDamageTest::warpCommitUndoesPastEveryPreview()
{
    pictura::PictureView* view = newDoc();
    QVERIFY(view != nullptr);
    const QString path = addType(*view, baseSpec());
    QVERIFY(!path.isEmpty());
    const QList<quint32> shown = pixels(*view);

    type_api::WarpSetting strong;
    strong.style = QStringLiteral("arc");
    strong.bend = 100.0;
    type_api::WarpSetting slight = strong;
    slight.bend = 5.0;
    {
        type_api::WarpSession session(*view, path);
        QVERIFY(session.preview(strong));
        QVERIFY(session.commit(slight));
    }
    const QList<quint32> warped = pixels(*view);
    QVERIFY(warped != shown);
    QVERIFY(view->undo());
    QCOMPARE(pixels(*view), shown);
}

void TypeEditDamageTest::stylePreviewStaysInsideTheText()
{
    pictura::PictureView* view = newDoc();
    QVERIFY(view != nullptr);
    const QString path = addType(*view, baseSpec());
    QVERIFY(!path.isEmpty());
    const type_api::TypeSpec spec = type_api::readLayer(*view, path).spec;
    QVERIFY(type_api::createParagraphStyle(*view, QStringLiteral("Probe"), spec.font,
                                           spec.character, spec.paragraph));
    QVERIFY(type_api::applyStyle(*view, path, QStringLiteral("Probe"), true));
    const int base = view->history_index();

    pictura::CharacterSetting recoloured = spec.character;
    recoloured.color = 0xff2040c0u;
    QSignalSpy damaged(view, &pictura::PictureView::regionDamaged);
    QVERIFY(type_api::previewParagraphStyle(*view, QStringLiteral("Probe"), spec.font, recoloured,
                                            spec.paragraph));
    QVERIFY2(damagedInside(damaged, rectOf(spec)), "the text's region only");
    QCOMPARE(view->history_index(), base);
}

void TypeEditDamageTest::aHugeBoxRefreshesItsInk()
{
    pictura::PictureView* view = newDoc();
    QVERIFY(view != nullptr);
    type_api::TypeSpec spec = baseSpec();
    spec.setting.x = 10.0;
    spec.setting.y = 10.0;
    spec.setting.boxed = true;
    spec.setting.box_right = 1580.0;
    spec.setting.box_bottom = 1180.0;
    const QString path = addType(*view, spec);
    QVERIFY(!path.isEmpty());

    spec = type_api::readLayer(*view, path).spec;
    spec.character.color = 0xff2040c0u;
    QSignalSpy damaged(view, &pictura::PictureView::regionDamaged);
    QVERIFY(type_api::updateLayer(*view, path, spec));
    QVERIFY2(damagedInside(damaged, QRect(0, 0, 500, 200)), "the words, not the box");
}

QTEST_MAIN(TypeEditDamageTest)
#include "tst_type_edit_damage.moc"
