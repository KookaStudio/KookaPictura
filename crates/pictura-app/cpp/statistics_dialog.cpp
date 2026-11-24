#include "statistics_dialog.h"

#include "dialogs.h"

#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtWidgets/QAbstractItemView>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QListWidgetItem>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

// Adobe's Statistics.jsx order; the data is the existing stack-mode code.
struct StackModeChoice {
    const char* label;
    int code;
};

const StackModeChoice kStackModes[] = {
    {"Mean", 3},
    {"Summation", 5},
    {"Minimum", 2},
    {"Maximum", 1},
    {"Median", 4},
    {"Variance", 7},
    {"Standard Deviation", 8},
    {"Skewness", 9},
    {"Kurtosis", 10},
    {"Range", 6},
    {"Entropy", 11},
};

} // namespace

StatisticsDialog::StatisticsDialog(const QStringList& openPaths, bool hasUnsavedOpenDocuments,
                                   int initialMode, bool initialAlign, const QString& startFolder,
                                   QWidget* parent)
    : QDialog(parent)
    , openPaths_(openPaths)
    , startFolder_(startFolder)
    , hasUnsavedOpenDocuments_(hasUnsavedOpenDocuments)
{
    setObjectName(QStringLiteral("statisticsDialog"));
    setWindowTitle(tr("Image Statistics"));
    auto* layout = new QVBoxLayout(this);

    auto* modeRow = new QHBoxLayout;
    modeRow->addWidget(new QLabel(tr("Choose Stack Mode:"), this));
    stackMode_ = new QComboBox(this);
    stackMode_->setObjectName(QStringLiteral("statisticsStackMode"));
    for (const StackModeChoice& mode : kStackModes) {
        stackMode_->addItem(tr(mode.label), mode.code);
    }
    const int requested = stackMode_->findData(initialMode);
    stackMode_->setCurrentIndex(requested >= 0 ? requested : stackMode_->findData(3));
    modeRow->addWidget(stackMode_, 1);
    layout->addLayout(modeRow);

    auto* group = new QGroupBox(tr("Source Files"), this);
    group->setObjectName(QStringLiteral("statisticsSourceGroup"));
    auto* groupLayout = new QVBoxLayout(group);

    auto* useRow = new QHBoxLayout;
    useRow->addWidget(new QLabel(tr("Use:"), group));
    use_ = new QComboBox(group);
    use_->setObjectName(QStringLiteral("statisticsUse"));
    use_->addItem(tr("Files"), QStringLiteral("files"));
    use_->addItem(tr("Folder"), QStringLiteral("folder"));
    useRow->addWidget(use_, 1);
    groupLayout->addLayout(useRow);

    auto* listRow = new QHBoxLayout;
    files_ = new QListWidget(group);
    files_->setObjectName(QStringLiteral("statisticsFiles"));
    files_->setSelectionMode(QAbstractItemView::ExtendedSelection);
    files_->setMinimumHeight(160);
    listRow->addWidget(files_, 1);

    auto* buttons = new QVBoxLayout;
    browse_ = new QPushButton(tr("Browse…"), group);
    browse_->setObjectName(QStringLiteral("statisticsBrowse"));
    remove_ = new QPushButton(tr("Remove"), group);
    remove_->setObjectName(QStringLiteral("statisticsRemove"));
    addOpen_ = new QPushButton(tr("Add Open Files"), group);
    addOpen_->setObjectName(QStringLiteral("statisticsAddOpen"));
    buttons->addWidget(browse_);
    buttons->addWidget(remove_);
    buttons->addWidget(addOpen_);
    buttons->addStretch(1);
    listRow->addLayout(buttons);
    groupLayout->addLayout(listRow);

    align_ = new QCheckBox(tr("Attempt to Automatically Align Source Images"), group);
    align_->setObjectName(QStringLiteral("statisticsAlign"));
    align_->setChecked(initialAlign);
    groupLayout->addWidget(align_);
    layout->addWidget(group);

    auto* actions = new QHBoxLayout;
    actions->addStretch(1);
    auto* cancel = new QPushButton(tr("Cancel"), this);
    cancel->setObjectName(QStringLiteral("statisticsCancel"));
    ok_ = new QPushButton(tr("OK"), this);
    ok_->setObjectName(QStringLiteral("statisticsOk"));
    ok_->setDefault(true);
    actions->addWidget(ok_);
    actions->addWidget(cancel);
    layout->addLayout(actions);

    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);
    connect(ok_, &QPushButton::clicked, this, &QDialog::accept);
    connect(browse_, &QPushButton::clicked, this, &StatisticsDialog::browse);
    connect(remove_, &QPushButton::clicked, this, &StatisticsDialog::removeSelected);
    connect(addOpen_, &QPushButton::clicked, this, &StatisticsDialog::addOpenFiles);
    connect(files_, &QListWidget::itemSelectionChanged, this, &StatisticsDialog::refreshButtons);
    refreshButtons();
}

