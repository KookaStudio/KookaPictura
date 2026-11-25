// The Performance page of Preferences (#636,
// docs/10-workflow-io/scratch-disks-and-memory.md): Memory Usage, History &
// Cache, Scratch Disks and the graphics processor.

#include "preferences_dialog.h"

#include <QtCore/QDir>
#include <QtCore/QLocale>
#include <QtCore/QSignalBlocker>
#include <QtCore/QStandardPaths>
#include <QtCore/QStorageInfo>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>

namespace pictura {

namespace {

constexpr int kMaxScratchVolumes = 4;

// The engine's default volume (`pictura_core::scratch::default_volume`).
QString defaultScratchVolume()
{
    return QStandardPaths::writableLocation(QStandardPaths::GenericCacheLocation)
           + QStringLiteral("/kooka-pictura/scratch");
}

// `path`'s free space, and a tmpfs warning; a volume not made yet reads the
// filesystem of its nearest existing ancestor.
QString scratchVolumeText(const QString& path)
{
    QDir dir(path);
    while (!dir.exists() && !dir.isRoot() && dir.cdUp()) {
    }
    const QStorageInfo storage(dir.absolutePath());
    QString text = path;
    if (storage.isValid()) {
        text += QStringLiteral(" — ")
                + PreferencesDialog::tr("%1 free")
                      .arg(QLocale().formattedDataSize(storage.bytesAvailable()));
        if (storage.fileSystemType() == "tmpfs") {
            text += PreferencesDialog::tr(" (tmpfs: uses RAM)");
        }
    }
    return text;
}

} // namespace

QWidget* PreferencesDialog::buildPerformancePage()
{
    auto* page = new QWidget(this);
    page->setObjectName(QStringLiteral("preferencesPerformancePage"));
    auto* layout = new QVBoxLayout(page);

    // Memory Usage (#1211): the share of the host's memory the governor's
    // ceiling allows. CS6 asks for a restart; ours applies at once.
    auto* memory = new QGroupBox(tr("Memory Usage"), page);
    auto* memoryLayout = new QVBoxLayout(memory);
    auto* memoryRow = new QHBoxLayout;
    memoryRow->addWidget(new QLabel(tr("Memory Usage:"), memory));
    memoryUsage_ = new QSpinBox(memory);
    memoryUsage_->setObjectName(QStringLiteral("preferencesMemoryUsage"));
    memoryUsage_->setRange(5, 100);
    memoryUsage_->setSuffix(QStringLiteral(" %"));
    memoryUsage_->setValue(60);
    memoryRow->addWidget(memoryUsage_);
    memorySlider_ = new QSlider(Qt::Horizontal, memory);
    memorySlider_->setObjectName(QStringLiteral("preferencesMemorySlider"));
    memorySlider_->setRange(5, 100);
    memorySlider_->setValue(60);
    memoryRow->addWidget(memorySlider_, 1);
    memoryLayout->addLayout(memoryRow);
    memoryCeiling_ = new QLabel(memory);
    memoryCeiling_->setObjectName(QStringLiteral("preferencesMemoryCeiling"));
    memoryLayout->addWidget(memoryCeiling_);
    setMemoryCeiling(-1);
    layout->addWidget(memory);
    connect(memoryUsage_, &QSpinBox::valueChanged, this, [this](int percent) {
        const QSignalBlocker block(memorySlider_);
        memorySlider_->setValue(percent);
        emit memoryUsagePercentChanged(percent);
    });
    connect(memorySlider_, &QSlider::valueChanged, memoryUsage_, &QSpinBox::setValue);

    auto* cache = new QGroupBox(tr("History && Cache"), page);
    auto* cacheLayout = new QFormLayout(cache);
    auto* presets = new QHBoxLayout;
    // CS6 picks each preset's values from RAM and CPU count; ours are fixed
    // (approximated) and set Cache Levels only, since the tile is fixed.
    const struct {
        const char* name;
        QString label;
        int levels;
    } presetButtons[] = {
        {"preferencesPresetTallThin", tr("Tall and Thin"), 4},
        {"preferencesPresetDefault", tr("Default"), 8},
        {"preferencesPresetBigFlat", tr("Big and Flat"), 8},
    };
    for (const auto& preset : presetButtons) {
        auto* button = new QPushButton(preset.label, cache);
        button->setObjectName(QString::fromLatin1(preset.name));
        const int levels = preset.levels;
        connect(button, &QPushButton::clicked, this, [this, levels]() {
            cacheLevels_->setValue(levels);
        });
        presets->addWidget(button);
    }
    cacheLayout->addRow(new QLabel(tr("Optimize Cache Levels for Documents that are:"), cache));
    cacheLayout->addRow(presets);
    historyStates_ = new QSpinBox(cache);
    historyStates_->setObjectName(QStringLiteral("preferencesHistoryStates"));
    historyStates_->setRange(1, 1000);
    historyStates_->setValue(20);
    cacheLayout->addRow(tr("History States:"), historyStates_);
    connect(historyStates_, &QSpinBox::valueChanged, this, &PreferencesDialog::historyStatesChanged);
    cacheLevels_ = new QSpinBox(cache);
    cacheLevels_->setObjectName(QStringLiteral("preferencesCacheLevels"));
    cacheLevels_->setRange(2, 8);
    cacheLevels_->setValue(8);
    cacheLayout->addRow(tr("Cache Levels:"), cacheLevels_);
    connect(cacheLevels_, &QSpinBox::valueChanged, this, &PreferencesDialog::cacheLevelsChanged);
    auto* tileSize = new QComboBox(cache);
    tileSize->setObjectName(QStringLiteral("preferencesCacheTileSize"));
    tileSize->addItem(tr("256 px (256 KB)"));
    tileSize->setEnabled(false);
    tileSize->setToolTip(tr("Kooka Pictura's tiles are a fixed 256 × 256 pixels."));
    cacheLayout->addRow(tr("Cache Tile Size:"), tileSize);
    layout->addWidget(cache);

    auto* scratch = new QGroupBox(tr("Scratch Disks"), page);
    auto* scratchLayout = new QHBoxLayout(scratch);
    scratchList_ = new QListWidget(scratch);
    scratchList_->setObjectName(QStringLiteral("preferencesScratchVolumes"));
    scratchLayout->addWidget(scratchList_, 1);
    auto* scratchButtons = new QVBoxLayout;
    const auto scratchButton = [&](const char* name, const QString& label) {
        auto* button = new QPushButton(label, scratch);
        button->setObjectName(QString::fromLatin1(name));
        scratchButtons->addWidget(button);
        return button;
    };
    scratchAdd_ = scratchButton("preferencesScratchAdd", tr("Add…"));
    scratchRemove_ = scratchButton("preferencesScratchRemove", tr("Remove"));
    scratchUp_ = scratchButton("preferencesScratchUp", tr("Move Up"));
    scratchDown_ = scratchButton("preferencesScratchDown", tr("Move Down"));
    scratchButtons->addStretch(1);
    scratchLayout->addLayout(scratchButtons);
    layout->addWidget(scratch);
    connect(scratchAdd_, &QPushButton::clicked, this, [this]() {
        const QString dir = QFileDialog::getExistingDirectory(this, tr("Choose Scratch Disk"));
        if (!dir.isEmpty()) {
            addScratchVolume(dir);
        }
    });
    connect(scratchRemove_, &QPushButton::clicked, this, [this]() {
        const int row = scratchList_->currentRow();
        if (row >= 0 && row < scratchVolumes_.size()) {
            scratchVolumes_.removeAt(row);
            showScratchVolumes();
            emit scratchVolumesChanged(scratchVolumes_);
        }
    });
    connect(scratchUp_, &QPushButton::clicked, this, [this]() { moveScratchVolume(-1); });
    connect(scratchDown_, &QPushButton::clicked, this, [this]() { moveScratchVolume(1); });
    connect(scratchList_, &QListWidget::currentRowChanged, this, [this]() {
        showScratchVolumes();
    });
    showScratchVolumes();

    auto* gpu = new QGroupBox(tr("Graphics Processor Settings"), page);
    gpu->setLayout(new QVBoxLayout);
    gpuAdapter_ = new QLabel(gpu);
    gpuAdapter_->setObjectName(QStringLiteral("preferencesGpuAdapter"));
    gpuAdapter_->setTextFormat(Qt::PlainText);
    gpu->layout()->addWidget(gpuAdapter_);
    setGpuAdapter(QString());
    makeCheckbox(QStringLiteral("useGpuCompute"), tr("Use GPU Compute"), gpu);
    layout->addWidget(gpu);
    layout->addStretch(1);
    return page;
}

void PreferencesDialog::setMemoryUsagePercent(int percent)
{
    const QSignalBlocker block(memoryUsage_);
    const QSignalBlocker blockSlider(memorySlider_);
    memoryUsage_->setValue(percent);
    memorySlider_->setValue(percent);
}

void PreferencesDialog::setMemoryCeiling(qint64 bytes)
{
    memoryCeiling_->setText(
        bytes < 0 ? tr("Let Kooka Pictura Use: —")
                  : tr("Let Kooka Pictura Use: %1 MB").arg(QLocale().toString(bytes >> 20)));
}

void PreferencesDialog::setHistoryStates(int states)
{
    const QSignalBlocker block(historyStates_);
    historyStates_->setValue(states);
}

void PreferencesDialog::setCacheLevels(int levels)
{
    const QSignalBlocker block(cacheLevels_);
    cacheLevels_->setValue(levels);
}

void PreferencesDialog::setScratchVolumes(const QStringList& volumes)
{
    scratchVolumes_ = volumes.mid(0, kMaxScratchVolumes);
    showScratchVolumes();
}

void PreferencesDialog::addScratchVolume(const QString& path)
{
    const QString clean = QDir::cleanPath(path);
    if (clean.isEmpty() || scratchVolumes_.contains(clean)
        || scratchVolumes_.size() >= kMaxScratchVolumes) {
        return;
    }
    // The default stays first: Add adds a volume, it never replaces one.
    const QString fallback = QDir::cleanPath(defaultScratchVolume());
    if (scratchVolumes_.isEmpty() && clean != fallback) {
        scratchVolumes_.append(fallback);
    }
    scratchVolumes_.append(clean);
    showScratchVolumes();
    scratchList_->setCurrentRow(int(scratchVolumes_.size()) - 1);
    emit scratchVolumesChanged(scratchVolumes_);
}

void PreferencesDialog::moveScratchVolume(int delta)
{
    const int row = scratchList_->currentRow();
    const int to = row + delta;
    if (row < 0 || row >= scratchVolumes_.size() || to < 0 || to >= scratchVolumes_.size()) {
        return;
    }
    scratchVolumes_.move(row, to);
    showScratchVolumes();
    scratchList_->setCurrentRow(to);
    emit scratchVolumesChanged(scratchVolumes_);
}

// Rebuild the rows only when the volumes changed, so a selection change
// just refreshes the buttons.
void PreferencesDialog::showScratchVolumes()
{
    QStringList shown;
    for (int i = 0; i < scratchList_->count(); ++i) {
        shown.append(scratchList_->item(i)->data(Qt::UserRole).toString());
    }
    if (shown != scratchVolumes_ || scratchVolumes_.isEmpty()) {
        const QSignalBlocker block(scratchList_);
        const int row = scratchList_->currentRow();
        scratchList_->clear();
        for (const QString& volume : scratchVolumes_) {
            auto* item = new QListWidgetItem(scratchVolumeText(volume), scratchList_);
            item->setData(Qt::UserRole, volume);
        }
        if (scratchVolumes_.isEmpty()) {
            auto* item = new QListWidgetItem(
                tr("Default: %1").arg(scratchVolumeText(defaultScratchVolume())), scratchList_);
            item->setFlags(Qt::NoItemFlags);
        } else {
            scratchList_->setCurrentRow(std::min(row, int(scratchVolumes_.size()) - 1));
        }
    }
    const int row = scratchVolumes_.isEmpty() ? -1 : scratchList_->currentRow();
    scratchAdd_->setEnabled(scratchVolumes_.size() < kMaxScratchVolumes);
    scratchRemove_->setEnabled(row >= 0);
    scratchUp_->setEnabled(row > 0);
    scratchDown_->setEnabled(row >= 0 && row + 1 < scratchVolumes_.size());
}

void PreferencesDialog::setGpuAdapter(const QString& name)
{
    gpuAdapter_->setText(name.isEmpty()
                             ? tr("No graphics processor found")
                             : tr("Detected Graphics Processor: %1").arg(name));
}

} // namespace pictura
