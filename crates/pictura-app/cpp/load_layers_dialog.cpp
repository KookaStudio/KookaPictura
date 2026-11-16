#include "load_layers_dialog.h"

#include "dialogs.h"

#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QListWidgetItem>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

// The sources the stack loader can decode: the raster formats Qt reads, WebP,
// and the native PSD/PSB documents.
QStringList supportedSuffixes()
{
    return {QStringLiteral("psd"),  QStringLiteral("psb"), QStringLiteral("png"),
            QStringLiteral("jpg"),  QStringLiteral("jpeg"), QStringLiteral("jpe"),
            QStringLiteral("gif"),  QStringLiteral("bmp"), QStringLiteral("tif"),
            QStringLiteral("tiff"), QStringLiteral("webp")};
}

QStringList stackFileFilters()
{
    return {QStringLiteral("All Supported (*.PSD *.PSB *.PNG *.JPG *.JPEG *.JPE *.GIF "
                           "*.BMP *.TIF *.TIFF *.WEBP)"),
            QStringLiteral("Photoshop (*.PSD *.PSB)"),
            QStringLiteral("Images (*.PNG *.JPG *.JPEG *.JPE *.GIF *.BMP *.TIF *.TIFF *.WEBP)"),
            QStringLiteral("All files (*)")};
}

} // namespace

LoadLayersDialog::LoadLayersDialog(const QStringList& openPaths, const QString& startFolder,
                                   QWidget* parent)
    : QDialog(parent)
    , openPaths_(openPaths)
    , startFolder_(startFolder)
{
    setObjectName(QStringLiteral("loadLayersDialog"));
    setWindowTitle(tr("Load Layers"));
    auto* layout = new QVBoxLayout(this);

    auto* actions = new QHBoxLayout;
    actions->addStretch(1);
    auto* cancel = new QPushButton(tr("Cancel"), this);
    cancel->setObjectName(QStringLiteral("loadLayersCancel"));
    ok_ = new QPushButton(tr("OK"), this);
    ok_->setObjectName(QStringLiteral("loadLayersOk"));
    ok_->setDefault(true);
    actions->addWidget(ok_);
    actions->addWidget(cancel);
    layout->addLayout(actions);

    auto* group = new QGroupBox(tr("Source Files"), this);
    group->setObjectName(QStringLiteral("loadLayersSourceGroup"));
    auto* groupLayout = new QVBoxLayout(group);
    auto* hint = new QLabel(tr("Choose two or more files to load into an image stack."), group);
    hint->setWordWrap(true);
    groupLayout->addWidget(hint);

    auto* useRow = new QHBoxLayout;
    useRow->addWidget(new QLabel(tr("Use:"), group));
    use_ = new QComboBox(group);
    use_->setObjectName(QStringLiteral("loadLayersUse"));
    use_->addItem(tr("Files"), QStringLiteral("files"));
    use_->addItem(tr("Folder"), QStringLiteral("folder"));
    useRow->addWidget(use_, 1);
    groupLayout->addLayout(useRow);

    auto* listRow = new QHBoxLayout;
    files_ = new QListWidget(group);
    files_->setObjectName(QStringLiteral("loadLayersFiles"));
    files_->setSelectionMode(QAbstractItemView::ExtendedSelection);
    files_->setMinimumHeight(160);
    listRow->addWidget(files_, 1);

    auto* buttons = new QVBoxLayout;
    browse_ = new QPushButton(tr("Browse…"), group);
    browse_->setObjectName(QStringLiteral("loadLayersBrowse"));
    remove_ = new QPushButton(tr("Remove"), group);
    remove_->setObjectName(QStringLiteral("loadLayersRemove"));
    addOpen_ = new QPushButton(tr("Add Open Files"), group);
    addOpen_->setObjectName(QStringLiteral("loadLayersAddOpen"));
    buttons->addWidget(browse_);
    buttons->addWidget(remove_);
    buttons->addWidget(addOpen_);
    buttons->addStretch(1);
    listRow->addLayout(buttons);
    groupLayout->addLayout(listRow);

    align_ = new QCheckBox(tr("Attempt to Automatically Align Source Images"), group);
    align_->setObjectName(QStringLiteral("loadLayersAlign"));
    groupLayout->addWidget(align_);
    smartObject_ = new QCheckBox(tr("Create Smart Object after Loading Layers"), group);
    smartObject_->setObjectName(QStringLiteral("loadLayersSmartObject"));
    groupLayout->addWidget(smartObject_);

    layout->addWidget(group);

    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);
    connect(ok_, &QPushButton::clicked, this, &QDialog::accept);
    connect(browse_, &QPushButton::clicked, this, &LoadLayersDialog::browse);
    connect(remove_, &QPushButton::clicked, this, &LoadLayersDialog::removeSelected);
    connect(addOpen_, &QPushButton::clicked, this, &LoadLayersDialog::addOpenFiles);
    connect(use_, &QComboBox::currentIndexChanged, this, &LoadLayersDialog::refreshMode);
    connect(files_, &QListWidget::itemSelectionChanged, this, &LoadLayersDialog::refreshButtons);
    refreshButtons();
}

QStringList LoadLayersDialog::paths() const
{
    QStringList paths;
    paths.reserve(files_->count());
    for (int row = 0; row < files_->count(); ++row) {
        paths.append(files_->item(row)->data(Qt::UserRole).toString());
    }
    return paths;
}

bool LoadLayersDialog::alignImages() const
{
    return align_->isChecked();
}

bool LoadLayersDialog::createSmartObject() const
{
    return smartObject_->isChecked();
}

void LoadLayersDialog::addFiles(const QStringList& paths)
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

void LoadLayersDialog::setFolder(const QString& folder)
{
    startFolder_ = folder;
    files_->clear();
    const QDir dir(folder);
    QStringList filters;
    for (const QString& suffix : supportedSuffixes()) {
        filters.append(QStringLiteral("*.") + suffix);
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

void LoadLayersDialog::browse()
{
    if (use_->currentData().toString() == QLatin1String("folder")) {
        const QString folder = QFileDialog::getExistingDirectory(
            this, tr("Choose a Folder"), startFolder_);
        if (!folder.isEmpty()) {
            setFolder(folder);
        }
        return;
    }
    const QStringList chosen = getOpenFileNames(
        this, tr("Load Files into Stack"), startFolder_, stackFileFilters());
    if (!chosen.isEmpty()) {
        startFolder_ = QFileInfo(chosen.first()).absolutePath();
        addFiles(chosen);
    }
}

void LoadLayersDialog::removeSelected()
{
    if (use_->currentData().toString() == QLatin1String("folder")) {
        return;
    }
    const QList<QListWidgetItem*> selected = files_->selectedItems();
    for (QListWidgetItem* item : selected) {
        delete files_->takeItem(files_->row(item));
    }
    refreshButtons();
}

void LoadLayersDialog::addOpenFiles()
{
    if (use_->currentData().toString() == QLatin1String("folder")) {
        return;
    }
    addFiles(openPaths_);
}

void LoadLayersDialog::refreshMode()
{
    files_->clear();
    refreshButtons();
}

void LoadLayersDialog::refreshButtons()
{
    const bool folder = use_->currentData().toString() == QLatin1String("folder");
    ok_->setEnabled(files_->count() > 0);
    remove_->setEnabled(!folder && !files_->selectedItems().isEmpty());
    addOpen_->setEnabled(!folder && !openPaths_.isEmpty());
}

} // namespace pictura
