#include <QtTest/QtTest>
#include <QtCore/QFile>
#include <QtCore/QTemporaryDir>
#include <QtCore/QSet>
#include <QtGui/QAction>
#include <QtWidgets/QMenu>
#include <QtWidgets/QMenuBar>
#include "commands.h"
#include "command_ids.h"
#include "frame.h"
#include "panels/layers_panel.h"
#include <memory>

#include "api/document_api.h"
#include "api/document_bridge.h"
#include "api/engine_bridge.h"
#include "api/history_api.h"
#include "api/layers_api.h"
#include "api/layers_bridge.h"
#include "api/type_api.h"
#include "qt_test_support.h"

namespace {
using pictura::PictureView;
using pictura::layers_api::rowForPath;
std::unique_ptr<PictureView> document()
{
    auto view = std::make_unique<PictureView>();
    if (!pictura::create_document(*view, 16, 16, QStringLiteral("rgb"), 8,
                                 0xffffffu, false, 72.0, false)) {
        return nullptr;
    }
    return view;
}
QVector<quint32> pixels(PictureView& view)
{
    QVector<quint32> result;
    const auto extent = pictura::layers_api::snapshot(view, 0, false);
    for (int y = 0; y < extent.documentHeight; ++y) {
        for (int x = 0; x < extent.documentWidth; ++x) { result.append(view.sample_argb(x, y)); }
    }
    return result;
}
QString type(PictureView& view)
{
    pictura::TypeSetting setting{};
    setting.x = 1; setting.y = 12; setting.xx = 1; setting.yy = 1;
    auto character = pictura::type_api::defaultCharacterSetting();
    character.size = 8;
    return pictura::type_api::commitLayer(view,
        {QStringLiteral("A"), QStringLiteral("Liberation Sans"), setting,
         character, pictura::type_api::defaultParagraphSetting()});
}
quint32 word(const QByteArray& bytes, qsizetype at, int size = 4)
{
    if (at < 0 || at + size > bytes.size()) { return 0; }
    quint32 value = 0;
    for (int i = 0; i < size; ++i) { value = (value << 8) | quint8(bytes[at + i]); }
    return value;
}
void setWord(QByteArray& bytes, qsizetype at, quint32 value)
{
    for (int i = 3; i >= 0; --i) { bytes[at + i] = char(value & 255); value >>= 8; }
}
struct Record {
    qsizetype sectionLength = -1;
    qsizetype infoLength = -1;
    qsizetype extraLength = -1;
    qsizetype extraBegin = -1;
    qsizetype extraEnd = -1;
    qsizetype flags = -1;
    int channels = 0;
};
Record record(const QByteArray& bytes, const QByteArray& tag = {})
{
    if (bytes.size() < 38 || bytes.left(4) != "8BPS" || word(bytes, 4, 2) != 1) { return {}; }
    qsizetype at = 26;
    at += 4 + word(bytes, at);
    at += 4 + word(bytes, at);
    Record result; result.sectionLength = at; result.infoLength = at + 4;
    at += 8;
    const int count = qAbs(qint16(word(bytes, at, 2))); at += 2;
    for (int i = 0; i < count && at + 34 <= bytes.size(); ++i) {
        at += 16;
        result.channels = int(word(bytes, at, 2)); at += 2 + result.channels * 6;
        result.flags = at + 10; at += 12;
        result.extraLength = at;
        result.extraBegin = at + 4;
        result.extraEnd = result.extraBegin + word(bytes, at);
        if (result.extraEnd > bytes.size()) { return {}; }
        if (tag.isEmpty() || bytes.mid(result.extraBegin, result.extraEnd - result.extraBegin)
                .contains(QByteArray("8BIM") + tag)) { return result; }
        at = result.extraEnd;
    }
    return {};
}
QByteArray tagBytes(const QByteArray& bytes, const QByteArray& key)
{
    const auto at = bytes.indexOf(QByteArray("8BIM") + key);
    return at < 0 ? QByteArray() : bytes.mid(at + 12, word(bytes, at + 8));
}
QByteArray withPrimary(QByteArray bytes)
{
    const auto found = record(bytes, "TySh");
    if (found.extraLength < 0) { return {}; }
    const QByteArray invert("8BIMnvrt\0\0\0\0", 12);
    for (const auto at : {found.sectionLength, found.infoLength, found.extraLength}) {
        setWord(bytes, at, word(bytes, at) + quint32(invert.size()));
    }
    bytes.insert(found.extraEnd, invert);
    return bytes;
}
QByteArray withoutPattern(QByteArray bytes, const QString& id)
{
    QByteArray encoded;
    for (QChar character : id) {
        encoded.append(char(character.unicode() >> 8)); encoded.append(char(character.unicode() & 255));
    }
    const auto tag = bytes.indexOf("8BIMPtFl");
    if (tag < 0 || encoded.isEmpty()) { return {}; }
    const auto payload = bytes.mid(tag + 12, word(bytes, tag + 8));
    const auto idAt = payload.indexOf(encoded);
    if (idAt < 0) { return {}; }
    for (qsizetype i = 0; i < encoded.size(); i += 2) {
        bytes[tag + 12 + idAt + i] = char(0); bytes[tag + 12 + idAt + i + 1] = 'x';
    }
    return bytes;
}
QByteArray readFile(const QString& path)
{
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray();
}
QAction* menuAction(QMenu* menu, const QString& text)
{
    if (!menu) { return nullptr; }
    for (auto* action : menu->actions()) {
        QString label = action->text(); label.remove(QLatin1Char('&'));
        if (label == text) { return action; }
    }
    return nullptr;
}

}

