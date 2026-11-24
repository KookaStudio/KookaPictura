#include <QtTest/QtTest>

#include <QtCore/QEvent>
#include <QtCore/QFile>
#include <QtCore/QPointer>
#include <QtCore/QtEndian>
#include <QtGui/QAction>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMenu>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QStatusBar>
#include <QtWidgets/QToolButton>

#include "api/document_bridge.h"
#include "api/paint_bridge.h"
#include "api/transform_bridge.h"
#include "commands.h"
#include "document_registry.h"
#include "image_view.h"
#include "panels/info_panel.h"
#include "frame.h"
#include "qt_test_support.h"
#include "tools.h"

using namespace pictura;

namespace {
const QString kDefault = QStringLiteral("image.analysis.set.measurement.scale.default");
const QString kCustom = QStringLiteral("image.analysis.set.measurement.scale.custom");

class ScopedEnvironment {
public:
    ScopedEnvironment(const char* name, const QByteArray& value)
        : name_(name), prior_(qgetenv(name))
    {
        qputenv(name_, value);
    }
    ~ScopedEnvironment()
    {
        if (prior_.isNull()) {
            qunsetenv(name_);
        } else {
            qputenv(name_, prior_);
        }
    }

private:
    const char* name_;
    QByteArray prior_;
};

struct Fixture {
    QTemporaryDir data;
    ScopedEnvironment edition;
    ScopedEnvironment dataHome;
    test::ScopedStateHome stateHome;
    std::unique_ptr<PicturaMainWindow> window;

    explicit Fixture(const QByteArray& value = "extended")
        : edition("PICTURA_EDITION", value), dataHome("XDG_DATA_HOME", data.path().toUtf8()),
          window(test::makeMainWindow())
    {
    }

    bool newDocument(const QString& name = QStringLiteral("Calibration"))
    {
        return window->newDocument(name, 128, 128, QStringLiteral("rgb"), 8,
                                   QStringLiteral("white"));
    }

    QDialog* custom()
    {
        if (!window->registry()->dispatch(kCustom)) {
            return nullptr;
        }
        return window->findChild<QDialog*>(QStringLiteral("measurementScaleDialog"));
    }

    QMenu* menu() const
    {
        auto* menu = window->findChild<QMenu*>(QStringLiteral("measurementScaleMenu"));
        if (menu) {
            emit menu->aboutToShow();
        }
        return menu;
    }
};

QLineEdit* field(QDialog& dialog, const char* name)
{
    return dialog.findChild<QLineEdit*>(QLatin1String(name));
}

void fillScale(QDialog& dialog, const QString& pixels, const QString& logical,
               const QString& units)
{
    field(dialog, "measurementPixelLength")->setText(pixels);
    field(dialog, "measurementLogicalLength")->setText(logical);
    field(dialog, "measurementLogicalUnits")->setText(units);
}

QPushButton* button(QDialog& dialog, const char* name)
{
    return dialog.findChild<QPushButton*>(QLatin1String(name));
}

QAction* namedAction(QMenu& menu, const QString& name)
{
    for (auto* action : menu.actions()) {
        if (action->text() == name) {
            return action;
        }
    }
    return nullptr;
}

QByteArray readBytes(const QString& path)
{
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray{};
}

QByteArray opaqueBlock()
{
    return QByteArray::fromHex("3842494d043200000000000400000011");
}

QByteArray scaleResource(const QByteArray& bytes)
{
    if (bytes.size() < 34 || !bytes.startsWith("8BPS")) {
        return {};
    }
    const qsizetype lengthOffset = 30 + qsizetype(qFromBigEndian<quint32>(bytes.constData() + 26));
    if (lengthOffset + 4 > bytes.size()) {
        return {};
    }
    qsizetype offset = lengthOffset + 4;
    const qsizetype end = offset + qFromBigEndian<quint32>(bytes.constData() + lengthOffset);
    while (end <= bytes.size() && offset + 12 <= end) {
        const auto* resource = bytes.constData() + offset;
        const auto pascalSize = qsizetype(quint8(resource[6])) + 1;
        const auto lengthAt = offset + 6 + pascalSize + pascalSize % 2;
        if (lengthAt + 4 > end) {
            break;
        }
        const qsizetype payload = qFromBigEndian<quint32>(bytes.constData() + lengthAt);
        const qsizetype next = lengthAt + 4 + payload + payload % 2;
        if (next > end) {
            break;
        }
        if (qFromBigEndian<quint16>(resource + 4) == 1074) {
            return bytes.mid(offset, next - offset);
        }
        offset = next;
    }
    return {};
}

bool makeOpaquePsd(Fixture& fixture, const QString& path)
{
    const QString seed = fixture.data.filePath(QStringLiteral("seed.psd"));
    if (!fixture.window->saveActiveAs(seed)) {
        return false;
    }
    QByteArray bytes = readBytes(seed);
    if (bytes.size() < 34) {
        return false;
    }
    const qsizetype lengthOffset = 30 + qFromBigEndian<quint32>(bytes.constData() + 26);
    if (lengthOffset + 4 > bytes.size()) {
        return false;
    }
    const quint32 resourceLength = qFromBigEndian<quint32>(bytes.constData() + lengthOffset);
    const QByteArray block = opaqueBlock();
    qToBigEndian<quint32>(resourceLength + static_cast<quint32>(block.size()),
                         bytes.data() + lengthOffset);
    bytes.insert(lengthOffset + 4, block);
    QFile file(path);
    return file.open(QIODevice::WriteOnly) && file.write(bytes) == bytes.size();
}
} // namespace