QStringList StatisticsDialog::paths() const
{
    QStringList paths;
    paths.reserve(files_->count());
    for (int row = 0; row < files_->count(); ++row) {
        paths.append(files_->item(row)->data(Qt::UserRole).toString());
    }
    return paths;
}

bool StatisticsDialog::alignImages() const
{
    return align_->isChecked();
}

int StatisticsDialog::stackModeCode() const
{
    return stackMode_->currentData().toInt();
}

void StatisticsDialog::addFiles(const QStringList& paths)
{
    for (const QString& path : paths) {
        if (path.isEmpty() || !QFileInfo(path).isFile()) {
            continue;
        }
        bool exists = false;
        for (int row = 0; row < files_->count() && !exists; ++row) {
            exists = files_->item(row)->data(Qt::UserRole).toString() == path;
        }
        if (exists) {
            continue;
        }
        auto* item = new QListWidgetItem(QFileInfo(path).fileName(), files_);
        item->setData(Qt::UserRole, path);
        item->setToolTip(path);
    }
    refreshButtons();
}

void StatisticsDialog::addFolder(const QString& folder)
{
    startFolder_ = folder;
    const QDir dir(folder);
    QStringList filters;
    for (const QString& suffix : stackSourceSuffixes()) {
        filters.append(QStringLiteral("*.") + suffix);
        filters.append(QStringLiteral("*.") + suffix.toUpper());
    }
    const QStringList names =
        dir.entryList(filters, QDir::Files | QDir::Readable, QDir::Name | QDir::IgnoreCase);
    QStringList paths;
    paths.reserve(names.size());
    for (const QString& name : names) {
        paths.append(dir.absoluteFilePath(name));
    }
    addFiles(paths);
}

void StatisticsDialog::addOpenFiles()
{
    if (hasUnsavedOpenDocuments_) {
        QMessageBox::warning(this, tr("Statistics"),
                             tr("Documents must be saved before they can be merged"));
    }
    addFiles(openPaths_);
}

void StatisticsDialog::browse()
{
    if (use_->currentData().toString() == QLatin1String("folder")) {
        const QString folder = QFileDialog::getExistingDirectory(
            this, tr("Choose a Folder"), startFolder_);
        if (!folder.isEmpty()) {
            addFolder(folder);
        }
        return;
    }
    const QStringList chosen = getOpenFileNames(
        this, tr("Statistics"), startFolder_, stackSourceFilters());
    if (!chosen.isEmpty()) {
        startFolder_ = QFileInfo(chosen.first()).absolutePath();
        addFiles(chosen);
    }
}

void StatisticsDialog::removeSelected()
{
    const QList<QListWidgetItem*> selected = files_->selectedItems();
    for (QListWidgetItem* item : selected) {
        delete files_->takeItem(files_->row(item));
    }
    refreshButtons();
}

void StatisticsDialog::refreshButtons()
{
    const bool two = files_->count() >= 2;
    ok_->setEnabled(two);
    align_->setEnabled(two);
    remove_->setEnabled(!files_->selectedItems().isEmpty());
    addOpen_->setEnabled(!openPaths_.isEmpty() || hasUnsavedOpenDocuments_);
}

} // namespace pictura

#include "moc_statistics_dialog.cpp"
