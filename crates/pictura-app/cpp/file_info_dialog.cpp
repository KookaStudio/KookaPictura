#include "file_info_dialog.h"

#include <QtCore/QSet>
#include <QtWidgets/QAbstractItemView>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QHeaderView>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPlainTextEdit>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QTableWidget>
#include <QtWidgets/QTableWidgetItem>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QWidget>

namespace pictura {

namespace {

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

bool isListId(const QString& id)
{
    return id == QStringLiteral("2:25")
        || id == QStringLiteral("Iptc4xmpExt:PersonInImage")
        || id == QStringLiteral("Iptc4xmpExt:OrganisationInImageName")
        || id == QStringLiteral("Iptc4xmpExt:OrganisationInImageCode");
}

} // namespace

FileInfoDialog::FileInfoDialog(const QStringList& readOnlyRows, const QStringList& editFields,
                               const QString& xmp, QWidget* parent)
    : QDialog(parent), readOnlyRows_(readOnlyRows), editFields_(editFields), xmp_(xmp)
{
    for (const QString& row : readOnlyRows_) {
        const QStringList parts = row.split(QLatin1Char('\t'));
        if (parts.size() >= 3) {
            rowsBySection_[parts[0]].append(parts[1] + QLatin1Char('\t') + parts.mid(2).join(QLatin1Char('\t')));
        }
    }
    for (const QString& row : editFields_) {
        const QStringList parts = row.split(QLatin1Char('\t'));
        if (parts.size() < 4) {
            continue;
        }
        const QString section = parts[0];
        const QString id = parts[1];
        const QString label = parts[2];
        const QString value = parts.mid(3).join(QLatin1Char('\t'));
        fieldsBySection_[section].append(id + QLatin1Char('\t') + label + QLatin1Char('\t') + value);
        if (!initial_.contains(id)) {
            initial_.insert(id, value);
        }
        if (isListId(id) && !value.isEmpty()) {
            initialLists_[id].append(value);
        }
    }

    setObjectName(QStringLiteral("fileInfoDialog"));
    setWindowTitle(tr("File Info"));
    setMinimumSize(640, 420);

    categories_ = {tr("Description"), tr("Camera Data"), tr("IPTC"), tr("IPTC Extension"),
                   tr("GPS Data"), tr("Origin"), tr("Photoshop"), tr("Raw Data")};

    categoryList_ = new QListWidget(this);
    categoryList_->setObjectName(QStringLiteral("fileInfoCategories"));
    categoryList_->setFixedWidth(160);

    stack_ = new QStackedWidget(this);
    stack_->setObjectName(QStringLiteral("fileInfoStack"));
    for (const QString& category : categories_) {
        new QListWidgetItem(category, categoryList_);
        if (category == tr("Raw Data")) {
            stack_->addWidget(buildRawPage());
        } else if (fieldsBySection_.contains(category)) {
            stack_->addWidget(buildEditablePage(category));
        } else if (rowsBySection_.contains(category)) {
            stack_->addWidget(buildTablePage(category));
        } else {
            auto* page = new QWidget(stack_);
            stack_->addWidget(page);
        }
    }

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
    templateMode_->setToolTip(tr("Replace clears managed fields the template omits."));
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

QWidget* FileInfoDialog::buildEditablePage(const QString& section)
{
    auto* page = new QWidget(stack_);
    auto* outer = new QVBoxLayout(page);
    auto* scroll = new QScrollArea(page);
    scroll->setWidgetResizable(true);
    auto* host = new QWidget(scroll);
    auto* form = new QFormLayout(host);

    QSet<QString> seen;
    for (const QString& row : fieldsBySection_.value(section)) {
        const QStringList parts = row.split(QLatin1Char('\t'));
        const QString id = parts.value(0);
        if (seen.contains(id)) {
            continue;
        }
        seen.insert(id);
        const QString label = parts.value(1);
        QStringList values;
        for (const QString& other : fieldsBySection_.value(section)) {
            const QStringList otherParts = other.split(QLatin1Char('\t'));
            if (otherParts.value(0) == id && !otherParts.value(2).isEmpty()) {
                values.append(otherParts.value(2));
            }
        }
        if (isListId(id)) {
            form->addRow(label, buildListField(id, values, host));
        } else if (id == QStringLiteral("xmpRights:Marked")) {
            auto* combo = new QComboBox(host);
            combo->setObjectName(QStringLiteral("fileInfoCombo_") + id);
            combo->addItems({tr("Unknown"), tr("Copyrighted"), tr("Public Domain")});
            const QString current = initial_.value(id);
            combo->setCurrentIndex(current == QStringLiteral("Copyrighted") ? 1
                                   : current == QStringLiteral("Public Domain") ? 2
                                                                                : 0);
            combos_.insert(id, combo);
            form->addRow(label, combo);
        } else {
            auto* edit = new QLineEdit(values.value(0), host);
            edit->setObjectName(QStringLiteral("fileInfoField_") + id);
            fields_[id].append(edit);
            form->addRow(label, edit);
            connect(edit, &QLineEdit::textChanged, this, [this, id](const QString& text) {
                if (updating_) {
                    return;
                }
                updating_ = true;
                for (QLineEdit* peer : fields_.value(id)) {
                    if (peer->text() != text) {
                        peer->setText(text);
                    }
                }
                updating_ = false;
            });
        }
    }

    scroll->setWidget(host);
    outer->addWidget(scroll, 1);
    if (rowsBySection_.contains(section) && !rowsBySection_.value(section).isEmpty()) {
        outer->addWidget(new QLabel(tr("Other records"), page));
        outer->addWidget(makeRowTable(rowsBySection_.value(section), page), 1);
    }
    return page;
}

QWidget* FileInfoDialog::buildTablePage(const QString& section)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QVBoxLayout(page);
    layout->addWidget(makeRowTable(rowsBySection_.value(section), page), 1);
    return page;
}

QWidget* FileInfoDialog::buildRawPage()
{
    auto* page = new QWidget(stack_);
    auto* layout = new QVBoxLayout(page);
    xmpEdit_ = new QPlainTextEdit(xmp_, page);
    xmpEdit_->setObjectName(QStringLiteral("fileInfoXmp"));
    xmpEdit_->setReadOnly(false);
    layout->addWidget(xmpEdit_, 1);
    return page;
}

QWidget* FileInfoDialog::buildListField(const QString& id, const QStringList& values, QWidget* parent)
{
    auto* host = new QWidget(parent);
    auto* layout = new QVBoxLayout(host);
    layout->setContentsMargins(0, 0, 0, 0);
    auto* list = new QListWidget(host);
    list->setObjectName(QStringLiteral("fileInfoList_") + id);
    list->addItems(values);
    list->setMaximumHeight(80);
    lists_.insert(id, list);

    auto* row = new QHBoxLayout();
    auto* input = new QLineEdit(host);
    auto* add = new QPushButton(tr("Add"), host);
    auto* remove = new QPushButton(tr("Remove"), host);
    connect(add, &QPushButton::clicked, list, [list, input] {
        if (!input->text().isEmpty()) {
            list->addItem(input->text());
            input->clear();
        }
    });
    connect(remove, &QPushButton::clicked, list, [list] {
        delete list->takeItem(list->currentRow());
    });
    row->addWidget(input, 1);
    row->addWidget(add);
    row->addWidget(remove);
    layout->addWidget(list);
    layout->addLayout(row);
    return host;
}

QStringList FileInfoDialog::edits() const
{
    QStringList out;
    for (auto it = fields_.cbegin(); it != fields_.cend(); ++it) {
        const QString id = it.key();
        QString value;
        if (combos_.contains(id)) {
            value = comboValue(id);
            const QString initial = initial_.value(id);
            const QString normalized = initial.isEmpty() ? QStringLiteral("Unknown") : initial;
            if (value != normalized) {
                out.append(id + QLatin1Char('\t') + value);
            }
            continue;
        }
        if (it.value().isEmpty()) {
            continue;
        }
        value = it.value().first()->text();
        if (value != initial_.value(id)) {
            out.append(id + QLatin1Char('\t') + value);
        }
    }
    for (auto it = combos_.cbegin(); it != combos_.cend(); ++it) {
        const QString id = it.key();
        const QString value = comboValue(id);
        const QString initial = initial_.value(id);
        const QString normalized = initial.isEmpty() ? QStringLiteral("Unknown") : initial;
        if (value != normalized) {
            out.append(id + QLatin1Char('\t') + value);
        }
    }
    for (auto it = lists_.cbegin(); it != lists_.cend(); ++it) {
        QStringList values;
        QListWidget* list = it.value();
        for (int i = 0; i < list->count(); ++i) {
            values.append(list->item(i)->text());
        }
        if (values == initialLists_.value(it.key())) {
            continue;
        }
        if (values.isEmpty()) {
            out.append(it.key() + QLatin1Char('\t'));
        } else {
            for (const QString& value : values) {
                out.append(it.key() + QLatin1Char('\t') + value);
            }
        }
    }
    if (xmpEdit_ && xmpEdit_->toPlainText() != xmp_) {
        out.append(QStringLiteral("xmp-packet\t") + xmpEdit_->toPlainText());
    }
    return out;
}

void FileInfoDialog::accept()
{
    if (!onApply) {
        QDialog::accept();
        return;
    }
    const QStringList rows = edits();
    if (rows.isEmpty()) {
        QDialog::accept();
        return;
    }
    if (onApply(rows)) {
        QDialog::accept();
    } else {
        auto* warning = new QMessageBox(QMessageBox::Warning, tr("File Info"),
                                        tr("The metadata could not be written."),
                                        QMessageBox::Ok, this);
        warning->setAttribute(Qt::WA_DeleteOnClose);
        warning->open();
    }
}

QString FileInfoDialog::comboValue(const QString& id) const
{
    const QComboBox* combo = combos_.value(id);
    if (!combo) {
        return {};
    }
    switch (combo->currentIndex()) {
    case 1:
        return QStringLiteral("Copyrighted");
    case 2:
        return QStringLiteral("Public Domain");
    default:
        return QStringLiteral("Unknown");
    }
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
    return rowsBySection_.value(category);
}

QString FileInfoDialog::xmpForTest() const
{
    return xmpEdit_ ? xmpEdit_->toPlainText() : QString();
}

QString FileInfoDialog::fieldForTest(const QString& id) const
{
    if (combos_.contains(id)) {
        return comboValue(id);
    }
    if (lists_.contains(id)) {
        QListWidget* list = lists_.value(id);
        return list->count() > 0 ? list->item(0)->text() : QString();
    }
    const QList<QLineEdit*> edits = fields_.value(id);
    return edits.isEmpty() ? QString() : edits.first()->text();
}

void FileInfoDialog::setFieldForTest(const QString& id, const QString& value)
{
    if (combos_.contains(id)) {
        QComboBox* combo = combos_.value(id);
        const int index = value == QStringLiteral("Copyrighted") ? 1
            : value == QStringLiteral("Public Domain") ? 2
                                                       : 0;
        combo->setCurrentIndex(index);
        return;
    }
    if (lists_.contains(id)) {
        QListWidget* list = lists_.value(id);
        list->clear();
        if (!value.isEmpty()) {
            list->addItem(value);
        }
        return;
    }
    const QList<QLineEdit*> edits = fields_.value(id);
    if (!edits.isEmpty()) {
        edits.first()->setText(value);
    }
}

void FileInfoDialog::addListValueForTest(const QString& id, const QString& value)
{
    if (QListWidget* list = lists_.value(id)) {
        list->addItem(value);
    }
}

QStringList FileInfoDialog::listValuesForTest(const QString& id) const
{
    QStringList out;
    QListWidget* list = lists_.value(id);
    if (!list) {
        return out;
    }
    for (int i = 0; i < list->count(); ++i) {
        out.append(list->item(i)->text());
    }
    return out;
}

void FileInfoDialog::setXmpForTest(const QString& packet)
{
    if (xmpEdit_) {
        xmpEdit_->setPlainText(packet);
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
