#include "file_info_dialog.h"

#include <QtWidgets/QAbstractItemView>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QHeaderView>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QPlainTextEdit>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QTableWidget>
#include <QtWidgets/QTableWidgetItem>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QWidget>

namespace pictura {

const QString FileInfoDialog::kCameraData = QStringLiteral("Camera Data");
const QString FileInfoDialog::kDescription = QStringLiteral("Description");
const QString FileInfoDialog::kIptc = QStringLiteral("IPTC");
const QString FileInfoDialog::kRawData = QStringLiteral("Raw Data");

namespace {

// A read-only two-column view of one category's `"Label\tValue"` rows.
QTableWidget* makeRowTable(const QStringList& rows, QWidget* parent)
{
    auto* table = new QTableWidget(rows.size(), 2, parent);
    table->setEditTriggers(QAbstractItemView::NoEditTriggers);
    table->setSelectionMode(QAbstractItemView::NoSelection);
    table->verticalHeader()->setVisible(false);
    table->horizontalHeader()->setStretchLastSection(true);
    table->setHorizontalHeaderLabels({QObject::tr("Field"), QObject::tr("Value")});
    for (int i = 0; i < rows.size(); ++i) {
        const QStringList parts = rows[i].split(QLatin1Char('\t'));
        table->setItem(i, 0, new QTableWidgetItem(parts.value(0)));
        table->setItem(i, 1, new QTableWidgetItem(parts.value(1)));
    }
    table->resizeColumnsToContents();
    return table;
}

} // namespace

FileInfoDialog::FileInfoDialog(const QStringList& exifRows, const QStringList& xmpRows,
                               const QStringList& iptcEditFields, const QStringList& iptcOtherRows,
                               const QString& xmp, QWidget* parent)
    : QDialog(parent), exifRows_(exifRows), xmpRows_(xmpRows), xmp_(xmp)
{
    // The read-only list holds only records outside the editable core set,
    // matched by field label.
    QStringList editableLabels;
    for (const QString& row : iptcEditFields) {
        editableLabels.append(row.section(QLatin1Char('\t'), 1, 1));
    }
    for (const QString& row : iptcOtherRows) {
        if (!editableLabels.contains(row.section(QLatin1Char('\t'), 0, 0))) {
            iptcOtherRows_.append(row);
        }
    }

    setObjectName(QStringLiteral("fileInfoDialog"));
    setWindowTitle(tr("File Info"));
    setMinimumSize(560, 360);

    categoryList_ = new QListWidget(this);
    categoryList_->setObjectName(QStringLiteral("fileInfoCategories"));
    categoryList_->setFixedWidth(150);
    for (const QString& name : {kCameraData, kDescription, kIptc, kRawData}) {
        new QListWidgetItem(name, categoryList_);
    }

    stack_ = new QStackedWidget(this);
    stack_->setObjectName(QStringLiteral("fileInfoStack"));
    stack_->addWidget(makeRowTable(exifRows_, stack_));
    stack_->addWidget(makeRowTable(xmpRows_, stack_));
    stack_->addWidget(buildIptcPage(iptcEditFields, iptcOtherRows_));
    xmpEdit_ = new QPlainTextEdit(xmp_, stack_);
    xmpEdit_->setObjectName(QStringLiteral("fileInfoXmp"));
    xmpEdit_->setReadOnly(true);
    stack_->addWidget(xmpEdit_);

    connect(categoryList_, &QListWidget::currentRowChanged, stack_,
            &QStackedWidget::setCurrentIndex);
    categoryList_->setCurrentRow(0);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &FileInfoDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &FileInfoDialog::reject);