class LayerContentTest : public QObject {
    Q_OBJECT
private slots:
    void rasterizeKeepsAppearanceAndHistory();
    void rowsAndCommandsFollowContent();
    void typeRowsUseNormalizedIdentity();
    void undoRedoRestoresContentAndPreservation();
    void vectorMaskTransitionsKeepRowsAndHistory();
private:
    pictura::test::ScopedStateHome stateHome_;
};

void LayerContentTest::rasterizeKeepsAppearanceAndHistory()
{
    QVERIFY(stateHome_.isValid());
    for (const QString& kind : {QStringLiteral("solid-color"), QStringLiteral("descriptor-solid"), QStringLiteral("gradient"),
                                QStringLiteral("pattern"), QStringLiteral("missing-pattern"), QStringLiteral("shape"),
                                QStringLiteral("type"), QStringLiteral("smart")}) {
        auto view = document(); QVERIFY(view);
        QString path;
        if (kind == QStringLiteral("solid-color")) { path = view->add_solid_fill(0xff123456u); }
        else if (kind == QStringLiteral("descriptor-solid")) {
            path = pictura::add_fill_layer(*view, QStringLiteral("solid-color"), QStringLiteral("123456"),
                QString(), QStringLiteral("Descriptor"), 0, QStringLiteral("norm"), 255, false);
        } else if (kind == QStringLiteral("gradient")) { path = view->add_gradient_fill(); }
        else if (kind == QStringLiteral("pattern")) {
            const QString fixture = QFINDTESTDATA("../../../../crates/pictura-codec/tests/fixtures/pattern_fill.psd");
            QVERIFY2(!fixture.isEmpty(), "the committed pattern fixture is required");
            QVERIFY(pictura::document_api::open(*view, fixture));
            const auto ids = pictura::available_pattern_ids(*view); QVERIFY(!ids.isEmpty());
            path = pictura::add_fill_layer(*view, kind, ids.first() + QStringLiteral("|100|1|0|0"),
                QString(), QStringLiteral("Pattern"), 0, QStringLiteral("norm"), 255, false);
            const auto thumbnail = pictura::layers_api::snapshot(*view, 16, false).rows;
            QSet<QRgb> colors;
            for (const auto& item : thumbnail) {
                if (item.path != path) { continue; }
                QVERIFY(!item.thumbnail.isNull());
                for (int y = 0; y < item.thumbnail.height(); ++y) {
                    for (int x = 0; x < item.thumbnail.width(); ++x) { colors.insert(item.thumbnail.pixel(x, y)); }
                }
            }
            QVERIFY(colors.size() > 1);
        } else if (kind == QStringLiteral("missing-pattern")) {
            const auto ids = pictura::available_pattern_ids(*view); QVERIFY(!ids.isEmpty());
            path = pictura::add_fill_layer(*view, QStringLiteral("pattern"), ids.first() + QStringLiteral("|100|1|0|0"),
                QString(), QStringLiteral("Missing"), 0, QStringLiteral("norm"), 255, false);
            QVERIFY(!path.isEmpty());
            QTemporaryDir dir; QVERIFY(dir.isValid());
            const QString file = dir.filePath(QStringLiteral("missing.psd"));
            QVERIFY(pictura::document_api::saveCopy(*view, file));
            const auto missing = withoutPattern(readFile(file), ids.first()); QVERIFY(!missing.isEmpty());
            QFile changed(file); QVERIFY(changed.open(QIODevice::WriteOnly | QIODevice::Truncate));
            QCOMPARE(changed.write(missing), qint64(missing.size())); changed.close();
            QVERIFY(pictura::document_api::open(*view, file));
            QVERIFY(pictura::layer_fill_pattern(*view, path).startsWith(QString(ids.first().size(), QLatin1Char('x')) + QLatin1Char('|')));
            const QRgb placeholder = view->sample_argb(0, 0);
            QCOMPARE(qRed(placeholder), qGreen(placeholder)); QCOMPARE(qGreen(placeholder), qBlue(placeholder));
            QVERIFY(qRed(placeholder) < 255);
        } else if (kind == QStringLiteral("shape")) {
            pictura::ShapeSpec square{}; square.kind = 0; square.boxed = true;
            square.x0 = 2; square.y0 = 2; square.x1 = 12; square.y1 = 12;
            path = pictura::shape_add_layer(*view, square, 0xff123456u);
        } else if (kind == QStringLiteral("type")) { path = type(*view); }
        else { path = view->add_layer_in(QString()); QVERIFY(view->convert_to_smart_object(path)); }
        QVERIFY2(!path.isEmpty(), qPrintable(kind));
        QCOMPARE(view->set_layers_opacity(QStringList{path}, 128), 1);
        QCOMPARE(view->set_layers_fill(QStringList{path}, 173), 1);
        QCOMPARE(view->set_layers_blend(QStringList{path}, QStringLiteral("mul ")), 1);
        QVERIFY(view->set_layer_name_path(path, QStringLiteral("Retained")));
        QVERIFY(pictura::layer_mask_add_path(*view, path, QStringLiteral("reveal-all")) || rowForPath(*view, path).hasMask);
        const auto before = pixels(*view);
        const auto row = rowForPath(*view, path);
        const int count = view->history_count();
        const bool changed = kind == QStringLiteral("shape") ? pictura::shape_rasterize(*view, path)
            : kind == QStringLiteral("type") ? view->rasterize_type(path)
            : kind == QStringLiteral("smart") ? view->rasterize_smart_object(path)
            : view->rasterize_fill_content(path);
        QVERIFY(changed);
        QCOMPARE(view->history_count(), count + 1);
        QCOMPARE(pixels(*view), before);
        const auto after = rowForPath(*view, path);
        QCOMPARE(after.kind, QStringLiteral("pixel"));
        QCOMPARE(after.opacity, row.opacity); QCOMPARE(after.fill, row.fill);
        QCOMPARE(after.name, row.name); QCOMPARE(after.blend, row.blend);
        QCOMPARE(after.hasMask, row.hasMask); QCOMPARE(after.lockBits, row.lockBits & ~3);
        QVERIFY(!after.shape && !after.smartObject && !after.hasAdjustment);
        QVERIFY(pictura::history_api::undo(*view)); QCOMPARE(pixels(*view), before);
        QVERIFY(pictura::history_api::redo(*view)); QCOMPARE(pixels(*view), before);
    }
    auto view = document(); QVERIFY(view);
    for (int which = 0; which < 3; ++which) {
        const QString path = which == 0 ? view->active_layer_path()
            : which == 1 ? view->add_group_in(QString()) : view->add_layer_in(QString());
        const auto before = pixels(*view); const int count = view->history_count();
        QVERIFY(!view->rasterize_layer(path));
        QVERIFY(!pictura::shape_rasterize(*view, path));
        QVERIFY(!view->rasterize_fill_content(path));
        QVERIFY(!view->rasterize_type(path));
        QVERIFY(!view->rasterize_smart_object(path));
        QCOMPARE(view->history_count(), count); QCOMPARE(pixels(*view), before);
    }
    QVERIFY(view->add_adjustment(QStringLiteral("invert")));
    const int count = view->history_count();
    QVERIFY(!view->rasterize_layer(view->active_layer_path()));
    QVERIFY(!pictura::shape_rasterize(*view, view->active_layer_path()));
    QVERIFY(!view->rasterize_fill_content(view->active_layer_path()));
    QCOMPARE(view->history_count(), count);
    const QString refusedType = type(*view); QVERIFY(!refusedType.isEmpty());
    const int refusedTypeCount = view->history_count(); const auto refusedTypePixels = pixels(*view);
    QVERIFY(!pictura::shape_rasterize(*view, refusedType));
    QCOMPARE(view->history_count(), refusedTypeCount); QCOMPARE(pixels(*view), refusedTypePixels);
    {
        auto undecodable = document(); QVERIFY(undecodable);
        const QString opaquePath = pictura::add_fill_layer(*undecodable, QStringLiteral("solid-color"), QStringLiteral("123456"),
            QString(), QStringLiteral("Opaque"), 0, QStringLiteral("norm"), 255, false);
        QVERIFY(!opaquePath.isEmpty());
        QTemporaryDir dir; QVERIFY(dir.isValid()); const QString file = dir.filePath(QStringLiteral("undecodable.psd"));
        QVERIFY(pictura::document_api::saveCopy(*undecodable, file));
        QByteArray bytes = readFile(file); const auto tag = bytes.indexOf("8BIMSoCo"); QVERIFY(tag >= 0);
        QVERIFY(word(bytes, tag + 8) > 4); bytes[tag + 12] = char(0x7f);
        QFile output(file); QVERIFY(output.open(QIODevice::WriteOnly | QIODevice::Truncate));
        QCOMPARE(output.write(bytes), qint64(bytes.size())); output.close();
        QVERIFY(pictura::document_api::open(*undecodable, file));
        QVERIFY(!undecodable->layer_is_fill_content(opaquePath));
        const auto appearance = pixels(*undecodable); const int history = undecodable->history_count();
        QVERIFY(!undecodable->rasterize_fill_content(opaquePath));
        QVERIFY(!undecodable->rasterize_layer(opaquePath));
        QCOMPARE(undecodable->history_count(), history); QCOMPARE(pixels(*undecodable), appearance);
    }
    qputenv("PICTURA_EDITION", "extended");
    auto window = pictura::test::makeMainWindow(); QVERIFY(window);
    QVERIFY(window->newDocument(QStringLiteral("Rasterize menu"), 16, 16, QStringLiteral("rgb"), 8, QStringLiteral("white")));
    auto* active = window->activeView(); QVERIFY(active);
    auto* registry = window->registry(); QVERIFY(registry);
    auto* panel = window->findChild<pictura::LayersPanel*>(QStringLiteral("layersPanel")); QVERIFY(panel);
    QMenu* layerMenu = nullptr;
    for (auto* action : window->menuBar()->actions()) {
        QString text = action->text(); text.remove(QLatin1Char('&'));
        if (text == QStringLiteral("Layer")) { layerMenu = action->menu(); }
    }
    auto* rasterize = menuAction(layerMenu, QStringLiteral("Rasterize")); QVERIFY(rasterize && rasterize->menu());
    registry->refresh();
    for (const QString& leaf : {QStringLiteral("Vector Mask"), QStringLiteral("Video"), QStringLiteral("3D"), QStringLiteral("Layer Style")}) {
        auto* action = menuAction(rasterize->menu(), leaf); QVERIFY2(action, qPrintable(leaf));
        QVERIFY(!action->isEnabled());
        const auto unchanged = pixels(*active); const int history = active->history_count();
        if (leaf != QStringLiteral("Vector Mask")) {
            QVERIFY(!registry->dispatch(pictura::commandIdForPath({QStringLiteral("Layer"), QStringLiteral("Rasterize"), leaf})));
        }
        QCOMPARE(active->history_count(), history); QCOMPARE(pixels(*active), unchanged);
    }
    const QString text = type(*active); QVERIFY(!text.isEmpty());
    panel->refresh(); panel->selectPaths({text}, text); registry->refresh();
    auto* typeAction = registry->action(QString::fromLatin1(pictura::command_ids::LayerRasterizeType));
    QVERIFY(typeAction && typeAction->isEnabled());
    const auto typePixels = pixels(*active); const int typeHistory = active->history_count();
    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::LayerRasterizeType)));
    QCOMPARE(active->history_count(), typeHistory + 1); QCOMPARE(pixels(*active), typePixels);
    QCOMPARE(rowForPath(*active, text).kind, QStringLiteral("pixel"));
    const QString smart = active->add_layer_in(QString()); QVERIFY(active->convert_to_smart_object(smart));
    panel->refresh(); panel->selectPaths({smart}, smart); registry->refresh();
    auto* smartAction = registry->action(QString::fromLatin1(pictura::command_ids::LayerRasterizeSmartObject));
    QVERIFY(smartAction && smartAction->isEnabled());
    const auto smartPixels = pixels(*active); const int smartHistory = active->history_count();
    QVERIFY(registry->dispatch(QString::fromLatin1(pictura::command_ids::LayerRasterizeSmartObject)));
    QCOMPARE(active->history_count(), smartHistory + 1); QCOMPARE(pixels(*active), smartPixels);
    QVERIFY(!rowForPath(*active, smart).smartObject);

}