class MeasurementScaleTest : public QObject {
    Q_OBJECT

private slots:
    void rejectsInvalidCalibration();
    void calibratesFromRulerAndTypedInput();
    void restoresScaleThroughHistoryAndTabSwitch();
    void cancelAndUnchangedScaleAreNoops();
    void standardKeepsUncalibratedRuler();
    void persistsAndDeletesPresetsIndependently();
    void cancelsDraftOnDocumentChange();
    void checksCurrentScale();
    void resampleResetsCalibration();
    void backgroundResampleNotice();
    void perspectiveWarpNotice();
    void resizePreservesCalibration();
    void resamplePreservesOpaqueScale();
    void opaqueResourceResetIsUndoable();
    void preservesCalibrationThroughEditionSave();
};

void MeasurementScaleTest::rejectsInvalidCalibration()
{
    Fixture fixture;
    QVERIFY(fixture.newDocument());
    auto* view = fixture.window->activeView();
    const auto before = view->history_count();
    auto* dialog = fixture.custom();
    QVERIFY(dialog);
    for (const auto& pixels : {QStringLiteral("0"), QStringLiteral("-1"),
                               QStringLiteral("1.25"), QStringLiteral("2147483648")}) {
        fillScale(*dialog, pixels, QStringLiteral("1"), QStringLiteral("mm"));
        QVERIFY(!button(*dialog, "measurementScaleApply")->isEnabled());
    }
    fillScale(*dialog, QStringLiteral("50"), QStringLiteral("NaN"), QStringLiteral("mm"));
    QVERIFY(!button(*dialog, "measurementScaleApply")->isEnabled());
    fillScale(*dialog, QStringLiteral("50"), QStringLiteral("1"), QStringLiteral("  "));
    QVERIFY(!button(*dialog, "measurementScaleApply")->isEnabled());
    QCOMPARE(view->history_count(), before);
    QVERIFY(measurement_scale_values(*view).empty());
}

