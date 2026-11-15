// File Info tabs: the eight categories, editable Description/Origin/IPTC
// Extension fields, the read-only GPS and Photoshop sheets, the raw packet,
// and one undo state per accept.

#include <QtTest/QtTest>

#include "api/view_bridge.h"
#include "commands.h"
#include "file_info_dialog.h"
#include "frame.h"

#include <QtGui/QAction>
#include <QtWidgets/QLineEdit>

#include <memory>

#include "qt_test_support.h"

namespace {

struct Fixture {
    std::unique_ptr<pictura::PicturaMainWindow> window;
    pictura::PictureView* view = nullptr;
};

QString metadataFixture()
{
    return QFINDTESTDATA("../../../../crates/pictura-codec/tests/fixtures/metadata.psd");
}

QString twoLayersFixture()
{
    return QFINDTESTDATA("../../../../crates/pictura-codec/tests/fixtures/two_layers.psd");
}

Fixture openFixture(const QString& path = metadataFixture())
{
    Fixture fixture;
    fixture.window = pictura::test::makeMainWindow();
    if (!path.isEmpty() && fixture.window->openDocumentAtPath(path)) {
        fixture.view = fixture.window->activeView();
    }
    return fixture;
}

std::unique_ptr<pictura::FileInfoDialog> makeDialog(Fixture& fixture)
{
    QStringList rows = fixture.view->exif_rows();
    rows += fixture.view->iptc_rows();
    rows += fixture.view->xmp_rows();
    auto dialog = std::make_unique<pictura::FileInfoDialog>(
        rows, fixture.view->file_info_edit_fields(), fixture.view->xmp_packet());
    auto* view = fixture.view;
    dialog->onApply = [view](const QStringList& edits) {
        return view->apply_metadata_edits(edits);
    };
    return dialog;
}

} // namespace

class FileInfoTest : public QObject {
    Q_OBJECT

private slots:
    void fileInfoDisabledWithoutDocument();
    void categoriesInCs6Order();
    void missingValueKeepsEditableControl();
    void descriptionFieldWritesBothChannels();
    void originFieldWritesBothChannels();
    void extensionFieldWritesXmpOnly();
    void sharedFieldMirrorsAcrossCategories();
    void rawAndFieldEditAreOneUndoState();
    void illFormedRawPacketKeepsDialogOpen();
    void extensionRefusalKeepsDialogOpen();
    void refusedExtensionKeepsEarlierState();
    void copyrightStatusWritesMarked();
    void cancelAppliesNothing();
    void acceptWithNoChangesRecordsNothing();
    void gpsRowsShowDecodedValues();
    void photoshopRowsAreReadOnly();
    void xmpOnlySharedFieldPrefillsControl();

private:
    pictura::test::ScopedStateHome stateHome_;
};

void FileInfoTest::fileInfoDisabledWithoutDocument()
{
    auto window = pictura::test::makeMainWindow();
    QAction* action = window->registry()->action(
        QString::fromLatin1(pictura::command_ids::FileInfo));
    QVERIFY(action);
    QVERIFY(!action->isEnabled());
}

void FileInfoTest::categoriesInCs6Order()
{
    Fixture fixture = openFixture();
    QVERIFY(fixture.view);
    auto dialog = makeDialog(fixture);
    const QStringList expected = {
        QStringLiteral("Description"), QStringLiteral("Camera Data"),
        QStringLiteral("IPTC"),        QStringLiteral("IPTC Extension"),
        QStringLiteral("GPS Data"),    QStringLiteral("Origin"),
        QStringLiteral("Photoshop"),   QStringLiteral("Raw Data")};
    QCOMPARE(dialog->categoriesForTest(), expected);
    QVERIFY(dialog->rowsForTest(QStringLiteral("Camera Data"))
                .contains(QStringLiteral("Make\tACME Cameras")));
    QVERIFY(dialog->rowsForTest(QStringLiteral("GPS Data"))
                .contains(QStringLiteral("Latitude\t48.858333")));
    QVERIFY(!dialog->categoriesForTest().contains(QStringLiteral("Video Data")));
    QVERIFY(!dialog->categoriesForTest().contains(QStringLiteral("Audio Data")));
    QVERIFY(!dialog->categoriesForTest().contains(QStringLiteral("DICOM")));
}
void FileInfoTest::missingValueKeepsEditableControl()
{
    pictura::FileInfoDialog dialog(
        {}, {QStringLiteral("Origin\t2:90\tCity\t")}, QString());
    auto* edit = dialog.findChild<QLineEdit*>(QStringLiteral("fileInfoField_2:90"));
    QVERIFY(edit);
    QCOMPARE(edit->text(), QString());
}