void LayerContentTest::rowsAndCommandsFollowContent()
{
    auto view = document(); QVERIFY(view);
    const auto background = rowForPath(*view, view->active_layer_path());
    QCOMPARE(background.kind, QStringLiteral("background"));
    QVERIFY(!background.hasAdjustment && !background.shape && !background.smartObject && !background.expandable);
    QString path = view->add_layer_in(QString());
    QCOMPARE(rowForPath(*view, path).kind, QStringLiteral("pixel"));
    QVERIFY(!rowForPath(*view, path).hasAdjustment && !rowForPath(*view, path).smartObject);
    QVERIFY(!pictura::layer_is_editable_fill(*view, path));
    QVERIFY(pictura::vector_mask_add_path(*view, path, QStringLiteral("reveal-all")));
    QVERIFY(!rowForPath(*view, path).shape);
    QVERIFY(view->convert_to_smart_object(path));
    const auto smartRow = rowForPath(*view, path);
    QCOMPARE(smartRow.kind, QStringLiteral("pixel"));
    QVERIFY(smartRow.smartObject && !smartRow.shape && !smartRow.hasAdjustment && !view->layer_is_type(path));
    QVERIFY(!pictura::layer_is_editable_fill(*view, path));
    path = view->add_group_in(QString());
    QCOMPARE(rowForPath(*view, path).kind, QStringLiteral("group"));
    QVERIFY(rowForPath(*view, path).expandable);
    QVERIFY(!rowForPath(*view, path).hasAdjustment && !rowForPath(*view, path).shape && !rowForPath(*view, path).smartObject);
    path = view->add_solid_fill(0xff00ff00u);
    QCOMPARE(rowForPath(*view, path).kind, QStringLiteral("adjustment"));
    QVERIFY(rowForPath(*view, path).hasAdjustment);
    QCOMPARE(rowForPath(*view, path).adjustmentKind, QStringLiteral("solid-color"));
    QVERIFY(!rowForPath(*view, path).shape && !rowForPath(*view, path).smartObject);
    QVERIFY(pictura::layer_is_editable_fill(*view, path));
    QVERIFY(pictura::layer_mask_add_path(*view, path, QStringLiteral("reveal-all")));
    QVERIFY(pictura::layer_is_editable_fill(*view, path));
    QVERIFY(pictura::vector_mask_add_path(*view, path, QStringLiteral("reveal-all")));
    QVERIFY(rowForPath(*view, path).shape && rowForPath(*view, path).hasAdjustment);
    QCOMPARE(rowForPath(*view, path).lockBits & 3, 3);
    QVERIFY(!pictura::layer_is_editable_fill(*view, path));
    path = type(*view); QVERIFY(!path.isEmpty());
    QCOMPARE(rowForPath(*view, path).kind, QStringLiteral("type"));
    QVERIFY(view->layer_is_type(path));
    QVERIFY(!rowForPath(*view, path).hasAdjustment && !rowForPath(*view, path).shape && !rowForPath(*view, path).smartObject);
    QCOMPARE(rowForPath(*view, path).lockBits & 3, 3);
    QVERIFY(view->add_adjustment(QStringLiteral("invert")));
    const auto adjustment = rowForPath(*view, view->active_layer_path());
    QCOMPARE(adjustment.kind, QStringLiteral("adjustment"));
    QCOMPARE(adjustment.adjustmentKind, QStringLiteral("invert"));
    QVERIFY(adjustment.hasAdjustment && !adjustment.shape && !adjustment.smartObject && !adjustment.expandable);
    QVERIFY(!pictura::layer_is_editable_fill(*view, adjustment.path));
}