void MeasurementScaleTest::calibratesFromRulerAndTypedInput()
{
    Fixture fixture;
    QVERIFY(fixture.newDocument());
    auto* dialog = fixture.custom();
    QVERIFY(dialog);
    QCOMPARE(fixture.window->activeTool(), ToolId::Ruler);
    auto* canvas = fixture.window->imageView();
    emit canvas->mousePressed(QPointF(10, 10), Qt::LeftButton, Qt::NoModifier);
    emit canvas->mouseMoved(QPointF(59.6, 10));
    emit canvas->mouseReleased(QPointF(59.6, 10));
    QCOMPARE(field(*dialog, "measurementPixelLength")->text(), QStringLiteral("50"));
    fillScale(*dialog, QStringLiteral("40"), QStringLiteral("1"), QStringLiteral("µm"));
    button(*dialog, "measurementScaleApply")->click();
    auto* view = fixture.window->activeView();
    QCOMPARE(measurement_scale_values(*view)[0], 40.0);

    dialog = fixture.custom();
    QVERIFY(dialog);
    fillScale(*dialog, QStringLiteral("50"), QStringLiteral("1"), QStringLiteral("µm"));
    button(*dialog, "measurementScaleApply")->click();
    clear_ruler(*view);
    emit canvas->mousePressed(QPointF(10, 20), Qt::LeftButton, Qt::NoModifier);
    emit canvas->mouseMoved(QPointF(110, 20));
    emit canvas->mouseReleased(QPointF(110, 20));
    auto* options = fixture.window->findChild<QLabel*>(QStringLiteral("optionsRulerReadout"));
    QVERIFY(options);
    QVERIFY(options->text().contains(QStringLiteral("2 µm")));
    auto* info = fixture.window->findChild<QLabel*>(QStringLiteral("infoMeasurementScale"));
    QVERIFY(info);
    auto* panel = fixture.window->findChild<InfoPanel*>(QStringLiteral("infoPanel"));
    QVERIFY(panel);
    QMenu infoMenu;
    auto* panelOptions = infoMenu.addAction(QStringLiteral("Panel Options…"));
    panel->updatePanelMenu(&infoMenu);
    QVERIFY(panelOptions->menu());
    auto* showScale = namedAction(*panelOptions->menu(), QStringLiteral("Measurement Scale"));
    QVERIFY(showScale);
    showScale->trigger();
    QVERIFY(!info->isHidden());
    QCOMPARE(info->text(), QStringLiteral("50 px = 1 µm"));
    auto* status = fixture.window->findChild<QToolButton*>(QStringLiteral("statusOptionsButton"));
    QVERIFY(status);
    auto* scaleMode = namedAction(*status->menu(), QStringLiteral("Measurement Scale"));
    QVERIFY(scaleMode);
    scaleMode->trigger();
    auto* text = fixture.window->findChild<QLabel*>(QStringLiteral("statusSizeLabel"));
    QVERIFY(text);
    QCOMPARE(text->text(), QStringLiteral("50 px = 1 µm"));
}

void MeasurementScaleTest::restoresScaleThroughHistoryAndTabSwitch()
{
    Fixture fixture;
    QVERIFY(fixture.newDocument());
    auto* one = fixture.window->activeView();
    const auto before = one->history_count();
    QVERIFY(apply_measurement_scale(*one, 50, 1, QStringLiteral("cm")));
    QCOMPARE(one->history_count(), before + 1);
    QVERIFY(fixture.newDocument(QStringLiteral("Second")));
    auto* two = fixture.window->activeView();
    QVERIFY(apply_measurement_scale(*two, 100, 2, QStringLiteral("mm")));
    fixture.window->activateView(one);
    QVERIFY(fixture.window->registry()->dispatch(kDefault));
    QCOMPARE(one->history_count(), before + 2);
    QVERIFY(measurement_scale_values(*one).empty());
    QVERIFY(one->undo());
    QCOMPARE(measurement_scale_units(*one), QStringLiteral("cm"));
    QVERIFY(one->redo());
    QVERIFY(measurement_scale_values(*one).empty());
    fixture.window->activateView(two);
    QCOMPARE(measurement_scale_units(*two), QStringLiteral("mm"));
}

void MeasurementScaleTest::cancelAndUnchangedScaleAreNoops()
{
    Fixture fixture;
    QVERIFY(fixture.newDocument());
    auto* view = fixture.window->activeView();
    QVERIFY(apply_measurement_scale(*view, 50, 1, QStringLiteral("mm")));
    const auto before = view->history_count();
    const bool dirty = view->is_dirty();
    auto* dialog = fixture.custom();
    QVERIFY(dialog);
    fillScale(*dialog, QStringLiteral("42"), QStringLiteral("1"), QStringLiteral("in"));
    dialog->reject();
    QCoreApplication::sendPostedEvents(nullptr, QEvent::DeferredDelete);
    dialog = fixture.custom();
    QVERIFY(dialog);
    button(*dialog, "measurementScaleApply")->click();
    QCOMPARE(view->history_count(), before);
    QCOMPARE(view->is_dirty(), dirty);
    QCOMPARE(measurement_scale_values(*view)[0], 50.0);
    QCOMPARE(measurement_scale_units(*view), QStringLiteral("mm"));
}

