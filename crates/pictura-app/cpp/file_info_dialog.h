#pragma once

#include <QtCore/QHash>
#include <QtCore/QList>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtWidgets/QDialog>

#include <functional>

class QComboBox;
class QLineEdit;
class QListWidget;
class QPlainTextEdit;
class QStackedWidget;
class QWidget;

namespace pictura {

// CS6 File Info: a category list over stacked pages. Read-only sheets for
// Camera Data, GPS Data, and Photoshop; editable forms for Description, IPTC,
// Origin, and IPTC Extension; an editable raw XMP packet.
class FileInfoDialog : public QDialog {
    Q_OBJECT

public:
    // `readOnlyRows` are `"Section\tLabel\tValue"`; `editFields` are
    // `"Section\tid\tLabel\tValue"` with repeated ids for list values.
    FileInfoDialog(const QStringList& readOnlyRows, const QStringList& editFields,
                   const QString& xmp, QWidget* parent = nullptr);

    // Changed fields as `"id\tValue"` rows plus the raw packet row.
    QStringList edits() const;

    // Metadata-template controls. The frame sets these.
    std::function<void()> onExportTemplate;
    std::function<void(int)> onApplyTemplate;
    // Applies the changed rows. Returning false keeps the dialog open.
    std::function<bool(const QStringList&)> onApply;

    int templateMode() const;

    // Test hooks.
    QStringList categoriesForTest() const;
    QStringList rowsForTest(const QString& category) const;
    QString xmpForTest() const;
    QString fieldForTest(const QString& id) const;
    void setFieldForTest(const QString& id, const QString& value);
    void addListValueForTest(const QString& id, const QString& value);
    QStringList listValuesForTest(const QString& id) const;
    void setXmpForTest(const QString& packet);
    bool hasTemplateControlsForTest() const;
    int mergeModeForTest() const;
    void setMergeModeForTest(int mode);

public:
    void accept() override;

private:
    QWidget* buildEditablePage(const QString& section);
    QWidget* buildTablePage(const QString& section);
    QWidget* buildRawPage();
    QWidget* buildListField(const QString& id, const QStringList& values, QWidget* parent);
    QString comboValue(const QString& id) const;

    QStringList readOnlyRows_;
    QStringList editFields_;
    QString xmp_;
    QHash<QString, QStringList> rowsBySection_;
    QHash<QString, QStringList> fieldsBySection_;
    QListWidget* categoryList_ = nullptr;
    QStackedWidget* stack_ = nullptr;
    QPlainTextEdit* xmpEdit_ = nullptr;
    QComboBox* templateMode_ = nullptr;
    QHash<QString, QList<QLineEdit*>> fields_;
    QHash<QString, QComboBox*> combos_;
    QHash<QString, QListWidget*> lists_;
    QHash<QString, QString> initial_;
    QHash<QString, QStringList> initialLists_;
    bool updating_ = false;
};

} // namespace pictura