void LayerContentTest::typeRowsUseNormalizedIdentity()
{
    auto view = document(); QVERIFY(view);
    const QString background = view->active_layer_path();
    QCOMPARE(view->layer_kind(background.toInt()), QStringLiteral("background"));
    QVERIFY(!view->layer_is_type(background));
    const QString path = type(*view); QVERIFY(!path.isEmpty());
    QVERIFY(view->layer_is_type(path));
    QCOMPARE(view->layer_kind(path.toInt()), QStringLiteral("type"));
    QCOMPARE(rowForPath(*view, path).kind, QStringLiteral("type"));
    QTemporaryDir dir; QVERIFY(dir.isValid());
    const QString file = dir.filePath(QStringLiteral("type.psd"));
    QVERIFY(pictura::document_api::saveCopy(*view, file));
    QFile input(file); QVERIFY(input.open(QIODevice::ReadOnly));
    QByteArray bytes = input.readAll(); input.close();
    const auto typeRecord = record(bytes, "TySh"); QVERIFY(typeRecord.extraLength >= 0);
    QVERIFY(typeRecord.channels > 0);
    PictureView parsed; QVERIFY(pictura::document_api::open(parsed, file));
    QCOMPARE(parsed.layer_kind(path.toInt()), QStringLiteral("type"));
    QCOMPARE(rowForPath(parsed, path).kind, QStringLiteral("type"));
    QVERIFY(parsed.layer_is_type(path)); QCOMPARE(pixels(parsed), pixels(*view));
    const auto tag = bytes.indexOf("TySh"); QVERIFY(tag >= 0);
    // Keep the entire proxy and block, but invalidate the type-tool version.
    QVERIFY(tag + 10 < bytes.size()); bytes[tag + 8] = char(0x7f);
    QVERIFY(input.open(QIODevice::WriteOnly | QIODevice::Truncate));
    QCOMPARE(input.write(bytes), qint64(bytes.size())); input.close();
    PictureView opaque; QVERIFY(pictura::document_api::open(opaque, file));
    QVERIFY(opaque.layer_is_type(path));
    QCOMPARE(opaque.layer_kind(path.toInt()), QStringLiteral("type"));
    QCOMPARE(rowForPath(opaque, path).kind, QStringLiteral("type"));
    QVERIFY(!rowForPath(opaque, path).hasAdjustment && !rowForPath(opaque, path).smartObject);
    QCOMPARE(rowForPath(opaque, path).lockBits & 3, 3);
    QCOMPARE(pixels(opaque), pixels(parsed));
    const QString copy = dir.filePath(QStringLiteral("opaque.psd"));
    QVERIFY(pictura::document_api::saveCopy(opaque, copy));
    QFile saved(copy); QVERIFY(saved.open(QIODevice::ReadOnly));
    const auto output = saved.readAll(); const auto outputTag = output.indexOf("TySh");
    QVERIFY(outputTag >= 0); QCOMPARE(output[outputTag + 8], bytes[tag + 8]);
    QCOMPARE(tagBytes(output, "TySh"), tagBytes(bytes, "TySh"));
    const QByteArray conflict = withPrimary(bytes); QVERIFY(!conflict.isEmpty());
    const QString conflictingFile = dir.filePath(QStringLiteral("conflict.psd"));
    QFile conflictOutput(conflictingFile); QVERIFY(conflictOutput.open(QIODevice::WriteOnly));
    QCOMPARE(conflictOutput.write(conflict), qint64(conflict.size())); conflictOutput.close();
    PictureView prioritized; QVERIFY(pictura::document_api::open(prioritized, conflictingFile));
    const auto prioritizedRow = rowForPath(prioritized, path);
    QCOMPARE(prioritizedRow.kind, QStringLiteral("adjustment"));
    QCOMPARE(prioritized.layer_kind(path.toInt()), QStringLiteral("adjustment"));
    QVERIFY(prioritizedRow.hasAdjustment); QCOMPARE(prioritizedRow.adjustmentKind, QStringLiteral("invert"));
    QVERIFY(!prioritizedRow.shape && !prioritizedRow.smartObject && !prioritized.layer_is_type(path));
    QVERIFY(!pictura::type_api::readLayer(prioritized, path).isType);
    QCOMPARE(prioritizedRow.lockBits & 3, 0);
    const QString conflictCopy = dir.filePath(QStringLiteral("conflict-copy.psd"));
    QVERIFY(pictura::document_api::saveCopy(prioritized, conflictCopy));
    QCOMPARE(tagBytes(readFile(conflictCopy), "TySh"), tagBytes(bytes, "TySh"));
    const QString pixel = opaque.add_layer_in(QString());
    QCOMPARE(rowForPath(opaque, pixel).kind, QStringLiteral("pixel"));
    QCOMPARE(opaque.layer_kind(pixel.toInt()), QStringLiteral("pixel"));
    QVERIFY(!opaque.layer_is_type(pixel));
}