void MeasurementScaleTest::standardKeepsUncalibratedRuler()
{
    Fixture fixture("standard");
    QVERIFY(fixture.newDocument());
    QVERIFY(!fixture.window->registry()->action(kDefault));
    QVERIFY(!fixture.window->registry()->action(kCustom));
    fixture.window->setActiveTool(ToolId::Ruler);
    QCOMPARE(fixture.window->activeTool(), ToolId::Ruler);
    auto* view = fixture.window->activeView();
    set_ruler(*view, 0, 0, 3, 4, false);
    QCOMPARE(ruler_measurement(*view)[5], 5.0);
    auto* status = fixture.window->findChild<QToolButton*>(QStringLiteral("statusOptionsButton"));
    QVERIFY(status);
    QVERIFY(!namedAction(*status->menu(), QStringLiteral("Measurement Scale")));
}

void MeasurementScaleTest::persistsAndDeletesPresetsIndependently()
{
    Fixture fixture;
    QVERIFY(fixture.newDocument());
    auto* dialog = fixture.custom();
    QVERIFY(dialog);
    fillScale(*dialog, QStringLiteral("50"), QStringLiteral("1"), QStringLiteral("µm"));
    auto* presets = dialog->findChild<QComboBox*>(QStringLiteral("measurementScalePresets"));
    QVERIFY(presets);
    const QString name = QStringLiteral("Microscopy");
    presets->setEditText(name);
    button(*dialog, "measurementScaleSavePreset")->click();
    fixture.window.reset();
    fixture.window = test::makeMainWindow();
    QVERIFY(fixture.newDocument());
    auto* one = fixture.window->activeView();
    auto* preset = namedAction(*fixture.menu(), name);
    QVERIFY(preset);
    preset->trigger();
    QVERIFY(fixture.newDocument(QStringLiteral("Second")));
    auto* two = fixture.window->activeView();
    preset = namedAction(*fixture.menu(), name);
    QVERIFY(preset);
    preset->trigger();
    dialog = fixture.custom();
    QVERIFY(dialog);
    presets = dialog->findChild<QComboBox*>(QStringLiteral("measurementScalePresets"));
    presets->setEditText(name);
    button(*dialog, "measurementScaleDeletePreset")->click();
    QVERIFY(!namedAction(*fixture.menu(), name));
    QCOMPARE(measurement_scale_values(*one)[0], 50.0);
    QCOMPARE(measurement_scale_values(*two)[0], 50.0);
}

void MeasurementScaleTest::cancelsDraftOnDocumentChange()
{
    Fixture fixture;
    QVERIFY(fixture.newDocument());
    auto* one = fixture.window->activeView();
    QVERIFY(fixture.newDocument(QStringLiteral("Second")));
    auto* two = fixture.window->activeView();
    const auto history = two->history_count();
    auto* dialog = fixture.custom();
    QVERIFY(dialog && dialog->isVisible());
    fillScale(*dialog, QStringLiteral("12"), QStringLiteral("1"), QStringLiteral("mm"));
    fixture.window->imageView()->setFocus();
    QVERIFY(dialog->isVisible());
    auto* registry = fixture.window->documentRegistry();
    const int secondIndex = registry->indexOfView(two);
    QVERIFY(secondIndex >= 0);
    QVERIFY(registry->addDocumentWindowForTest(registry->docs().at(secondIndex)));
    QCOMPARE(fixture.window->activeView(), two);
    QVERIFY(dialog->isVisible());
    fixture.window->activateView(one);
    QVERIFY(!dialog->isVisible());
    QCOMPARE(two->history_count(), history);
    QCoreApplication::sendPostedEvents(nullptr, QEvent::DeferredDelete);
    fixture.window->activateView(two);
    dialog = fixture.custom();
    QVERIFY(dialog && dialog->isVisible());
    const QPointer<QDialog> guarded(dialog);
    fixture.window->closeDocument(fixture.window->activeDocumentIndex(), false);
    QVERIFY(!guarded || !guarded->isVisible());
    QVERIFY(measurement_scale_values(*one).empty());
}