void FileInfoTest::descriptionFieldWritesBothChannels()
{
    Fixture fixture = openFixture();
    QVERIFY(fixture.view);
    auto dialog = makeDialog(fixture);
    const int states = fixture.view->history_count();
    dialog->setFieldForTest(QStringLiteral("2:80"), QStringLiteral("New Author"));
    dialog->accept();
    QVERIFY(fixture.view->file_info_edit_fields().contains(
        QStringLiteral("IPTC\t2:80\tBy-line\tNew Author")));
    QVERIFY(fixture.view->xmp_packet().contains(QStringLiteral("New Author")));
    QVERIFY(fixture.view->is_dirty());
    QCOMPARE(fixture.view->history_count(), states + 1);
    QVERIFY(fixture.view->undo());
    QVERIFY(fixture.view->file_info_edit_fields().contains(
        QStringLiteral("IPTC\t2:80\tBy-line\tAda Lovelace")));
}

void FileInfoTest::originFieldWritesBothChannels()
{
    Fixture fixture = openFixture();
    QVERIFY(fixture.view);
    auto dialog = makeDialog(fixture);
    dialog->setFieldForTest(QStringLiteral("2:90"), QStringLiteral("Berlin"));
    dialog->accept();
    QVERIFY(fixture.view->file_info_edit_fields().contains(
        QStringLiteral("Origin\t2:90\tCity\tBerlin")));
    QVERIFY(fixture.view->xmp_packet().contains(QStringLiteral("Berlin")));
    QVERIFY(fixture.view->undo());
}

void FileInfoTest::extensionFieldWritesXmpOnly()
{
    Fixture fixture = openFixture();
    QVERIFY(fixture.view);
    auto dialog = makeDialog(fixture);
    dialog->addListValueForTest(QStringLiteral("Iptc4xmpExt:PersonInImage"),
                                QStringLiteral("New Person"));
    dialog->accept();
    QVERIFY(fixture.view->xmp_packet().contains(QStringLiteral("New Person")));
    QVERIFY(!fixture.view->iptc_rows().join(QLatin1Char('\n'))
                 .contains(QStringLiteral("New Person")));
    QVERIFY(fixture.view->undo());
    QVERIFY(!fixture.view->xmp_packet().contains(QStringLiteral("New Person")));
}

void FileInfoTest::sharedFieldMirrorsAcrossCategories()
{
    pictura::FileInfoDialog dialog(
        {},
        {QStringLiteral("Description\t2:5\tDocument Title\tOld"),
         QStringLiteral("IPTC\t2:5\tObject Name\tOld")},
        QString());
    dialog.setFieldForTest(QStringLiteral("2:5"), QStringLiteral("Shared"));
    const QList<QLineEdit*> edits =
        dialog.findChildren<QLineEdit*>(QStringLiteral("fileInfoField_2:5"));
    QVERIFY(edits.size() >= 2);
    for (const QLineEdit* edit : edits) {
        QCOMPARE(edit->text(), QStringLiteral("Shared"));
    }
}

void FileInfoTest::rawAndFieldEditAreOneUndoState()
{
    Fixture fixture = openFixture();
    QVERIFY(fixture.view);
    auto dialog = makeDialog(fixture);
    QString raw = fixture.view->xmp_packet();
    raw.replace(QStringLiteral("Fixture Title"), QStringLiteral("Raw Title"));
    dialog->setXmpForTest(raw);
    dialog->setFieldForTest(QStringLiteral("2:90"), QStringLiteral("Lyon"));
    const int states = fixture.view->history_count();
    dialog->accept();
    QVERIFY(fixture.view->xmp_packet().contains(QStringLiteral("Raw Title")));
    QVERIFY(fixture.view->xmp_packet().contains(QStringLiteral("Lyon")));
    QCOMPARE(fixture.view->history_count(), states + 1);
    QVERIFY(fixture.view->undo());
    QVERIFY(fixture.view->xmp_packet().contains(QStringLiteral("Fixture Title")));
    QVERIFY(!fixture.view->xmp_packet().contains(QStringLiteral("Lyon")));
}

void FileInfoTest::illFormedRawPacketKeepsDialogOpen()
{
    Fixture fixture = openFixture();
    QVERIFY(fixture.view);
    auto dialog = makeDialog(fixture);
    dialog->setXmpForTest(QStringLiteral("<x:xmpmeta>"));
    dialog->show();
    dialog->accept();
    QVERIFY(dialog->isVisible());
    dialog->reject();
}

void FileInfoTest::extensionRefusalKeepsDialogOpen()
{
    Fixture fixture = openFixture();
    QVERIFY(fixture.view);
    auto dialog = makeDialog(fixture);
    QString raw = fixture.view->xmp_packet();
    raw.replace(QStringLiteral("<Iptc4xmpExt:LocationCreated>"),
                QStringLiteral("<Iptc4xmpExt:LocationCreated><!--keep-->"));
    dialog->setXmpForTest(raw);
    dialog->setFieldForTest(
        QStringLiteral("Iptc4xmpExt:LocationCreated/Iptc4xmpExt:ProvinceState"),
        QStringLiteral("Ile-de-France"));
    dialog->show();
    dialog->accept();
    QVERIFY(dialog->isVisible());
    QVERIFY(fixture.view->xmp_packet().contains(QStringLiteral("<!--keep-->")));
    dialog->reject();
}