void LayerContentTest::undoRedoRestoresContentAndPreservation()
{
    auto view = document(); QVERIFY(view);
    const QString path = view->add_solid_fill(0xff123456u);
    const auto initial = pixels(*view); const int count = view->history_count();
    QVERIFY(pictura::set_layer_fill_color(*view, path, 0xffabcdefu));
    const auto edited = pixels(*view); QVERIFY(edited != initial);
    QCOMPARE(view->history_count(), count + 1);
    QVERIFY(pictura::history_api::undo(*view));
    QCOMPARE(pictura::layer_fill_color(*view, path), 0xff123456u); QCOMPARE(pixels(*view), initial);
    QVERIFY(pictura::history_api::redo(*view));
    QCOMPARE(pictura::layer_fill_color(*view, path), 0xffabcdefu); QCOMPARE(pixels(*view), edited);
    QTemporaryDir dir; QVERIFY(dir.isValid());
    const QString file = dir.filePath(QStringLiteral("edit.psd"));
    QVERIFY(pictura::document_api::saveCopy(*view, file));
    PictureView reopened; QVERIFY(pictura::document_api::open(reopened, file));
    QCOMPARE(pictura::layer_fill_color(reopened, path), 0xffabcdefu); QCOMPARE(pixels(reopened), edited);
    const auto savedFill = tagBytes(readFile(file), "SoCo"); QVERIFY(!savedFill.isEmpty());
    const int rasterizeCount = view->history_count();
    QVERIFY(view->rasterize_layer(path)); QCOMPARE(view->history_count(), rasterizeCount + 1);
    QVERIFY(pictura::history_api::undo(*view)); QVERIFY(rowForPath(*view, path).hasAdjustment);
    QVERIFY(pictura::document_api::saveCopy(*view, file));
    QCOMPARE(tagBytes(readFile(file), "SoCo"), savedFill);
    QCOMPARE(pixels(*view), edited);
    QVERIFY(pictura::history_api::redo(*view)); QCOMPARE(rowForPath(*view, path).kind, QStringLiteral("pixel"));
    QVERIFY(pictura::document_api::saveCopy(*view, file));
    QVERIFY(pictura::document_api::open(reopened, file));
    QCOMPARE(rowForPath(reopened, path).kind, QStringLiteral("pixel")); QCOMPARE(pixels(reopened), edited);
    QVERIFY(tagBytes(readFile(file), "SoCo").isEmpty());
}