void MeasurementScaleTest::checksCurrentScale()
{
    Fixture fixture;
    QVERIFY(fixture.newDocument());
    auto* one = fixture.window->activeView();
    QVERIFY(fixture.menu());
    QVERIFY(fixture.window->registry()->action(kDefault)->isChecked());
    auto* dialog = fixture.custom();
    QVERIFY(dialog);
    auto* presets = dialog->findChild<QComboBox*>(QStringLiteral("measurementScalePresets"));
    QVERIFY(presets);
    fillScale(*dialog, QStringLiteral("50"), QStringLiteral("1"), QStringLiteral("mm"));
    for (const auto& name : {QStringLiteral("Zulu"), QStringLiteral("Alpha")}) {
        presets->setEditText(name);
        button(*dialog, "measurementScaleSavePreset")->click();
    }
    button(*dialog, "measurementScaleApply")->click();
    QVERIFY(namedAction(*fixture.menu(), QStringLiteral("Alpha"))->isChecked());
    dialog = fixture.custom();
    QVERIFY(dialog);
    presets = dialog->findChild<QComboBox*>(QStringLiteral("measurementScalePresets"));
    QVERIFY(presets);
    presets->setEditText(QStringLiteral("Alpha"));
    button(*dialog, "measurementScaleDeletePreset")->click();
    QVERIFY(namedAction(*fixture.menu(), QStringLiteral("Zulu"))->isChecked());
    dialog->reject();
    QVERIFY(apply_measurement_scale(*one, 100, 2, QStringLiteral("mm")));
    fixture.menu();
    QVERIFY(fixture.window->registry()->action(kCustom)->isChecked());
    QVERIFY(one->undo());
    QVERIFY(namedAction(*fixture.menu(), QStringLiteral("Zulu"))->isChecked());
    QVERIFY(one->redo());
    fixture.menu();
    QVERIFY(fixture.window->registry()->action(kCustom)->isChecked());
    QVERIFY(apply_measurement_scale(*one, 1, 1, QStringLiteral("in")));
    QVERIFY(fixture.window->registry()->action(kCustom)->isChecked());
    QVERIFY(fixture.newDocument(QStringLiteral("Second")));
    fixture.menu();
    QVERIFY(fixture.window->registry()->action(kDefault)->isChecked());
    fixture.window->activateView(one);
    fixture.menu();
    QVERIFY(fixture.window->registry()->action(kCustom)->isChecked());
    QVERIFY(fixture.window->registry()->dispatch(kDefault));
    QVERIFY(fixture.window->registry()->action(kDefault)->isChecked());
}

void MeasurementScaleTest::resampleResetsCalibration()
{
    for (const bool uniform : {true, false}) {
        Fixture fixture;
        QVERIFY(fixture.newDocument());
        auto* originalView = fixture.window->activeView();
        QVERIFY(apply_measurement_scale(*originalView, 50, 1, QStringLiteral("mm")));
        const auto seed = fixture.data.filePath(QStringLiteral("seed.psd"));
        const auto original = fixture.data.filePath(QStringLiteral("original.psd"));
        QVERIFY(fixture.window->saveActiveAs(seed));
        QVERIFY(QFile::copy(seed, original));
        const auto originalResource = scaleResource(readBytes(original));
        QVERIFY(!originalResource.isEmpty());
        QVERIFY(fixture.window->openDocumentAtPath(original, false));
        QTRY_VERIFY(fixture.window->activeView() && fixture.window->activeView() != originalView
                    && fixture.window->activeView()->has_document());
        auto* view = fixture.window->activeView();
        if (uniform) {
            QVERIFY(image_size_apply(*view, QStringLiteral("nearest"), 256, 256, 72.0, false));
        } else {
            QVERIFY(view->resize_image(QStringLiteral("nearest"), 256, 128));
        }
        QVERIFY(measurement_scale_values(*view).empty());
        QVERIFY(measurement_scale_needs_recalibration(*view));
        QVERIFY(fixture.window->statusBar()->currentMessage().contains(QStringLiteral("Recalibrate")));
        const auto reset = fixture.data.filePath(QStringLiteral("resampled.psd"));
        QVERIFY(fixture.window->saveActiveAs(reset));
        QVERIFY(scaleResource(readBytes(reset)).isEmpty());
        QVERIFY(view->undo());
        QCOMPARE(measurement_scale_values(*view)[0], 50.0);
        QVERIFY(!measurement_scale_needs_recalibration(*view));
        const auto undoSave = fixture.data.filePath(QStringLiteral("restored.psd"));
        QVERIFY(fixture.window->saveActiveAs(undoSave));
        QCOMPARE(scaleResource(readBytes(undoSave)), originalResource);

        QVERIFY(fixture.window->openDocumentAtPath(reset, false));
        QTRY_VERIFY(fixture.window->activeView() && fixture.window->activeView() != view
                    && fixture.window->activeView()->has_document());
        auto* reopenedReset = fixture.window->activeView();
        QVERIFY(measurement_scale_values(*reopenedReset).empty());
        const auto restoredCopy = fixture.data.filePath(QStringLiteral("restored-copy.psd"));
        QVERIFY(QFile::copy(undoSave, restoredCopy));
        QVERIFY(fixture.window->openDocumentAtPath(restoredCopy, false));
        QTRY_VERIFY(fixture.window->activeView() && fixture.window->activeView() != reopenedReset
                    && fixture.window->activeView()->has_document());
        QCOMPARE(measurement_scale_values(*fixture.window->activeView())[0], 50.0);
    }
}