    auto* exportButton = new QPushButton(tr("Export Template…"), this);
    exportButton->setObjectName(QStringLiteral("fileInfoExportTemplate"));
    auto* applyButton = new QPushButton(tr("Apply Template…"), this);
    applyButton->setObjectName(QStringLiteral("fileInfoApplyTemplate"));
    templateMode_ = new QComboBox(this);
    templateMode_->setObjectName(QStringLiteral("fileInfoTemplateMode"));
    templateMode_->addItems({tr("Append"), tr("Replace"), tr("Keep Original")});
    connect(exportButton, &QPushButton::clicked, this, [this] {
        if (onExportTemplate) {
            onExportTemplate();
        }
    });
    connect(applyButton, &QPushButton::clicked, this, [this] {
        if (onApplyTemplate) {
            onApplyTemplate(templateMode());
        }
    });

    auto* templateRow = new QHBoxLayout();
    templateRow->addWidget(exportButton);
    templateRow->addWidget(applyButton);
    templateRow->addWidget(new QLabel(tr("Merge Mode"), this));
    templateRow->addWidget(templateMode_);
    templateRow->addStretch(1);

    auto* top = new QHBoxLayout();
    top->addWidget(categoryList_);
    top->addWidget(stack_, 1);
    auto* outer = new QVBoxLayout(this);
    outer->addLayout(top, 1);
    outer->addLayout(templateRow);
    outer->addWidget(buttons);
}

QWidget* FileInfoDialog::buildIptcPage(const QStringList& editFields, const QStringList& otherRows)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QVBoxLayout(page);
    auto* form = new QFormLayout();
    for (const QString& row : editFields) {
        const QStringList parts = row.split(QLatin1Char('\t'));
        const QString id = parts.value(0);
        auto* edit = new QLineEdit(parts.value(2), page);
        edit->setObjectName(QStringLiteral("fileInfoIptc_") + id);
        fieldIds_.append(id);
        fields_.insert(id, edit);
        initial_.insert(id, parts.value(2));
        form->addRow(parts.value(1), edit);
    }
    layout->addLayout(form);
    if (!otherRows.isEmpty()) {
        layout->addWidget(new QLabel(tr("Other records"), page));
        layout->addWidget(makeRowTable(otherRows, page), 1);
    }
    layout->addStretch(0);
    return page;
}

QStringList FileInfoDialog::edits() const
{
    // Only changed fields are emitted, so an untouched (possibly non-UTF-8)
    // value is never rewritten.
    QStringList out;
    for (const QString& id : fieldIds_) {
        const QString value = fields_.value(id)->text();
        if (value != initial_.value(id)) {
            out.append(id + QLatin1Char('\t') + value);
        }
    }
    return out;
}

QStringList FileInfoDialog::categoriesForTest() const
{
    QStringList out;
    for (int i = 0; i < categoryList_->count(); ++i) {
        out.append(categoryList_->item(i)->text());
    }
    return out;
}

QStringList FileInfoDialog::rowsForTest(const QString& category) const
{
    if (category == kCameraData) {
        return exifRows_;
    }
    if (category == kDescription) {
        return xmpRows_;
    }
    if (category == kIptc) {
        return iptcOtherRows_;
    }
    return {};
}

QString FileInfoDialog::xmpForTest() const
{
    return xmpEdit_->toPlainText();
}

QString FileInfoDialog::fieldForTest(const QString& id) const
{
    QLineEdit* edit = fields_.value(id);
    return edit ? edit->text() : QString();
}

void FileInfoDialog::setFieldForTest(const QString& id, const QString& value)
{
    if (QLineEdit* edit = fields_.value(id)) {
        edit->setText(value);
    }
}

int FileInfoDialog::templateMode() const
{
    return templateMode_ ? templateMode_->currentIndex() : 0;
}

bool FileInfoDialog::hasTemplateControlsForTest() const
{
    return templateMode_ != nullptr
        && findChild<QPushButton*>(QStringLiteral("fileInfoExportTemplate"))
        && findChild<QPushButton*>(QStringLiteral("fileInfoApplyTemplate"));
}

int FileInfoDialog::mergeModeForTest() const
{
    return templateMode();
}

void FileInfoDialog::setMergeModeForTest(int mode)
{
    if (templateMode_) {
        templateMode_->setCurrentIndex(mode);
    }
}

} // namespace pictura

#include "moc_file_info_dialog.cpp"