void LayerContentTest::vectorMaskTransitionsKeepRowsAndHistory()
{
    for (bool rasterize : {false, true}) {
        auto view = document(); QVERIFY(view);
        const QString path = view->add_solid_fill(0xff123456u);
        QCOMPARE(view->set_layers_opacity({path}, 137), 1);
        QCOMPARE(view->set_layers_fill({path}, 173), 1);
        const auto before = pixels(*view);
        const auto initial = rowForPath(*view, path);
        QTemporaryDir dir; QVERIFY(dir.isValid());
        const QString file = dir.filePath(QStringLiteral("mask.psd"));
        QVERIFY(pictura::document_api::saveCopy(*view, file));
        const auto original = readFile(file); const auto initialRecord = record(original);
        QVERIFY(initialRecord.flags >= 0);
        const quint32 flags = word(original, initialRecord.flags, 1);
        const QByteArray fill = tagBytes(original, "SoCo"); QVERIFY(!fill.isEmpty());
        const auto check = [&](PictureView& target, bool shape, bool mask, bool vector, bool disabled = false) {
            const auto row = rowForPath(target, path);
            QCOMPARE(row.shape, shape); QCOMPARE(row.hasMask, mask); QCOMPARE(row.hasVectorMask, vector);
            QCOMPARE(row.vectorMaskDisabled, disabled); QVERIFY(row.hasAdjustment && !row.smartObject);
            QCOMPARE(row.kind, QStringLiteral("adjustment"));
            QCOMPARE(row.lockBits, shape ? (initial.lockBits | 3) : initial.lockBits);
            QCOMPARE(pictura::layer_is_editable_fill(target, path), !shape);
            QCOMPARE(row.name, initial.name); QCOMPARE(row.opacity, initial.opacity);
            QCOMPARE(row.fill, initial.fill); QCOMPARE(row.blend, initial.blend);
            QCOMPARE(pixels(target), before);
        };
        const auto saved = [&](bool shape, bool mask, bool vector, bool disabled = false) {
            check(*view, shape, mask, vector, disabled);
            QVERIFY(pictura::document_api::saveCopy(*view, file));
            const auto bytes = readFile(file); const auto current = record(bytes); QVERIFY(current.flags >= 0);
            QCOMPARE(word(bytes, current.flags, 1), flags);
            QCOMPARE(tagBytes(bytes, "SoCo"), fill);
            PictureView reopened; QVERIFY(pictura::document_api::open(reopened, file));
            check(reopened, shape, mask, vector, disabled);
        };
        saved(false, initial.hasMask, false);
        const int count = view->history_count();
        QVERIFY(pictura::vector_mask_add_path(*view, path, QStringLiteral("reveal-all")));
        QCOMPARE(view->history_count(), count + 1); saved(true, initial.hasMask, true);
        QVERIFY(pictura::history_api::undo(*view)); saved(false, initial.hasMask, false);
        QVERIFY(pictura::history_api::redo(*view)); saved(true, initial.hasMask, true);

        const int enabledCount = view->history_count();
        QVERIFY(pictura::vector_mask_set_enabled_path(*view, path, false));
        QCOMPARE(view->history_count(), enabledCount + 1); saved(true, initial.hasMask, true, true);
        QVERIFY(pictura::history_api::undo(*view)); saved(true, initial.hasMask, true);
        QVERIFY(pictura::history_api::redo(*view)); saved(true, initial.hasMask, true, true);
        QVERIFY(pictura::vector_mask_set_enabled_path(*view, path, true));
        QCOMPARE(view->history_count(), enabledCount + 2); saved(true, initial.hasMask, true);

        const int removalCount = view->history_count();
        QVERIFY(rasterize ? pictura::vector_mask_rasterize_path(*view, path)
                          : pictura::vector_mask_delete_path(*view, path));
        QCOMPARE(view->history_count(), removalCount + 1); saved(false, initial.hasMask || rasterize, false);
        QVERIFY(pictura::history_api::undo(*view)); saved(true, initial.hasMask, true);
        QVERIFY(pictura::history_api::redo(*view)); saved(false, initial.hasMask || rasterize, false);

        const QString pixel = view->add_layer_in(QString()); QVERIFY(!pixel.isEmpty());
        const auto pixelBefore = pixels(*view);
        QVERIFY(pictura::document_api::saveCopy(*view, file));
        const auto pixelBytes = readFile(file); const auto pixelRecord = record(pixelBytes); QVERIFY(pixelRecord.flags >= 0);
        const auto pixelFlags = word(pixelBytes, pixelRecord.flags, 1);
        const auto pixelState = [&](bool vector) {
            const auto row = rowForPath(*view, pixel);
            QCOMPARE(row.kind, QStringLiteral("pixel")); QCOMPARE(row.hasVectorMask, vector);
            QVERIFY(!row.shape && !row.hasAdjustment && !row.smartObject);
            QCOMPARE(row.lockBits & 3, 0);
            QVERIFY(!pictura::layer_is_editable_fill(*view, pixel)); QCOMPARE(pixels(*view), pixelBefore);
            QVERIFY(pictura::document_api::saveCopy(*view, file));
            const auto bytes = readFile(file); const auto current = record(bytes); QVERIFY(current.flags >= 0);
            QCOMPARE(word(bytes, current.flags, 1), pixelFlags);
            PictureView reopened; QVERIFY(pictura::document_api::open(reopened, file));
            const auto back = rowForPath(reopened, pixel);
            QCOMPARE(back.kind, QStringLiteral("pixel")); QCOMPARE(back.hasVectorMask, vector);
            QVERIFY(!back.shape && !back.hasAdjustment); QCOMPARE(back.lockBits & 3, 0);
            QCOMPARE(pixels(reopened), pixelBefore);
        };
        pixelState(false);
        const int pixelCount = view->history_count();
        QVERIFY(pictura::vector_mask_add_path(*view, pixel, QStringLiteral("reveal-all")));
        QCOMPARE(view->history_count(), pixelCount + 1); pixelState(true);
        QVERIFY(pictura::history_api::undo(*view)); pixelState(false);
        QVERIFY(pictura::history_api::redo(*view)); pixelState(true);
    }
}

QTEST_MAIN(LayerContentTest)
#include "tst_layer_content.moc"