void MeasurementScaleTest::backgroundResampleNotice()
{
    Fixture fixture;
    QVERIFY(fixture.newDocument());
    auto* first = fixture.window->activeView();
    QVERIFY(apply_measurement_scale(*first, 50, 1, QStringLiteral("mm")));
    QVERIFY(fixture.newDocument(QStringLiteral("Second")));
    fixture.window->statusBar()->clearMessage();
    QVERIFY(first->resize_image(QStringLiteral("nearest"), 256, 128));
    QVERIFY(!fixture.window->statusBar()->currentMessage().contains(QStringLiteral("Recalibrate")));
    QVERIFY(first->undo());
    fixture.window->activateView(first);
    QCoreApplication::processEvents();
    QVERIFY(!fixture.window->statusBar()->currentMessage().contains(QStringLiteral("Recalibrate")));
    fixture.window->activateView(fixture.window->documentRegistry()->viewAt(1));
    QVERIFY(first->redo());
    fixture.window->activateView(first);
    QTRY_VERIFY(fixture.window->statusBar()->currentMessage().contains(QStringLiteral("Recalibrate")));
    QVERIFY(first->undo());
    QVERIFY(!fixture.window->statusBar()->currentMessage().contains(QStringLiteral("Recalibrate")));
    QVERIFY(first->redo());
    QVERIFY(fixture.window->statusBar()->currentMessage().contains(QStringLiteral("Recalibrate")));
    fixture.window->activateView(fixture.window->documentRegistry()->viewAt(1));
    QVERIFY(!fixture.window->statusBar()->currentMessage().contains(QStringLiteral("Recalibrate")));
}

void MeasurementScaleTest::perspectiveWarpNotice()
{
    Fixture fixture;
    QVERIFY(fixture.newDocument());
    auto* view = fixture.window->activeView();
    QVERIFY(apply_measurement_scale(*view, 50, 1, QStringLiteral("mm")));
    const double corners[] = {0, 0, 128, 0, 120, 128, 0, 128};
    QVERIFY(perspective_crop_commit_sized(*view, ::rust::Slice<const double>(corners, 8), 128, 128));
    QVERIFY(measurement_scale_needs_recalibration(*view));
    QVERIFY(fixture.window->statusBar()->currentMessage().contains(QStringLiteral("Recalibrate")));
    QVERIFY(view->undo());
    QCOMPARE(measurement_scale_values(*view)[0], 50.0);
    QVERIFY(!fixture.window->statusBar()->currentMessage().contains(QStringLiteral("Recalibrate")));
}

void MeasurementScaleTest::resizePreservesCalibration()
{
    Fixture fixture;
    QVERIFY(fixture.newDocument());
    auto* view = fixture.window->activeView();
    QVERIFY(apply_measurement_scale(*view, 50, 1, QStringLiteral("mm")));
    QVERIFY(image_size_apply(*view, QString(), 128, 128, 150.0, false));
    const auto before = view->history_count();
    QVERIFY(!image_size_apply(*view, QStringLiteral("nearest"), 128, 128, 150.0, false));
    QVERIFY(!view->resize_image(QStringLiteral("nearest"), 0, 128));
    QCOMPARE(view->history_count(), before);
    QCOMPARE(measurement_scale_values(*view)[0], 50.0);
    QVERIFY(!measurement_scale_needs_recalibration(*view));
}