void FileInfoTest::refusedExtensionKeepsEarlierState()
{
    Fixture fixture = openFixture();
    QVERIFY(fixture.view);
    auto dialog = makeDialog(fixture);
    const int states = fixture.view->history_count();
    QString raw = fixture.view->xmp_packet();
    raw.replace(QStringLiteral("<Iptc4xmpExt:LocationCreated>"),
                QStringLiteral("<Iptc4xmpExt:LocationCreated><!--keep-->"));
    dialog->setXmpForTest(raw);
    dialog->setFieldForTest(QStringLiteral("2:90"), QStringLiteral("Berlin"));
    dialog->setFieldForTest(
        QStringLiteral("Iptc4xmpExt:LocationCreated/Iptc4xmpExt:ProvinceState"),
        QStringLiteral("Ile-de-France"));
    dialog->show();
    dialog->accept();
    QVERIFY(dialog->isVisible());
    QCOMPARE(fixture.view->history_count(), states + 1);
    QVERIFY(fixture.view->file_info_edit_fields().contains(
        QStringLiteral("Origin\t2:90\tCity\tBerlin")));
    QVERIFY(fixture.view->xmp_packet().contains(QStringLiteral("Berlin")));
    QVERIFY(fixture.view->undo());
    QVERIFY(fixture.view->file_info_edit_fields().contains(
        QStringLiteral("Origin\t2:90\tCity\tParis")));
    dialog->reject();
}

void FileInfoTest::copyrightStatusWritesMarked()
{
    Fixture fixture = openFixture();
    QVERIFY(fixture.view);
    auto dialog = makeDialog(fixture);
    const int states = fixture.view->history_count();
    dialog->setFieldForTest(QStringLiteral("xmpRights:Marked"),
                            QStringLiteral("Public Domain"));
    dialog->accept();
    QVERIFY(fixture.view->xmp_packet().contains(
        QStringLiteral("<xmpRights:Marked>False</xmpRights:Marked>")));
    QCOMPARE(fixture.view->history_count(), states + 1);
    QVERIFY(fixture.view->undo());
    QVERIFY(fixture.view->xmp_packet().contains(
        QStringLiteral("<xmpRights:Marked>True</xmpRights:Marked>")));
}

void FileInfoTest::cancelAppliesNothing()
{
    Fixture fixture = openFixture();
    QVERIFY(fixture.view);
    auto dialog = makeDialog(fixture);
    const int states = fixture.view->history_count();
    const QString before = fixture.view->file_info_edit_fields().join(QLatin1Char('\n'));
    dialog->setFieldForTest(QStringLiteral("2:90"), QStringLiteral("Nowhere"));
    dialog->reject();
    QCOMPARE(fixture.view->history_count(), states);
    QCOMPARE(fixture.view->file_info_edit_fields().join(QLatin1Char('\n')), before);
}

void FileInfoTest::acceptWithNoChangesRecordsNothing()
{
    Fixture fixture = openFixture();
    QVERIFY(fixture.view);
    auto dialog = makeDialog(fixture);
    const int states = fixture.view->history_count();
    const QString before = fixture.view->file_info_edit_fields().join(QLatin1Char('\n'));
    dialog->accept();
    QCOMPARE(fixture.view->history_count(), states);
    QCOMPARE(fixture.view->file_info_edit_fields().join(QLatin1Char('\n')), before);
}

void FileInfoTest::gpsRowsShowDecodedValues()
{
    Fixture fixture = openFixture();
    QVERIFY(fixture.view);
    QVERIFY(fixture.view->exif_rows().contains(
        QStringLiteral("GPS Data\tLatitude\t48.858333")));
}

void FileInfoTest::photoshopRowsAreReadOnly()
{
    Fixture fixture = openFixture();
    QVERIFY(fixture.view);
    const QStringList rows = fixture.view->xmp_rows();
    QVERIFY(rows.contains(QStringLiteral("Photoshop\tHistory\tFixture history")));
    QVERIFY(rows.join(QLatin1Char('\n')).contains(QStringLiteral("MetadataDate")));
}

void FileInfoTest::xmpOnlySharedFieldPrefillsControl()
{
    Fixture fixture = openFixture(twoLayersFixture());
    QVERIFY(fixture.view);
    auto dialog = makeDialog(fixture);
    const QString packet = QStringLiteral(
        "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">"
        "<rdf:Description xmlns:photoshop=\"http://ns.adobe.com/photoshop/1.0/\">"
        "<photoshop:City>Paris</photoshop:City></rdf:Description></rdf:RDF></x:xmpmeta>");
    dialog->setXmpForTest(packet);
    dialog->accept();
    auto reopened = makeDialog(fixture);
    QCOMPARE(reopened->fieldForTest(QStringLiteral("2:90")), QStringLiteral("Paris"));
}

QTEST_MAIN(FileInfoTest)
#include "tst_file_info.moc"