void MeasurementScaleTest::resamplePreservesOpaqueScale()
{
    for (const QByteArray& edition : {QByteArray("extended"), QByteArray("standard")}) {
        Fixture fixture(edition);
        QVERIFY(fixture.newDocument());
        const auto input = fixture.data.filePath(QStringLiteral("opaque.psd"));
        QVERIFY(makeOpaquePsd(fixture, input));
        auto* original = fixture.window->activeView();
        QVERIFY(fixture.window->openDocumentAtPath(input, false));
        QTRY_VERIFY(fixture.window->activeView() && fixture.window->activeView() != original
                    && fixture.window->activeView()->has_document());
        auto* view = fixture.window->activeView();
        QVERIFY(view->resize_image(QStringLiteral("nearest"), 256, 128));
        QVERIFY(measurement_scale_needs_recalibration(*view));
        QVERIFY(fixture.window->statusBar()->currentMessage().contains(QStringLiteral("Recalibrate")));
        const auto output = fixture.data.filePath(QStringLiteral("resized.psd"));
        QVERIFY(fixture.window->saveActiveAs(output));
        QCOMPARE(scaleResource(readBytes(output)), opaqueBlock());
        const auto reopened = fixture.data.filePath(QStringLiteral("reopened.psd"));
        QVERIFY(QFile::copy(output, reopened));
        QVERIFY(fixture.window->openDocumentAtPath(reopened, false));
        QTRY_VERIFY(fixture.window->activeView() && fixture.window->activeView() != view
                    && fixture.window->activeView()->has_document());
        QVERIFY(measurement_scale_values(*fixture.window->activeView()).empty());
    }
}

void MeasurementScaleTest::opaqueResourceResetIsUndoable()
{
    Fixture fixture;
    QVERIFY(fixture.newDocument());
    const auto input = fixture.data.filePath(QStringLiteral("opaque.psd"));
    QVERIFY(makeOpaquePsd(fixture, input));
    auto* original = fixture.window->activeView();
    QVERIFY(fixture.window->openDocumentAtPath(input, false));
    QTRY_VERIFY(fixture.window->activeView() && fixture.window->activeView() != original
                && fixture.window->activeView()->has_document());
    auto* view = fixture.window->activeView();
    QVERIFY(reset_measurement_scale(*view));
    const auto reset = fixture.data.filePath(QStringLiteral("reset.psd"));
    QVERIFY(fixture.window->saveActiveAs(reset));
    QVERIFY(!readBytes(reset).contains(opaqueBlock()));
    QVERIFY(view->undo());
    const auto undone = fixture.data.filePath(QStringLiteral("undone.psd"));
    QVERIFY(fixture.window->saveActiveAs(undone));
    QVERIFY(readBytes(undone).contains(opaqueBlock()));
}

void MeasurementScaleTest::preservesCalibrationThroughEditionSave()
{
    for (const QByteArray& edition : {QByteArray("extended"), QByteArray("standard")}) {
        for (const bool recognized : {true, false}) {
            Fixture fixture(edition);
            QVERIFY(fixture.newDocument());
            const auto input = fixture.data.filePath(QStringLiteral("input.psd"));
            if (recognized) {
                const auto seed = fixture.data.filePath(QStringLiteral("seed.psd"));
                ScopedEnvironment authoringEdition("PICTURA_EDITION", "extended");
                QVERIFY(apply_measurement_scale(*fixture.window->activeView(), 75, 2.5,
                                                QStringLiteral("µm")));
                QVERIFY(fixture.window->saveActiveAs(seed));
                QVERIFY(QFile::copy(seed, input));
            } else {
                QVERIFY(makeOpaquePsd(fixture, input));
            }
            const auto originalResource = scaleResource(readBytes(input));
            QVERIFY(!originalResource.isEmpty());
            auto* original = fixture.window->activeView();
            QVERIFY(fixture.window->openDocumentAtPath(input, false));
            QTRY_VERIFY(fixture.window->activeView() && fixture.window->activeView() != original
                        && fixture.window->activeView()->has_document());
            auto* view = fixture.window->activeView();
            QCOMPARE(!measurement_scale_values(*view).empty(), recognized);
            QVERIFY(image_size_apply(*view, QString(), 128, 128, 150.0, false));
            const auto output = fixture.data.filePath(QStringLiteral("resolution.psd"));
            QVERIFY(fixture.window->saveActiveAs(output));
            QCOMPARE(scaleResource(readBytes(output)), originalResource);
        }
    }
}

QTEST_MAIN(MeasurementScaleTest)
#include "tst_measurement_scale.moc"
